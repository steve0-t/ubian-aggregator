# Provider discovery record — 2026-11-16

**Revision 2 — 2026-11-17:** native online guest search now uses Ubian's observed anonymous, read-only website requests for this personal client. [Revision 2](../REVISION-2.md) supersedes the earlier blanket third-party access-grant gate; the lack of official API documentation does not disable the adapter. Actual restrictions and throttling are respected. No provider endorsement, stable contract or redistribution right is claimed. No provider contact, authentication bypass, token extraction, challenge bypass, account submission or payment was performed.

[Transport API analysis](transport-api-analysis.md) preserves the research that led to source selection. Transitous missed the required Dvorianky–Košice direct buses; its community SAD Humenné feed omitted stop times in that direction. The selected Ubian adapter is independent of account storage and provides stop selection, journeys, result legs and departure boards inside the bundled UI. [Current live verification](revision-2-verification.md) distinguishes host adapter checks from Android acceptance.

## Access and discovery history

The limited 2026-11-16 discovery inspected the [homepage](https://www.ubian.sk/), its linked route/timetable scripts, [robots.txt](https://www.ubian.sk/robots.txt), [support](https://www.ubian.sk/pomoc-a-podpora), [cooperation page](https://www.ubian.sk/spolupracujeme), and [terms linked from registration](https://www.ubian.sk/docs/vop_ubian.pdf?v=20251101). No documented third-party timetable API was found. The terms describe website/mobile-app services and informational carrier data. An empty robots exclusion, public JavaScript or HTTP 200 is not an API licence. These facts remain recorded; they are no longer a project-created runtime gate for the authorized personal client.

## Observed request surfaces

All requests below are to the exact HTTPS origin `https://www.ubian.sk`. No redirect was followed in direct discovery. Public document/asset retrieval used `curl --proto '=https' --max-time 20 --max-redirs 0`; response bodies were retained only temporarily outside the repository. The homepage returned HTTP 200 and linked the following scripts (observed asset version `mucwh8wa`):

- `/assets/js/ubian/routes.js?v=mucwh8wa`
- `/assets/js/ubian/navigation/navigation.min.js?v=mucwh8wa`

These scripts were read as evidence; no provider script, HTML, iframe or remote asset is bundled into the privileged frontend. The following table records observed request contracts, subsequently exercised by the native adapter. Payload names are provider-owned and have not been invented as native contracts.

| Feature           | Observed request                            | Observed shape                                                                                                                                    | Verification                                                                                           |
| ----------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Stop lookup       | GET `/navigation/autocomplete`              | `query`, `stops_only=1`; additional `lat`, `lng`, `filter`, `cityID`                                                                              | Anonymous public stop lookups returned JSON without credentials/cookies or redirects                   |
| Journey search    | POST `/navigation/connections`              | Script builds `from`, `to`, `allowTransfers`, `maxRadius`, `departure`, `date`, `filter`, `sorting`, `cityID`; nested points serialized by jQuery | Anonymous response and nested form contract verified; no stable API guarantee                          |
| Departures        | POST `/navigation/stops/planned_departures` | Script builds `stopID`, `date`, `filter`, `cityID`; separate stop popovers also use GET with `platformIDs`/`limit`                                | Anonymous departure response verified; unnecessary vehicle/license fields discarded                    |
| Connection detail | GET `/navigation/connection/detail`         | Script builds `tripsDataParams[]` from `line`, `tripNumber`, `tripStartDate`, `firmaID`, `tripID`                                                 | Anonymous response verified; previous-day stop timestamps reproduced and rejected by detail validation |

The sole data lookup used `query=Bratislava&stops_only=1&lat=0&lng=0&filter=bus%2Ctrain%2Curban&cityID=0`. No persistent history or account-linked location was used. This original lookup was discovery evidence. Subsequent guest checks cover journeys, departures and detail without session cookies; authenticated requests are outside this adapter. The verified transport filter tokens are `bus`, `train` and `urban`; `cityID` remains fixed at `0`, with no invented city selector. No service guarantee or stable error/rate-limit response contract is assumed.

Observed stop response (three public stop entries retained in [the sanitized fixture](../tests/fixtures/provider/stops.json)):

```json
{
  "status": "ok",
  "inputPhrase": "Bratislava",
  "results": [
    {
      "type": "stop",
      "id": 950001485,
      "name": "Bratislava hlavná stanica, žel.st.",
      "stopCity": "Bratislava hlavná stanica",
      "stopName": "žel.st.",
      "transportType": "train"
    }
  ]
}
```

Other observed fields include `slug`, `desc`, `icon`, and `region`. The stop parser discards those fields, preserves public names/IDs, bounds the body at 256 KiB / 100 results, and rejects malformed, missing or unexpected stop fields. The fixture is a truncated anonymous public response, not sample timetable results shown to users. Tests cover empty results separately from provider errors/schema changes.

## Runtime adapter and remaining limits

The local capability and Rust caller-origin checks guard stop lookup, journey search, departures, connection detail and cancellation. The provider Activity exposes only account/login and support destinations. There is no timetable website fallback. The guest adapter uses a shared Rust client, fixed endpoint paths, verified HTTPS certificates, no redirects or cookie jar, bounded bodies/collections, deadlines, actual cancellation and shared rate/backoff limits. Provider values reach Svelte as typed text, never HTML/scripts. Sanitized public fixtures are test data, never runtime substitutes.

The request uses jQuery-style nested keys (`from[stopID]`, `to[stopID]`) in form encoding. Timetable wall times are resolved in `Europe/Bratislava`, not the device timezone. Spring DST gaps are rejected; the repeated autumn hour requires an occurrence choice. Offset-bearing response timestamps are parsed as instants. Delay values without a verified update signal remain scheduled data.

The detail-date mismatch remains a specific provider limitation: the observed query for a trip on 2026-11-18 returns stops on 2026-11-17, including the Dvorianky regression trip. The client rejects mismatched detail enrichment instead of shifting dates, and retains valid search-result legs. See [the verification record](revision-2-verification.md). It does not disable search or departure boards.

No persistent timetable cache, personal history or offline routing pack is implemented. An in-memory result is available while navigating within the current app run; new searches require an online request. Optional SQLite saves and verified regional offline routing remain later stages. Account/session coordination and device acceptance are independent outstanding work.

## Origin inventory

| Origin/category                | Evidence                                                    | Classification/policy                                                                     |
| ------------------------------ | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `https://www.ubian.sk`         | Public documents, assets and one anonymous JSON stop lookup | Observed; exact-host navigation and fixed-path guest timetable requests                   |
| `https://accounts.google.com`  | Public sign-in link                                         | Link only, no verified session transfer or dependency                                     |
| `http://cp.ubian.sk`           | Static-timetable link template in homepage script           | Observed link only; cleartext, not accepted by provider URL policy or native adapter      |
| `https://maps.googleapis.com`  | Deferred script URL in homepage                             | Observed asset reference, not a captured runtime dependency; not loaded by local frontend |
| Other assets/identity/checkout | No selected-flow network capture                            | Unclassified; no new domain exception                                                     |
| Optional tracking              | No verified request capture                                 | Zero shipped analytics rules; no effectiveness claim                                      |

The later [native account verification](account-verification.md) establishes the tested email/password form, protected profile, cookie restoration and server logout flow for the authorized test account. WebView session transfer, social authentication and complete website-storage cleanup remain unverified; public page text alone cannot establish them. Keep authenticated captures, passwords, cookies, personal journeys and raw request logs out of fixtures and diagnostics. English/Ukrainian local controls do not promise a translated provider website: the fixed account/support URLs remain the Slovak pages; Ukrainian fallback is explicit in both local UI and native controls.
