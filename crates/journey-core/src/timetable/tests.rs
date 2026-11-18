use super::*;
use serde_json::{json, Value};
const JOURNEYS: &[u8] = include_bytes!("../../tests/fixtures/journeys.json");
const RURAL: &[u8] = include_bytes!("../../tests/fixtures/rural-journeys.json");
const DEPARTURES: &[u8] = include_bytes!("../../tests/fixtures/departures.json");
const WRONG_DETAIL: &[u8] = include_bytes!("../../tests/fixtures/detail-wrong-date.json");
fn registry() -> Mutex<DetailRegistry> {
    Mutex::new(DetailRegistry::default())
}
fn query() -> JourneyQuery {
    JourneyQuery {
        from: Stop {
            id: 1321,
            name: "Dvorianky, č.d.143".into(),
        },
        to: Stop {
            id: 8195,
            name: "Košice, AS".into(),
        },
        date: "2026-11-18".into(),
        time: "06:00".into(),
        arrive_by: false,
        direct_only: true,
        transport: Transport::Bus,
        time_occurrence: None,
    }
}
#[test]
fn observed_nested_form_encoding_and_filters() {
    let mut request = query();
    let form = request.form().unwrap();
    let fields: HashMap<_, _> = url::form_urlencoded::parse(form.as_bytes())
        .into_owned()
        .collect();
    assert_eq!(fields["from[stopID]"], "1321");
    assert_eq!(fields["to[stopID]"], "8195");
    assert_eq!(fields["date"], "2026-11-18 06:00+0100");
    assert_eq!(fields["departure"], "1");
    assert_eq!(fields["allowTransfers"], "0");
    assert_eq!(fields["filter"], "bus");
    assert!(form.contains("from%5BstopID%5D=1321"));
    assert!(form.contains("%2B0100"));
    request.arrive_by = true;
    request.direct_only = false;
    request.transport = Transport::All;
    let fields: HashMap<_, _> = url::form_urlencoded::parse(request.form().unwrap().as_bytes())
        .into_owned()
        .collect();
    assert_eq!(fields["departure"], "0");
    assert_eq!(fields["allowTransfers"], "1");
    assert_eq!(fields["filter"], "bus,train,urban");
    request.to = request.from.clone();
    assert_eq!(request.form(), Err(TimetableError::InvalidQuery));
}
#[test]
fn bratislava_time_midnight_offsets_and_both_dst_transitions() {
    assert_eq!(
        local_time("2026-11-18", "00:05", None).unwrap().timestamp(),
        1794956700
    );
    assert_eq!(
        local_time("2026-07-18", "06:00", None)
            .unwrap()
            .format("%z")
            .to_string(),
        "+0200"
    );
    assert_eq!(
        local_time("2026-11-18", "06:00", None)
            .unwrap()
            .format("%z")
            .to_string(),
        "+0100"
    );
    assert_eq!(
        local_time("2026-03-29", "02:30", None),
        Err(TimetableError::InvalidTime)
    );
    assert_eq!(
        local_time("2026-10-25", "02:30", None),
        Err(TimetableError::AmbiguousTime)
    );
    let early = local_time("2026-10-25", "02:30", Some(TimeOccurrence::Earlier)).unwrap();
    let late = local_time("2026-10-25", "02:30", Some(TimeOccurrence::Later)).unwrap();
    assert_eq!((late - early).num_hours(), 1);
    for (date, time) in [
        ("2026-02-30", "06:00"),
        ("2026-11-18", "24:00"),
        ("2026-1-18", "06:00"),
        ("2026-11-18", "6:00"),
    ] {
        assert_eq!(
            local_time(date, time, None),
            Err(TimetableError::InvalidTime)
        );
    }
    assert_eq!(
        timestamp("Wed, 18 Nov 2026 06:44:00 +0100").unwrap(),
        timestamp("2026-11-18T05:44:00Z").unwrap()
    );
    assert!(timestamp("2026-11-18T06:44:00").is_err());
}
#[test]
fn stop_schema_and_input_bounds() {
    let stops = parse_stops(include_bytes!(
        "../../../../tests/fixtures/provider/stops.json"
    ))
    .unwrap();
    assert_eq!(stops[1].id, 950001485);
    assert!(
        parse_stops(br#"{"status":"ok","inputPhrase":"xx","results":[]}"#)
            .unwrap()
            .is_empty()
    );
    for query in ["", "a", "ab\n", "ab\u{0000}", &"a".repeat(101)] {
        assert_eq!(
            StopQuery {
                query: query.into()
            }
            .validate(),
            Err(TimetableError::InvalidQuery)
        );
    }
    for body in [
        r#"{"status":"error","inputPhrase":"xx","results":[]}"#,
        r#"{"status":"ok","results":[]}"#,
        r#"{"status":"ok","inputPhrase":"xx","results":[{"type":"stop","id":0,"name":"X"}]}"#,
        r#"{"status":"ok","inputPhrase":"xx","results":[{"type":"city","id":1,"name":"X"}]}"#,
        "<html>Challenge</html>",
    ] {
        assert_eq!(
            parse_stops(body.as_bytes()),
            Err(TimetableError::ProviderChanged)
        );
    }
    assert_eq!(
        parse_stops(&vec![b' '; MAX_STOPS_BODY + 1]),
        Err(TimetableError::ProviderChanged)
    );
    assert_eq!(
        serde_json::from_str::<Language>("\"uk\"").unwrap(),
        Language::Uk
    );
}
#[test]
fn actual_search_legs_walking_operator_platform_and_ignored_filters() {
    let store = registry();
    let rural = parse_journeys(RURAL, &store, Transport::Bus).unwrap();
    assert_eq!(rural[0].departure, "2026-11-18T06:44:00+01:00");
    assert_eq!(rural[0].arrival, "2026-11-18T07:40:00+01:00");
    assert_eq!(rural[0].duration_minutes, 56);
    assert_eq!(rural[0].transfers, 0);
    let service = rural[0].legs[0].service.as_ref().unwrap();
    assert_eq!(service.line, "073104");
    assert_eq!(service.operator, "SAD Humenné a.s.");
    assert_eq!(rural[0].legs[0].from_platform.as_deref(), Some("1"));
    assert_eq!(rural[0].legs[0].from.name, "Dvorianky, č.d.143");
    assert!(parse_journeys(RURAL, &store, Transport::Train)
        .unwrap()
        .is_empty());
    let journeys = parse_journeys(JOURNEYS, &store, Transport::All).unwrap();
    assert!(journeys[0].legs[0].service.is_none());
    assert_eq!(journeys[0].legs[0].walking_meters, 77);
    let serialized = serde_json::to_string(&journeys).unwrap();
    assert!(!serialized.contains("delay"));
    assert!(!serialized.contains("realTime"));
    assert!(!serialized.contains("vehicle"));
}
#[test]
fn schema_errors_are_distinct_from_empty_searches_and_bounds() {
    let store = registry();
    assert!(parse_journeys(
        br#"{"status":"ok","connections":[]}"#,
        &store,
        Transport::All
    )
    .unwrap()
    .is_empty());
    for body in [
        b"{}".as_slice(),
        b"<html/>".as_slice(),
        br#"{"status":"error","connections":[]}"#,
    ] {
        assert_eq!(
            parse_journeys(body, &store, Transport::All).unwrap_err(),
            TimetableError::ProviderChanged
        );
    }
    let initial: Value = serde_json::from_slice(RURAL).unwrap();
    for mutation in 0..5 {
        let mut fixture = initial.clone();
        match mutation {
            0 => fixture["connections"][0]["parts"][0]["fromDepartureTime"] = json!("changed"),
            1 => fixture["connections"][0]["parts"][0]["fromStop"]["stopID"] = json!(0),
            2 => fixture["connections"][0]["totalMinutes"] = json!(50),
            3 => {
                fixture["connections"][0]["parts"][0]["timeTableTrip"]["timeTableLine"]["line"] =
                    json!("x".repeat(65))
            }
            _ => fixture["connections"][0]["parts"] = json!([]),
        }
        assert_eq!(
            parse_journeys(
                &serde_json::to_vec(&fixture).unwrap(),
                &store,
                Transport::All
            )
            .unwrap_err(),
            TimetableError::ProviderChanged
        );
    }
    assert_eq!(
        parse_journeys(&vec![0; MAX_BODY + 1], &store, Transport::All).unwrap_err(),
        TimetableError::ProviderChanged
    );
}
#[test]
fn departure_board_is_scheduled_and_locally_honors_transport() {
    let stop = Stop {
        id: 24355,
        name: "Bratislava, Autobusová stanica".into(),
    };
    let store = registry();
    let departures = parse_departures(DEPARTURES, &stop, &store, Transport::All).unwrap();
    assert_eq!(departures.len(), 2);
    assert_eq!(departures[0].departure, "2026-11-18T09:00:00+01:00");
    assert_eq!(departures[0].service.operator, "Dopravný podnik Bratislava");
    assert!(parse_departures(DEPARTURES, &stop, &store, Transport::Bus)
        .unwrap()
        .is_empty());
    let json = serde_json::to_string(&departures).unwrap();
    assert!(!json.contains("delay"));
    assert!(!json.contains("Licence"));
    assert!(parse_departures(
        br#"{"status":"ok","departures":[]}"#,
        &stop,
        &store,
        Transport::All
    )
    .unwrap()
    .is_empty());
}
#[test]
fn observed_detail_date_mismatch_rejected_without_disabling_valid_legs() {
    let store = registry();
    let journeys = parse_journeys(JOURNEYS, &store, Transport::All).unwrap();
    let token = journeys[0].legs[1].detail_token.as_ref().unwrap();
    let reference = store.lock().unwrap().get(token).unwrap();
    let fields: HashMap<_, _> = url::form_urlencoded::parse(reference.form().as_bytes())
        .into_owned()
        .collect();
    assert_eq!(fields["tripsDataParams[0][line]"], "102515");
    assert_eq!(
        fields["tripsDataParams[0][tripStartDate]"],
        "Wed, 18 Nov 2026 09:30:00 +0100"
    );
    assert_eq!(
        parse_detail(WRONG_DETAIL, &reference).unwrap_err(),
        TimetableError::DetailUnavailable
    );
    assert_eq!(journeys[0].legs[1].from.name, "Bratislava, AS");
    // Synthetic valid-day control verifies parser behavior only. Production
    // never shifts a mismatched provider timestamp this way.
    let mut fixture: Value = serde_json::from_slice(WRONG_DETAIL).unwrap();
    for stop in fixture["trips"]["1040022799"]["stops"]
        .as_array_mut()
        .unwrap()
    {
        stop["plannedDepartureTimestamp"] =
            json!(stop["plannedDepartureTimestamp"].as_i64().unwrap() + 86400);
    }
    let detail = parse_detail(&serde_json::to_vec(&fixture).unwrap(), &reference).unwrap();
    assert_eq!(detail.stops.len(), 8);
    assert_eq!(detail.stops[0].departure, "2026-11-18T09:30:00+01:00");
    assert!(store.lock().unwrap().get("invented").is_err());
}
#[test]
fn bounded_expiring_detail_identifiers_and_text_safety() {
    let mut store = DetailRegistry::default();
    let registry = registry();
    let journeys = parse_journeys(RURAL, &registry, Transport::Bus).unwrap();
    let reference = registry
        .lock()
        .unwrap()
        .get(journeys[0].legs[0].detail_token.as_ref().unwrap())
        .unwrap();
    for _ in 0..1000 {
        store.put(reference.clone());
    }
    assert_eq!(store.references.len(), 512);
    let mut old = reference;
    old.created = Instant::now() - Duration::from_secs(601);
    let token = store.put(old);
    assert!(store.get(&token).is_err());
    let mut fixture: Value = serde_json::from_slice(RURAL).unwrap();
    fixture["connections"][0]["parts"][0]["fromStop"]["stopRawName"] =
        json!("<script>alert(1)</script>");
    let journeys = parse_journeys(
        &serde_json::to_vec(&fixture).unwrap(),
        &registry,
        Transport::Bus,
    )
    .unwrap();
    assert_eq!(journeys[0].legs[0].from.name, "<script>alert(1)</script>");
}
