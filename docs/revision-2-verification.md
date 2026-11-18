# Revision 2 verification

Date: **2026-11-17**. These are bounded guest timetable checks for the personal
client. They do not establish Android runtime acceptance or an API stability
guarantee. No account, credentials, cookies or private capture were used.

## Detail date investigation

The downloaded public website script's `getConnectionTripsDataParams` passes
each result leg's `fromDepartureTime` directly as `tripStartDate`. Repeating that
exact request shape reproduced the defect:

| Trip                                          | Requested result departure                 | Detail endpoint departure |
| --------------------------------------------- | ------------------------------------------ | ------------------------- |
| 1040022799, line 102515, trip 1, company 1040 | Bratislava, AS, 2026-11-18 09:30 +0100     | 2026-11-17 09:30 +0100    |
| 1002093022, line 073104, trip 6, company 1002 | Dvorianky, č.d.143, 2026-11-18 06:44 +0100 | 2026-11-17 06:44 +0100    |

One diagnostic request using `2026-11-18T06:44:00+01:00` on the second trip
returned the same wrong day. A new date encoding therefore did not resolve the
defect. The adapter checks the selected leg's actual endpoint timestamp against
the response and rejects this enrichment as `detail_unavailable`. It never
shifts the returned day. Search result legs remain available inside the app.
The zero delay and uninitialized communication timestamp in these responses do
not confirm realtime or on-time operation.

## Operator comparison

The [operator's current line 073104 PDF](https://www.sadhe.sk/linky/073104.pdf)
was fetched again on the verification date. It is valid from 2026-09-01 through
2026-12-12. Its second page, trip 6, shows Dvorianky, č.d.143 at **06:44** and
Košice, AS at **07:40**, with the working-day service marker. Wednesday
2026-11-18 falls within that validity period. This verifies the specific direct
bus regression against the current operator schedule; it does not verify every
returned journey or nationwide coverage.

## Repeating the adapter checks

The opt-in example exercises the production Rust client, rather than injecting
fixtures or calling a separate HTTP implementation:

```sh
cargo run --manifest-path crates/journey-core/Cargo.toml --example check_ubian_live -- 2026-11-18
```

It makes a small serial set of stop lookups, direct and reverse bus searches,
arrival search, a journey with transfers allowed, an empty-result candidate,
a departure board and one detail enrichment request. Requests are deliberately
spaced, and the example does not retry or poll. Supply a date with an active
timetable when repeating it later. Current service times are printed and are
never used as runtime fixtures.

The production Rust adapter completed this flow successfully on 2026-11-17 for
service date 2026-11-18:

| Check                                     | Observed adapter result                                                                                         |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Autocomplete                              | Resolved Dvorianky, č.d.143 to 1321 and Košice, AS to 8195 by name                                              |
| Direct buses, after 06:00                 | 10 journeys; first 06:44–07:40, then 07:44–08:40 and 08:44–09:40, line 073104, SAD Humenné a.s.; zero transfers |
| Reverse direct buses, after 06:00         | 10 journeys; first Košice 06:20 → Dvorianky 07:12, line 073104                                                  |
| Arrival by 08:00                          | Three journeys, arriving at 06:10, 06:40 and 07:40                                                              |
| Bratislava → Trnava, transfers allowed    | 10 journeys; seven with transfers, including 09:40–11:28 via lines 632 and 666; walking legs preserved          |
| Dvorianky → Trnava, Veterná, direct buses | Empty result                                                                                                    |
| Dvorianky departure board, after 06:00    | 23 departures, beginning 06:03 line 807419, 06:13 line 811431 and 06:38 line 807419                             |
| First direct journey detail               | `detail_unavailable`; mismatching enrichment rejected while valid search legs remain available                  |

The initial sandboxed invocation returned `offline` because network resolution
was restricted. The explicitly approved network-enabled invocation completed
with exit status 0. This is a host integration check, not an Android device test.

## Access and data limitations

An additional diagnostic journey request with `filter=train` from the rural
bus-only stop returned bus legs. The website advertises `bus`, `train`, `urban`
and their combinations; the observed endpoint may ignore a filter for some
stop selections. Requested transport and transfer constraints must therefore
be checked against returned trips. An empty response means the source supplied
no matching result; it is not proof that no service operates.

Raw anonymous response bodies and the operator PDF used for diagnostics remain
in `/tmp`. Only sanitized public fixtures needed for parser regression tests
belong in the repository.

## Host and interface checks

The following checks passed for the completed source:

- `npm run check`: Svelte/TypeScript, zero errors and warnings.
- `npm test`: preference, startup, translation/time/state and capability/backup checks.
- `npm run build`: final production frontend bundle, embedded in the APK.
- `npm run lint`: all repository formatting checks passed.
- Core Cargo tests: 21 tests, including observed fixtures, nested form encoding, body/schema bounds, HTTP status/content-type/redirect failures, actual HTTP cancellation and connection closure, shared backoff, stale departures, midnight, offsets and both DST transitions. Loopback-listener tests required running outside the sandbox; the initial sandbox refusal was `Operation not permitted`.
- Core and full Tauri application Clippy with warnings denied; Rust formatting for app, plugin and core. Full app host compilation also passed.
- Chromium appearance checks and `npm run check:timetable-browser`: selection invalidation, swap/debounce races, typed requests, result/detail Back with preserved scroll, unavailable enrichment retaining legs, cancellation/late completion, invalid replacement submission, offline/empty states and departures. The timetable form also fits 320 px with 200% text. Browser IPC fixtures are confined to the test harness; they are not a runtime data source.

Run the browser suites against the production preview with an existing Playwright/Chromium installation:

```sh
npm run preview -- --host 127.0.0.1 --port 1422
PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs CHROMIUM_EXECUTABLE=/absolute/path/to/chromium npm run check:browser
PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs CHROMIUM_EXECUTABLE=/absolute/path/to/chromium npm run check:timetable-browser
```

Browser checks do not establish Android WebView, hardware Back, TalkBack or native networking behavior. `adb devices -l` returned an empty connected-device list. No device installation, instrumentation, account login or payment was performed.

## Android debug artifact

The current Revision 2 ARM64 debug APK built successfully with:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 npm run android:build -- --debug --target aarch64
```

The build overrides omit Rust debug symbols and incremental objects to conserve disk space; this remains the Android debug variant. The first sandboxed build could not resolve `static.crates.io`; the approved network-enabled build completed with exit status 0.

- Artifact: `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`
- ABI: `arm64-v8a` only; minimum SDK 26, target/compile SDK 37.
- Size: 123,629,155 bytes.
- SHA-256: `5cc95aa36980fbdf7ab06640aedd78364a8c614624083a1c9557e4b727a1310f`.
- `apksigner verify --verbose` passed (debug v2 signature); `zipalign -c -P 16 4` passed.
- Packaged native library is byte-for-byte identical to the final compiled library and contains the final frontend asset marker `index-t7-U8Z9b.js` plus the new native search command.
- APK has native library and DEX files; bundled network policy matches the maintained JSON. Manifest inspection confirms backup disabled, cleartext disabled and the provider Activity not exported. Both backup-rule formats and network-security XML resources are packaged; exclusions cover app data for backup and device transfer.

The alignment check is a packaging check, not execution on a 16 KB-page device. No Android device/emulator was available for install, networking, TalkBack, hardware Back, lifecycle, native Keystore or restore tests. Other ABIs and release variants were not built. The APK stays outside Git.
