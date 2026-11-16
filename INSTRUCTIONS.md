# Ubian Aggregator — Android application instructions

## 1. Expanded project prompt

Build an unofficial Android client for **https://www.ubian.sk/** using **Rust, Tauri 2, and Svelte with TypeScript**. The application should provide a useful mobile interface for Ubian's browser-accessible services, allow users to sign in to their existing Ubian account, and offer explicit, optional storage of login credentials in an encrypted local vault. Keep core application logic in Rust and use a small Kotlin integration where Android platform APIs are necessary.

Provide a bundled Svelte application for navigation, account controls, saved journeys, privacy settings, and application status. Display Ubian pages in a separate, restricted website view where reproducing the service through a verified integration is impractical. Preserve working website functionality while progressively adding local Svelte views for suitable features. Identify services that require the official Ubian app or another application and provide a clear handoff when available.

Minimize optional tracking through conservative request filtering, restrictive cookie defaults, and permission controls. Add no application analytics or advertising SDKs. Explain what protection is active and where it ends. Prioritize reliable login, secure credential handling, usable journey planning, and safe handling of account and payment flows over superficial feature completeness.

Deliver reproducible source, Android build instructions, focused tests, an installable APK, and a documented feature coverage matrix. Do not represent mocked data or untested authentication as a working Ubian integration.

**Current task boundary:** this document is the first deliverable. Application implementation follows in a separate task. Requirements below describe intended behavior; they are not claims that the application already exists.

## 2. Starting point and decisions

- The workspace currently contains a Nix development environment (`flake.nix`, `flake.lock`, and `.envrc`), without an application scaffold.
- Preserve the existing environment and unrelated files. Extend it when implementing the application instead of replacing it without cause.
- Target Android first. A desktop build may help development but does not demonstrate Android support. iOS is outside the initial scope.
- Use Tauri 2, compatible current stable Rust tooling, Svelte, TypeScript, and Vite. A client-rendered Svelte application is sufficient; no server-side rendering or hosted backend is required.
- Propose Android 8.0 / API 26 as the initial minimum, subject to dependency verification. Record the actual minimum, compile SDK, target SDK, JDK, NDK, and Rust targets used. Verify distribution requirements when preparing a release.
- Start with one active Ubian account. Support multiple transport cards within that account where the website does. Multiple simultaneous accounts are a later enhancement.
- Default the application interface to Slovak, with English translations for the local interface. Preserve provider content and its available language options.
- Use an original app name and icon with a short statement that the client is unofficial. Do not imply Ubian endorsement.
- Application operation should not require a developer-operated proxy, telemetry service, cloud account, or credential relay.

## 3. Research baseline and feature coverage

Public pages were inspected on **2026-11-14**. No authenticated account, payment, private API, or Android device flow was tested. Public descriptions establish discovery targets, not proof that every feature is usable by this client.

