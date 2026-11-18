# ubian-aggregator

An unofficial Android client for [Ubian](https://www.ubian.sk/), using Rust, Tauri 2, Svelte and TypeScript. ubian-aggregator is not affiliated with Ubian.

Download the [v0.2.0 ARM64 debug preview](https://github.com/steve0-t/ubian-aggregator/releases/tag/v0.2.0), or the [initial v0.1.0 build](https://github.com/steve0-t/ubian-aggregator/releases/tag/v0.1.0). Each release includes an APK, SHA-256 checksums, description and changelog. These builds require Android 8.0 or newer; Android device acceptance remains outstanding.

**Revision 2 implements native online guest timetable search.** The bundled Svelte interface uses a narrow Rust Ubian adapter for real stop selection, journeys and departure boards, without login or a website handoff. It keeps searches in memory and labels results as scheduled data. Connection-detail enrichment is date-checked: Ubian currently returns a previous-day stop sequence for the tested trips, so that enrichment is unavailable while search legs remain usable. See [implementation status](docs/implementation-status.md) and [verification](docs/revision-2-verification.md).

The [transport API analysis](docs/transport-api-analysis.md) and [provider discovery](docs/provider-discovery.md) document the anonymous website interfaces. Revision 2 authorizes their use for this personal client; they are not a supported public API. Ubian covers the required Dvorianky–Košice buses that were missing from the tested Transitous feed. Native email/password sign-in, account summary, session refresh/logout and opt-in encrypted session storage are now implemented; see [account verification](docs/account-verification.md). Cards, balances, purchases and saved-password reuse remain separate work.

The implementation follows [INSTRUCTIONS.md](INSTRUCTIONS.md), amended by [Revision 1](REVISION.md) and [Revision 2](REVISION-2.md). The `revision-1` work is merged into `master`. Device acceptance, including TalkBack, Back and Android runtime networking, still requires a connected device. Build and host checks are recorded separately from device checks.

## Run the local interface

```sh
nix develop .
npm ci
npm run dev
```

The browser preview contains no provider iframe, fake schedules or simulated login. Timetable and memory-only account commands run in Tauri on Android or desktop; the plain browser preview explains when native commands are unavailable. Encrypted session persistence and restricted provider/support views require Android. Slovak is the default; English and **Українська** are selectable in Settings and persist across restarts. Local accessibility labels, validation and status text are translated. Provider stop names and identifiers stay unchanged. Ukrainian native provider controls use Ukrainian text around the Slovak provider page; this does not claim that Ubian offers a Ukrainian website.

The local interface has a desktop sidebar and mobile bottom navigation. Appearance controls live together in **Settings → Appearance**: System, Light, Dark and True black / AMOLED, six accent presets, a custom color picker or `#RRGGBB` input, and reset. True black uses `#000000` for the main and navigation backgrounds; cards retain subtle borders and tones. A bundled blocking startup script applies the saved base palette before app initialization. Explicit Black stays selected when the device theme changes; System follows light/dark. Android system-bar matching is implemented but needs device verification.

Theme, accent and language preferences use local browser/WebView storage. Account records are separate; searches and personal journey history are not persisted. If preference storage is unavailable, appearance changes still work for the current run. Navigation has scoped tap-highlight and text-selection suppression with accent feedback and visible keyboard focus; the reported Android blue flash has not been reproduced or device-verified.

Account has a bundled email/password form and native summary. Sign-in starts in memory; **Save session on this device** and **Unlock saved session** use a separate Android Keystore AES-GCM record with supported strong-biometric authorization. The password is never saved. Saved profile data is marked cached until the session is verified online. Logout removes the native session/cache and verifies server revocation when possible; Forget removes local native account data. Secure persistence needs Android runtime validation; unsupported devices retain memory-only login. Registration/recovery and support retain fixed restricted provider entry points, whose website sessions remain independent. See [authentication](docs/authentication.md).

## Verify independent work

```sh
npm run check
npm run lint
npm run build
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo fmt --manifest-path plugins/android-platform/Cargo.toml -- --check
cargo fmt --manifest-path crates/journey-core/Cargo.toml -- --check
cargo clippy --manifest-path crates/journey-core/Cargo.toml --all-targets --locked -- -D warnings
npm run test:core
```

`npm run lint` checks formatting. Svelte/TypeScript diagnostics, including compiler accessibility warnings, are in `npm run check`. `npm test` covers preferences, translations, Bratislava date/time formatting, local timetable state and security configuration. The core Rust checks cover account form/session/profile parsing, cookie persistence/expiry, cancellation/backoff, request encoding, bounded timetable parsing, Bratislava times/DST, detail-date validation, vault metadata and URL policy. Browser account, timetable and appearance suites use test-only IPC responses. `npm run check:kotlin` additionally checks native sources and runs a synthetic storage harness when the compiler/SDK environment described in [android-build.md](docs/android-build.md) is supplied. JVM substitutes do not establish Android Keystore or biometric behavior.

The independent core crate and Tauri app have committed Cargo lockfiles. For the small opt-in live adapter regression, see [verification](docs/revision-2-verification.md). There is no SQLite cache or offline planner; a network connection is required for new searches. See [implementation status](docs/implementation-status.md) for checks actually completed in this revision; a build alone does not establish runtime accessibility, native timetables or secure account storage.

## Continue on Android

Android Studio is not required. The Nix shell supplies the pinned command-line SDK, build tools, NDK and JDK, reusing packages already in the Nix store. `npm run tauri` delegates to the separately installed, versioned Cargo Tauri CLI. See [Android setup and build instructions](docs/android-build.md).

```sh
nix develop .
npm run doctor
npm run android:build -- --debug
```

Run these commands from the repository root after installing frontend dependencies and the pinned Tauri CLI. The Android host is committed; use `npm run android:init` if it needs regeneration. The build command reapplies the maintained Android configuration before invoking Gradle through Tauri.

The maintained native integration is in `plugins/android-platform`. Regeneration settings live in `scripts/configure-android.mjs`. The generated Android host, Gradle wrapper and app dependency lockfile are retained in source control; build output and regenerable Tauri bindings remain ignored. Debug APKs can be attached to releases as assets rather than committed to Git.

Read the [architecture](docs/architecture.md), [authentication boundary](docs/authentication.md), [security](docs/security.md), [privacy](docs/privacy.md), and [feature matrix](docs/feature-coverage.md) before extending the spike.
