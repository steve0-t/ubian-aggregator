# Native account authentication and storage

Updated **2026-11-18**. Native email/password sign-in, account identity/status,
refresh, logout and explicit saved-session actions are implemented. The authorized
host check verified successful sign-in, profile refresh, restoration, server logout
and rejection of the pre-logout session. Android biometric/storage runtime checks
remain unrun. See [account verification](account-verification.md).

## Native account flow

The bundled Svelte form sends email/password once to a dedicated typed Rust
command and clears its controls before awaiting IPC. Rust first reads the observed
`https://www.ubian.sk/login` form, preserves any bounded hidden fields, and submits
its observed URL-encoded `email` and `password` fields to `/login`. There is no
claimed public JSON authentication API. Google/Facebook integration remains
outside this native flow; registration and recovery retain the restricted
provider login entry point.

The observed successful POST redirects to `/eshop` with a query. Rust validates
that this is an exact-origin, bounded success-candidate redirect, discards its
query and does not fetch the shop or replay an account/card action. It independently
reads `/user/profile` and requires the verified `form#js-profile-form` schema.
The identity comes from `#profile-login`, and must match the submitted/restored
login identity. Contact email is a separate field and cannot replace that check.
Only login email and the supplied display name reach the native account summary.
HTTP 200, a cookie or absence of a login form alone cannot prove authentication.

The actual rejected-login response includes an inline `ubian.wrongPassword(...)`
call whose arguments echo submitted credentials. The parser recognizes only the
fixed marker. It never executes, displays, logs or saves the provider script or
its arguments. Authenticated HTML never enters the privileged frontend.

## Session ownership and cached data

The Rust account client owns an independent bounded cookie jar with provider
scope, Secure/HttpOnly/SameSite flags, paths and expiry preserved. It shares no
cookies or credentials with the anonymous timetable client or provider WebView.
Account and guest networking remain independent. Opening provider pages does not
transfer the native session; those pages may require their own sign-in.

The frontend receives typed states (`signed_out`, `signing_in`, `signed_in`,
`cached`, `expired`), the small profile summary, its last verification time and
storage metadata. Cookies and saved-session plaintext have no frontend read
command. A refreshed protected account page establishes `signed_in`; cached
identity alone does not. Offline/timeout restoration can show the previously
saved profile as `cached`, with its original timestamp. Rejected/expired sessions
clear the active profile and cookie jar. No balance, card, transaction or purchase
integration is claimed by this profile-only flow.

## Explicit encrypted session persistence

Sign-in starts in memory. **Save session on this device** explicitly authorizes
writing a provider-issued session and the profile cache, without the password.
**Unlock saved session** requires a fresh supported strong-biometric operation,
then revalidates the session online when available. Persistence is unavailable
on unsupported/unconfigured devices and desktop; manual memory-only sign-in
continues to work.

The new record is separate from the earlier synthetic vault. It uses AES-256-GCM
with a platform-generated 12-byte nonce, authenticated magic/version/purpose and
a 128-bit tag. A non-exportable Android Keystore key requires per-operation
biometric authorization. API 30+ requests strong biometrics; API 29's preliminary
capability check remains provisional. API 26–28 and unavailable biometrics decline
persistence; there is no plaintext or device-credential fallback.

- Alias: `cestou.native-session.v1`.
- File: `noBackupFilesDir/native-session.v1.bin`, atomic replacement, mode 0600.
- Format: `CSES`, version 1, purpose 2; bounded at 256 KiB plaintext. Rust accepts
  at most 128 KiB of versioned session/cache data.
- Snapshot includes exact source, saved/verified timestamps, typed profile and
  provider cookies; the password is absent. Restoring snapshots older than 30
  days is rejected, and provider expiry still applies. Rejected ciphertext stays
  deletable; physical retention lasts until Logout/Forget removes it.
- Unsupported versions, corruption/tampering, missing/invalidated keys and I/O
  failures return local codes. A missing key never silently regenerates a usable
  session for existing ciphertext.

Saved sessions are excluded from Android backup/transfer with both existing rule
formats. The earlier `SyntheticVault` and public Keystore probe remain separate
namespaces; their status does not enable password saving. Real passwords are not
persisted by this implementation.

## Cancellation, lifecycle and cleanup

Rust serializes account mutations. Request IDs, bounded cancellation tombstones
and epochs fence superseded callbacks, including cancellation after authentication
commits while status metadata is being read. Native storage has a process-owned
coordinator, Activity leases and an independent epoch floor to reject delayed
old registrations. Save/Unlock prompts time out; bridge calls also have deadlines.
Cancellation or destruction revokes pending prompts and prevents late writes.

Logout clears local account state regardless of network success, attempts the
observed `/logout` once and verifies revocation using a separate copy of the old
session cookies against the protected profile. The observed expired-session path
is `/user/profile` → `/login_form` → `/login`. If revocation cannot be confirmed,
local logout still completes and the UI reports that limit. Logout removes the
saved encrypted session. Forget removes the native account/session/cache locally
without claiming server logout or provider-account deletion. Cleanup continues
after UI navigation/cancellation and completes before another account operation.
Deletion errors explicitly say saved data may remain and leave a retry available.

The bundled account view requests screenshot/recent-app protection and disables
WebView form saving/autofill for the local credential form. Native prompt and
account-view flag ownership is coordinated. Actual device behavior, OEM backup,
biometric enrollment/lock changes, Activity recreation and crash durability remain
verification work; host substitutes and APK compilation do not establish them.

## Restricted website sessions and other account features

The separate provider WebView still requests cookie/cache/storage cleanup on
open/close. Cookie deletion has a completion callback; complete WebStorage and
service-worker cleanup remains an unresolved device check. Native encrypted
session storage does not encrypt or share WebView-managed state. Website session
remembering, saved-password delivery, profile edits, cards, balances, payments and
social-login session transfer remain separate, unverified work.
