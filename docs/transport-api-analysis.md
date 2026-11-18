# Ubian API and Slovak transport data analysis

Inspected: **2026-11-17**. This is the research baseline preceding Revision 2, not an Android acceptance result. Revision 2 subsequently implemented the personal-use Ubian adapter; see [current implementation status](implementation-status.md) and [verification](revision-2-verification.md). The observations below are preserved as research history.

Ubian's website exposes anonymous JSON endpoints that can technically support native journey screens. No documented third-party API contract or reuse grant was found. **Transitous** offers a documented public routing API, but a follow-up on the user's Dvorianky–Košice example confirmed missing direct buses. It is not sufficient as this app's default nationwide source. Prioritize a source that covers the required rural journeys; Ubian returned them in the tested case. Open timetable data cannot supply Ubian accounts, transport-card balances or purchases.

## App state at the time of research

[`lookup_stops`](../crates/journey-core/src/timetable.rs) validates the query and always returns `SourceUnavailable`. There is no production HTTP transport, journey-search command, departure command or detail command. The frontend and synthetic vault therefore do not constitute a usable transport integration. The account/session coordinator described in [authentication](authentication.md) is also still unimplemented.

## Ubian: observed requests and responses

The public [homepage](https://www.ubian.sk/) links [routes.js](https://www.ubian.sk/assets/js/ubian/routes.js?v=mucwh8wa), [navigation.min.js](https://www.ubian.sk/assets/js/ubian/navigation/navigation.min.js?v=mucwh8wa) and [ubian.min.js](https://www.ubian.sk/assets/js/ubian/ubian.min.js?v=mucwh8wa). These were downloaded and read, not executed or bundled.

Five small data requests followed those observed request shapes: two stop lookups and one request each for journeys, departures and detail. All returned HTTP 200 JSON without sending credentials, an API key, cookies or a CSRF token. No cookie jar or redirects were used. This establishes the behavior of these anonymous examples, not every request or future availability.

| Feature           | Method and path on `https://www.ubian.sk`   | Observed response                                                                                 |
| ----------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Stop lookup       | GET `/navigation/autocomplete`              | `status`, `inputPhrase`, `results[]`; stops include numeric `id`, `name`, `type`, `transportType` |
| Journey search    | POST `/navigation/connections`              | `status`, `connections[]`, `startEndTimes`; 10 connections in the tested response                 |
| Departures        | POST `/navigation/stops/planned_departures` | `status`, `departures[]`; 100 departures in the tested response                                   |
| Connection detail | GET `/navigation/connection/detail`         | `status`, `trips`, keyed by trip ID; stop sequence and vehicle metadata                           |

The journey POST uses `application/x-www-form-urlencoded` with nested jQuery-style keys. The tested query was Bratislava → Trnava, bus only, departing after 09:00 on 2026-11-18:

```text
from[stopID]=24355
to[stopID]=8796
allowTransfers=1
maxRadius=400
departure=1
date=2026-11-18 09:00+0100
filter=bus
sorting=
cityID=0
```

Connections contain `tripStartTime`, `tripEndTime`, `totalMinutes`, `transfers`, `parts[]` and provider identifiers. Parts contain endpoint stops, platforms, scheduled times, `delayMinutes` and nullable `timeTableTrip`; a null trip can represent walking. Trips expose line, operator, destination and cancellation metadata. Dates in the observed journey response are RFC-style strings with an explicit offset.

The tested departure request supplied `stopID=24355`, the same date, `filter=bus`, and `cityID=0`. Entries include `timeTableTrip`, `plannedDepartureTime`, `plannedDepartureTimestamp`, `platformNumber` and `delayMinutes`. Vehicle/license fields also occur; an adapter should discard unnecessary vehicle identifiers.

Detail requests serialize `tripsDataParams[0][line]`, `[tripNumber]`, `[tripStartDate]`, `[firmaID]` and `[tripID]`, using values returned by search. The tested trip `1040022799` returned eight stops. **Date handling needs investigation:** despite supplying the journey's 2026-11-18 departure string, its first detail timestamp resolved to 2026-11-17. HTTP 200 and a plausible stop sequence are insufficient to validate detail dates. Do not ship this request shape without resolving that mismatch.

The scripts also reference nearby stops, line timetables, route geometry and vehicle updates. Those requests were not exercised. Zero delay values, zero coordinates and an uninitialized last-communication timestamp in the observed detail do not establish live tracking or an on-time service.

These are website-specific interfaces. Public reachability does not establish a supported integration or redistribution terms; authentication requirements, errors, rate limits and change guarantees remain unconfirmed. The earlier [Ubian discovery snapshot](provider-discovery.md) records the access questions for a production Ubian adapter.

## Ubian accounts are a separate integration

The public [login page](https://www.ubian.sk/login) has a form posting to `/login`, with `email`, `password` and `rememberme`. The observed primary form's remember checkbox is checked by the provider. No hidden CSRF input was present in that form; this does not establish the absence of server-side CSRF protections.

The linked route script includes `/oauth_login`, `/user/update_profile`, `/user/change_password_action`, `/card/set_active`, `/purse/get_values` and `/ajax/transactions/800`. These are script references only. Their authenticated request/response contracts, authorization and session semantics were not tested. The OAuth routes support the site's social sign-in flows; they do not establish a third-party OAuth client or a transferable application session.

There was no login submission, account creation, account modification, card operation or payment. No authenticated capture was made. Native account management still requires a verified sign-in signal, session ownership, expiry, logout and cleanup, followed by verification of the account/card views. Manual login in the restricted provider WebView can be implemented independently of optional saved passwords. GTFS and Transitous cannot replace that work.

## Transitous: a documented hosted routing API

[Transitous developer documentation](https://transitous.org/api/) explicitly offers an API for applications, powered by MOTIS. The documentation linked the [MOTIS v2.11.4 OpenAPI definition](https://raw.githubusercontent.com/motis-project/motis/refs/tags/v2.11.4/openapi.yaml) during this inspection. The following versioned endpoints were tested on `https://api.transitous.org`, without an API key:

| Feature        | Endpoint                | Key inputs and observed output                                                                                                    |
| -------------- | ----------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Stop search    | GET `/api/v1/geocode`   | `text`, `type=STOP`, `language=sk`, `numResults`; array of named stops with string IDs and coordinates                            |
| Journey search | GET `/api/v6/plan`      | `fromPlace`, `toPlace`, ISO timestamp `time`, `transitModes`, `arriveBy`; itineraries with legs, transfers and pagination cursors |
| Departures     | GET `/api/v6/stoptimes` | `stopId`, `time`, `n`; `stopTimes[]`, stop information and pagination cursors                                                     |
| Trip detail    | GET `/api/v6/trip`      | `tripId`; trip itinerary including intermediate stops                                                                             |

Two geocoding requests, one bus-routing request, one departure request and one trip request all returned HTTP 200 JSON. Requests identified this research prototype and its repository in the User-Agent.

The historical bus query used these inputs:

```text
fromPlace=sk-ARRIVA-Trnava_S-f56f3a5b
toPlace=sk-ARRIVA-Trnava_S-7a5b4c32
time=2026-11-18T08:00:00Z
transitModes=BUS,COACH
numItineraries=3
maxItineraries=3
language=sk
```

It returned three bus itineraries. One departed Bratislava at 09:44 local time, transferred at Senec and reached Trnava at 11:28, using lines 632 and 666. The departure probe returned three bus entries. The selected line 632 trip detail included 35 intermediate stops. Tested bus legs had `realTime: false`; departure entries explicitly reported `NO_RT_DATA`. These are scheduled results, not verified delays.

### Slovak coverage and a concrete limitation

The current [Slovak feed manifest](https://github.com/public-transport/transitous/blob/main/feeds/sk.json) and [loaded sources](https://transitous.org/sources/) list rail data, Bratislava MHD, IDS BK regional buses, Prešov MHD, multiple ARRIVA and SAD operators, Slovak Lines Express, RegioJet/Student Agency and smaller sources. Many regional feeds are community conversions or mirrors, rather than direct operator GTFS publications. A listed operator does not establish complete, current coverage of all its services.

For the same Bratislava–Trnava date/time search, Ubian returned a direct LeviTour bus leg at **09:30–10:25**, preceded by a four-minute walk between stop records. That service was absent from the three Transitous itineraries, which all involved transfers. Different stop grouping, routing behavior or feed coverage can affect the comparison; the test does not prove that Transitous lacks every direct service. It does show that the providers cannot be treated as interchangeable or claimed to produce equally complete results.

Coverage acceptance should compare representative routes in the intended regions, including rural buses and MHD. A missing result must not be presented as proof that no service exists.

### Follow-up: Dvorianky–Košice missing direction

The user's counterexample was reproduced on 2026-11-17 using **2026-11-18 after 06:00 Europe/Bratislava**, bus only, with transfers disabled. Stop IDs were resolved through each provider's autocomplete, rather than equated across providers.

| Source     | Query/result                                                                                                                                                                                                           |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ubian      | `from[stopID]=1321`, `to[stopID]=8195`, `allowTransfers=0`; 10 direct results, including Dvorianky, č.d.143 → Košice, AS at **06:44–07:40**, **07:44–08:40** and **08:44–09:40**; line **073104**, SAD Humenné a.s.    |
| Transitous | `fromPlace=sk-ARRIVA-Michalovce_S-f6c5b539`, `toPlace=sk-ARRIVA-Michalovce_S-ae1f90d9`, `time=2026-11-18T05:00:00Z`, `transitModes=BUS,COACH`, `maxTransfers=0`; HTTP 200 with empty `itineraries` and `direct` arrays |

The operator's [current line 073104 timetable](https://www.sadhe.sk/linky/073104.pdf), valid 2026-09-01 through 2026-12-12, includes both stops. The older line 713421 PDF found by search expired in December 2025 and must not be used as the current timetable.

There is also a [community SAD Humenné GTFS source](https://transiq.xhyrom.dev/gtfs/sk/sad-humenne-as.zip), referenced by the Transitous manifest. Its [feed definition](https://github.com/xhyrom/transiq/blob/main/feeds/sk/sad-humenne-as.ts) describes XLSX conversion from portal.cp.sk. Inspection of that ZIP and [Transitous's processed copy](https://api.transitous.org/gtfs/sk_SAD-Humenne.gtfs.zip) found the same defect: route `L073104` has **46 trip records, but only 22 have any stop times**. The usable sequences all run from Košice towards Vranov/Humenné. All 23 trips with destination Košice, plus one other trip, have no rows in `stop_times.txt`. Both endpoint stops exist with plausible coordinates.

This locates an actual incompleteness in the published community feed before routing. It suggests a conversion/publishing problem; the converter's precise failure has not been traced. It does **not** establish that SAD refuses to supply any data or that the entire operator is absent. Repairing this feed may fix this direction, but would not establish completeness of the other lines or operators.

The earlier recommendation to implement Transitous first was too strong for this app's required coverage. Self-hosting the same GTFS would retain the missing direction. A fallback triggered only by empty results would also fail to catch cases where incomplete data produces plausible detours.

### Hosted-service conditions

The [published policy](https://transitous.org/api/) requires an open-source, noncommercial application, modest resource use, identification/contact information and source/OSM attribution. Service is provided on a best-effort basis. Traffic involving more than 720 planning requests per 24 hours is an example of usage requiring coordination, not a promised quota or per-user allowance. No contact was sent during this analysis.

Light prototype testing before publishing the source is explicitly allowed. No project `LICENSE` file was found in this checkout; an appropriate published open-source license is needed for ongoing application use. Meet the source attribution requirements and budget/debounce requests before enabling a runtime adapter. Commercial use would need another hosting arrangement and suitable data rights.

## Other data options

| Source                                                                                                                                                                                            | What is available                                                                              | Implication for this app                                                                                                         |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| [IDS BK official open data](https://www.idsbk.sk/system/open-data/)                                                                                                                               | Bratislava MHD GTFS and regional bus GTFS/JDF; links to ArcGIS and Google Drive respectively   | Official schedule inputs; require a routing engine to answer journeys with transfers                                             |
| [ŽSR GTFS information](https://www.zsr.sk/sluzby-verejnosti/pre-cestujucich/fgtfs-info) and [national catalogue dataset](https://data.slovensko.sk/datasety/ebeeedf1-aca2-451a-bdc0-35d536714888) | Railway timetable feed                                                                         | Scheduled rail input; does not by itself supply a routing API or live delays                                                     |
| [Transiq](https://github.com/xhyrom/transiq)                                                                                                                                                      | Community GTFS mirrors/conversions used by Transitous, including IDS BK and regional operators | Useful for feed discovery or future ingestion; the tool's Apache license does not automatically license every underlying dataset |
| [CP.sk public API documentation](https://www.inprop.sk/files/products/tt/CP_API.pdf)                                                                                                              | Links that prefill or submit the CP.sk website's search forms                                  | A website handoff, not a documented JSON routing service for native results                                                      |

The [Transiq CIS notes](https://github.com/xhyrom/transiq/tree/main/cis) describe converting published Slovak timetable files; their account of national feed availability is dated August 2025 and should not be treated as a verified nationwide status in November 2026. This investigation did not identify a documented government-hosted journey API that guarantees complete Slovak coverage.

GTFS is a schedule format, not a journey-search service. Operating [MOTIS](https://github.com/motis-project/motis) or [OpenTripPlanner](https://docs.opentripplanner.org/en/latest/Basic-Tutorial/) with selected GTFS and OpenStreetMap data is another option. It adds feed updates and server operations, does not fill absent data, and would change the project's current goal of requiring no developer-operated backend.

## Recommended direction at the time of research

The later [Revision 2](../REVISION-2.md) authorizes the narrow anonymous personal-use adapter and supersedes the earlier blanket integration-grant gate. The detail-date defect is handled by rejecting enrichment while keeping valid result legs. The original recommendations follow.

1. Prioritize a native Ubian adapter as the coverage candidate: it returned the required Dvorianky–Košice direct buses. Resolve its supported integration/access conditions and detail-date mismatch before a production adapter. Include this rural route in source acceptance alongside other regions; a functioning API alone is insufficient.
2. Replace the current Ubian-specific numeric stop ID assumption with source-tagged string identifiers. Transitous IDs differ from Ubian IDs; even matching stop names can have multiple feed records. Preserve stop names and use the provider's grouping behavior rather than equating records by name.
3. Keep networking in typed Rust commands with bounded bodies, deadlines, cancellation, shared throttling and explicit failure states. Render bundled Svelte results. Parse offset-bearing timestamps as instants and display Slovak timetable time in `Europe/Bratislava`.
4. Implement Ubian account sessions separately in the restricted provider view, then verify account/card behavior with an authorized account. Complete the session coordinator before enabling remembered sessions or real saved credentials.
5. Treat Transitous as an optional source pending feed repair and broader coverage verification. An alternative is to repair the XLSX-to-GTFS conversion and get corrected feeds into a routing service, subject to source terms. Record omissions explicitly and avoid inferring that partial results represent all available buses. Keep guest timetable use independent of Ubian login.

Public response bodies and downloaded scripts used for this inspection stayed in `/tmp`; only the selected public identifiers and observations above were recorded. No application networking, production source selection, credentials or account state were changed.
