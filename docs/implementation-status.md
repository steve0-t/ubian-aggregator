# Implementation status — native timetables and accounts, 2026-11-18

**Native online guest timetables are implemented; Android runtime acceptance remains unverified.** The app uses Ubian directly through typed Rust commands and bundled Svelte screens. It does not require login, a backend, a local timetable database or completion of the account vault. The current branch remains `revision-1`. Native email/password account integration was added after Revision 2 at the user’s request; see [account verification](account-verification.md). [Revision 2](../REVISION-2.md) authorizes the observed anonymous read-only interactions for personal use and supersedes the earlier blanket source-access gate.

## Implemented behavior

- Real stop lookup with debouncing, selected provider IDs/names, invalidation on edit and origin/destination swap.
- Date/time in `Europe/Bratislava`, departure/arrival selection, direct-only search and observed transport filters. DST gaps are rejected and repeated autumn-hour occurrences are selectable. City-specific filters are not exposed without verified city data.
- Form-encoded journey requests with nested stop keys. Results include scheduled departure/arrival, duration, transfers, service/operator, endpoints, supplied platforms and walking legs. Detail navigation preserves results and restores scroll.
- A selected-stop departure board inside the bundled UI. No fixture results, automatic browser fallback, remote iframe or provider scripts in the privileged view.
- A shared Rust client with fixed HTTPS origin/paths, verified TLS, no redirects/cookies, validated inputs, bounded bodies/collections, deadlines, cancellation, shared rate limits and Retry-After backoff. Local messages cover loading, empty results, offline, timeout, cancellation, throttling and schema/provider failures. Generation checks reject late UI completions.
- Complete SK/EN/UK local labels and errors; existing appearance, accent, startup palette and restricted account/support view retained. Provider names render as text.
- Searches/results remain in memory. There is no SQLite cache, saved-result download or offline routing. These remain optional later stages, with explicit persistence opt-in and independently verified data coverage required.

## Specific detail and data limits

Ubian's connection-detail endpoint still returns previous-day stop timestamps for the tested 2026-11-18 trips, including the Dvorianky regression bus. The client checks enrichment against its originating result and rejects a date/time mismatch. It never silently shifts the stop sequence. Valid search results and legs remain usable, with an explicit detail-unavailable message. An isolated detail failure does not disable search or departures.

Responses do not establish verified realtime operation. Scheduled data stays labelled scheduled; an absent update or zero delay does not imply on-time service. Transitous is not the default: the preserved [source analysis](transport-api-analysis.md) reproduced missing Dvorianky–Košice direct buses and missing stop times in the converted SAD Humenné feed. Self-hosting that feed would retain the defect.

## Verification

The current ARM64 debug APK built and passed signature, ZIP alignment and packaged manifest/backup/asset checks. Core tests (21), full app/core Clippy, Svelte/TypeScript, Node tests, browser regressions, formatting and the live adapter flow passed. Exact commands, artifact/hash and limits are recorded in [Revision 2 verification](revision-2-verification.md). Fixture/parser/transport tests and a host live guest flow are distinct from Android networking and accessibility acceptance. No connected device was found with `adb devices -l`; Android touch, Back/lifecycle, TalkBack, system bars, offline behavior and native guest flow still require a device or emulator.

Earlier Revision 1 checks passed Svelte/TypeScript, production build, Node tests, Rust tests/Clippy, Kotlin host substitutes and Chromium theme/language/layout checks. Its APK attempt failed with `No space left on device`; the earlier [November 16 APK](android-build.md#verified-debug-artifact--2026-11-16) predates both revisions. Those historical artifacts do not verify this change. The Vite HTML scan remains limited to `index.html` to avoid scanning generated Android reports and Nix links.

## Native account implementation and remaining device work

The bundled account form and dedicated Rust client now support email/password sign-in, verified profile identity, refresh, expiry, logout/Forget and explicit saved-session actions. The authorized production-client host flow passed login, refresh, session export without the password, restoration, server logout and rejection of the previous cookie identity. No account edits, cards or purchases were attempted.

The real-session vault is separate from synthetic preparation: opt-in provider cookies/profile cache, AES-GCM, authentication-bound Android Keystore key, private atomic file and strong-biometric save/unlock. The UI never receives cookies or saved-record plaintext. Cached data cannot establish an active session. Password saving remains unavailable. Provider WebView sessions remain independent, with their existing cleanup limitations.

Core tests, browser account failure/cancellation checks and native synthetic-key storage tests support the implementation. Android Keystore/biometric/restore, Activity recreation, screenshots, physical file durability and the full device guest/account flow remain unrun without a connected device. See [authentication](authentication.md), [account verification](account-verification.md) and [the device backlog](vault-device-checks.md). Cards, balances, account edits, social login and payments remain outside the implemented native account summary.
