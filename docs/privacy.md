# Privacy status

ubian-aggregator adds no app analytics, advertising SDK, remote crash reporter or background polling. The spike records only a fixed native error code and a provider lifecycle state in memory. It does not retain full URLs, searches or request payloads. No diagnostic export exists yet.

The native source disables third-party cookies, file/content access, mixed content, browser form saving and autofill for the provider view. It enables Safe Browsing, rejects certificate errors and intercepts WebView resource and service-worker requests. These settings still need Android regression testing.

**There are zero verified optional-tracking rules.** Public-page extraction cannot establish which requests are optional analytics. `policy/network-policy.json` therefore ships an empty `optionalRules` list. The UI states this limitation. No Balanced/Strict/Compatibility selector is offered before its behavior is proven. The request evaluator is implemented and host-tested with synthetic rules; that does not demonstrate tracking reduction on Ubian.

Rules require a normalized exact host, an explicit subdomain choice, an optional path prefix, a category and an evidence description. First-party paths do not automatically bypass filtering. Rust and Kotlin consume the same policy file; the native module bundles it directly as an asset. The shared synthetic URL corpus checks lookalike hosts, userinfo, ports, schemes and parser ambiguity. Add sanitized network evidence and journey/login regressions before shipping a real rule.

Android's [WebViewClient interception APIs](https://developer.android.com/reference/android/webkit/WebViewClient) do not observe every redirect target or traffic surface. Navigation callbacks do not cover every POST. Service workers have a separate client, and cached responses, WebSockets, beacons and nested frames require explicit tests. The presence of callbacks is not proof of full filtering. No protection is claimed for a system browser or another application.

The provider view requests cookie/cache/web-storage deletion before opening and after closing. Cookie deletion is awaited and timed out; requests are blocked during cleanup. General WebStorage deletion is requested but its completion and service-worker behavior are unresolved device checks. Android's sandbox may contain temporary or persistent web state. This is not an incognito guarantee and vault encryption does not encrypt WebView files.

Ubian and necessary providers receive requests and IP addresses and may collect activity server-side. Reducing optional website tracking cannot prevent that, all fingerprinting or provider account/payment processing. No consent dialog is automatically accepted. Location is currently denied, and local journey history/favorites are not implemented or transmitted.
