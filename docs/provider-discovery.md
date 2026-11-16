# Provider discovery record — 2026-11-16

**Native timetables remain incomplete.** Anonymous stop lookup was technically verified, but a permitted integration for this client has not been established. There is no production HTTP adapter. The local `lookup_stops` command validates input and returns `source_unavailable`; it never substitutes fixtures or opens a website. Journey search, departure boards and connection detail have not been implemented or live-verified.

## Scope and access blocker

Limited public discovery inspected the [homepage](https://www.ubian.sk/), its two linked route/timetable scripts, [robots.txt](https://www.ubian.sk/robots.txt), [support](https://www.ubian.sk/pomoc-a-podpora), [cooperation page](https://www.ubian.sk/spolupracujeme), and [terms linked from registration](https://www.ubian.sk/docs/vop_ubian.pdf?v=20251101). Search on the provider/operator sites did not locate a documented third-party timetable API or access grant. This does not prove one does not exist.

The terms describe the provider website/mobile app and informational carrier data, but do not establish a third-party integration grant or integration limits. `robots.txt` has an empty `Disallow` for `*`; this is not an API licence. Public JavaScript and an HTTP 200 likewise do not establish permission to ship a native adapter. **The exact blocker is confirmation from the provider of permitted independent-client access, supported request/response contracts and applicable limits.** No authentication bypass, token extraction, challenge bypass, account submission or payment was attempted. No permission has been inferred from technical accessibility. No provider contact was sent.

Before enabling traffic, record the access grant/documentation and retention/attribution/rate restrictions, then observe a minimal permitted journey → departures → detail flow and capture sanitized response fixtures. The Android architecture gate also remains open; no device was connected in this session.

## Observed request surfaces

All requests below are to the exact HTTPS origin `https://www.ubian.sk`. No redirect was followed in direct discovery. Public document/asset retrieval used `curl --proto '=https' --max-time 20 --max-redirs 0`; response bodies were retained only temporarily outside the repository. The homepage returned HTTP 200 and linked the following scripts (observed asset version `mucwh8wa`):

- `/assets/js/ubian/routes.js?v=mucwh8wa`
- `/assets/js/ubian/navigation/navigation.min.js?v=mucwh8wa`

These scripts were read as evidence; no provider script, HTML, iframe or remote asset is bundled into the privileged frontend. The following table distinguishes script observation from actual response verification. Payload names are provider-owned and have not been invented as native contracts.

| Feature           | Observed request                            | Observed shape                                                                                                                                    | Verification                                                                                                                             |
| ----------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Stop lookup       | GET `/navigation/autocomplete`              | `query`, `stops_only=1`; additional `lat`, `lng`, `filter`, `cityID`                                                                              | One anonymous lookup of the public city name `Bratislava` returned HTTP 200 `application/json`, no credentials/cookies sent; no redirect |
| Journey search    | POST `/navigation/connections`              | Script builds `from`, `to`, `allowTransfers`, `maxRadius`, `departure`, `date`, `filter`, `sorting`, `cityID`; nested points serialized by jQuery | Script only; no response or stable schema verified                                                                                       |
| Departures        | POST `/navigation/stops/planned_departures` | Script builds `stopID`, `date`, `filter`, `cityID`; separate stop popovers also use GET with `platformIDs`/`limit`                                | Script only; no departure response verified                                                                                              |
| Connection detail | GET `/navigation/connection/detail`         | Script builds `tripsDataParams[]` from `line`, `tripNumber`, `tripStartDate`, `firmaID`, `tripID`                                                 | Script only; no detail response verified                                                                                                 |

The sole data lookup used `query=Bratislava&stops_only=1&lat=0&lng=0&filter=bus%2Ctrain%2Curban&cityID=0`. No persistent history or account-linked location was used. This is discovery evidence, not a supported production API or a permitted live-flow acceptance result. CSRF/session requirements, redirect behavior on data requests, error/rate-limit schemas, time-offset behavior, supported filters and service guarantees remain unverified. Do not extrapolate request authentication from this one response.

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

Other observed fields include `slug`, `desc`, `icon`, and `region`. The parsing spike discards those fields, preserves public names/IDs, bounds the body at 256 KiB / 100 results, and rejects malformed, missing or unexpected stop fields. The fixture is a truncated anonymous public response, not sample timetable results shown to users. Tests cover empty results separately from provider errors/schema changes.

## Runtime gate and next adapter requirements

No timetable frontend networking permission was added. The existing local capability/origin checks also guard `lookup_stops`. The provider Activity destination enum now includes only account/login and support. There is no timetable website fallback. The local screen is explicitly incomplete and independent of login/storage. Its stop-query scaffold has local validation, cancellation/late-response suppression and translated errors, but does not satisfy the real-connection acceptance flow.

Once permission and schemas are verified, implement a narrowly scoped Rust transport with exact destination paths, HTTPS certificate checks, redirects disabled (or individually revalidated), bounded response sizes, timeout, cancellation, shared rate/backoff limits and no automatic retry of unrelated requests. Use no WebView cookies or general URL command. Render provider text as text. Do not implement connection/departure/detail DTOs from guessed responses.

Use `Europe/Bratislava` for timetable wall times, independently of device locale/timezone. Locale formatting helpers and midnight/DST-instant tests are present; parsing ambiguous/nonexistent wall times and a full search through DST transitions remain blocked with the actual source adapter. There is currently no timetable cache/history. Future caching must be memory-bounded, mark fetch time, retain scheduled/realtime provenance, and never imply on-time service when realtime data is absent.

## Origin inventory

| Origin/category                | Evidence                                                    | Classification/policy                                                                     |
| ------------------------------ | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `https://www.ubian.sk`         | Public documents, assets and one anonymous JSON stop lookup | Observed; exact-host provider navigation, production timetable access unresolved          |
| `https://accounts.google.com`  | Public sign-in link                                         | Link only, no verified session transfer or dependency                                     |
| `http://cp.ubian.sk`           | Static-timetable link template in homepage script           | Observed link only; cleartext, not accepted by provider URL policy or native adapter      |
| `https://maps.googleapis.com`  | Deferred script URL in homepage                             | Observed asset reference, not a captured runtime dependency; not loaded by local frontend |
| Other assets/identity/checkout | No selected-flow network capture                            | Unclassified; no new domain exception                                                     |
| Optional tracking              | No verified request capture                                 | Zero shipped analytics rules; no effectiveness claim                                      |

Website session/form/CSRF/cookie/logout semantics remain unknown. Public page text cannot establish them. Keep authenticated captures, passwords, cookies, personal journeys and raw request logs out of fixtures and diagnostics. English/Ukrainian local controls do not promise a translated provider website: the fixed account/support URLs remain the Slovak pages; Ukrainian fallback is explicit in both local UI and native controls.
