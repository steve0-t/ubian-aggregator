# Cestou

An unofficial Android client for [Ubian](https://www.ubian.sk/), using Rust, Tauri 2, Svelte and TypeScript. Cestou is not affiliated with Ubian.

**Current state: milestone 1 integration spike, not a usable Android release.** The local frontend builds and the shared URL policy has host tests. Android rendering, actual Keystore behavior, authentication and privacy behavior still require a full Android build and device validation. No APK has been produced.

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

The independent core crate has an included Cargo lockfile. The Tauri app lockfile cannot yet be resolved because Tauri is absent from this session's offline cache. Full application checks remain required; see [verification results](docs/implementation-status.md).

## Continue on Android

Use [Android setup and build instructions](docs/android-build.md), then run `npm run doctor`. `npm run tauri` delegates to the separately installed, versioned Cargo Tauri CLI.

The maintained native integration is in `plugins/android-platform`. Regeneration settings live in `scripts/configure-android.mjs`; no Android project has been fabricated to imitate a successful initialization. Run Android initialization once, apply that configuration, and retain/review the resulting host and dependency locks when the build is proven.

Read the [architecture](docs/architecture.md), [authentication boundary](docs/authentication.md), [security](docs/security.md), [privacy](docs/privacy.md), and [feature matrix](docs/feature-coverage.md) before extending the spike.
