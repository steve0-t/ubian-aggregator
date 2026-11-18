# Security boundary and remaining work

The trusted frontend is bundled Svelte. Narrow local-only commands cover guest timetables, account sign-in/status/session actions, appearance, fixed provider entry points and storage diagnostics. There is no generic HTTP, filesystem, shell, script or secret-read command. Every Rust account command repeats the main-view/origin check. Raw internal mobile restore/store commands are absent from public permissions. The provider Activity has no Tauri or application bridge.

Native guest timetables use the narrow Rust Ubian adapter authorized for personal use by Revision 2. Typed local-only commands retain caller origin/label checks; no generic fetch or script execution command is exposed. The adapter uses fixed HTTPS paths, verified TLS, no redirects or account cookies, bounded responses, deadlines, cancellation and shared throttling/backoff. Sanitized public fixtures exist only for tests. Provider text renders as text, never downloaded HTML or scripts in the privileged interface. See [provider discovery](provider-discovery.md).

## Native authentication and saved sessions

The local form clears its fields before awaiting IPC. Passwords have no Debug/Serialize/output/storage contract in Rust and are held in zeroizing wrappers where practical. Provider login responses can echo credentials in inline error scripts; those bodies stay in bounded Rust memory, are never executed/rendered/logged, and only a fixed error marker is recognized. Successful login requires the verified protected profile schema and matching login identity. Provider text reaches the frontend as typed text only.

A dedicated account cookie jar preserves provider flags/scope/expiry and shares no state with guest timetables or the provider WebView. An explicit saved-session snapshot includes provider cookies, a minimal profile cache and source/version/timestamps, without the password. It is encrypted in `noBackupFilesDir/native-session.v1.bin` with an authentication-bound, non-exportable Keystore AES-256-GCM key (`cestou.native-session.v1`). The `CSES` v1/purpose 2 header is authenticated; the nonce is fresh and platform-generated; file writes use atomic replacement and 0600. The record is distinct from synthetic diagnostics. Missing/invalidated keys, corruption, unsupported versions and failed authorization decline access without plaintext fallback.

Save/Unlock require supported strong-biometric authorization per operation; manual login works without persistence. Mutations are serialized, request tombstones/epochs fence canceled login and native prompt races, and local deletion continues after UI cancellation. Mobile bridge calls are bounded, with prompt timeout/cancellation. Logout verifies server revocation against the previous cookie identity and always clears the local native account. Native secure storage is implemented and host-tested with substitute keys; real device authorization, recreation, restore and file crash durability remain unverified. See [account verification](account-verification.md).

## Public test probe

`ProbeVault.kt` remains a public-data diagnostic. It uses non-exportable Android Keystore AES-256/GCM/NoPadding, a fresh platform-generated 96-bit nonce, a 128-bit tag and authenticated version/purpose data. Its record/file/key namespace is separate from account preparation. It tests round-trip, tampering, nonce variation and missing-key rejection, then removes its public artifacts. Reported hardware backing comes from `KeyInfo`; hardware backing is device-dependent.

**The public probe is not a credential vault.** Its key deliberately does not require user authentication, and a successful probe does not enable real account storage. Its actual Keystore behavior has not been exercised on a device in this revision.

## Synthetic account record

The internal foundation uses only the fixed synthetic value created inside `SyntheticVault`. No website password or frontend-supplied secret enters it. Unlock verifies that value and returns status only; decrypted bytes are cleared in a `finally` block. The initial platform authorizer uses the same Cipher CryptoObject before and after a successful biometric prompt. The process-owned status instance deliberately has no usable authorizer for frontend save/use.

| Field                             | Size                | Version 1 meaning                                   |
| --------------------------------- | ------------------- | --------------------------------------------------- |
| Magic                             | 4 bytes             | `CVAU` (`0x43564155`)                               |
| Version                           | 4 bytes, big endian | `1`                                                 |
| Purpose                           | 1 byte              | `1`, synthetic account-storage preparation only     |
| Nonce                             | 12 bytes            | Platform-generated AES-GCM IV for this write        |
| Ciphertext and authentication tag | Up to 4,112 bytes   | At most 4,096 plaintext bytes and a 128-bit GCM tag |

The complete 9-byte magic/version/purpose header is additional authenticated data. Reads reject malformed lengths, magic, purpose and unsupported versions before use; failed authentication returns no partial plaintext. All writes initialize a new encryption Cipher and use its platform-generated IV. The engine also rejects an IV equal to the previous record's IV. This tests fresh nonce behavior but is not a claim that a short test proves global random uniqueness.

The synthetic alias is `cestou.account-preparation.synthetic.v1`, and ciphertext is `account-preparation-synthetic.v1.bin` inside app-private `noBackupFilesDir`. Public probes use `cestou.probe.*` and a separate probe file. Appearance/language preferences retain their browser-storage record and never hold account data. Saved passwords have no record purpose, namespace, endpoint or reuse path. Native session cookies and the profile cache use the separate real-session record described above; the synthetic status remains synthetic-only.

There is no automatic migration. Unknown versions fail as `unsupported_version` and remain unusable until an explicit authenticated migration is implemented. A future migration must decrypt with the correct old key/purpose only after authorization, validate the old schema, reseal with fresh nonce and new authenticated metadata, and atomically commit before retiring the old key/record. It must preserve the old committed record on pre-commit failure. No password-derived key or plaintext fallback is permitted.

