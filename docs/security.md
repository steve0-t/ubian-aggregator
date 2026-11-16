# Security boundary and remaining work

The milestone-1 implementation exposes only status, fixed-destination navigation and a public-data Keystore probe. There is no generic HTTP, filesystem, shell, script or secret command. Provider content is never inserted into the bundled frontend. Android-only functionality is unavailable on desktop/browser builds.

## Test-record encryption

`ProbeVault.kt` uses Android Keystore AES-256/GCM/NoPadding with a fresh platform-generated 96-bit nonce and a 128-bit tag. Version 1 and purpose `keystore-probe` are authenticated additional data. A record contains a version integer, nonce and ciphertext/tag. Unknown versions, invalid lengths, authentication failures and missing keys throw; there is no plaintext fallback.

The key is non-exportable through the Android Keystore interface. Ciphertext uses `AtomicFile` in `noBackupFilesDir`; writes use file-descriptor mode 0600, commit atomically or fail back to the prior file. Reads are size-bounded. The public probe checks a disk round-trip, tag modification, nonce variation, missing-key rejection and reported hardware backing, then deletes its file/key. A subsequent probe removes only stale keys in its dedicated public-test namespace after process death. No credential keys exist.

**This is not the saved-credential vault.** The public test key deliberately does not require user authentication. Device/biometric authorization, enrollment/lock changes, key invalidation and authenticated secret reuse must be implemented and tested before storing a real password. The probe never receives a secret from Svelte and returns only booleans. Hardware backing is reported by `KeyInfo`, not assumed. See [Android Keystore](https://developer.android.com/privacy-and-security/keystore).

## Website and Android data

Website cookies, IndexedDB, local storage and caches remain Android-managed data. No cookie export or custom token persistence is implemented. Cookie cleanup has a completion gate; WebStorage/service-worker deletion still needs device evidence. Do not enable remember-session, account switching or saved login while that gate is unresolved.

The maintained module and generated-host configuration disable backup and supply both legacy backup exclusions and Android 12+ cloud/device-transfer exclusions, including credential- and device-protected domains. `noBackupFilesDir` is an additional exclusion. Source configuration tests do not prove an OEM's restore behavior; inspect the merged manifest and packaged rules and test restore/key loss on device. See [Android backup configuration](https://developer.android.com/identity/data/autobackup).

Production network configuration trusts system certificates and denies cleartext. A separate debug XML permits only localhost, 127.0.0.1 and the emulator host for the frontend dev server; provider policy still requires HTTPS. Additional LAN development hosts need an explicit debug-only rule. Release builds must not inherit a development exception.

Login/registration navigation sets `FLAG_SECURE` for the remainder of that provider view. Public journey views are not intentionally screenshot-restricted. Test screenshot and recent-app behavior, including redirects to authenticated pages, before claiming full account-screen coverage.

## Limits

The intended threat model includes accidental file/backup extraction, ordinary cross-app access and untrusted website content. It does not protect against a compromised OS, a compromised running app or the provider receiving a password during login. JVM/JavaScript memory cannot be promised immediate erasure. There are no real credentials or authenticated captures in the test fixtures. Instrumentation tests are present but unrun; the full Tauri/native trust boundary has not been device-tested.
