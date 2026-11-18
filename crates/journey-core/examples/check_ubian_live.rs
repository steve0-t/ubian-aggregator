//! Explicit, bounded online smoke check. Never runs as part of the test suite.
//! Run only when desired: cargo run --example check_ubian_live -- 2026-11-18
use journey_core::timetable::{
    local_time, DepartureQuery, DetailQuery, Journey, JourneyQuery, Stop, StopQuery,
    TimetableClient, TimetableError, Transport,
};
use std::time::Duration;

async fn pause() {
    // Keep these deliberate checks comfortably below the shared request limit.
    tokio::time::sleep(Duration::from_millis(1200)).await;
}

fn query(
    from: &Stop,
    to: &Stop,
    date: &str,
    time: &str,
    arrive_by: bool,
    direct_only: bool,
) -> JourneyQuery {
    JourneyQuery {
        from: from.clone(),
        to: to.clone(),
        date: date.into(),
        time: time.into(),
        arrive_by,
        direct_only,
        transport: Transport::Bus,
        time_occurrence: None,
    }
}

fn summary(label: &str, journeys: &[Journey]) {
    println!("{label}: {} journeys", journeys.len());
    for journey in journeys.iter().take(3) {
        let services = journey
            .legs
            .iter()
            .filter_map(|leg| leg.service.as_ref())
            .map(|service| format!("{} ({})", service.line, service.operator))
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "  {} -> {}; {} min; {} transfers; {} legs; {}",
            journey.departure,
            journey.arrival,
            journey.duration_minutes,
            journey.transfers,
            journey.legs.len(),
            services
        );
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let date = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "2026-11-18".into());
    let client = TimetableClient::new()?;
    let origin = client
        .lookup_stops(
            "live-origin".into(),
            StopQuery {
                query: "Dvorianky".into(),
            },
        )
        .await?
        .into_iter()
        .find(|stop| stop.name == "Dvorianky, č.d.143")
        .ok_or("required origin not found")?;
    pause().await;
    let destination = client
        .lookup_stops(
            "live-destination".into(),
            StopQuery {
                query: "Košice, AS".into(),
            },
        )
        .await?
        .into_iter()
        .find(|stop| stop.name == "Košice, AS")
        .ok_or("required destination not found")?;
    println!(
        "resolved: {} [{}] -> {} [{}]",
        origin.name, origin.id, destination.name, destination.id
    );

    pause().await;
    let direct = client
        .search_journeys(
            "live-direct".into(),
            query(&origin, &destination, &date, "06:00", false, true),
        )
        .await?;
    summary("direct buses", &direct);
    if direct.is_empty() {
        return Err("required direct-bus regression returned no journeys".into());
    }
    if direct.iter().any(|journey| journey.transfers != 0) {
        return Err("direct filter returned transfers".into());
    }
    let detail_token = direct
        .iter()
        .flat_map(|journey| &journey.legs)
        .find_map(|leg| leg.detail_token.clone());

    pause().await;
    let reverse = client
        .search_journeys(
            "live-reverse".into(),
            query(&destination, &origin, &date, "06:00", false, true),
        )
        .await?;
    summary("reverse direct buses", &reverse);
    if reverse.is_empty() || reverse.iter().any(|journey| journey.transfers != 0) {
        return Err("reverse direct-bus check did not return direct journeys".into());
    }

    pause().await;
    let arrival = client
        .search_journeys(
            "live-arrival".into(),
            query(&origin, &destination, &date, "08:00", true, true),
        )
        .await?;
    summary("arrival by 08:00", &arrival);
    let arrive_by = local_time(&date, "08:00", None)?;
    if arrival.is_empty()
        || arrival.iter().any(|journey| {
            chrono::DateTime::parse_from_rfc3339(&journey.arrival)
                .map_or(true, |actual| actual > arrive_by)
        })
    {
        return Err("arrival search did not respect the requested arrival time".into());
    }

    let bratislava = Stop {
        id: 24355,
        name: "Bratislava, Autobusová stanica".into(),
    };
    let trnava = Stop {
        id: 8796,
        name: "Trnava, AS".into(),
    };
    pause().await;
    let transfers = client
        .search_journeys(
            "live-transfers".into(),
            query(&bratislava, &trnava, &date, "09:00", false, false),
        )
        .await?;
    summary("Bratislava to Trnava with transfers allowed", &transfers);
    println!(
        "  journeys with transfers: {}",
        transfers
            .iter()
            .filter(|journey| journey.transfers > 0)
            .count()
    );
    if !transfers.iter().any(|journey| journey.transfers > 0) {
        return Err("transfer scenario did not return a journey with transfers".into());
    }

    let veterna = Stop {
        id: 814000295,
        name: "Trnava, Veterná".into(),
    };
    pause().await;
    let empty = client
        .search_journeys(
            "live-empty".into(),
            query(&origin, &veterna, &date, "06:00", false, true),
        )
        .await?;
    summary(
        "Dvorianky to Trnava Veterná direct buses (empty candidate)",
        &empty,
    );
    if !empty.is_empty() {
        return Err(
            "empty-result candidate now has journeys; select another real empty scenario".into(),
        );
    }

    pause().await;
    let board = client
        .departures(
            "live-departures".into(),
            DepartureQuery {
                stop: origin,
                date,
                time: "06:00".into(),
                transport: Transport::Bus,
                time_occurrence: None,
            },
        )
        .await?;
    println!("departure board: {} departures", board.len());
    if board.is_empty() {
        return Err("departure-board scenario returned no departures".into());
    }
    for entry in board.iter().take(3) {
        println!(
            "  {}; {}; {}; {}",
            entry.departure, entry.service.line, entry.service.destination, entry.service.operator
        );
    }

    if let Some(token) = detail_token {
        pause().await;
        match client.connection_detail("live-detail".into(), DetailQuery { token }).await {
            Ok(detail) => println!("detail: {} verified stops", detail.stops.len()),
            Err(TimetableError::DetailUnavailable) => println!("detail: unavailable (date/endpoint validation rejected enrichment); journey legs remain usable"),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
