# Architecture

The trusted interface is bundled Svelte. It provides only Journeys, Account and Settings entry points for the integration spike. Rust validates callers and typed destinations. Android owns the provider Activity and platform cryptography. No website parsing adapter or app-operated backend exists.

| Boundary          | Implementation                                                | Available actions                                                                                  |
| ----------------- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Bundled main view | `src/`, `src-tauri/`                                          | Read platform status, open a fixed provider destination, run a public-data encryption probe        |
| Rust coordinator  | `plugins/android-platform/src/lib.rs`                         | Validate local caller, destination enum and policy; dispatch native actions off the async executor |
| Provider website  | Native `ProviderActivity`, newly constructed platform WebView | Website JavaScript and HTTPS resources, restricted navigation; no application bridge               |
| Keystore probe    | `ProbeVault.kt`, background worker                            | AES-GCM test records only; no credential endpoint                                                  |
| Shared policy     | `policy/network-policy.json`, `crates/journey-core`           | Versioned origin and optional-request rules; fast immutable native evaluation                      |

The main window is created programmatically with a navigation guard. Its capability names only `main`, is local-only and grants three plugin commands. Rust checks the calling view label and bundled origin again. A debug build also permits its configured frontend development origin. Production loads bundled assets with a CSP that disallows frames, remote scripts and form submissions. `withGlobalTauri` supplies the documented Tauri API to those bundled assets without adding a separate frontend API dependency. It is never injected into the provider WebView.

`ProviderActivity` is not exported. Native and Rust boundaries independently accept only journeys, login and support destinations. The Activity constructs an ordinary WebView rather than reusing Tauri's bridged WebView. External HTTPS links require a user gesture and a native destination confirmation. Unknown schemes, arbitrary intents, nonstandard ports, ambiguous authorities, certificate errors and automatic external redirects are refused. Uploads, downloads, popups and location are unavailable pending verification.

Back navigates provider history, then closes the Activity. Close returns to the local interface after cookie cleanup completes or fails visibly. Rotation uses Activity configuration handling; recreation after process death closes the old flow rather than restoring or replaying website requests. Insets, IME resizing and predictive Back have source implementations but have not been exercised on Android.

Provider session state is process-local and serialized through opening/open/cleaning/closed states. Cleanup generations prevent an earlier cookie callback from reopening a view after closure. Request hooks deny work while the provider is closed or cleaning. No browser history or transaction is automatically restored. This is a privilege boundary, not a separate WebView process/profile or a claim of incognito operation.

**Open architecture gate:** the Android host, plugin registration, manifest merge, view lifecycle, provider functionality, storage deletion, request interception and real Keystore operations need device validation. `WebStorage.deleteAllData()` has no completion callback and is not proof that every service-worker/IndexedDB/cache artifact is gone. Resolve this before account switching or persistent authentication. See [the Android API](<https://developer.android.com/reference/android/webkit/WebStorage#deleteAllData()>).
