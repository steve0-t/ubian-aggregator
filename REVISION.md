# Revision 1 — interface, native timetables, and account storage preparation

Date: 2026-11-16

This file describes the next implementation changes. It supplements
[INSTRUCTIONS.md](INSTRUCTIONS.md) and takes precedence for the requirements below,
including the earlier website-first timetable direction and SK/EN-only interface.
The existing Rust, Tauri 2, Svelte, TypeScript, and maintained Android plugin
architecture still applies. These are implementation instructions, not claims of
completed or device-verified functionality.

## 1. Keep appearance customization in Settings

- Remove the Appearance quick card from the Journeys/main page and the redundant
  palette/customization shortcut in its header.
- Keep theme, accent presets, custom accent color, and appearance reset controls
  together under **Settings → Appearance**, reached through the existing navigation
  menu on desktop and mobile.
- Keep the home page focused on timetable use. Preserve the Account entry point
  and ensure removing the appearance card leaves a balanced, responsive layout.

Acceptance: the main page has no appearance controls or appearance shortcut;
Settings remains accessible from navigation and contains the appearance controls.

## 2. Add Ukrainian translation

- Add `uk` to the supported language types and a complete Ukrainian dictionary in
  `src/lib/i18n/index.ts`. Show **Українська** in the language selector alongside
  Slovenčina and English; retain Slovak as the default.
- Translate all app-owned text, including navigation, accessibility labels,
  validation, errors, timetable states, settings, and storage explanations.
  Include new text introduced by this revision.
- Carry the language through preference validation and persistence, document
  language, TypeScript/native command payloads, the Rust language enum, and Kotlin
  dialogs/provider-view controls. Adding a dictionary alone is insufficient:
  `openProvider` and native handling currently accept only `sk`/`en`.
- Keep dictionary keys aligned and format app-owned dates and times appropriately
  for the chosen language while preserving provider timetable timezone semantics.
  Preserve provider stop names and identifiers. Do not imply the provider website
  itself has a Ukrainian translation; define its language fallback explicitly.

Acceptance: selecting Ukrainian updates the local interface immediately, survives
a restart, and does not break native commands. Check longer labels, narrow screens,
and large text for clipping or untranslated app-owned strings.

## 3. Add a true black background option

- Add **True black / AMOLED** as a distinct option under Settings → Appearance,
  alongside System, Light, and Dark. Give it a stable preference value such as
  `black`; preserve existing saved preferences and the current System default.
- Use actual `#000000` for the main background, navigation, and other base surfaces
  in this mode. Elevated cards and inputs may use subtle tonal differences or
  borders where needed to remain distinguishable.
- Provide readable text, icons, borders, disabled controls, and focus indicators
  with every preset and custom accent. Extend theme tokens and accent contrast
  calculations to cover the black palette.
- Apply the preference before first paint, persist it across restarts, and keep
  it selected when the device theme changes. System mode continues to follow the
  existing light/dark behavior. Match Android system bars where supported.

Acceptance: computed base background colors are `#000000`, the option persists,
and navigation, settings, and timetable screens remain readable in all four modes.

## 4. Fix the blue navigation background on tap and press

Treat the reported blue background behind a selected or briefly held navigation
item as an unwanted interaction effect.

- Reproduce it on Android for both a normal tap and a brief press/hold. Inspect
  browser/WebView tap highlighting, text selection, focus, and CSS active states
  before choosing the fix; the cause is not established yet.
- Remove the unintended blue overlay/selection flash from navigation controls.
  Scope any tap-highlight or text-selection suppression to those controls.
- Retain deliberate selected, hover, pressed, and keyboard-focus feedback using
  the app's theme and selected accent. A user-selected blue accent remains valid;
  avoid a hardcoded blue effect independent of that preference.
- Preserve `aria-current`, visible keyboard focus, TalkBack support, and adequate
  touch targets. Apply the fix to both sidebar and mobile bottom navigation.

Acceptance: tapping, holding, releasing, and dragging off a navigation item cause
no unwanted blue flash or stuck highlight. The selected destination remains clear
in light, dark, and true black modes with multiple accent colors.

## 5. Render timetables inside the application — priority

Replace the current `open("journeys")` website handoff with an app-owned timetable
experience. Here, native means bundled Svelte screens inside the Android app,
backed by typed Rust data handling; a separate Kotlin screen rewrite is unnecessary.
Opening the provider WebView or a browser does not satisfy this requirement.

- First discover and document a permitted, verified source for journey search and
  departures. Prefer an official integration where available. Record observed
  origins, request/response shapes, restrictions, and sanitized fixtures in
  `docs/provider-discovery.md`; do not invent endpoints or assume access rights.
