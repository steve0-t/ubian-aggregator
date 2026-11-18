# Native account verification

Date: **2026-11-18**. A dedicated user-authorized test account was used locally.
No credential, real profile value, session cookie or authenticated response body
belongs in source, fixtures, diagnostics or this record. The local credential
file is Git-ignored and access-restricted. Public endpoint/selector names below
are the contract evidence; test fixtures use synthetic values.

## Verified live behavior

The bounded opt-in `check_ubian_account` example uses the production Rust client.
It reads a JSON login request from stdin and prints only validation flags. The
completed run returned:

```text
login: verified_identity=true
refresh: signed_in=true
session_export: password_absent=true
restore: verified_identity=true
logout: server_confirmed=true, local_cleared=true
revocation: previous_session_rejected=true
```

The observed form is GET/POST `/login`, with `email`, `password`, optional
`rememberme` and no hidden field in the inspected version. The adapter preserves
bounded hidden fields if introduced. Successful POST returns a same-origin
`/eshop` redirect with a query; its query is discarded and that destination is
not fetched. Authentication is verified by the protected `/user/profile` form,
using the account login field and matching the expected identity. An empty test
account's website ultimately lands on `/card/new`; the adapter never loads or
submits that form.

A rejected test submission returned the provider's inline `ubian.wrongPassword`
marker. Its echoed credential arguments were neither logged nor kept in fixtures.
The native parser treats this as invalid credentials and does not execute provider
scripts. The first probe's file-label parser was corrected before successful
verification; no password guessing or reset was performed.

A protected profile without a valid session redirects through `/login_form` to
`/login`. Logout confirmation deliberately reuses an isolated copy of the
pre-logout cookies, so deletion of a local cookie alone cannot establish success.
The provider rejected that old session after logout in the completed run.

No account/profile changes, card creation, purchases, recovery submission or
payment were made. No Google/Facebook authentication or WebView/native cookie
transfer was attempted.

## Repeating a deliberate check

```sh
cargo run --manifest-path crates/journey-core/Cargo.toml --locked --example check_ubian_account < /secure/path/to/test-login.json
```

The input is a bounded JSON object with `email` and `password`; do not supply
secrets as command-line arguments. The example performs a small serial set of
requests, without polling/retries, and signs its test session out. Ordinary unit
and browser tests do not contact the account service.

## Verification limits

The live result establishes the tested host account/session contract. It does
not verify the Android biometric prompt, Keystore key authorization, screenshot
protection, Activity recreation, filesystem crash durability or OEM backup/restore.
Native storage tests use synthetic values/substitute keys, and Android
instrumentation is compiled rather than executed until a device is available.
The native profile is login email/display name only; cards, balances, edits and
payments remain outside this implementation.

## Completed source and build checks

- `npm run check`: zero Svelte/TypeScript errors or warnings; production build
  and `npm test` passed.
- Core Rust tests: 33 passed, covering request encoding/CSRF preservation,
  identity/schema checks, cookie attributes/expiry, snapshot bounds/age,
  cancellation, backoff, cached restoration and protected-page logout/expiry.
- Account plugin tests: 3 passed, covering cancellation before registration,
  delayed old cancels, bounded metadata, authentication rollback and cleanup
  continuing after cancellation/navigation.
- Core and full application Clippy passed with warnings denied; Rust formatting
  passed for app, plugin and core.
- Chromium account tests passed: login/status, save/unlock, cached/offline/expired
  states, password clearing, late completions, logout/forget, deletion-failure
  retry and no account data in browser storage. Appearance and timetable browser
  regressions also passed. IPC fixtures exist only in the test harness.
- The configured Kotlin host check passed the existing policy/synthetic-vault
  checks and new real-session codec/coordinator harness with substitute keys.
  It covers bounds, tampering, key loss/invalidation, nonce variation, wiping,
  callback failures, queued cancellation/deletion and native epoch ordering.
- The maintained Android plugin and instrumentation sources compile. This does
  not execute instrumentation or verify actual Keystore/biometric behavior.
- The test credential values were absent from non-ignored source/doc/test files;
  temporary authenticated HTML captures were removed.

## Current ARM64 debug APK

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 npm run android:build -- --debug --target aarch64
```

The initial sandboxed attempt compiled Rust but could not open Tauri's local
build socket (`Operation not permitted`). The approved build outside the sandbox
completed successfully. Debug-symbol/incremental overrides conserve disk space;
the artifact remains a debug variant.

- Artifact: `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`.
- ABI: `arm64-v8a`; size 123,629,155 bytes.
- SHA-256: `ef70cdbe4ca0bf4ef2634e21e4c4d1adaba17d7295e05bcaf46c3a0fd38527de`.
- Debug signature verified with `apksigner`; `zipalign -c -P 16 4` passed.
- Packaged native library matches the final compiled library and contains the
  final frontend marker `index-DsxKvz5h.js` and native account commands.
- Native library/DEX and backup/network XML resources are packaged; bundled
  network policy matches source. Backup/transfer exclusions remain maintained.

`adb devices -l` on 2026-11-18 returned no connected device. No install, Android
runtime login, biometric save/unlock, TalkBack or restore test was performed.
Other ABIs, release signing and publishing remain outside this personal build.