`AndroidVaultFiles` uses `AtomicFile`, file mode 0600, bounded reads and an explicit file-descriptor sync before replacement. It checks the replacement because Android's commit API returns no success value. Interrupted/replacement failure handling needs actual filesystem/device fault testing; JVM substitute tests are not proof of AtomicFile durability. An existing record with a missing key is rejected rather than silently assigned a new key. Restore/key loss requires fresh sign-in in any future real-account flow.

## Authentication and lifecycle

The synthetic key uses non-exportable AES-256/GCM with randomized encryption, authentication required for every encrypt/decrypt operation, and invalidation on biometric enrollment changes. API 30+ binds key use to `AUTH_BIOMETRIC_STRONG` with a zero-second validity window. API 29 uses the per-use CryptoObject path; its capability precheck may accept a device whose available biometric cannot authorize the key, so successful capability detection is provisional. API 26–28 decline secure persistence. Device-credential fallback and equivalence across devices are not claimed; StrongBox is not required. See [Android Keystore](https://developer.android.com/privacy-and-security/keystore) and [biometric authorization guidance](https://developer.android.com/identity/sign-in/biometric-auth).

The initial native prompt handles canceled, unavailable, finishing/destroyed Activity and failed-scan paths and sets `FLAG_SECURE` during the synthetic prompt. Enrollment/lock changes, hardware/OS variations and real authorization must still be verified on devices. Typed failures include authentication cancellation/unavailability, missing/invalidated keys, tampering, unsupported version, malformed record, busy operation, nonce reuse and I/O failure. Errors contain no secret, provider response or raw exception text.

`SyntheticVault` serializes operations and uses generations to reject obsolete authorization callbacks. Delete cancels the prompt, revokes its generation, attempts key deletion before file deletion and attempts both even after partial failure. Its internal API has no arbitrary secret-read or script operation. The host harness covers reentrant callbacks, throwing completions, deletion races, substituted Ciphers and repeated-IV rejection; equivalent device behavior remains unverified. Full session cleanup remains a specified future coordinator, described in [authentication](authentication.md); the synthetic vault does not establish provider logout or website deletion.

## Website persistence and backup

WebView cookies, IndexedDB, local storage, cache and service-worker state remain Android-managed data. They are not encrypted with the application vault key merely because application records are encrypted. The provider wrapper does not export its WebView cookies. Its remembered website session and saved-password reuse remain unavailable; opt-in native session persistence is a separate Rust/native flow. Cleanup is requested before provider open and after normal close; a new opening requests cleanup after process death as well. Cookie deletion has a completion gate, while WebStorage/service-worker completion remains unresolved.

Maintained module and regenerated-host configuration disable backup and supply legacy full-backup exclusions plus Android 12+ cloud-backup/device-transfer exclusions. Both formats exclude credential-protected and device-protected root/file/database/shared-preference domains, and other app domains; `noBackupFilesDir` is an additional exclusion for ciphertext. Host configuration tests check regeneration and rules. Current APK packaging checks are recorded in [account verification](account-verification.md); they do not replace an OEM restore test. Inspect packaged manifest/rules, exercise cloud backup/device transfer, and verify that restored ciphertext without its original Keystore key cannot be used. See [Android backup configuration](https://developer.android.com/identity/data/autobackup).

The Android WebView network configuration trusts system certificates and denies cleartext. The Rust timetable client separately verifies TLS using the maintained Mozilla root store, without platform-verifier JNI initialization. A separate debug XML permits only localhost, 127.0.0.1 and the emulator host for the frontend development server; provider policy still requires HTTPS. Release builds must not inherit a development exception. Login/registration navigation sets `FLAG_SECURE` for the remainder of that provider view. Public local timetable screens are not intentionally screenshot-restricted. Screenshot and recent-app behavior across redirects still need device evidence.

## Verification limits and backlog

The JVM harness uses synthetic values with substitute keys, files and authentication. It covers the shared record/state-machine behavior, not Android Keystore, biometric security, backup behavior or filesystem crash durability. Rust checks cover metadata serialization and the absence of a serializable secret payload. Android plugin and instrumentation Kotlin compile successfully with Gradle. The instrumentation sources are a verification path; no device is currently connected, so device checks remain unrun. See [the focused device backlog](vault-device-checks.md).

Before real storage can be enabled, verify:

- Authentication-bound encryption/decryption round-trip, rejected unauthenticated key use, canceled/unavailable prompts, weak-only biometrics on API 29, enrollment changes, lock changes and key invalidation/loss.
- Record/header/tag tampering, nonce variation, bounded reads, interrupted writes, sync/rename/storage failures, migration refusal and restart recovery.
- Deletion and failed partial deletion, late unlock/save callbacks, concurrent logout/navigation, process death and cleanup retries without resurrecting account state.
- Packaged backup rules, cloud/device-transfer restore without the original key, website-state persistence/cleanup, screenshots, recent-app previews and absence of known synthetic secrets in logs/storage/events/URLs/diagnostics.
- Provider authenticated-state detection and safe, fixed credential delivery independent of vault encryption.

The threat model includes accidental file/backup extraction, ordinary cross-app access and untrusted page content. It does not protect against a compromised OS, compromised running application or the provider receiving a password during login. JVM/JavaScript memory cannot be promised immediate complete erasure. No real credentials or authenticated captures belong in test fixtures.
