//! Narrow anonymous Ubian timetable integration. No cookies, credentials, remote
//! scripts, persistent history, generic URLs, or automatic retries are used.

mod transport;

use chrono::{DateTime, Datelike, FixedOffset, LocalResult, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Europe::Bratislava;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    sync::Mutex,
    time::{Duration, Instant},
};

pub use transport::TimetableClient;

pub(crate) const MAX_BODY: usize = 2 * 1024 * 1024;
pub(crate) const MAX_STOPS_BODY: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Sk,
    En,
    Uk,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimetableError {
    SourceUnavailable,
    InvalidQuery,
    ProviderChanged,
    Offline,
    Timeout,
    Cancelled,
    RateLimited,
    InvalidTime,
    AmbiguousTime,
    DetailUnavailable,
}
impl fmt::Display for TimetableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Error strings never contain URLs, response bodies, or personal inputs.
        let code = match self {
            Self::SourceUnavailable => "source_unavailable",
            Self::InvalidQuery => "invalid_query",
            Self::ProviderChanged => "provider_changed",
            Self::Offline => "offline",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::RateLimited => "rate_limited",
            Self::InvalidTime => "invalid_time",
            Self::AmbiguousTime => "ambiguous_time",
            Self::DetailUnavailable => "detail_unavailable",
        };
        f.write_str(code)
    }
}
impl std::error::Error for TimetableError {}
type Result<T> = std::result::Result<T, TimetableError>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StopQuery {
    pub query: String,
}
impl StopQuery {
    pub fn validate(&self) -> Result<&str> {
        let trimmed = self.query.trim();
        if !(2..=100).contains(&trimmed.chars().count()) || self.query.chars().any(char::is_control)
        {
            return Err(TimetableError::InvalidQuery);
        }
        Ok(trimmed)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    pub id: u32,
    pub name: String,
}
impl Stop {
    fn validate(&self) -> Result<()> {
        if self.id == 0 || !valid_text(&self.name, 256, false) {
            return Err(TimetableError::InvalidQuery);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    #[default]
    All,
    Bus,
    Train,
    Urban,
}
impl Transport {
    fn filter(self) -> &'static str {
        match self {
            Self::All => "bus,train,urban",
            Self::Bus => "bus",
            Self::Train => "train",
            Self::Urban => "urban",
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeOccurrence {
    Earlier,
    Later,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JourneyQuery {
    pub from: Stop,
    pub to: Stop,
    pub date: String,
    pub time: String,
    pub arrive_by: bool,
    pub direct_only: bool,
    pub transport: Transport,
    #[serde(default)]
    pub time_occurrence: Option<TimeOccurrence>,
}
impl JourneyQuery {
    pub fn form(&self) -> Result<String> {
        self.from.validate()?;
        self.to.validate()?;
        if self.from.id == self.to.id {
            return Err(TimetableError::InvalidQuery);
        }
        let date = local_time(&self.date, &self.time, self.time_occurrence)?
            .format("%Y-%m-%d %H:%M%z")
            .to_string();
        Ok(encode(&[
            ("from[stopID]", self.from.id.to_string()),
            ("to[stopID]", self.to.id.to_string()),
            (
                "allowTransfers",
                if self.direct_only { "0" } else { "1" }.into(),
            ),
            ("maxRadius", "400".into()),
            ("departure", if self.arrive_by { "0" } else { "1" }.into()),
            ("date", date),
            ("filter", self.transport.filter().into()),
            ("sorting", "".into()),
            ("cityID", "0".into()),
        ]))
    }
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DepartureQuery {
    pub stop: Stop,
    pub date: String,
    pub time: String,
    pub transport: Transport,
    #[serde(default)]
    pub time_occurrence: Option<TimeOccurrence>,
}
impl DepartureQuery {
    pub fn form(&self) -> Result<String> {
        self.stop.validate()?;
        let date = local_time(&self.date, &self.time, self.time_occurrence)?
            .format("%Y-%m-%d %H:%M%z")
            .to_string();
        Ok(encode(&[
            ("stopID", self.stop.id.to_string()),
            ("date", date),
            ("filter", self.transport.filter().into()),
            ("cityID", "0".into()),
        ]))
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetailQuery {
    pub token: String,
}

/// Interpret input in the Slovak timetable zone independently of the device.
/// Missing spring hours fail; repeated autumn hours require an explicit choice.
pub fn local_time(
    date: &str,
    time: &str,
    occurrence: Option<TimeOccurrence>,
) -> Result<DateTime<FixedOffset>> {
    if date.len() != 10 || time.len() != 5 {
        return Err(TimetableError::InvalidTime);
    }
    let naive = NaiveDateTime::parse_from_str(&format!("{date}T{time}"), "%Y-%m-%dT%H:%M")
        .map_err(|_| TimetableError::InvalidTime)?;
    if !(2000..=2100).contains(&naive.year())
        || naive.format("%Y-%m-%dT%H:%M").to_string() != format!("{date}T{time}")
    {
        return Err(TimetableError::InvalidTime);
    }
    let chosen = match Bratislava.from_local_datetime(&naive) {
        LocalResult::Single(value) => value,
        LocalResult::None => return Err(TimetableError::InvalidTime),
        LocalResult::Ambiguous(a, b) => match occurrence {
            Some(TimeOccurrence::Earlier) => a.min(b),
            Some(TimeOccurrence::Later) => a.max(b),
            None => return Err(TimetableError::AmbiguousTime),
        },
    };
    Ok(chosen.fixed_offset())
}
fn encode(fields: &[(&str, String)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in fields {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}
fn valid_text(value: &str, max: usize, empty: bool) -> bool {
    (empty || !value.trim().is_empty())
        && value.chars().count() <= max
        && !value.chars().any(char::is_control)
}
fn timestamp(raw: &str) -> Result<DateTime<FixedOffset>> {
    if raw.len() > 100 {
        return Err(TimetableError::ProviderChanged);
    }
    let parsed = DateTime::parse_from_rfc2822(raw)
        .or_else(|_| DateTime::parse_from_rfc3339(raw))
        .map_err(|_| TimetableError::ProviderChanged)?;
    if !(2000..=2100).contains(&parsed.year()) {
        return Err(TimetableError::ProviderChanged);
    }
    Ok(parsed)
}
fn unix_timestamp(value: i64) -> Result<DateTime<FixedOffset>> {
    let time = DateTime::<Utc>::from_timestamp(value, 0).ok_or(TimetableError::ProviderChanged)?;
    if !(2000..=2100).contains(&time.year()) {
        return Err(TimetableError::ProviderChanged);
    }
    Ok(time.with_timezone(&Bratislava).fixed_offset())
}
fn provider_json<T: serde::de::DeserializeOwned>(bytes: &[u8], max: usize) -> Result<T> {
    if bytes.len() > max {
        return Err(TimetableError::ProviderChanged);
    }
    serde_json::from_slice(bytes).map_err(|_| TimetableError::ProviderChanged)
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub line: String,
    pub name: String,
    pub operator: String,
    pub destination: String,
    pub cancelled: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Leg {
    pub from: Stop,
    pub to: Stop,
    pub departure: String,
    pub arrival: String,
    pub from_platform: Option<String>,
    pub to_platform: Option<String>,
    pub walking_meters: u32,
    pub service: Option<Service>,
    pub detail_token: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Journey {
    pub id: String,
    pub departure: String,
    pub arrival: String,
    pub duration_minutes: u32,
    pub transfers: u32,
    pub legs: Vec<Leg>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Departure {
    pub id: String,
    pub stop: Stop,
    pub departure: String,
    pub platform: Option<String>,
    pub service: Service,
    pub detail_token: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailStop {
    pub stop: Stop,
    pub departure: String,
    pub platform: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDetail {
    pub stops: Vec<DetailStop>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StopResponse {
    status: String,
    input_phrase: String,
    results: Vec<StopItem>,
}
#[derive(Deserialize)]
struct StopItem {
    #[serde(rename = "type")]
    kind: String,
    id: u32,
    name: String,
}
pub fn parse_stops(bytes: &[u8]) -> Result<Vec<Stop>> {
    let response: StopResponse = provider_json(bytes, MAX_STOPS_BODY)?;
    if response.status != "ok"
        || !valid_text(&response.input_phrase, 100, true)
        || response.results.len() > 100
    {
        return Err(TimetableError::ProviderChanged);
    }
    let mut stops = Vec::with_capacity(response.results.len());
    for item in response.results {
        let stop = Stop {
            id: item.id,
            name: item.name,
        };
        if item.kind != "stop"
            || stop.validate().is_err()
            || stops.iter().any(|s: &Stop| s.id == stop.id)
        {
            return Err(TimetableError::ProviderChanged);
        }
        stops.push(stop);
    }
    Ok(stops)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderStop {
    #[serde(rename = "stopID")]
    id: u32,
    stop_raw_name: String,
}
impl ProviderStop {
    fn into_stop(self) -> Result<Stop> {
        let stop = Stop {
            id: self.id,
            name: self.stop_raw_name,
        };
        stop.validate()
            .map_err(|_| TimetableError::ProviderChanged)?;
        Ok(stop)
    }
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderLine {
    line: String,
    line_name: String,
    #[serde(rename = "firmaID")]
    firma_id: u32,
    ez_is_train: bool,
    ez_is_urban: bool,
    ez_is_bus: bool,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderTrip {
    #[serde(rename = "tripID")]
    trip_id: u64,
    trip: u32,
    destination: String,
    time_table_line: ProviderLine,
    operator_name: String,
    canceled: bool,
}
impl ProviderTrip {
    fn matches(&self, transport: Transport) -> bool {
        match transport {
            Transport::All => true,
            Transport::Bus => self.time_table_line.ez_is_bus,
            Transport::Train => self.time_table_line.ez_is_train,
            Transport::Urban => self.time_table_line.ez_is_urban,
        }
    }
    fn service(&self) -> Result<Service> {
        if self.trip_id == 0
            || self.trip == 0
            || self.time_table_line.firma_id == 0
            || !valid_text(&self.time_table_line.line, 64, false)
            || !valid_text(&self.time_table_line.line_name, 1024, true)
            || !valid_text(&self.operator_name, 256, true)
            || !valid_text(&self.destination, 512, true)
        {
            return Err(TimetableError::ProviderChanged);
        }
        Ok(Service {
            line: self.time_table_line.line.clone(),
            name: self.time_table_line.line_name.clone(),
            operator: self.operator_name.clone(),
            destination: self.destination.clone(),
            cancelled: self.canceled,
        })
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderPart {
    time_table_trip: Option<ProviderTrip>,
    distance_meters_walking: u32,
    from_departure_time: String,
    from_stop: ProviderStop,
    from_platform: Option<u32>,
    to_arrival_time: String,
    to_stop: ProviderStop,
    to_platform: Option<u32>,
    from_platform_name: Option<String>,
    to_platform_name: Option<String>,
}
fn platform(number: Option<u32>, name: Option<String>) -> Result<Option<String>> {
    if let Some(value) = &name {
        if !valid_text(value, 128, true) {
            return Err(TimetableError::ProviderChanged);
        }
        if !value.trim().is_empty() {
            return Ok(Some(value.clone()));
        }
    }
    // Provider's 99 is still provider data; never invent a platform label.
    Ok(number.filter(|n| *n > 0).map(|n| n.to_string()))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderJourney {
    uid: String,
    trip_start_time: String,
    trip_end_time: String,
    total_minutes: u32,
    transfers: u32,
    parts: Vec<ProviderPart>,
}
#[derive(Deserialize)]
struct JourneyResponse {
    status: String,
    connections: Vec<ProviderJourney>,
}

/// Private identifiers for detail are retained for a short bounded period only.
#[derive(Clone)]
pub(crate) struct DetailReference {
    trip: ProviderTrip,
    departure_raw: String,
    expected_stop: u32,
    expected_departure: i64,
    created: Instant,
}
impl DetailReference {
    pub(crate) fn form(&self) -> String {
        encode(&[
            (
                "tripsDataParams[0][line]",
                self.trip.time_table_line.line.clone(),
            ),
            ("tripsDataParams[0][tripNumber]", self.trip.trip.to_string()),
            (
                "tripsDataParams[0][tripStartDate]",
                self.departure_raw.clone(),
            ),
            (
                "tripsDataParams[0][firmaID]",
                self.trip.time_table_line.firma_id.to_string(),
            ),
            ("tripsDataParams[0][tripID]", self.trip.trip_id.to_string()),
        ])
    }
}
#[derive(Default)]
pub(crate) struct DetailRegistry {
    serial: u64,
    references: HashMap<String, DetailReference>,
}
impl DetailRegistry {
    fn put(&mut self, reference: DetailReference) -> String {
        self.references
            .retain(|_, r| r.created.elapsed() < Duration::from_secs(600));
        if self.references.len() >= 512 {
            if let Some(oldest) = self
                .references
                .iter()
                .min_by_key(|(_, r)| r.created)
                .map(|(id, _)| id.clone())
            {
                self.references.remove(&oldest);
            }
        }
        self.serial = self.serial.wrapping_add(1);
        let token = format!("detail-{}", self.serial);
        self.references.insert(token.clone(), reference);
        token
    }
    pub(crate) fn get(&mut self, token: &str) -> Result<DetailReference> {
        self.references
            .retain(|_, r| r.created.elapsed() < Duration::from_secs(600));
        self.references
            .get(token)
            .cloned()
            .ok_or(TimetableError::DetailUnavailable)
    }
}
pub(crate) fn parse_journeys(
    bytes: &[u8],
    registry: &Mutex<DetailRegistry>,
    transport: Transport,
) -> Result<Vec<Journey>> {
    let response: JourneyResponse = provider_json(bytes, MAX_BODY)?;
    if response.status != "ok" || response.connections.len() > 100 {
        return Err(TimetableError::ProviderChanged);
    }
    let mut journeys = Vec::with_capacity(response.connections.len());
    for item in response.connections {
        // Live evidence shows Ubian sometimes ignores the requested filter.
        // Retain only journeys whose transit legs match verified provider flags.
        if item.parts.iter().any(|part| {
            part.time_table_trip
                .as_ref()
                .is_some_and(|trip| !trip.matches(transport))
        }) {
            continue;
        }
        let departure = timestamp(&item.trip_start_time)?;
        let arrival = timestamp(&item.trip_end_time)?;
        if !valid_text(&item.uid, 128, false)
            || item.parts.is_empty()
            || item.parts.len() > 24
            || item.transfers > 16
            || item.total_minutes > 7 * 24 * 60
            || arrival < departure
            || (arrival - departure).num_minutes() != i64::from(item.total_minutes)
            || journeys.iter().any(|j: &Journey| j.id == item.uid)
        {
            return Err(TimetableError::ProviderChanged);
        }
        let mut legs = Vec::with_capacity(item.parts.len());
        let mut previous = departure;
        for part in item.parts {
            let from = part.from_stop.into_stop()?;
            let to = part.to_stop.into_stop()?;
            let start = timestamp(&part.from_departure_time)?;
            let end = timestamp(&part.to_arrival_time)?;
            if start < previous
                || end < start
                || start < departure
                || end > arrival
                || part.distance_meters_walking > 100_000
            {
                return Err(TimetableError::ProviderChanged);
            }
            previous = end;
            let service = part
                .time_table_trip
                .as_ref()
                .map(ProviderTrip::service)
                .transpose()?;
            let detail_token = if let Some(trip) = part.time_table_trip {
                let reference = DetailReference {
                    trip,
                    departure_raw: part.from_departure_time,
                    expected_stop: from.id,
                    expected_departure: start.timestamp(),
                    created: Instant::now(),
                };
                Some(
                    registry
                        .lock()
                        .map_err(|_| TimetableError::SourceUnavailable)?
                        .put(reference),
                )
            } else {
                None
            };
            legs.push(Leg {
                from,
                to,
                departure: start.to_rfc3339(),
                arrival: end.to_rfc3339(),
                from_platform: platform(part.from_platform, part.from_platform_name)?,
                to_platform: platform(part.to_platform, part.to_platform_name)?,
                walking_meters: part.distance_meters_walking,
                service,
                detail_token,
            });
        }
        if legs
            .first()
            .is_none_or(|leg| timestamp(&leg.departure).ok() != Some(departure))
            || legs
                .last()
                .is_none_or(|leg| timestamp(&leg.arrival).ok() != Some(arrival))
        {
            return Err(TimetableError::ProviderChanged);
        }
        journeys.push(Journey {
            id: item.uid,
            departure: departure.to_rfc3339(),
            arrival: arrival.to_rfc3339(),
            duration_minutes: item.total_minutes,
            transfers: item.transfers,
            legs,
        });
    }
    Ok(journeys)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderDeparture {
    time_table_trip: ProviderTrip,
    planned_departure_timestamp: i64,
    planned_departure_time: i64,
    platform_number: Option<u32>,
    platform_name: Option<String>,
}
#[derive(Deserialize)]
struct DepartureResponse {
    status: String,
    departures: Vec<ProviderDeparture>,
}
pub(crate) fn parse_departures(
    bytes: &[u8],
    stop: &Stop,
    registry: &Mutex<DetailRegistry>,
    transport: Transport,
) -> Result<Vec<Departure>> {
    let response: DepartureResponse = provider_json(bytes, MAX_BODY)?;
    if response.status != "ok" || response.departures.len() > 500 {
        return Err(TimetableError::ProviderChanged);
    }
    response
        .departures
        .into_iter()
        .filter(|item| item.time_table_trip.matches(transport))
        .enumerate()
        .map(|(index, item)| {
            if item.planned_departure_timestamp != item.planned_departure_time {
                return Err(TimetableError::ProviderChanged);
            }
            let departure = unix_timestamp(item.planned_departure_timestamp)?;
            let service = item.time_table_trip.service()?;
            let id = format!(
                "{}-{}-{}",
                item.time_table_trip.trip_id, item.planned_departure_timestamp, index
            );
            let reference = DetailReference {
                trip: item.time_table_trip,
                departure_raw: departure.format("%a, %d %b %Y %H:%M:%S %z").to_string(),
                expected_stop: stop.id,
                expected_departure: departure.timestamp(),
                created: Instant::now(),
            };
            let detail_token = Some(
                registry
                    .lock()
                    .map_err(|_| TimetableError::SourceUnavailable)?
                    .put(reference),
            );
            Ok(Departure {
                id,
                stop: stop.clone(),
                departure: departure.to_rfc3339(),
                platform: platform(item.platform_number, item.platform_name)?,
                service,
                detail_token,
            })
        })
        .collect()
}
#[derive(Deserialize)]
struct DetailResponse {
    status: String,
    trips: HashMap<String, ProviderDetail>,
}
#[derive(Deserialize)]
struct ProviderDetail {
    stops: Vec<ProviderDetailStop>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderDetailStop {
    #[serde(rename = "stopID")]
    id: u32,
    stop_raw_name: String,
    stop_order: u32,
    planned_departure_timestamp: i64,
    platform_number: Option<u32>,
    platform_name: Option<String>,
}
pub(crate) fn parse_detail(bytes: &[u8], reference: &DetailReference) -> Result<ConnectionDetail> {
    let mut response: DetailResponse = provider_json(bytes, MAX_BODY)?;
    if response.status != "ok" || response.trips.len() > 24 {
        return Err(TimetableError::ProviderChanged);
    }
    let detail = response
        .trips
        .remove(&reference.trip.trip_id.to_string())
        .ok_or(TimetableError::DetailUnavailable)?;
    if detail.stops.is_empty() || detail.stops.len() > 512 {
        return Err(TimetableError::ProviderChanged);
    }
    // Reproduce and reject the observed provider defect, rather than shifting a
    // returned service day to fit the requested one. Matching one endpoint's
    // actual instant also handles overnight trips and explicit UTC offsets.
    if !detail.stops.iter().any(|s| {
        s.id == reference.expected_stop
            && s.planned_departure_timestamp == reference.expected_departure
    }) {
        return Err(TimetableError::DetailUnavailable);
    }
    let mut previous = None;
    let mut first_departure = None;
    let mut previous_order = None;
    let mut stops = Vec::with_capacity(detail.stops.len());
    for item in detail.stops {
        let stop = ProviderStop {
            id: item.id,
            stop_raw_name: item.stop_raw_name,
        }
        .into_stop()?;
        let departure = unix_timestamp(item.planned_departure_timestamp)?;
        if previous.is_some_and(|p| departure < p)
            || previous_order.is_some_and(|p| item.stop_order <= p)
        {
            return Err(TimetableError::ProviderChanged);
        }
        let first = *first_departure.get_or_insert(departure);
        if (departure - first).num_days() > 7 {
            return Err(TimetableError::ProviderChanged);
        }
        previous = Some(departure);
        previous_order = Some(item.stop_order);
        stops.push(DetailStop {
            stop,
            departure: departure.to_rfc3339(),
            platform: platform(item.platform_number, item.platform_name)?,
        });
    }
    Ok(ConnectionDetail { stops })
}

#[cfg(test)]
mod tests;
