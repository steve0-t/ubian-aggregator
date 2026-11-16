# Implementation status — 2026-11-14

This is the initial implementation snapshot. Android tooling and build blockers listed here were subsequently resolved for an ARM64 debug build; see the [2026-11-16 build record](android-build.md#verified-debug-artifact--2026-11-16). Device validation and the architecture gate remain open.

**Milestone 1 is in progress and blocked on Android build/device validation.** No milestone is marked complete and no APK is available. Per the plan's architecture gate, broad guest-data, authenticated, commerce and release functionality has not been built on an unproven Android integration.

## Independent work implemented

- Bundled client-rendered Svelte/TypeScript/Vite application, Slovak/English interface, original ubian-aggregator name/icon, development-state notices and narrowly typed native wrappers.
- Tauri 2 host source with a guarded local main view, restrictive CSP and a three-command local capability.
- Maintained Rust/Kotlin Android plugin source with a non-exported provider Activity, a fresh platform WebView without an application bridge, native Close/Back/inset/error controls, minimal permissions and TLS/origin rules.
- Shared versioned request policy, native WebView/service-worker hooks, Rust/Kotlin matching implementations and sanitized malicious-URL fixtures. No analytics hosts have been invented or blocked.
- Public-data Keystore encryption probe with authenticated records, atomic backup-excluded storage, explicit failure behavior and instrumentation source. Credential saving/reuse is unavailable.
- Repeatable generated-host configuration, Nix environment extension, diagnostic command, frontend/core lockfiles, security configuration tests and coverage/privacy/authentication documentation.

## Verification actually run

| Check                                                     | Result                                     | Scope                                                                                                          |
| --------------------------------------------------------- | ------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| `npm ci` using local cache                                | Passed                                     | 53 packages; generated package-lock included                                                                   |
| `npm run check`                                           | Passed, 0 errors / 0 warnings              | Svelte/TypeScript and Vite configuration                                                                       |
| `npm run build`                                           | Passed                                     | Bundled frontend only                                                                                          |
| `npm run lint`                                            | Passed                                     | Prettier formatting                                                                                            |
| `npm test` / direct Node test run                         | Passed, 3 assertions groups                | Local capability/CSP, honest initial policy, idempotent secure host configuration and unknown-template refusal |
| Core Rust tests                                           | Passed, 4 tests                            | 27 URL cases, entry destinations, policy validation, path/subdomain rules                                      |
| Core Clippy, all targets, warnings denied                 | Passed                                     | Matched Rustup rustc/Clippy binaries; offline locked dependencies                                              |
| Rust formatting                                           | Passed                                     | App, plugin and independent core                                                                               |
| `npm run check:kotlin`                                    | Passed with a platform deprecation warning | Five native source files against Android API 37.2 jar; 27 shared URL cases plus synthetic path/subdomain rules |
| Full app Rust Clippy/tests/lock resolution                | Blocked                                    | Offline cache has no `tauri` package                                                                           |
| Android init and Android APK build                        | Blocked                                    | `cargo tauri` unavailable                                                                                      |
| Keystore instrumentation / Android integration            | Not run                                    | No usable ADB/device connection or NDK/Android Rust targets                                                    |
| Authenticated login / payment / actual tracking reduction | Not run                                    | No account, network capture or payment test flow                                                               |

## Reproducible blockers

Commands below were attempted in this workspace. Node commands used an available Node 22 store binary because the initial shell had no `node`/`npm` on PATH. Their dependencies were installed from a writable copy of the existing npm cache. The original user cache was not modified.

| Reproduction                                                                 | Observed failure                                                                                    | Required environment change                                                                                                                 |
| ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `git status --short`                                                         | `fatal: not a git repository`                                                                       | The session exposes a read-only placeholder `.git`; real Git metadata is unavailable. No repository initialization or commit was attempted. |
| `nix develop --command …`                                                    | Nix treats the placeholder as a Git repository and cannot open it                                   | Use a real checkout or the explicit `path:.` flake reference                                                                                |
| `nix develop path:. --command …`                                             | Cannot connect to `/nix/var/nix/daemon-socket/socket`: `Operation not permitted`                    | Nix daemon access in the executing environment                                                                                              |
| `curl -I --max-time 15 https://registry.npmjs.org/svelte`                    | `Could not resolve host`                                                                            | Dependency-download network/DNS access; the separate web reader can inspect public docs but does not supply build dependencies              |
| `cargo generate-lockfile --manifest-path src-tauri/Cargo.toml --offline`     | `no matching package named tauri found`                                                             | Network access or a complete reviewed Tauri vendor/cache set; generate and commit the genuine app Cargo.lock                                |
| `cargo clippy … src-tauri/Cargo.toml … --offline` / `cargo test … --offline` | Same missing `tauri` package                                                                        | Same dependency prerequisite; no full-app Rust success claimed                                                                              |
| `npm run tauri -- android init`                                              | `error: no such command: tauri`                                                                     | Install pinned Tauri CLI, configure SDK/NDK and Android targets                                                                             |
| `npm run tauri -- android build --apk`                                       | Same missing CLI                                                                                    | Complete initialization and full build prerequisites; then run APK build                                                                    |
| `rustup target list --installed`                                             | Only `x86_64-unknown-linux-gnu`                                                                     | Install planned Android targets on a writable, network-enabled Rustup setup                                                                 |
| `adb devices -l`                                                             | Daemon cannot install its socket listener: `Operation not permitted`; USB initialization also fails | ADB/USB or emulator access outside this restricted session                                                                                  |

An additional host-tool mismatch initially caused Clippy E0514: dependencies used Nix rustc (LLVM 21) while Clippy came from Rustup (LLVM 22), both labelled Rust 1.95.0. It was resolved for the core check by selecting the Rustup `rustc` explicitly and using its Cargo/Clippy with a separate target directory. Keep toolchain binaries consistent; do not fix this by editing source or ignoring warnings.

## Next actions, in plan order

1. Complete the setup in [android-build.md](android-build.md), resolve the genuine app lockfile, initialize/configure the host, compile the Tauri-facing Rust/Kotlin plugin, inspect merged resources and produce a debug APK.
2. Prove the view/Back/IME/rotation lifecycle, bridge exclusion, actual Keystore behavior, request hooks and web-state cleanup on Android. Evaluate completion-capable AndroidX browsing-data deletion and feature support before enabling account switching. Record device/OS/WebView versions and actual SDK/Gradle/NDK/ABI/artifact details.
3. Close the architecture gate, then implement milestone 2 local saved data/guest functionality. Collect sanitized dependency/optional-request evidence before adding real Balanced rules.
4. Use an authorized account through a secure local method for milestone 3; implement and verify authentication, expiry/logout/deletion and authentication-bound credential storage/reuse. Do not request credentials through source files or chat logs.
5. Proceed through tested coverage/privacy modes and release validation. Payments need provider test support or a safe pre-payment stop; real purchases and publication are outside routine validation.
