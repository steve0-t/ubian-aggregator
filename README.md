# ubian-aggregator

An unofficial Android client for [Ubian](https://www.ubian.sk/), using Rust, Tauri 2, Svelte and TypeScript. ubian-aggregator is not affiliated with Ubian.

**Current state: milestone 1 integration spike, not a usable Android release.** An ARM64 debug APK builds successfully, the local frontend builds and the shared URL policy has host tests. Android rendering, actual Keystore behavior, authentication and privacy behavior still require device validation. See the [verified build record](docs/android-build.md#verified-debug-artifact--2026-11-16).

The implementation follows [INSTRUCTIONS.md](INSTRUCTIONS.md). Its original document-only boundary predates this implementation task. The Android architecture gate remains open; later milestones have not been declared complete.

## Run the local interface

```sh
nix develop path:.
npm ci
npm run dev
```

The browser preview contains no provider iframe, fake schedules or simulated login. Native actions are unavailable outside the Android application. Slovak is the default; English is selectable in Settings. The app has an original route-line icon in `public/icon.svg`.

The local interface uses a Notesnook-inspired neutral palette, a desktop sidebar and mobile bottom navigation. **Settings → Appearance** offers system, light and dark themes, six accent presets, and a custom color picker or `#RRGGBB` input. Colors update immediately, with contrasting text for custom accents. Theme, accent and language preferences are saved in local browser/WebView storage; no account or browsing data is stored there by this interface. If storage is unavailable, appearance changes still work for the current run.

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

`npm run lint` checks formatting; Svelte/TypeScript diagnostics, including compiler accessibility warnings, are in `npm run check`. Runtime accessibility is unverified.

The independent core crate and Tauri app have committed Cargo lockfiles. Full application checks remain required; see the [build record](docs/android-build.md) and earlier [verification results](docs/implementation-status.md).

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