The [Ubian homepage](https://www.ubian.sk/) exposes connection search, departure boards, transport filters, maps, recent searches, and favorites. The [login page](https://www.ubian.sk/login) presents email/password login, a remember-login option, Google and Facebook entry points, registration, and password recovery. Preserve these supported paths without assuming that social authentication accepts embedded browsers.

Maintain `docs/feature-coverage.md` during implementation. For each feature, record the observed provider page, chosen integration, privacy-mode behavior, verification date, and status: `verified`, `implemented but unverified`, `external handoff`, `unsupported`, or `deferred`.

| Area | Required behavior | Initial integration direction |
| --- | --- | --- |
| Journey search | Start/destination, swap, date/time, departure/arrival, transport/city filters, results and connection detail. | Provider website first; local interface when a verified adapter is practical. |
| Departures and maps | Stop search, departure detail, available delay/vehicle information, map controls, optional current location. | Preserve browser behavior; load location-dependent content on demand. |
| Favorites and recents | Local favorites and recent searches; expose provider favorites where available. | Local storage first; distinguish local data from account-synced data. |
| Login and account | Sign-in, explicit remember options, expiry, logout, registration/recovery links, account pages. | Isolated provider login and narrowly scoped authentication coordinator. |
| Transport cards | Available card details, balances, validity, discounts, and transaction views. | Discover authenticated pages; never invent unavailable fields or endpoints. |
| Credit and tickets | Access available top-ups, ticket purchase, cart, and purchase status. | Provider checkout; confirm actual web availability per flow. |
| Card/student services | Reach browser-supported application, renewal, or management forms. | Provider pages with system document/photo picker when requested. |
| Support and other services | Preserve access to help and discover additional website services. | Provider navigation or a clearly identified external destination. |

Use the [credit top-up page](https://www.ubian.sk/dobit-kartu) and [ticket purchase page](https://www.ubian.sk/kupit-listok) as purchase discovery starting points. The [privacy notice](https://www.ubian.sk/ochrana-osobnych-udajov) describes account, favorites, card, and transaction data processing; it does not document an integration API.

Treat advertised mobile services separately. The [parking page](https://www.ubian.sk/parkovanie) and [MHD ticket page](https://www.ubian.sk/mhd-listok) describe services associated with the official app. Ubian's [support page](https://www.ubian.sk/pomoc-a-podpora) describes virtual transport cards stored in Google Wallet. Do not promise NFC ticket validation, virtual card provisioning, or official-app-only ticket/parking functions merely because the client displays their information pages.

### Discovery requirements

Before implementing authentication, native data screens, or blocking rules:

1. Inspect relevant public pages and, when an account is available, user-authorized account flows. Record actual origins, redirects, form methods, CSRF behavior, cookies, content types, and necessary dependencies.
2. Look for an official documented integration before adapting browser requests. Record any applicable access restrictions. Do not assume a public API exists.
3. Use minimal, rate-limited requests. Do not crawl the whole site, guess credentials, bypass challenges, or extract unrelated account information.
4. Build an origin inventory for Ubian, assets/maps, identity providers, checkout providers, and optional analytics. Mark each classification as observed or provisional.
5. Sanitize any fixtures or network evidence. Never commit real credentials, cookies, account identifiers, personal journeys, payment data, or raw authenticated traffic captures.

## 4. Architecture and trust boundaries

### Bundled application

The trusted frontend consists entirely of bundled Svelte assets. It owns navigation, local saved items, account/vault controls, permission explanations, and privacy status. Rust owns application state, validation, local persistence, authentication coordination, and any verified network adapters. Keep long-lived secrets out of Svelte stores and browser storage.

### Provider website view

Render Ubian in a **separate Android WebView with no application bridge**, controlled through a narrow Rust/Kotlin integration. Prefer this approach for broad browser coverage in the first release. Keep the bundled Tauri view and provider view as separate trust contexts; closing the provider view must return to the local interface.

- Do not load remote pages into the privileged Svelte view.
- Do not embed Ubian in an iframe inside that view. Tauri documents that Android cannot distinguish some iframe IPC requests from their containing window; see [Tauri capabilities](https://v2.tauri.app/security/capabilities/).
- Give provider pages no Tauri capabilities, vault access, filesystem access, generic HTTP command, or `addJavascriptInterface` object. Follow the [Android native bridge guidance](https://developer.android.com/privacy-and-security/risks/insecure-webview-native-bridges).
- Implement native behavior in a maintained plugin/module, rather than depending on manual changes that Android project generation will overwrite. Use [Tauri mobile plugin development](https://v2.tauri.app/develop/plugins/develop-mobile/) as the integration reference.
- First prove the view lifecycle, Back handling, keyboard/insets, rotation, permissions, and navigation control in an Android spike. Do not assume desktop child-WebView APIs behave identically on Android.
- Limit messages from the native layer to typed status information needed by the local UI. A page must not select arbitrary commands or event destinations.

A separate WebView is an application privilege boundary, not a claim of complete process, cookie, or storage isolation. Verify actual Android WebView behavior before making stronger statements.

### Progressive local functionality

Add Rust adapters only for observed, permitted, stable service interactions. Prefer typed responses and small adapters over a general website proxy. If parsing HTML is necessary, isolate selectors, use sanitized fixtures, and detect provider changes explicitly.

Render provider-derived text as text in Svelte. Do not inject downloaded HTML into the trusted interface. Do not download provider scripts into the privileged context or bypass the provider's CSP/frame restrictions.

Rust and the provider WebView do not automatically share cookies. Assign session ownership explicitly. Keep login and account flows in the WebView initially; add shared session handling only after verifying cookie attributes, CSRF requirements, expiry, and logout behavior. A working `reqwest` login alone does not establish an authenticated WebView session.

## 5. Authentication and session lifecycle

### Required behavior

1. Allow guest access to public functionality without forcing registration or vault creation.
2. Start manual login at the verified HTTPS Ubian login page in the restricted provider view. Support email/password login without requiring credential storage.
3. Preserve provider CSRF protections, secure cookies, redirects, and challenges. Surface invalid credentials, timeout, offline state, rate limiting, and session expiry distinctly without exposing response bodies.
4. Confirm login using a verified authenticated signal, not simply the disappearance of the login form or receipt of HTTP 200.
5. Offer separate controls for **remember session** and **save login credentials**. Both are off initially. Explain their effect in plain language.
6. Use the provider session as issued. Do not invent a refresh-token flow. Reauthenticate when necessary and avoid background password retry loops.
7. Keep registration and password recovery on the provider's supported pages. Never capture or store a Google/Facebook password.
8. Use a system browser/Custom Tab for social login where required. Returning to the application is supported only if a legitimate provider callback/session-transfer mechanism has been verified. Browser and WebView sessions must not be assumed to transfer automatically.
9. While social login integration is unverified, label it accurately and retain a working email/password path. Do not fabricate OAuth client credentials, redirect URIs, or domain ownership.
10. On expiry or account switch, stop authenticated work and remove stale account UI. Resume pending read-only navigation after login; do not replay purchases or other state-changing requests automatically.

### Optional saved-credential flow

Implement a local, explicit save/update action; do not silently scrape the password from the remote page. A local credential form may accept the user's Ubian email/password, transfer them once to the native/Rust coordinator, and clear its fields promptly. Explain that the credential is being saved for Ubian login.

Before releasing saved-login reuse, prove a narrowly scoped delivery mechanism. Prefer a verified authentication adapter when supported; otherwise a fixed native form-fill operation may target the verified Ubian login page. It must check the exact HTTPS origin, expected top-level document, form destination, and navigation state immediately before use. Refuse unknown/cross-origin frames and cancel on navigation. Do not expose an arbitrary script evaluator or a general secret-read command to the frontend.

Only release credentials after an explicit sign-in/unlock action. Let the user submit the provider form unless a verified adapter safely completes submission. Any inserted value must be encoded as data, never concatenated as JavaScript source. Ubian's page necessarily receives the password during a web login; the vault does not protect it from the service itself.

Treat a failing or changed form as an integration error and fall back to manual login. Do not keep injecting credentials or assume that storing a password proves automatic login works.

### Logout and deletion

- Logout cancels requests, attempts the verified provider logout when online, and clears local sessions, account-specific caches, and provider website state. Report if server-side logout could not be confirmed; local logout must still complete.
- Offer an explicit choice to remove the saved login. Retain it only when the user chooses that behavior.
- “Forget account” deletes all local credentials, session material, account caches, and associated key aliases; clear website cookies/storage before another account is used.
- “Delete all local data” additionally removes local favorites, recents, settings, and diagnostic history. Explain that this does not delete the Ubian account or data retained by Ubian.
- Wait for asynchronous cookie/storage deletion to finish. Prevent concurrent navigation or requests from recreating state during cleanup.

## 6. Secure local storage

### Default Android design

Use a small **versioned encrypted file** for credentials and any application-managed persistent authentication material. Implement encryption through Android platform cryptography using a non-exportable Android Keystore AES key. A narrowly scoped Kotlin module can perform encrypt/decrypt operations for the Rust coordinator. The [Android Keystore documentation](https://developer.android.com/privacy-and-security/keystore) describes key-use restrictions and hardware-backed protection; hardware backing is device-dependent.

- Use a standard authenticated cipher such as `AES/GCM/NoPadding`, a generated 256-bit key, a fresh unique nonce per encryption, and an authentication tag. Bind the format version and record purpose as authenticated additional data.
- Keep the key in Android Keystore and ciphertext in app-private, backup-excluded storage. Never put a usable key next to the ciphertext, in source, in an environment file, or in frontend assets.
- Use atomic replacement and restrictive file access. Handle interrupted writes, schema migration, authentication failure, missing keys, and key invalidation explicitly.
- Reject corrupted/tampered records without returning partial plaintext. If secure storage is unavailable, allow manual login and decline persistence; never silently write plaintext.
- Protect saved-credential reuse with device authentication/biometrics where supported. Tie the intended protection to key-use authorization where feasible, and test canceled prompts, device-lock changes, and invalidated keys.
- Do not require StrongBox-capable hardware. Report the protection actually available rather than claiming all devices have equivalent hardware security.
- Keep decrypted values in narrow-lived native/Rust memory. Use secret wrappers/zeroization where practical. Clear local form state, avoid copies, and never put secrets into debug formatting, URLs, clipboard, logs, crash reports, or application events. JavaScript/JVM memory cannot be promised immediate complete erasure.
- Exclude credentials, authentication state, website storage, and sensitive caches from cloud backup and device transfer using the applicable Android rules. Test both supported rule formats; see [Android backup configuration](https://developer.android.com/identity/data/autobackup).
- A restored installation without its original Keystore key must ask the user to sign in again.

SQLCipher or a vetted vault library may replace the encrypted file if there is a demonstrated need. Document Android support and key management first. SQLite, JSON, Tauri Store, and a plugin name alone are not encryption. [Tauri Stronghold](https://v2.tauri.app/plugin/stronghold/) is an alternative to evaluate, not an automatic substitute for a secure unlock/key-management design. Do not implement a custom cryptographic primitive or reuse the Ubian password as the local encryption key.

### Website persistence is a separate concern

Encrypting the vault does **not** automatically encrypt WebView's cookie database, IndexedDB, local storage, caches, or browser-managed saved form data with the application's key. Inventory what persists on actual devices. Cookies and tokens are authentication secrets too.

- With remember-session disabled, clear provider state before opening a new session and during logout/normal teardown; also clean up before opening after a process crash. Do not rely only on an application-exit callback.
- Do not claim that this prevents every temporary disk write or makes the WebView incognito. Describe provider web state as Android-sandboxed state unless stronger protection is proven.
- Enabling remember-session must disclose any reliance on WebView-managed persistence. Exclude that state from backup and clear it on forget/logout.
- Any authentication material deliberately persisted by application code must use the encrypted vault. Do not export cookies through `document.cookie` or assume a simple cookie string preserves HttpOnly, expiry, SameSite, domain/path, and partition attributes.
- If stricter encryption of all website authentication state becomes a requirement, treat it as an architecture change and verify a suitable storage/session strategy before advertising it.

### Other local data

Keep settings separate from credentials. Store local favorites/recents in an app-private database or structured file; encrypt location history and account-linked cached data when persisted. Default to bounded recents, provide a disable-history setting, and offer clear-history controls. Show retention rules in settings.

The threat model covers accidental disclosure, extracted app files/backups, untrusted page content, and ordinary cross-app access. Document that a compromised operating system or compromised running application can defeat protections; do not market encryption as protection against every rooted-device or runtime attack.

## 7. Privacy and telemetry controls

### Defaults and user controls

Add no app analytics, advertising identifiers, session recording, or automatic remote crash reporting. Keep diagnostic output local, minimal, and redacted. Support an explicit sanitized diagnostic export only when the user requests it.

Offer these website modes:

| Mode | Behavior |
| --- | --- |
| Balanced — default | Block verified nonessential analytics/advertising requests, disable third-party cookies, and preserve observed functional dependencies. |
| Strict — optional | Apply additional tested restrictions, explain potential feature failures, and retain a one-step return to Balanced. |
| Compatibility — explicit | Temporarily relax optional website filtering for a failing flow; show that the mode is active and restore the previous mode when the flow ends. |

Never silently disable protection after an error. Compatibility mode does not grant website IPC access, disable TLS validation, or relax credential-origin checks. Any third-party-cookie exception must be deliberate and scoped to the affected flow. Use the [Android CookieManager controls](https://developer.android.com/reference/android/webkit/CookieManager) and test behavior across supported WebView versions.

### Filtering implementation

1. Apply policy before network requests where supported, not solely by hiding page elements or overriding JavaScript APIs after the page loads.
2. Keep rules local, versioned, reviewable, and accompanied by observed purpose and regression coverage. Do not claim an analytics vendor/domain was detected without evidence.
3. Match parsed URLs with exact host or deliberate subdomain rules. Keep functional origin rules separate from optional telemetry rules; an entire first-party host must not automatically bypass path-specific blocking.
4. Apply the same reviewed policy to Rust requests and provider-view requests where applicable. Keep callbacks fast and thread-safe; avoid synchronous Rust/UI round trips for every asset.
5. Use Android resource-request hooks and service-worker hooks where available. Include cached content and existing service workers in mode-change/cleanup testing.
6. Test XHR/fetch, images/scripts, frames, beacons, service workers, redirects, and WebSockets. Record what the selected interception mechanism cannot inspect or block.
7. Limit the privacy dashboard to blocked host/category/count and active policy. Avoid retaining full URLs, query strings, payloads, or a travel browsing history.
8. Do not add a VPN, root dependency, certificate interception, or developer-operated proxy to implement the initial blocker.

Navigation callbacks alone are not a request blocker. Android documents that `shouldOverrideUrlLoading` does not cover POST navigations, while resource interception does not report every redirect target or every scheme; see [WebViewClient](https://developer.android.com/reference/android/webkit/WebViewClient). Service workers have a separate [ServiceWorkerClient](https://developer.android.com/reference/android/webkit/ServiceWorkerClient) interception surface. These limits must inform both implementation and public claims.

### Accurate privacy promises

Describe the feature as **reducing optional website tracking**. Ubian and necessary providers still receive requests, IP addresses, account activity, purchase details, and data needed to answer searches. Blocking some analytics cannot stop server-side event collection, same-origin tracking, all fingerprinting, or tracking in an external browser/payment app. Respect provider consent controls; do not silently accept optional tracking to make a page load.

Request location only after a location-dependent action. Allow manual stop/address entry and approximate location where sufficient. No background location by default. Do not send app-local recents or favorites to Ubian unless required by a clearly identified user action or an explicitly enabled sync feature.

## 8. Navigation, permissions, and payment handling

- Parse URLs using a real URL parser. Validate scheme, exact normalized host, port, and purpose; reject lookalike hosts, user-info tricks, unexpected schemes, and unapproved redirects in application-controlled requests.
- Keep separate rules for top-level provider navigation, subresources, authentication, checkout, downloads, and external applications. Unknown external links should have a clear browser handoff rather than entering the privileged view.
- Serve the local interface only from bundled assets in production. Use a restrictive CSP without remote scripts or arbitrary frames. Scope Tauri capabilities to the local view and validate command callers and arguments server-side/native-side.
- Avoid generic `fetch_any_url`, `read_any_file`, `get_secret`, `evaluate_script`, or shell commands exposed to the frontend. Use typed commands for concrete actions.
- Require HTTPS for remote services. Keep certificate validation and supported Safe Browsing protections enabled. Never proceed through certificate errors. Permit development-only cleartext access only through an explicitly separate development configuration.
- Disable unnecessary file/content access and universal access from file URLs in provider views. Enable only web features required by observed functionality.
- Handle external intents with purpose-specific allowlists and user-driven actions. Never dispatch an arbitrary `intent://` URI supplied by a page without validation.
- Request only needed Android permissions. Internet is necessary; foreground location and document/photo selection are conditional. Do not request contacts, SMS, broad storage, advertising ID, background location, or NFC merely because Ubian advertises a related service.
- Use the system picker for explicit uploads and scoped download/export flows. Validate MIME type, size, destination, and authenticated download handling. Do not expose provider downloads as executable application code.
- Keep card entry, identity-provider credentials, and payment authorization in provider-controlled interfaces. Never store card numbers or CVV in this app.
- Verify checkout redirects, popups, 3-D Secure, cancellation, external-app return, and purchase status. Do not infer success from a return URL alone; re-read authoritative state where supported.
- Do not automatically retry a charge, purchase, top-up, or form submission after a timeout. Show an unknown/pending state until status can be established.
- Opening a system browser does not guarantee that it shares the WebView's cart or session. Provide a verified checkout handoff or report the limitation honestly.

## 9. Android and Svelte experience

Provide a coherent local navigation structure, for example **Journeys**, **Saved**, **Account**, and **Settings**, with direct entry points to provider services. Keep provider content identified and show the destination when handing off externally.

Requirements:

- Support system Back/predictive Back as appropriate, provider page history, and a reliable return to the local interface. Avoid accidental application exit in multi-step flows.
- Handle status/navigation bar insets, keyboard resizing, rotation, process recreation, and external browser return without leaking credentials or repeating transactions.
- Support light/dark system themes, readable contrast, large fonts, TalkBack labels, and at least 48dp touch targets for local controls.
- Give clear loading, empty, offline, expired-session, permission-denied, blocked-resource, and provider-change states. Provide retry only where safe.
- Distinguish offline saved data from live results. Show fetch time for cached schedules, balances, or account data; never present a cached balance as authoritative.
- Use `Europe/Bratislava` where provider timetable semantics require it, test daylight-saving changes, preserve Slovak diacritics, and format euro amounts correctly. Device locale/timezone must not silently change a requested journey time.
- Avoid background polling by default. Pause work when appropriate and use bounded timeouts, cancellation, caching, and backoff for safe reads.
- Protect login/vault screens from screenshots and recent-app previews where supported. Make any wider screenshot restriction a deliberate choice that does not unnecessarily prevent sharing a journey.

## 10. Suggested project structure

Adapt file names to the scaffold while preserving these responsibilities:

```text
INSTRUCTIONS.md
README.md
package.json
src/
  App.svelte
  lib/components/
  lib/features/{journeys,saved,account,settings}/
  lib/api/                 # typed wrappers for narrowly scoped Tauri commands
  lib/i18n/
src-tauri/
  Cargo.toml
  tauri.conf.json
  capabilities/            # local-view permissions only
  src/
    lib.rs
    commands/
    auth/
    provider/              # verified adapters and origin inventory
    privacy/               # policy models and rule evaluation
    storage/               # vault interface and local data repositories
  gen/android/             # generated Android host and required configuration
plugins/
  android-platform/        # maintained Rust/Kotlin platform integration
docs/
  architecture.md
  authentication.md
  security.md
  privacy.md
  feature-coverage.md
  android-build.md
tests/fixtures/            # sanitized provider examples only
```

Keep application-managed secrets and derived private data outside the source tree. Commit dependency lockfiles. Document which Android host files are generated and which manifest/backup/network configurations must be retained.

## 11. Implementation sequence

### Milestone 1 — prove Android integration

Create the Svelte/Tauri scaffold, extend the development environment, and run a minimal Android build. Prove restricted provider rendering, local UI return, resource filtering, origin checks, and Keystore encryption on Android. Record any toolchain/device blockers with exact reproduction steps. Resolve architectural blockers before building a broad UI.

### Milestone 2 — functional browsing and guest use

Implement local navigation, journey/departure access, saved items, state/error handling, and website navigation policy. Start the feature matrix with observed evidence. Apply Balanced privacy rules only for verified optional requests.

### Milestone 3 — authentication and encrypted storage

Complete manual email/password login, authenticated state detection, expiry, logout, forget-account cleanup, and optional saved credentials with secure reuse. Verify persistence and cleanup on device, including WebView-managed state. Add social login only through supported, tested flows.

### Milestone 4 — website coverage and privacy controls

Verify account/card pages, available commerce flows, uploads/downloads, mode changes, and external handoffs. Add Strict and Compatibility modes with regression evidence. Document every official-app-only or unverified function.

### Milestone 5 — release validation and handoff

Run focused checks, validate on an emulator and a physical Android device, build an APK, and finish setup/privacy/security/coverage documentation. Produce an AAB if distribution is requested. Keep signing material private. Publishing or making real purchases is not part of routine implementation validation.

## 12. Development and verification

The existing Nix shell includes Rustup, Node.js 22, JDK 17, and Linux desktop dependencies. Android SDK/NDK configuration still needs to be established and documented. Follow the current [Tauri Android prerequisites](https://v2.tauri.app/start/prerequisites/#android), verify compatible tool versions, and record a reproducible setup rather than relying on untracked machine state.

After scaffolding, define package scripts so these commands work from the repository root:

```sh
npm ci
npm run check
npm run lint
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri -- android init
npm run tauri -- android dev
npm run tauri -- android build --apk
```

Run Android initialization once as part of setup, not before every build. Refer to the [Tauri CLI Android build reference](https://v2.tauri.app/reference/cli/#android-build) for build options. Record the build variant, artifact location, signing status, supported ABIs, and installation steps. Host-side Rust checks do not cover Android-only Kotlin/Rust code; include an Android build and device tests. Check native-library compatibility with the selected SDK/NDK and applicable Android page-size requirements.

### Focused automated coverage

- Vault round-trip, tampering/tag failure, nonce uniqueness behavior, migration/interrupted writes, missing/invalidated keys, and persistence failure without plaintext fallback. Include Android instrumentation for actual Keystore behavior.
- URL parsing and rule matching with malicious hosts, suffix tricks, user-info, unexpected ports/schemes, redirects, frames, and path-specific telemetry rules.
- Authentication state transitions: failed login, expiry, canceled unlock, concurrent logout/navigation, account change, and no secret-bearing errors/events.
- Fixture-based parsing and explicit schema-change failures for any provider adapter. No live credentials in CI.
- UI state and accessibility checks for login/privacy/error controls. Test privacy behavior, not only that a toggle changes a variable.

### Android integration checks

Test cold start, process death/relaunch, rotation, background/foreground, Back, denied location, offline login, expired sessions, cookie cleanup, compatibility-mode restoration, and external navigation.

With a user-provided account available through a secure local method, verify login, app restart under both remember settings, account pages, saved-credential unlock, logout, and forget-account behavior. Inspect debug app storage and logs for known test secrets and document what remains in website-managed storage. Inspect packaged backup rules and exercise restore/key-loss behavior where feasible.

Compare network behavior between privacy modes with sanitized evidence. Prove that a targeted optional request is prevented and that journey search/login remain usable. Record gaps such as redirected or unobservable traffic. Do not claim full telemetry elimination.

Validate checkout with a provider-supported test flow when available. Otherwise inspect up to a safe stopping point and mark actual payment completion unverified. Do not spend money, create accounts, or alter real cards to make a test pass without explicit authorization.

## 13. Completion criteria

The first usable Android release is complete when:

- A fresh checkout can produce and install an APK using documented tooling.
- Guest journey/departure functions work against Ubian, and provider pages remain accessible through the restricted view.
- Email/password login and authenticated state are verified on Android; failure, expiry, and logout behave correctly.
- Optional saved credentials survive restart only as encrypted application-managed records, with functioning unlock/reuse and deletion. No usable key is stored beside ciphertext.
- Remote pages cannot invoke application commands or retrieve vault data. Malicious URL/frame cases are covered.
- Website cookie/storage persistence and backup behavior are documented and tested separately from the credential vault.
- Balanced mode measurably blocks verified optional tracking while preserving tested core flows; limitations and temporary exceptions are visible.
- The feature matrix accounts for browser functionality and clearly marks native-app-only, deferred, external, and unverified flows.
- Source, lockfiles, APK details, setup instructions, focused test results, and known limitations are supplied. No credentials, private captures, or signing keys are committed.

If account access, Android tooling, a physical device, or a provider test environment is unavailable, complete independent work and report the exact missing verification. A scaffold, mocked screen, desktop preview, or successful frontend build alone does not satisfy these criteria.
