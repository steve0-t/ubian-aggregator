# Authentication status

There is no authenticated-state coordinator yet. Opening `https://www.ubian.sk/login` is a website entry point, not a verified login integration. The local interface never claims the account is signed in, stores a password, fills a form, extracts cookies or replays a purchase.

Public [login page](https://www.ubian.sk/login) inspection on 2026-11-14 showed email/password controls, a remember option, social entry points, registration and recovery. Parsed page content did not establish form destination/method, CSRF mechanism, cookie attributes, authenticated signals, logout semantics or permitted session transfer. Direct network discovery is blocked in this session. No account was supplied and no login attempt was made.

Before milestone 3:

1. Capture a minimal, sanitized origin/cookie/form inventory on an Android test installation with an authorized account. Preserve provider challenges and CSRF protections. Find a stable authenticated signal and test expiry and invalid credentials.
2. Verify provider logout and local cleanup separately, including concurrency, offline logout, crash/relaunch, WebView storage, cookies and server-session revocation. Never equate closing the view with server logout.
3. Keep remember-session and save-credentials as distinct, initially disabled options. The spike always requests cleanup and cannot enable persistence. Audit any provider-side remember option separately; the wrapper does not intercept its form.
4. Implement an explicit local credential-save action with authentication-bound Keystore protection. No password capture from website controls. Handle canceled authentication, lock changes, key invalidation, key loss and atomic migration before enabling saving.
5. Prove a fixed, origin/form/navigation-checked delivery mechanism. Never add arbitrary script evaluation or a secret-read command. Keep any plaintext in short-lived native/Rust memory and clear local form state.
6. Test deletion and account switching with known nonproduction secrets and inspect app storage/logs. The current public-data encryption probe does not establish saved-login support.

Social authentication, checkout returns and browser session transfer remain deferred. An external-browser confirmation explicitly says that the ubian-aggregator session will not transfer. No OAuth client ID, callback, token endpoint or domain ownership has been invented. The initial integration owns sessions in the provider WebView; Rust has no HTTP client or shared cookie jar.
