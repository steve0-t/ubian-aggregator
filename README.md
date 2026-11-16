# ubian-aggregator

An unofficial Android client for [Ubian](https://www.ubian.sk/), using Rust, Tauri 2, Svelte and TypeScript. ubian-aggregator is not affiliated with Ubian.

**Current state: Revision 1 is in progress, not a usable Android release.** Interface changes and synthetic account-storage preparation are implemented. Native timetables remain incomplete: an anonymous stop response was inspected, but permitted integration access has not been established. The app shows this blocker locally and does not hand timetable searches to a website or return sample schedules. See [provider discovery](docs/provider-discovery.md) and [implementation status](docs/implementation-status.md).

An earlier ARM64 debug APK was built successfully; see the [2026-11-16 build record](docs/android-build.md#verified-debug-artifact--2026-11-16). That artifact does not verify the current revision or Android behavior. No Android device is currently connected. The Android architecture gate remains open.

The implementation follows [INSTRUCTIONS.md](INSTRUCTIONS.md), as amended by [REVISION.md](REVISION.md). The original document-only boundary predates this implementation task.

## Run the local interface

```sh
nix develop path:.
npm ci
npm run dev
```

The browser preview contains no provider iframe, fake schedules or simulated login. Native actions require the Android app. Slovak is the default; English and **Українська** are selectable in Settings and persist across restarts. Local accessibility labels, validation and status text are translated. Provider stop names and identifiers stay unchanged. Ukrainian native provider controls use Ukrainian text around the Slovak provider page; this does not claim that Ubian offers a Ukrainian website.

The local interface has a desktop sidebar and mobile bottom navigation. Appearance controls live together in **Settings → Appearance**: System, Light, Dark and True black / AMOLED, six accent presets, a custom color picker or `#RRGGBB` input, and reset. True black uses `#000000` for the main and navigation backgrounds; cards retain subtle borders and tones. A bundled blocking startup script applies the saved base palette before app initialization. Explicit Black stays selected when the device theme changes; System follows light/dark. Android system-bar matching is implemented but needs device verification.

Theme, accent and language preferences use local browser/WebView storage. Account records are separate; searches and personal journey history are not persisted. If preference storage is unavailable, appearance changes still work for the current run. Navigation has scoped tap-highlight and text-selection suppression with accent feedback and visible keyboard focus; the reported Android blue flash has not been reproduced or device-verified.

Account and support retain fixed, restricted provider entry points. Remember session and Save login credentials are distinct, off and unavailable. The synthetic storage scaffold cannot save or reuse real credentials. WebView-managed cookies and website storage are a separate persistence boundary; encrypting app records does not encrypt those stores with the vault key.

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

`npm run lint` checks formatting. Svelte/TypeScript diagnostics, including compiler accessibility warnings, are in `npm run check`. `npm test` covers preferences, translations, Bratislava date/time formatting, local timetable state and security configuration. The core Rust checks cover source input/parsing and internal vault metadata as well as URL policy. `npm run check:kotlin` additionally checks native sources and runs a synthetic storage harness when the compiler/SDK environment described in [android-build.md](docs/android-build.md) is supplied. JVM substitutes do not establish Android Keystore or biometric behavior.

The independent core crate and Tauri app have committed Cargo lockfiles. See [implementation status](docs/implementation-status.md) for checks actually completed in this revision; a build alone does not establish runtime accessibility, native timetables or secure account storage.

## Continue on Android

Android Studio is not required. The Nix shell supplies the pinned command-line SDK, build tools, NDK and JDK, reusing packages already in the Nix store. `npm run tauri` delegates to the separately installed, versioned Cargo Tauri CLI. See [Android setup and build instructions](docs/android-build.md).

```sh
nix develop path:.
npm run doctor
npm run android:build -- --debug
```

Run these commands from the repository root after installing frontend dependencies and the pinned Tauri CLI. The Android host is committed; use `npm run android:init` if it needs regeneration. The build command reapplies the maintained Android configuration before invoking Gradle through Tauri.

The maintained native integration is in `plugins/android-platform`. Regeneration settings live in `scripts/configure-android.mjs`. The generated Android host, Gradle wrapper and app dependency lockfile are retained in source control; build output and regenerable Tauri bindings remain ignored. Debug APKs can be attached to releases as assets rather than committed to Git.

Read the [architecture](docs/architecture.md), [authentication boundary](docs/authentication.md), [security](docs/security.md), [privacy](docs/privacy.md), and [feature matrix](docs/feature-coverage.md) before extending the spike.
