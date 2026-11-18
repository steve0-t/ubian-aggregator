# Privacy status

ubian-aggregator adds no app analytics, advertising SDK, remote crash reporter or background polling. Platform status records only a fixed native error code and a provider lifecycle state in memory. Native timetable selections, results, departure boards and short-lived detail tokens also remain in memory for the current app run. It does not write full URLs, searches or request payloads to storage or logs. No diagnostic export exists yet.

The native source disables third-party cookies, file/content access, mixed content, browser form saving and autofill for the provider view. It enables Safe Browsing, rejects certificate errors and intercepts WebView resource and service-worker requests. These settings still need Android regression testing.

**There are zero verified optional-tracking rules.** Public-page extraction cannot establish which requests are optional analytics. `policy/network-policy.json` therefore ships an empty `optionalRules` list. The UI states this limitation. No Balanced/Strict/Compatibility selector is offered before its behavior is proven. The request evaluator is implemented and host-tested with synthetic rules; that does not demonstrate tracking reduction on Ubian.

Rules require a normalized exact host, an explicit subdomain choice, an optional path prefix, a category and an evidence description. First-party paths do not automatically bypass filtering. Rust and Kotlin consume the same policy file; the native module bundles it directly as an asset. The shared synthetic URL corpus checks lookalike hosts, userinfo, ports, schemes and parser ambiguity. Add sanitized network evidence and journey/login regressions before shipping a real rule.

Android's [WebViewClient interception APIs](https://developer.android.com/reference/android/webkit/WebViewClient) do not observe every redirect target or traffic surface. Navigation callbacks do not cover every POST. Service workers have a separate client, and cached responses, WebSockets, beacons and nested frames require explicit tests. The presence of callbacks is not proof of full filtering. No protection is claimed for a system browser or another application.

The provider view requests cookie/cache/web-storage deletion before opening and after closing. Cookie deletion is awaited and timed out; requests are blocked during cleanup. General WebStorage deletion is requested but its completion and service-worker behavior are unresolved device checks. Android's sandbox may contain temporary or persistent web state. This is not an incognito guarantee and vault encryption does not encrypt WebView files.

Native guest timetable actions send typed public stop IDs or stop-name queries, selected timetable date/time and filters directly to Ubian over HTTPS. They send no account credentials or WebView cookies. Lookup is debounced, cancellation stops network work, and shared throttling/backoff prevents automatic retry loops. The adapter keeps no cookie jar, diagnostic response log or persisted query history. No SQLite cache, saved-result download or offline routing pack is implemented. Closing/reloading the app loses timetable results; new searches require connectivity.

Ubian and necessary providers receive requests and IP addresses and may collect activity server-side. Reducing optional website tracking cannot prevent that, all fingerprinting or provider account/payment processing. No consent dialog is automatically accepted. Location is currently denied, and local journey history/favorites are not persisted. User-entered timetable searches are transmitted only when needed for the requested online lookup.

## Native account data

Email/password login is sent directly from Rust to Ubian over verified HTTPS.
The frontend clears its input fields promptly; neither value goes into browser
storage, app diagnostics or fixtures. The password is not retained in the saved
session. Ubian may echo submitted values in an error response; the app never
renders, executes or logs that response body.

Native account cookies, a minimal login-email/display-name summary and the last
verification time stay in memory unless the user explicitly saves the session.
Save writes a separate authenticated encrypted, backup-excluded private file with
an Android Keystore key and supported biometric authorization. Unlock reads it
internally into Rust; the frontend has no cookie or plaintext-record read API.
Restored data stays labelled cached until online verification succeeds. Saved
snapshots older than 30 days are rejected on restore; encrypted files remain until
Logout/Forget deletes them. Provider cookie expiry still applies.

Logout/Forget clears native account state and removes the saved native session;
delete failures say data may remain and permit retry. Native logout attempts
server revocation once and reports when confirmation is unavailable. These actions
do not delete the Ubian account. The independent provider WebView's storage and
its unresolved full-cleanup device checks remain separate. No card/balance,
transaction or payment cache is implemented. See [authentication](authentication.md).
