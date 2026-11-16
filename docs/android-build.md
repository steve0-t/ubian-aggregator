# Android setup and build

**An ARM64 debug APK was built on 2026-11-16.** Signature verification and packaged asset/resource checks passed. Device behavior, other ABIs, release builds and fresh-checkout reproducibility remain unverified.

## Selected and observed tooling

| Component                      | Selected configuration                           | Actually available/tested here                                   |
| ------------------------------ | ------------------------------------------------ | ---------------------------------------------------------------- |
| Minimum Android                | API 26 / Android 8                               | Confirmed in merged debug manifest; no device                    |
| Compile/target SDK             | 37                                               | Confirmed in packaged APK                                        |
| SDK platform packages          | `platforms;android-36`, `platforms;android-37.0` | Both supplied by Nix and used by Gradle                          |
| Build tools                    | 36.0.0 and 37.0.0                                | Both installed; SDK 37 aapt2 used for packaging                  |
| NDK                            | 29.0.14206865                                    | ARM64 native library compiled and packaged                       |
| JDK                            | 17                                               | 17.0.21 used for the Gradle build                                |
| Rust                           | Stable, at least 1.90                            | 1.95.0; tests and Clippy pass with consistent compiler selection |
| Rust Android targets           | aarch64, armv7, i686, x86_64                     | All four installed; aarch64 debug build verified                 |
| Tauri runtime/build helper     | 2.12.5 / 2.7.4                                   | Compiled in the ARM64 debug build                                |
| Tauri CLI                      | 2.12.4                                           | Installed in `~/.cargo/bin`; exposed by the Nix shell            |
| Kotlin                         | 2.2.10                                           | Generated host and platform plugin compiled                      |
| Node                           | Node 22 LTS                                      | 22.23.5                                                          |
| Gradle / Android Gradle plugin | 9.6.1 / 9.3.1                                    | Generated wrapper used for successful debug APK build            |

Tauri's [Android prerequisites](https://v2.tauri.app/start/prerequisites/#android), [mobile plugin guide](https://v2.tauri.app/develop/plugins/develop-mobile/), and [CLI-generated Android template](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/templates/mobile/android/app/build.gradle.kts) informed this configuration. Reconcile SDK/package and Gradle requirements when upgrading these dependencies.

## Command-line setup

**Android Studio is not required.** Run all commands from the repository root. The locked Nix shell supplies JDK 17, ADB, SDK command-line tools 20.0, platforms 36 and 37.0, build tools 36.0.0 and 37.0.0, and NDK 29.0.14206865. Tauri's Android library uses SDK/build tools 36; the app uses 37. Both versions must be available because Gradle cannot install missing packages into the read-only Nix store. It reuses existing Nix store packages and downloads only missing dependencies. No emulator or IDE is included. `flake.nix` declares Android SDK license acceptance for these packages.

The shell exports `JAVA_HOME`, `ANDROID_HOME`, `ANDROID_SDK_ROOT` and `NDK_HOME`, and adds SDK commands and `~/.cargo/bin` to `PATH`. Valid SDK/NDK paths supplied by the caller take precedence; empty or stale default directories fall back to the Nix toolchain. It also selects the SDK's patched `aapt2` through `GRADLE_OPTS`, so Gradle does not try to run an incompatible Maven binary on NixOS.

```sh
nix develop path:.
npm ci
cargo tauri --version
```

After changing the SDK composition, run `direnv reload` or leave and re-enter the Nix shell. Nesting `nix develop` inside an older shell can retain its valid but incomplete `ANDROID_HOME`.

If the CLI is missing, install it with `cargo install tauri-cli --version 2.12.4 --locked`. Reuse an existing installation of that version. Likewise, add only missing Rust Android targets:

```sh
rustup target list --installed
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
rustup component add clippy rustfmt
```

### Use an existing SDK outside Nix

If your command-line tools are already installed, set the following paths to that installation. `JAVA_HOME` must point to a standalone JDK 17. Keep `ANDROID_HOME` and the legacy `ANDROID_SDK_ROOT` identical if both are set. For a standard command-line tools layout:

```sh
export JAVA_HOME="/path/to/jdk-17"
export ANDROID_HOME="/path/to/android-sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"
export PATH="$HOME/.cargo/bin:$JAVA_HOME/bin:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$PATH"
sdkmanager --list_installed
```

Skip package installation when the pinned packages are already present. For a writable SDK that is missing packages, install them and review/accept licenses entirely from the terminal:

```sh
sdkmanager --sdk_root="$ANDROID_HOME" "platform-tools" "platforms;android-36" "platforms;android-37.0" "build-tools;36.0.0" "build-tools;37.0.0" "ndk;29.0.14206865"
sdkmanager --sdk_root="$ANDROID_HOME" --licenses
```