- Implement a narrowly scoped Rust adapter and typed commands for stop lookup,
  journey search, departures, and connection detail as supported by the verified
  source. Validate inputs, constrain destinations and redirects, and use bounded
  requests, cancellation, and rate limiting. Keep networking out of the frontend
  and avoid a generic URL-fetch command.
- Render origin/destination selection, swap, date/time and departure/arrival
  options, supported filters, results, departure boards, and connection details
  in bundled Svelte components. Preserve search inputs, results, scroll position,
  and sensible Back behavior when opening a result and returning.
- Show line/service identifiers, stops, scheduled times, duration, and transfers
  where supplied. Distinguish verified real-time delays from scheduled data;
  missing real-time information must not imply an on-time service.
- Handle loading, no results, offline, timeout, rate limiting, cancellation, and
  provider changes locally. Use `Europe/Bratislava` for timetable semantics and
  cover midnight/daylight-saving transitions. Label cached data with its fetch
  time; avoid unrequested persistence of personal journey history.
- Render provider text safely as text. Do not embed downloaded HTML, provider
  scripts, an iframe, or a remote page in the privileged frontend. Keep the
  existing capability/origin checks when adding typed commands.
- Keep guest timetable use independent of account storage. Account, support, and
  other services may retain their restricted provider flows. Any optional website
  link must be clearly labeled and user initiated; failures must not silently
  redirect timetable users away from the native screens.

Acceptance: on Android, a guest can search for a real connection or stop, view
results/departures, open details, and return without leaving the app-owned screens.
Test adapter parsing against sanitized fixtures and verify a live permitted flow.
If source access is unresolved, document the exact blocker and keep native
timetables marked incomplete; mocked results or the existing handoff do not pass.

## 6. Begin preparation for secure on-device account storage — priority

Prepare the storage foundation and its verification path. Full saved-login reuse
is a later deliverable. The current `ProbeVault.kt` stores public test data with
a key that does not require user authentication; it is not a credential vault.

- Define internal Rust/Kotlin boundaries for vault availability, explicit
  save/update, unlock/use, and delete operations. Return only status and
  nonsensitive metadata to Svelte; provide no general secret-read or script API.
  Keep provider pages unable to invoke these operations.
- Specify a versioned encrypted record format, record purpose, key aliases,
  migration behavior, atomic writes, and typed failure states. Keep account
  records separate from appearance/language preferences and public probe records.
- Build the initial storage scaffold using synthetic data only. Follow the
  existing Android Keystore design: a non-exportable AES-256 key, authenticated
  AES-GCM encryption, a fresh unique nonce for every write, authenticated version
  and purpose, and app-private backup-excluded ciphertext. Never fall back to
  plaintext or derive the encryption key from the Ubian password.
- Define and begin implementing device-authentication/biometric authorization for
  secret use, including cancellation, unavailable authentication, lock/enrollment
  changes, key invalidation/loss, tampered records, and interrupted writes. Keep
  decrypted values short-lived in native/Rust memory and out of logs, events,
  browser storage, URLs, and diagnostics.
- Document session ownership and separate **Remember session** from **Save login
  credentials**, both initially off. Encrypted app records do not establish that
  WebView cookies, caches, or website storage are encrypted with the vault key.
- Specify logout, Forget account, and Delete all local data behavior, including
  cleanup ordering and races with in-flight requests. Audit cloud backup and
  device-transfer exclusions in the maintained Android configuration and packaged
  app; a restore without the original key must require sign-in again.
- Extend host checks and Android instrumentation with synthetic secrets for
  round-trip, tamper rejection, nonce variation, missing/invalidated keys, canceled
  unlock, atomic failure, and deletion. Record unrun device checks accurately.
- Keep real credential saving and reuse unavailable until authentication-bound
  key use, provider authentication/session cleanup, and safe credential delivery
  have passed their existing gates. Never capture passwords from provider pages.

Acceptance for this preparation phase: internal contracts, a versioned storage
scaffold exercised with synthetic data, a session/cleanup design, and focused
tests or an explicit device-verification backlog exist. Update
`docs/authentication.md` and `docs/security.md` to distinguish implemented
preparation from the remaining work before real account storage can be enabled.

## Implementation and verification

Complete the interface changes, prioritize the native timetable flow, and begin
the independent secure-storage foundation without making guest access depend on
login. Preserve the open Android architecture gate from the existing instructions.

For implementation, run the repository's relevant formatting, Svelte/TypeScript,
frontend build, preference/security, Rust, and Kotlin checks. Add focused coverage
for language/theme persistence, provider parsing and timetable state handling,
and storage failure behavior. Validate touch interactions and the timetable flow
on Android; host checks alone cannot establish these behaviors.

Update README and the architecture, feature-coverage, authentication, security, and
implementation-status documents as behavior changes. Separate implemented,
device-verified, and blocked work. Do not mark native timetables or secure account
storage complete based only on this revision file or a successful frontend build.
