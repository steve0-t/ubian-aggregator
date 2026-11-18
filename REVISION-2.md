# Revision 2 — working native Ubian search for personal use

Date: 2026-11-17

This revision supplements [INSTRUCTIONS.md](INSTRUCTIONS.md) and
[Revision 1](REVISION.md), taking precedence for timetable source selection and
the implementation scope below. Preserve the Rust, Tauri 2, Svelte/TypeScript and
maintained Android plugin architecture, the existing appearance settings, and
Slovak/English/Ukrainian translations.

**Implement this revision on the current branch, `revision-1`.** The revision
number does not request a new branch. Do not create or switch to a `revision-2`
branch or another worktree. Preserve the existing uncommitted research and other
work on this branch.

The current request is to prepare these implementation instructions. This file
does not establish that any of the functionality below is implemented.

## 1. Personal-use scope and source decision

The user is building this application for personal use and does not plan to
publish or distribute it. Prioritize working journey search inside the app.
Publishing, app-store preparation, a developer-operated backend and a nationwide
offline data collection are outside this revision's initial deliverable.

Use Ubian's observed anonymous, read-only timetable interactions as the primary
source. The user authorizes implementing those requests for this personal client.
The absence of official developer API documentation must not, by itself, keep the
adapter disabled or leave the app returning `source_unavailable`. This supersedes
the earlier project-created blanket gate requiring a documented third-party
integration grant before implementing any timetable traffic. It does not assert
a provider endorsement, stable API contract or right to redistribute datasets.
Respect actual provider access restrictions and throttling; do not bypass access
controls or build a whole-site crawler.

[Transport API analysis](docs/transport-api-analysis.md) records observed requests,
response shapes and live examples. Reuse that evidence and revalidate narrowly
where needed rather than restarting general source research. Transitous is not
the default: the user's Dvorianky–Košice example reproduced missing direct buses,
including a missing direction in the community SAD Humenné GTFS conversion.
Self-hosting the same incomplete feed does not resolve that gap.

## 2. First deliverable: native online search

Implement app-owned timetable screens backed by a narrow Rust Ubian adapter.
Native means the bundled Svelte interface inside Tauri; a provider WebView,
browser handoff or remote iframe does not satisfy this requirement. The app can
query Ubian online without first shipping a local timetable database.

The observed paths below all use the exact HTTPS origin `https://www.ubian.sk`.
Keep their transport in Rust and expose typed local commands, not a general
URL-fetch or script-execution command.

| Feature           | Observed method and path                    |
| ----------------- | ------------------------------------------- |
| Stop lookup       | GET `/navigation/autocomplete`              |
| Journey search    | POST `/navigation/connections`              |
| Departure board   | POST `/navigation/stops/planned_departures` |
| Connection detail | GET `/navigation/connection/detail`         |

These are website interfaces, not a promised public API. Build against observed
responses and detect changes explicitly. The guest adapter must not depend on
Ubian login, WebView cookies, saved credentials or completion of the account vault.

### Search and results

- Replace the deliberate `lookup_stops` failure with real stop lookup. Let users
  select an origin and destination by name, preserving provider IDs and names;
  editing a selected name must invalidate that selection until resolved again.
- Add swap, date/time, departure/arrival choice, direct-only search and verified
  transport filters. Keep any city-specific filter tied to actual provider data.
- Submit the observed form-encoded journey request, including nested stop keys;
  do not assume the provider accepts an invented JSON contract.
- Render actual results: departure and arrival, duration, transfers, line/service
  and operator names, walking legs, endpoint stops and supplied platforms. Show
  a selected journey's legs and available detail inside the bundled app.
- Add departure lookup for a selected stop. Preserve search inputs, results and
  scroll position when opening a detail and returning. Do not silently send
  failed searches to a website or substitute fixture results.
- Handle loading, empty results, offline, timeout, cancellation, throttling and
  provider/schema errors with translated local messages. Cancel network work and
  prevent late completions from replacing newer results or canceled state.
- Keep UI text and labels complete in SK/EN/UK, preserve keyboard and TalkBack
  usability, and keep the existing theme/accent behavior.

### Data handling

- Use validated inputs, fixed endpoint paths, verified TLS, constrained redirects,
  bounded response bodies, request deadlines and shared rate/backoff limits.
  Debounce stop lookup and avoid polling or retry loops unrelated to user actions.
- Render provider values as text. Do not inject downloaded HTML or provider
  scripts into the privileged frontend. Retain the local capability and origin
  checks when adding commands.
- Use `Europe/Bratislava` for user-entered timetable times. Parse explicit
  offsets correctly and handle midnight, date changes and both DST transitions.
- Distinguish scheduled data from verified realtime information. An absent
  update or a bare zero delay must not be presented as confirmation of on-time
  operation.