Do not use `sdkmanager` to modify the read-only Nix store SDK; update the composition in `flake.nix` instead. The [Android command-line tools](https://developer.android.com/tools) are distributed separately from Android Studio.

The SDK platform must be recognized as API 37 by the chosen Android Gradle plugin. A toolchain/package mismatch is an initialization blocker, not a reason to silently raise the minimum SDK or remove security configuration.

### Regenerate the Android host when needed

The generated host and app Cargo lockfile are committed. Fresh checkouts can run `npm run doctor` and proceed to the build. If the host needs regeneration after a tooling update:

```sh
npm run android:init
npm run doctor
```

`android:init` runs `cargo tauri android init --ci`, then `android:configure`. It does not open an IDE or prompt to install SDK components. Tauri may print “Generating Android Studio project”; this names the Gradle project format and does not require the IDE. `android:configure` applies API/build-tools/NDK pins, backup/transfer exclusions and network configuration to the generated host, and refuses unfamiliar templates. The maintained platform module is linked by its Tauri plugin build script. The script has idempotence and regeneration-security tests; the debug build's merged manifest was inspected as recorded below.

Review host, Gradle wrapper and dependency lock changes after regeneration. Local SDK paths, `.gradle`, build output, signing material, Tauri API copies, generated bindings and generated permission/schema files remain ignored. The local plugin's build script regenerates command permissions from its command list. Do not hand-edit the generated host as the sole source of a fix: keep changes in the configuration script or native module and reapply them after regeneration.

## Build and check

```sh
npm run check
npm run lint
npm run build
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run android:build -- --debug
```

`android:build` reapplies the maintained configuration and invokes `cargo tauri android build --apk --ci`. Tauri runs the generated Gradle wrapper, so a system Gradle installation is unnecessary. The first build needs network access for uncached Cargo and Gradle dependencies. For a single ABI, use `npm run android:build -- --debug --target aarch64` (or `x86_64` for an emulator).

### Verified debug artifact — 2026-11-16

`npm run android:build -- --debug --target aarch64` succeeded in a refreshed Nix shell:

- Artifact: `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`
- ABI: `arm64-v8a` only, despite the generated variant name `universalDebug`.
- Size: 123,629,155 bytes; debug signing verified with `apksigner verify`.
- SHA-256: `322266961d3d312df7510a689bc576deb1c090f6b6d9527a6ba1a80b59950702`
- APK checks: native library and DEX present; bundled `network-policy.json` matches the maintained policy; backup, transfer and network-security XML resources present.
- Merged manifest: minimum SDK 26, target SDK 37, backup disabled and provider Activity not exported.

The build fixes keep asset configuration in the platform module's `sourceSets { … }` block, avoiding AGP 9.3's legacy `AndroidLibrarySourceSet` cast, and supply SDK/build tools 36 alongside 37 in Nix. `npm test`, Gradle `help`, and `npm run doctor` also passed. No device installation or release build was performed.

Keep APKs out of Git. The debug APK may be attached to a release as an explicitly labelled ARM64 debug asset with its SHA-256 and the validation limits above. The attachment remains a debug build and does not establish release-build or device validation.

### Install for device checks

Use a clean debug test installation first. Locate the actual artifact with:

```sh
rg --files src-tauri/gen/android/app/build/outputs -g '*.apk'
adb devices -l
adb install -r path/to/the-reported-debug.apk
```

`npm run tauri -- android dev` is available for development. The debug network XML permits only localhost, 127.0.0.1 and 10.0.2.2 cleartext for the frontend dev server; use ADB reverse on ports 1420/1421 and a loopback dev host where appropriate. A physical-device LAN host needs a deliberate debug-only exception and matching dev CSP. HTTPS remains mandatory for provider content. A bundled debug APK avoids dev-server dependencies during lifecycle tests.

For the required release-build attempt:

```sh
npm run android:build
```

Record the actual variant, artifact path, signing status, hashes and ABIs. A debug APK uses local development signing; release signing must be configured with private material outside this repository. No publishing, AAB distribution or real purchase is authorized by routine validation.

## Native checks and device gate

Instrumentation lives in the native library's `src/androidTest`. In the generated host, use `./gradlew projects` to identify the library's task path, then run its `connectedDebugAndroidTest` task against a clean emulator/device. The tests exercise the shared URL corpus, first-party path/subdomain rules, real Keystore round-trip/tamper/nonce/key-loss behavior, unknown versions and interrupted atomic replacement. They have **not** run here.

The optional `npm run check:kotlin` host check requires `ANDROID_JAR`, `KOTLIN_COMPILER_CLASSPATH` (compiler and runtime dependency jars), `KOTLIN_STDLIB`, and optionally `JAVA_HOME`. It compiles the platform Activity/filter/cleanup/crypto sources and runs the shared policy corpus on the JVM. It excludes the Tauri-facing plugin class and Android instrumentation. It is not an APK or Keystore test.

Before closing milestone 1, test on an emulator and physical device:

1. Open guest timetables, search and view departures; close/back reliably; rotate, resize the keyboard, use large fonts and TalkBack. Confirm remote pages and frames have no Tauri/native bridge and cannot invoke the plugin commands.
2. Exercise cold start, process death/recreation, background/foreground and browser return. Never restore a purchase or POST automatically. Confirm capture protection on login and ordinary sharing for public journeys.
3. Test offline/TLS failures, disallowed URLs, denied location and visible unsupported popup/upload/download flows. Check main-frame POST and redirect gaps separately from resource filtering.
4. Seed and inspect cookies, local storage, IndexedDB, caches and service workers, including writes during closure and before relaunch. Verify the cleanup timeout and overlapping-close paths. Consider the completion-capable [AndroidX WebStorageCompat API](https://developer.android.com/reference/androidx/webkit/WebStorageCompat) with runtime feature detection; the current platform-only cleanup is not enough evidence for account switching.
5. Inspect the merged manifest and packaged backup/network resources. Exercise backup/transfer restore and key loss. Confirm permissions remain minimal and a restored installation cannot reuse an absent Keystore key.
6. Compare sanitized request evidence with/without any future optional rules. Check fetch/XHR, scripts/images, frames, beacons, service workers, redirects and WebSockets before making privacy claims.
7. Validate native libraries for 16 KB pages using current Android tooling, APK alignment inspection and an appropriate emulator/device (`adb shell getconf PAGE_SIZE`). An NDK version alone is not proof; recheck distribution requirements before release.

Authentication, persistence/unlock and payments require their additional gates in [authentication.md](authentication.md) and [feature-coverage.md](feature-coverage.md). No test account or payment sandbox is presently available.
