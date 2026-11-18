# Account-storage device verification backlog

No checks in this file have been run for Revision 1. The user confirmed that no Android device is currently connected. Host JVM tests use synthetic keys and authorization substitutes; they cannot establish Keystore authorization, actual biometric prompts, filesystem durability, lifecycle behavior, or backup/transfer behavior.

`AccountVaultTest.kt` adds Android instrumentation source for the V1 record codec, actual `AtomicFile` rollback/interrupted replacement/deletion, bounded reads, missing-key refusal, and authentication-bound encryption rejection without a `CryptoObject` authorization. Record and file tests deliberately use an ephemeral JCE AES key. The authorization test uses a real Android Keystore key and is skipped when supported enrolled biometrics and a secure lock are unavailable. A skip must be reported separately from a pass.

Run the maintained native library's `connectedDebugAndroidTest` task after identifying its project path with `./gradlew projects` in `src-tauri/gen/android`. Record the commit, test task, instrumented package, device/OS/API, WebView version, biometric class/enrollment, key security level, test counts and skips. Do not use a production account, real password, or provider page for these checks.

## Authentication and lifecycle checks still requiring a device harness

The application intentionally exposes no synthetic save/unlock command to Svelte and no real credential-save/reuse operation. An internal instrumentation Activity must own `AndroidVaultAuthorization` and one `SyntheticVault` instance for each test record. Do not temporarily turn off `setUserAuthenticationRequired` to make these tests pass.

Use a unique alias `cestou.account-preparation.test.<random>` and a unique file in `noBackupFilesDir`; delete both in a `finally` block. The only payload is the scaffold's fixed synthetic record. Invoke each operation explicitly from the test Activity, retain only its typed status, and never print the payload or decrypted bytes.

1. On API 30+ with an enrolled strong biometric, call `saveOrUpdateSynthetic`, approve its prompt, then call `unlockAndVerifySynthetic` and approve a separate prompt. Both operations must succeed, and metadata must keep `realCredentialsEnabled=false`. Repeat a save/update with another approval and verify that its nonce differs. The test must never reuse an already finalized cipher.
2. On API 29, verify the legacy biometric path with a biometric that can authorize a Keystore crypto operation. `canAuthenticate()` alone does not establish strong-crypto support. A device offering only a weaker biometric must decline key use and return a typed unavailable state. API 26–28 currently decline this preparation path; guest use and manual provider login must remain available.
3. Cancel the save and unlock prompts using their negative button and system Back. Expect `authentication_cancelled`, no newly committed file, no changed previously committed record, and no decrypted result returned to the caller. Cancel programmatically while a prompt is opening and after it becomes visible. Check `FLAG_SECURE` restoration when the Activity was already secure and when it was not.
4. Start a save/unlock, then call `deleteSynthetic` before approving the prompt. Approve or dismiss any late prompt callback. Deletion must finish, the operation callback must report cancellation exactly once, and the late callback must not recreate the alias/file. Repeat after Activity rotation, backgrounding, destruction, and process death using a fresh test Activity/coordinator. Do not restore an authorized cipher across process recreation.
5. Save a synthetic record, modify its tag/ciphertext, then approve an unlock. Expect `tampered`; no partial plaintext may escape. Change the purpose or version and verify refusal before use. Unknown versions currently have no migration; the record must remain unavailable until explicitly deleted.
6. Delete the Keystore alias while preserving ciphertext, restart the test process, and attempt unlock and update. Expect `key_missing`, no replacement key, and no sign-in state. Remove the record explicitly before a new save. A new key must never make an orphaned record look like an existing account.
7. With a test record present, add/remove biometric enrollment, remove/change the secure lock, trigger biometric lockout, and restart the device. Record the actual platform failure. Enrollment/lock changes that invalidate the key must produce `key_invalidated` or the appropriate typed unavailable state, preserve no usable session, and require sign-in again. Do not claim this from a simulated exception alone.

## Persistence, cleanup and restore checks

- Kill the test process during replacement before fsync, after fsync but before commit, and immediately after commit. On relaunch, accept only the old complete authenticated record or the new complete authenticated record. Reject truncation and preserve a typed failure. Repeat with an induced full-filesystem/write failure and verify that no plaintext fallback or partially authenticated value is used.
- Exercise deletion with an interrupted replacement and key/file removal failures. Both removals must be attempted; failures must remain visible and retryable. Inspect the app-private record's mode, committed and temporary filenames, and the alias namespace. The existing public `cestou.probe.*` files/keys must stay separate from account preparation.
- Inspect the merged and packaged manifest, legacy `backup_rules.xml`, Android 12+ cloud-backup rules and device-transfer rules for the exact candidate APK. Verify both credential- and device-protected domains. Source XML checks alone do not establish packaged or OEM restore behavior.
- Exercise supported cloud restore and device transfer on disposable test installations. Neither ciphertext, WebView authentication state nor key material should transfer. A copied ciphertext without its original Keystore key must require sign-in again. Report OEM behavior and unsupported restore tooling accurately.
- Inspect known synthetic values in app files, WebView storage, logs, events, URLs, crash diagnostics, screenshots and recent-app previews. This test does not establish immediate JVM memory erasure. WebView cookies/caches/website storage are not encrypted by this vault key.
- Verify logout, Forget account and Delete all local data only after the provider session coordinator is implemented. Cancellation/generation fencing, navigation shutdown and asynchronous website cleanup must complete before another account/navigation can recreate state. Local cleanup must finish even when remote logout cannot be confirmed. These account flows remain gated.

Passing these checks would verify the synthetic preparation on the recorded device. Enabling real account storage still requires verified provider authentication/session cleanup, safe fixed-origin credential delivery, explicit user saving, and the remaining Android architecture gates.

## Native saved-session follow-up — 2026-11-18

The real-session vault is now separate from the synthetic foundation. Run
`RealSessionVaultTest` instrumentation and the native Account flow on an actual
device: explicit Save/Unlock with strong-biometric CryptoObjects, canceled/failed
prompts, Activity/process recreation, enrollment/lock changes, key loss, corrupt
and unknown-version records, atomic file failures, interrupted deletion and
backup/transfer/restore. Check queued/late callbacks and cold-start cleanup;
confirm old-Activity lifecycle callbacks do not cancel a replacement lease.

Inspect screenshot/recent-app protection while typing and while a biometric
prompt is active; confirm local WebView form saving/autofill is disabled. Verify
Save/Unlock failure leaves memory-only sign-in usable. Check cached/offline profile
labels and expiry clearing, and confirm Logout/Forget removes both the native
session record and key even after navigation or partial failure. Host tests with
substitute keys and compiled instrumentation do not establish these properties.
See [account verification](account-verification.md) for the host live evidence.
