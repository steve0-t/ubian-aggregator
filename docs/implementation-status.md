# Implementation status — Revision 1, 2026-11-16

**Revision 1 is in progress. Native timetables are incomplete and real account storage is unavailable.** The Android architecture gate remains open. An earlier ARM64 debug APK was built and inspected; see the [historical build record](android-build.md#verified-debug-artifact--2026-11-16). That artifact and its resolved tooling prerequisites do not prove the current revision or Android runtime behavior. No device is currently connected.

## Implemented preparation

- Settings owns all appearance controls; the home Appearance card and palette shortcut are removed, while Account and navigation remain available.
- Complete local SK/EN/UK dictionaries and typed language handling through preference validation/persistence, document language, TypeScript, Rust and Kotlin controls. Slovak remains the default. Ukrainian native controls wrap the Slovak provider page; no provider Ukrainian translation is claimed.
- Distinct saved `black` theme, true-black base/navigation tokens, blocking startup palette application, contrast-adjusted accents and Android system-bar synchronization source. System retains light/dark device following; explicit Black is stable.
- Navigation-scoped tap-highlight/text-selection suppression with selected/hover/pressed/accent and visible keyboard focus. The Android blue interaction effect is not yet reproduced or device-verified.
- Local timetable preparation that explicitly reports the source blocker. Former `journeys` provider destination removed from frontend/Rust/Kotlin. Stop query validation, a fixture-tested parser for an observed anonymous JSON response, local error/loading/cancellation handling and Bratislava formatting helpers exist. Runtime lookup returns `source_unavailable`, performs no HTTP and returns no sample data.
- Internal Rust/Kotlin synthetic vault contracts, versioned AES-GCM record codec, platform Keystore/file adapters and initial per-use biometric CryptoObject adapter. Only nonsensitive status reaches the frontend; real credential saving/reuse remains unavailable. Remember session and Save login credentials are separate, disabled and off.
- Session ownership and future logout/Forget/Delete cleanup ordering specified in [authentication](authentication.md), distinct from the existing partial provider cleanup implementation.

## Blocked and remaining functionality

A minimal anonymous stop lookup returned JSON, but this is not an access grant. No documented integration permission or official timetable API agreement was established. [Provider discovery](provider-discovery.md) records observations and the next evidence required. No runtime timetable transport, journey query/results/details, departure board, real-time semantics, bounded network cancellation/rate limiting or live permitted guest flow is implemented. Native timetable acceptance is not met.

The synthetic vault is preparation only. Its process-owned status instance deliberately lacks a usable authorizer for frontend save/use. The internal Android biometric path, Keystore authorization, enrollment/lock/key changes, AtomicFile behavior, restore and full cleanup need device testing. No general secret-read operation, local real-password form or credential delivery exists. Account login detection, expiry/logout, account switch and persistent sessions remain unverified/unimplemented.

## Verification record for this revision

The checks below distinguish host compilation, browser inspection and unrun device behavior.

| Check                                                                   | Current evidence                                | Scope / limits                                                                                                                                                                    |
| ----------------------------------------------------------------------- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `npm run check`                                                         | Passed, 0 errors / 0 warnings                   | Svelte/TypeScript; does not establish runtime accessibility                                                                                                                       |
| `npm run build`                                                         | Passed                                          | Bundled frontend only                                                                                                                                                             |
| `npm test`                                                              | Passed                                          | Preference/language/theme persistence and contrast, startup palette, dictionary alignment, Bratislava formatting, local cancellation/error state, capability/backup configuration |
| `npm run test:core`                                                     | Passed, 9 tests                                 | URL policy, observed sanitized stop parser/input failures, `uk` enum, nonsensitive vault metadata                                                                                 |
| Core Clippy with warnings denied                                        | Passed after removing a constant-only assertion | Independent core crate                                                                                                                                                            |
| Rust formatting                                                         | Passed                                          | App, plugin and core manifests                                                                                                                                                    |
| `npm run lint`                                                          | Passed                                          | Repository formatting                                                                                                                                                             |
| `npm run check:kotlin`                                                  | Passed; platform deprecation warning            | Platform source type-check and JVM policy/synthetic-vault harness; substitutes do not verify Android Keystore                                                                     |
| Full application Rust Clippy/tests                                      | Passed                                          | Host Clippy warnings denied; application test targets compile/run (0 tests), focused coverage in journey-core                                                                     |
| Chromium interface checks                                               | Passed (host Chromium)                          | 320/360px mobile at 200% text; language/theme reload, black surfaces, custom accents, keyboard focus and release-off-control; not Android                                         |
| Current Android build and packaged backup audit                         | Failed: disk exhausted                          | Current ARM64 APK attempt failed: disk exhausted during Rust compilation; desktop build cache removed afterwards                                                                  |
| Android touch, Back/lifecycle, TalkBack, bars and native timetable flow | Not run                                         | No device connected; permitted native source also unresolved                                                                                                                      |
| Real Keystore/biometric/restore instrumentation                         | Not run                                         | No device connected                                                                                                                                                               |
| Authenticated login, logout/forget, safe credential delivery, payment   | Not run                                         | No authorized account/provider test flow; real saving remains disabled                                                                                                            |

Host test coverage is narrower than device acceptance. The Kotlin harness uses synthetic values with substitute key/file/authentication providers to exercise state and failure behavior. A passing host harness does not prove Android authentication-bound key use or AtomicFile crash durability.

## Next verification and implementation gates

1. Free enough build space and rerun the current ARM64 Android APK build. Inspect its packaged manifest, permission scope and both backup-rule formats. The attempted build failed with `No space left on device`; source and Git commits were preserved. Affected source was restored from the revision checkpoint, and smaller checks were rerun.
2. Establish permitted timetable-source access, verify the actual journey/departure/detail contracts and implement a narrow bounded Rust transport. Preserve Bratislava request semantics across midnight and both DST transitions; distinguish scheduled data from verified real-time information and label any cache fetch time. Do not persist personal journey history by default.
3. Implement the bundled journey/departure/detail flow and verify search, results, Back, preserved input/results/scroll and failures on Android without website handoff. Until then, keep native timetables marked incomplete.
4. On a device, reproduce tap/hold/release/drag-off navigation with multiple accents and themes; test narrow/large Ukrainian labels, keyboard focus, TalkBack, insets, bars, Back/rotation/process recreation and provider bridge exclusion.
5. Exercise synthetic authenticated storage: round-trip, unauthenticated-use refusal, canceled/unavailable prompts, API 29 weak-only biometric capability, enrollment/lock/key changes, tampering, nonce variation, interrupted writes, partial deletion, late callbacks and restore without the original key.
6. With an authorized account supplied through a secure local method, verify provider authentication/session cleanup and fixed credential delivery. Only after those gates pass may real credentials or remember-session behavior be enabled. Keep purchases and publication outside routine verification.

## Android compilation and resource limit

`npm run android:build -- --target aarch64 --debug` reached Rust compilation but failed with `No space left on device`; it did not produce a current-revision APK. The full desktop Rust tests had completed successfully (application targets contain no unit tests; focused tests are in journey-core). The generated desktop Cargo cache was removed to recover space. No source, commits or user data were intentionally removed. A stylesheet write interrupted by the full disk was restored from the revision checkpoint and rebuilt.

After recovery, Gradle compiled both the maintained plugin and instrumentation with:

```sh
cd src-tauri/gen/android
./gradlew :tauri-plugin-android-platform:compileDebugKotlin :tauri-plugin-android-platform:compileDebugAndroidTestKotlin --no-configuration-cache --no-daemon --max-workers=2 -Dorg.gradle.jvmargs=-Xmx768m
```

This passed (28 tasks, 8 executed) after downloading the pinned AndroidX test dependencies. An earlier offline attempt lacked `androidx.test:monitor:1.7.2`; an earlier unrestricted attempt exited 143. Neither is the final compilation result. Compilation does not run instrumentation, prove real Keystore authorization or audit a freshly packaged APK. Source backup/regeneration checks pass; current packaged and restore checks remain unrun. See [the device backlog](vault-device-checks.md).