- Investigate the documented connection-detail date mismatch: a request for
  2026-11-18 returned stop timestamps on 2026-11-17. Do not silently shift dates
  or display that response as a verified detail. Keep valid journey search and
  result-leg displays usable while any unresolved detail enrichment is clearly
  marked unavailable. An isolated detail defect must not disable all search.

Acceptance: a guest can select real stops, search a real connection, inspect its
legs, use a departure board and return to the results inside the app-owned UI.
Any remaining detail limitation must be specific and accurately documented.

## 3. Optional next stage: SQLite cache

A local database is useful but is not a prerequisite for the first working
online search. Start with in-memory state. Add persistent SQLite storage only
through explicit user opt-in or a user-initiated save/download action, preserving
the existing default of not recording personal journey history.

For a cache implementation:

- Keep database ownership and queries in Rust, with a versioned schema and
  bounded storage. Keep timetable/cache records separate from account secrets.
- Record the source, fetch time, requested service date, query/filter identity
  and available validity information. Cached queries must match their inputs;
  yesterday's result must not be returned as today's timetable.
- Make saved results available offline with visible cached/stale status. Old
  delay observations must not become live-delay claims.
- Provide clear-cache/delete controls and atomic writes. Define retention and
  backup behavior; follow the existing privacy requirements for persisted
  personal journeys.

A cache supports previously downloaded results. It does not establish the
ability to calculate arbitrary new offline journeys.

## 4. Later stage: offline timetable packs

An embedded database plus Rust routing logic can support genuinely new offline
searches. A pack could be bundled with a personal APK or downloaded separately,
but obtaining and validating a complete dataset is a separate deliverable.
Begin with a useful selected region or corridor rather than all of Slovakia.

An offline pack needs stops, routes, trips, ordered stop times, service calendars,
date exceptions and the transfer/walking connections required by its coverage.
Preserve service-day semantics, including times after 24:00. Implement a
time-dependent journey search; storing rows in SQLite alone is not a planner.

Prefer complete operator/GTFS inputs or a verified, bounded import/conversion
workflow. Ubian search responses can populate a result cache, but a handful of
queried journeys cannot establish complete routes or calendars. Do not invent
a bulk Ubian export endpoint or claim completeness from sampled results. If
Ubian-derived timetables are used, verify the required data is actually available
and suitable for the intended local use before promising an offline pack.

Validate both directions and service calendars before importing a feed. The SAD
Humenné line 073104 defect documented in the analysis must be repaired or replaced
with verified data before that pack can cover Dvorianky → Košice. Do not infer
missing reverse trips or operating days from the forward timetable.

Each pack must identify its coverage, source, build time and validity range,
support atomic replacement and deletion, and clearly handle expiry. Updates
must not require reinstalling the app. Incomplete packs must not turn a missing
route into a claim that no bus operates. Offline packs supply scheduled data;
live delays require a separate online source.

Offline packs are a later enhancement and must not hold up native online search.

## 5. Account features remain independent

Keep account and support entry points in the restricted provider view. Native
guest search and public timetable caches must work without a Ubian account.
Route data cannot supply card balances, account management or purchases.

Retain Revision 1's secure-storage preparation and its verification requirements.
Do not enable real credential persistence merely because timetable requests now
work. Account/session coordination, login-state detection, expiry and cleanup
remain separate implementation work; preserve their accurately stated status.

## 6. Implementation order and verification

1. Implement and test the Rust adapter and typed commands using sanitized public
   fixtures. Cover request encoding, parser failures and bounds, transport errors,
   cancellation, throttling and date/time behavior. Keep credentials and private
   traffic out of fixtures and logs.
2. Connect the native search/results/detail and departure UI to those commands.
   Verify state preservation and failure behavior, not just successful parsing.
3. Exercise a small live guest flow. Include **Dvorianky, č.d.143 → Košice, AS**
   with buses only and transfers disabled, on a date with an active timetable.
   The historical regression example is SAD Humenné line 073104 at
   06:44–07:40 on 2026-11-18; revalidate the service rather than hardcoding it.
   Also check a transfer journey, the reverse direction, no results and arrival
   search. Compare results with the current provider/operator timetable.
4. Run the relevant frontend, Rust and native checks using the existing project
   tooling. Build the Android debug APK and exercise the flow on a device or
   emulator when available. Record exact failures or unavailable device checks;
   host/live-adapter checks do not establish Android runtime acceptance.
5. Update README and the provider-discovery, implementation-status, architecture,
   feature-coverage and privacy documents to reflect actual behavior. Remove
   obsolete blanket source-blocked claims when the adapter works. Distinguish
   online search, cached results and offline routing explicitly.
6. Add optional SQLite persistence after the online flow works. Treat offline
   pack ingestion and routing as subsequent work with their own data verification.

Complete the native online flow before expanding data collection or account
features. A new scaffold, a static database example, mock schedules or another
analysis-only deliverable does not satisfy implementation of this revision.
