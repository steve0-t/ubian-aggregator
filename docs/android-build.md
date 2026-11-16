# Android setup and build

**No APK has been built in this session.** These are the selected setup steps and the checks needed to prove them. Fresh-checkout Android reproducibility remains an open release criterion; do not treat a successful frontend build as Android validation.

## Selected and observed tooling

| Component                      | Selected configuration                       | Actually available/tested here                                             |
| ------------------------------ | -------------------------------------------- | -------------------------------------------------------------------------- |
| Minimum Android                | API 26 / Android 8                           | Configuration only; no device                                              |
| Compile/target SDK             | 37                                           | Native source type-checked with an installed API 37.2 jar; no Gradle build |
| SDK platform package           | `platforms;android-37.0`                     | Not configured in the development shell                                    |
| Build tools                    | 37.0.0                                       | Store copy exists, not used for packaging                                  |
| NDK                            | 29.0.14206865                                | Missing                                                                    |
| JDK                            | 17                                           | 17.0.21 used for the Kotlin host check                                     |
| Rust                           | Stable, at least 1.90                        | 1.95.0; tests and Clippy pass with consistent compiler selection           |
| Rust Android targets           | aarch64, armv7, i686, x86_64                 | None installed; only x86_64-unknown-linux-gnu exists                       |
| Tauri runtime/build helper     | 2.12.5 / 2.7.4                               | Source dependencies pinned; not cached or compiled                         |
| Tauri CLI                      | 2.12.4                                       | Missing                                                                    |
| Kotlin                         | Version supplied by generated host           | Host type-check used cached compiler 2.2.21; Gradle version unverified     |
| Node/npm                       | Node 22 LTS                                  | 22.23.4 / 10.9.9                                                           |
| Gradle / Android Gradle plugin | Version supplied by pinned CLI host template | Host not generated; actual versions must be recorded after initialization  |

Tauri's current [Android prerequisites](https://v2.tauri.app/start/prerequisites/#android), [mobile plugin guide](https://v2.tauri.app/develop/plugins/develop-mobile/), and [CLI-generated Android template](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/templates/mobile/android/app/build.gradle.kts) informed this configuration. Reconcile any SDK/package or Gradle requirements during the first real build and record the actual successful versions.

## Set up once on a network-enabled machine

Preserve `flake.nix`, `flake.lock` and `.envrc`. The shell now includes ADB and conventional SDK/NDK environment defaults; it does not silently install a multi-gigabyte SDK or accept SDK licenses.

```sh
nix develop path:.
npm ci
cargo install tauri-cli --version 2.12.4 --locked
```

Install Android command-line tools through Android Studio's SDK Manager, then select the pinned packages. Accept SDK licenses through the SDK Manager as part of setup. Set `ANDROID_HOME` to your actual SDK location and `JAVA_HOME` to JDK 17 if not using the Nix shell.

```sh
export ANDROID_HOME="$HOME/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"
sdkmanager "platform-tools" "platforms;android-37.0" "build-tools;37.0.0" "ndk;29.0.14206865"
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
rustup component add clippy rustfmt
```

The SDK platform must be recognized as API 37 by the chosen Android Gradle plugin. A toolchain/package mismatch is an initialization blocker, not a reason to silently raise the minimum SDK or remove security configuration.

Resolve the missing app dependency lockfile and retain it in source control:

```sh
cargo generate-lockfile --manifest-path src-tauri/Cargo.toml
npm run tauri -- android init
npm run android:configure
npm run doctor
```

Initialize once. `android:configure` applies API/build-tools/NDK pins, backup/transfer exclusions and network configuration to the generated host, and refuses unfamiliar templates. The maintained platform module is linked by its Tauri plugin build script. The script has idempotence and regeneration-security tests; the actual manifest merge is still unverified.

Once a full build works, review and commit the generated host, wrapper version/checksums and relevant dependency locks; adjust its temporary `.gitignore` exclusion. Exclude local SDK paths, `.gradle`, build output and signing material. Do not hand-edit the generated host as the sole source of a fix: keep changes in the configuration script or native module and reapply them after regeneration.

## Build and check

```sh
npm run check
npm run lint
npm run build
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri -- android build --apk --debug
```

Use a clean debug test installation first. Locate the actual artifact with:

```sh
rg --files src-tauri/gen/android/app/build/outputs -g '*.apk'
adb devices -l
adb install -r path/to/the-reported-debug.apk
```

`npm run tauri -- android dev` is available for development. The debug network XML permits only localhost, 127.0.0.1 and 10.0.2.2 cleartext for the frontend dev server; use ADB reverse on ports 1420/1421 and a loopback dev host where appropriate. A physical-device LAN host needs a deliberate debug-only exception and matching dev CSP. HTTPS remains mandatory for provider content. A bundled debug APK avoids dev-server dependencies during lifecycle tests.

For the required release-build attempt:

```sh
npm run tauri -- android build --apk
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
