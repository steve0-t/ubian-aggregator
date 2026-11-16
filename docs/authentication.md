# Authentication and storage preparation

There is no authenticated-state coordinator yet. Opening the fixed [Ubian login page](https://www.ubian.sk/login) is a restricted website entry point, not a verified login integration. The local interface does not claim a signed-in account, capture website passwords, fill a form, extract cookies or replay transactions. Guest timetable preparation is independent of account and vault state.

Public inspection showed email/password controls, a provider remember option, social entry points, registration and recovery. No authorized account was supplied and no login attempt was made. Form/CSRF behavior, cookie attributes, authenticated signals, expiry, server logout and supported session transfer still need verification. Social authentication, checkout returns and browser session transfer remain deferred. No OAuth client ID, callback or domain ownership has been invented.

## Implemented synthetic foundation

`crates/journey-core/src/vault.rs` defines internal availability, explicit synthetic save/update, unlock/use and deletion contracts with typed status/failure values. Kotlin implements the synthetic state machine, versioned authenticated record format, platform key/file adapters and an initial biometric authorization adapter. Synthetic unlock verifies a fixed test value entirely in native memory and returns only success/failure. There is no general secret-read API or credential input field.

The local-only `vault_status` command returns availability, whether a synthetic record exists, and `realCredentialsEnabled: false`. Rust forces that last value to false. The process-owned `AccountVault` instance intentionally has no usable authorizer for save/use; real credential operations are not frontend commands. Controlled native tests can construct the internal biometric adapter. Provider pages have no bridge and cannot invoke any vault operation.

The key design requires per-operation authorization for both encryption and decryption. On API 30+ it requests strong biometrics and binds the key to that authorization class with no validity window. API 29 uses a per-use key and CryptoObject prompt; its preliminary capability query does not distinguish weak from strong biometrics, so a positive availability result is provisional and does not prove that key authorization will succeed. API 26–28 and unavailable/unconfigured authentication decline persistence. Device-credential fallback remains unimplemented. Manual provider login remains independent of this scaffold.

The synthetic engine handles canceled/unavailable authentication, busy operations, missing/invalidated keys, tampering, unknown versions, malformed records, nonce reuse and I/O failure with nonsensitive typed failures. Deletion cancels the prompt, advances its generation, attempts key removal before ciphertext removal, and rejects late unlock callbacks. These are implementation boundaries, not device-verified guarantees. See [security](security.md) for record layout, migrations and verification scope.

## Session ownership and remember options

Provider sessions belong to the restricted Android WebView. Rust has no HTTP client or shared cookie jar, and browser/WebView sessions are not assumed to transfer. The app offers separate, disabled **Remember session** and **Save login credentials** controls; both start off. Neither can currently enable persistence or reuse.

Remember session would retain provider-issued WebView state across launches, subject to a verified cleanup and backup design. Save login credentials would persist an explicit user-provided Ubian login in an authentication-bound encrypted record, then release it only for a separately verified sign-in action. One option must not imply the other. The wrapper does not intercept the provider's own remember checkbox, so its behavior also needs device inspection.

Encrypted application records do not establish that WebView cookies, caches, IndexedDB, local storage or browser-managed forms are encrypted with the vault key. Current normal open/close requests cleanup, and a new provider opening requests cleanup again after process death. Cookie removal has a completion callback; WebStorage deletion does not prove that every artifact was removed. The current code is not a complete logout implementation.

## Required cleanup coordinator

The following lifecycle is specified for later implementation; it is not an available user flow yet.

1. Enter a serialized cleaning state and increment an account/session generation before accepting further work. Block new authenticated requests, provider navigation and vault saves/uses; cancel active requests, pending unlocks and any credential-delivery operation. Every callback must check its captured generation before updating UI, opening a view or writing storage.
2. Stop ordinary provider work and clear account UI/read caches. If online and a provider logout mechanism has been verified, attempt it once using the owned session before destroying the provider view and session material. Then destroy the view. Distinguish confirmed server revocation from local logout; an offline or failed server call must not prevent local cleanup or trigger a replay of state-changing work.
3. Complete cookie deletion/flush and all verified website storage, cache and service-worker cleanup. Wait for asynchronous completion and verify unsupported surfaces explicitly. The existing `ProviderSession` callback gate is only part of this requirement. Fail closed for account reopening/switching when cleanup cannot be established.
4. Apply the requested local deletion scope. Cancel native vault work and revoke its generation before removing keys/ciphertext; attempt all cleanup steps even if one fails. Return only nonsensitive, retryable status. Do not allow a late save, cookie callback or request to recreate deleted state.
5. Release the cleanup gate only after all required local operations complete. Cold-start recovery must finish any interrupted cleanup before opening another provider session. A restored record without its original key requires fresh sign-in; never silently regenerate a key and treat the old account as usable.

| Action                | Required local scope                                                                                                                   | Saved login policy                                                                 | Server/provider scope                                                           |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Logout                | Cancel authenticated work/unlocks; clear local session, account UI/caches and provider website state                                   | Offer an explicit retain/remove choice; retain only by user choice and keep locked | Attempt verified logout when possible; report unconfirmed revocation separately |
| Forget account        | Logout cleanup plus all account records, session material and associated aliases; block account switch until website cleanup completes | Always remove                                                                      | Does not delete the Ubian account or provider-held data                         |
| Delete all local data | Forget account plus app preferences, favorites, recents and diagnostics; stop preference writers before clearing                       | Always remove; clean synthetic/public test artifacts too                           | Does not delete the Ubian account or provider-held data                         |

No local favorites, recent searches, account cache or diagnostic history is currently persisted by the bundled UI. The existing preference writer must be coordinated before an eventual Delete all local data flow, so reactive saves do not recreate settings during cleanup.

## Gates before real account storage

Real saving and reuse must stay unavailable until all of these pass:

- Verify authenticated signals, expiry, invalid credentials, cookie/CSRF behavior, logout and account switch with an authorized test account.
- Prove authentication-bound key use on devices: save/update/unlock prompts, canceled prompts, lock/enrollment changes, missing/invalidated keys, tampering, crash/interrupted writes, backup/transfer/restore, deletion and concurrent cleanup.
- Complete website session cleanup independently of vault encryption, including process death and in-flight request races.
- Implement an explicit local save/update action, clear input state promptly, and prove fixed origin/form/navigation-checked credential delivery. Never scrape provider password controls or expose arbitrary scripts/plaintext reads.
- Inspect known synthetic secrets in app files, logs, events, URLs and diagnostics. JVM memory cannot be promised immediate complete erasure.

No Android device is currently connected. Host fixtures and JVM substitute tests support preparation only; they do not establish biometric authorization, safe credential reuse or account-storage completion.
