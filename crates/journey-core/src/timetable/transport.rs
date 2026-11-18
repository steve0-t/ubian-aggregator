use super::*;
use reqwest::{header, redirect::Policy, Client, Method, StatusCode};
use std::{future::Future, sync::Arc};
use tokio::sync::{watch, Semaphore};

const DEADLINE: Duration = Duration::from_secs(20);
const INTERVAL: Duration = Duration::from_millis(750);
const MAX_ACTIVE: usize = 8;
const MAX_TOMBSTONES: usize = 128;

#[derive(Clone, Copy)]
enum Endpoint {
    Stops,
    Journeys,
    Departures,
    Detail,
}
impl Endpoint {
    fn path(self) -> &'static str {
        match self {
            Self::Stops => "/navigation/autocomplete",
            Self::Journeys => "/navigation/connections",
            Self::Departures => "/navigation/stops/planned_departures",
            Self::Detail => "/navigation/connection/detail",
        }
    }
    fn method(self) -> Method {
        match self {
            Self::Stops | Self::Detail => Method::GET,
            _ => Method::POST,
        }
    }
}
#[derive(Default)]
struct Requests {
    active: HashMap<String, watch::Sender<bool>>,
    cancelled: HashMap<String, Instant>,
    backoff: Option<Instant>,
    backoff_indefinite: bool,
}
struct Active<'a> {
    id: String,
    requests: &'a Mutex<Requests>,
    cancellation: watch::Receiver<bool>,
}
impl Drop for Active<'_> {
    fn drop(&mut self) {
        if let Ok(mut requests) = self.requests.lock() {
            requests.active.remove(&self.id);
        }
    }
}

/// One app-scoped client shares rate limits, backoff and cancellation across all
/// commands. The only selectable destinations are the four verified paths.
pub struct TimetableClient {
    client: Client,
    requests: Mutex<Requests>,
    serial: Semaphore,
    next_request: tokio::sync::Mutex<Instant>,
    details: Mutex<DetailRegistry>,
    #[cfg(test)]
    base_url: Option<url::Url>,
}
impl TimetableClient {
    pub fn new() -> Result<Self> {
        // Explicit maintained Mozilla roots work identically on Android and the
        // host, without platform-verifier JNI initialization or account cookies.
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| TimetableError::SourceUnavailable)?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let client = Client::builder()
            .tls_backend_preconfigured(tls)
            .https_only(true)
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(8))
            .timeout(DEADLINE)
            .pool_max_idle_per_host(1)
            .user_agent("Cestou-personal-client/0.1 (anonymous timetable search)")
            .build()
            .map_err(|_| TimetableError::SourceUnavailable)?;
        Ok(Self {
            client,
            requests: Mutex::new(Requests::default()),
            serial: Semaphore::new(1),
            next_request: tokio::sync::Mutex::new(Instant::now()),
            details: Mutex::new(DetailRegistry::default()),
            #[cfg(test)]
            base_url: None,
        })
    }
    pub async fn lookup_stops(&self, request_id: String, request: StopQuery) -> Result<Vec<Stop>> {
        let query = request.validate()?;
        let form = encode(&[
            ("query", query.into()),
            ("stops_only", "1".into()),
            ("filter", "bus,train,urban".into()),
            ("cityID", "0".into()),
            ("lat", "0".into()),
            ("lng", "0".into()),
        ]);
        self.run(request_id, async {
            let bytes = self.fetch(Endpoint::Stops, form).await?;
            let stops = parse_stops(&bytes)?;
            // Bind autocomplete results to the input instead of accepting a
            // stale/misrouted response with a different phrase.
            let response: StopResponse = provider_json(&bytes, MAX_STOPS_BODY)?;
            if response.input_phrase != query {
                return Err(TimetableError::ProviderChanged);
            }
            Ok(stops)
        })
        .await
    }
    pub async fn search_journeys(
        &self,
        request_id: String,
        request: JourneyQuery,
    ) -> Result<Vec<Journey>> {
        let form = request.form()?;
        let boundary = local_time(&request.date, &request.time, request.time_occurrence)?;
        self.run(request_id, async {
            let bytes = self.fetch(Endpoint::Journeys, form).await?;
            let mut journeys = parse_journeys(&bytes, &self.details, request.transport)?;
            journeys.retain(|journey| {
                (!request.direct_only
                    || (journey.transfers == 0
                        && journey
                            .legs
                            .iter()
                            .filter(|leg| leg.service.is_some())
                            .count()
                            <= 1))
                    && if request.arrive_by {
                        timestamp(&journey.arrival).is_ok_and(|t| t <= boundary)
                    } else {
                        timestamp(&journey.departure).is_ok_and(|t| t >= boundary)
                    }
            });
            Ok(journeys)
        })
        .await
    }
    pub async fn departures(
        &self,
        request_id: String,
        request: DepartureQuery,
    ) -> Result<Vec<Departure>> {
        let form = request.form()?;
        let boundary = local_time(&request.date, &request.time, request.time_occurrence)?;
        self.run(request_id, async {
            let bytes = self.fetch(Endpoint::Departures, form).await?;
            let mut departures =
                parse_departures(&bytes, &request.stop, &self.details, request.transport)?;
            let supplied = departures.len();
            departures.retain(|departure| {
                timestamp(&departure.departure).is_ok_and(|time| time >= boundary)
            });
            if supplied > 0 && departures.is_empty() {
                return Err(TimetableError::ProviderChanged);
            }
            Ok(departures)
        })
        .await
    }
    pub async fn connection_detail(
        &self,
        request_id: String,
        request: DetailQuery,
    ) -> Result<ConnectionDetail> {
        if !valid_text(&request.token, 64, false) {
            return Err(TimetableError::InvalidQuery);
        }
        let reference = self
            .details
            .lock()
            .map_err(|_| TimetableError::SourceUnavailable)?
            .get(&request.token)?;
        self.run(request_id, async {
            let bytes = self.fetch(Endpoint::Detail, reference.form()).await?;
            parse_detail(&bytes, &reference)
        })
        .await
    }
    /// Tombstones also cover cancellation delivered before the matching invoke.
    /// They expire and are bounded, so canceled input is never a history store.
    pub fn cancel_request(&self, request_id: &str) -> Result<()> {
        validate_request_id(request_id)?;
        let mut state = self
            .requests
            .lock()
            .map_err(|_| TimetableError::SourceUnavailable)?;
        prune_tombstones(&mut state);
        if let Some(sender) = state.active.get(request_id) {
            let _ = sender.send(true);
        }
        if state.cancelled.len() >= MAX_TOMBSTONES {
            if let Some(oldest) = state
                .cancelled
                .iter()
                .min_by_key(|(_, at)| *at)
                .map(|(id, _)| id.clone())
            {
                state.cancelled.remove(&oldest);
            }
        }
        state.cancelled.insert(request_id.into(), Instant::now());
        Ok(())
    }
    fn begin(&self, id: String) -> Result<Active<'_>> {
        validate_request_id(&id)?;
        let mut state = self
            .requests
            .lock()
            .map_err(|_| TimetableError::SourceUnavailable)?;
        prune_tombstones(&mut state);
        if state.cancelled.contains_key(&id) {
            return Err(TimetableError::Cancelled);
        }
        if state.active.contains_key(&id) {
            return Err(TimetableError::InvalidQuery);
        }
        if state.active.len() >= MAX_ACTIVE
            || state.backoff_indefinite
            || state.backoff.is_some_and(|until| until > Instant::now())
        {
            return Err(TimetableError::RateLimited);
        }
        let (sender, cancellation) = watch::channel(false);
        state.active.insert(id.clone(), sender);
        Ok(Active {
            id,
            requests: &self.requests,
            cancellation,
        })
    }
    async fn run<T>(&self, id: String, work: impl Future<Output = Result<T>>) -> Result<T> {
        let mut active = self.begin(id)?;
        let result = tokio::select! {
            biased;
            _=active.cancellation.changed()=>Err(TimetableError::Cancelled),
            result=tokio::time::timeout(DEADLINE,work)=>result.unwrap_or(Err(TimetableError::Timeout)),
        };
        // Cancellation can arrive while bounded synchronous parsing is running.
        if *active.cancellation.borrow() {
            return Err(TimetableError::Cancelled);
        }
        result
    }
    async fn fetch(&self, endpoint: Endpoint, form: String) -> Result<Vec<u8>> {
        let _permit = self
            .serial
            .acquire()
            .await
            .map_err(|_| TimetableError::SourceUnavailable)?;
        {
            let mut next = self.next_request.lock().await;
            tokio::time::sleep_until((*next).into()).await;
            let state = self
                .requests
                .lock()
                .map_err(|_| TimetableError::SourceUnavailable)?;
            if state.backoff_indefinite || state.backoff.is_some_and(|until| until > Instant::now())
            {
                return Err(TimetableError::RateLimited);
            }
            *next = Instant::now() + INTERVAL;
        }
        let mut url = url::Url::parse("https://www.ubian.sk")
            .map_err(|_| TimetableError::SourceUnavailable)?;
        #[cfg(test)]
        if let Some(base) = &self.base_url {
            url = base.clone();
        }
        url.set_path(endpoint.path());
        let method = endpoint.method();
        if method == Method::GET {
            url.set_query(Some(&form));
        }
        let mut request = self
            .client
            .request(method.clone(), url)
            .header(header::ACCEPT, "application/json");
        if method == Method::POST {
            request = request
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(form);
        }
        let mut response = request.send().await.map_err(transport_error)?;
        let retry_after = response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|h| h.to_str().ok());
        if response.status() == StatusCode::TOO_MANY_REQUESTS
            || (response.status() == StatusCode::SERVICE_UNAVAILABLE && retry_after.is_some())
        {
            let delay = backoff_duration(retry_after, Utc::now());
            let mut state = self
                .requests
                .lock()
                .map_err(|_| TimetableError::SourceUnavailable)?;
            state.backoff = Instant::now().checked_add(delay);
            state.backoff_indefinite = state.backoff.is_none();
            return Err(TimetableError::RateLimited);
        }
        if matches!(
            response.status(),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::SERVICE_UNAVAILABLE
        ) {
            return Err(TimetableError::SourceUnavailable);
        }
        if !response.status().is_success() {
            return Err(TimetableError::ProviderChanged);
        }
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if content_type != "application/json" && !content_type.ends_with("+json") {
            return Err(TimetableError::ProviderChanged);
        }
        let max = if matches!(endpoint, Endpoint::Stops) {
            MAX_STOPS_BODY
        } else {
            MAX_BODY
        };
        if response
            .content_length()
            .is_some_and(|size| size > max as u64)
        {
            return Err(TimetableError::ProviderChanged);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if bytes.len().saturating_add(chunk.len()) > max {
                return Err(TimetableError::ProviderChanged);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}
fn validate_request_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(TimetableError::InvalidQuery);
    }
    Ok(())
}
fn prune_tombstones(state: &mut Requests) {
    state
        .cancelled
        .retain(|_, at| at.elapsed() < Duration::from_secs(60));
}
fn transport_error(error: reqwest::Error) -> TimetableError {
    if error.is_timeout() {
        TimetableError::Timeout
    } else if error.is_connect() || error.is_body() || error.is_request() {
        TimetableError::Offline
    } else {
        TimetableError::ProviderChanged
    }
}
fn backoff_duration(raw: Option<&str>, now: DateTime<Utc>) -> Duration {
    let seconds = raw
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| {
            let time = DateTime::parse_from_rfc2822(raw?).ok()?;
            Some((time.with_timezone(&Utc) - now).num_seconds().max(0) as u64)
        })
        .unwrap_or(30)
        .max(2);
    Duration::from_secs(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn mock_http_client() -> Client {
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();
        Client::builder()
            .tls_backend_preconfigured(tls)
            .redirect(Policy::none())
            .build()
            .unwrap()
    }

    async fn mock_client(response: Vec<u8>) -> (TimetableClient, tokio::task::JoinHandle<Vec<u8>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 4096];
            let n = stream.read(&mut request).await.unwrap();
            request.truncate(n);
            // A bounded reader may correctly close before an oversized mock is
            // fully written, so a broken pipe is expected in that test.
            let _ = stream.write_all(&response).await;
            request
        });
        let mut client = TimetableClient::new().unwrap();
        client.client = mock_http_client();
        client.base_url = Some(url::Url::parse(&format!("http://{address}")).unwrap());
        (client, server)
    }
    fn response(status: &str, headers: &str, body: &[u8]) -> Vec<u8> {
        let mut bytes =
            format!("HTTP/1.1 {status}\r\nConnection: close\r\n{headers}\r\n").into_bytes();
        bytes.extend_from_slice(body);
        bytes
    }
    #[tokio::test]
    async fn observed_method_query_encoding_and_no_cookie_transport() {
        let body = r#"{"status":"ok","inputPhrase":"Košice","results":[]}"#.as_bytes();
        let (client, server) = mock_client(response(
            "200 OK",
            "Content-Type: application/json\r\n",
            body,
        ))
        .await;
        let stops = client
            .lookup_stops(
                "lookup".into(),
                StopQuery {
                    query: "Košice".into(),
                },
            )
            .await
            .unwrap();
        assert!(stops.is_empty());
        let request = String::from_utf8(server.await.unwrap()).unwrap();
        assert!(request.starts_with("GET /navigation/autocomplete?query=Ko%C5%A1ice&stops_only=1&"));
        assert!(!request.to_ascii_lowercase().contains("cookie:"));
        assert!(!request.to_ascii_lowercase().contains("authorization:"));
    }
    #[tokio::test]
    async fn changed_schema_content_types_redirects_status_and_body_bounds() {
        for (status, headers, body, expected) in [
            (
                "200 OK",
                "Content-Type: text/html\r\n",
                b"challenge".to_vec(),
                TimetableError::ProviderChanged,
            ),
            (
                "200 OK",
                "Content-Type: application/json\r\n",
                b"{}".to_vec(),
                TimetableError::ProviderChanged,
            ),
            (
                "302 Found",
                "Location: https://attacker.invalid/\r\n",
                vec![],
                TimetableError::ProviderChanged,
            ),
            (
                "403 Forbidden",
                "",
                vec![],
                TimetableError::SourceUnavailable,
            ),
            (
                "500 Internal Server Error",
                "",
                vec![],
                TimetableError::ProviderChanged,
            ),
            (
                "200 OK",
                "Content-Type: application/json\r\nContent-Length: 9999999\r\n",
                vec![],
                TimetableError::ProviderChanged,
            ),
            (
                "200 OK",
                "Content-Type: application/json\r\n",
                vec![b' '; MAX_STOPS_BODY + 1],
                TimetableError::ProviderChanged,
            ),
        ] {
            let (client, server) = mock_client(response(status, headers, &body)).await;
            let failure = client
                .lookup_stops(
                    "test".into(),
                    StopQuery {
                        query: "test".into(),
                    },
                )
                .await
                .unwrap_err();
            assert_eq!(failure, expected, "{status} {headers}");
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn server_throttling_stops_other_endpoints_and_transport_errors_stay_local() {
        let (client, server) = mock_client(response(
            "429 Too Many Requests",
            "Retry-After: 3600\r\n",
            b"",
        ))
        .await;
        assert_eq!(
            client
                .lookup_stops(
                    "first".into(),
                    StopQuery {
                        query: "test".into()
                    }
                )
                .await,
            Err(TimetableError::RateLimited)
        );
        server.await.unwrap();
        assert!(
            client
                .requests
                .lock()
                .unwrap()
                .backoff
                .unwrap()
                .duration_since(Instant::now())
                > Duration::from_secs(3500)
        );
        assert_eq!(
            client.run("journey".into(), async { Ok(1) }).await,
            Err(TimetableError::RateLimited)
        );
        let (client, server) = mock_client(vec![]).await;
        assert_eq!(
            client
                .lookup_stops(
                    "broken".into(),
                    StopQuery {
                        query: "test".into()
                    }
                )
                .await,
            Err(TimetableError::Offline)
        );
        server.await.unwrap();
    }
    #[tokio::test]
    async fn previous_day_departures_are_not_a_board_for_the_requested_date() {
        let body = include_bytes!("../../tests/fixtures/departures.json");
        let (client, server) = mock_client(response(
            "200 OK",
            "Content-Type: application/json\r\n",
            body,
        ))
        .await;
        let query = DepartureQuery {
            stop: Stop {
                id: 24355,
                name: "Bratislava, Autobusová stanica".into(),
            },
            date: "2026-11-19".into(),
            time: "06:00".into(),
            transport: Transport::All,
            time_occurrence: None,
        };
        assert_eq!(
            client
                .departures("stale-board".into(), query)
                .await
                .unwrap_err(),
            TimetableError::ProviderChanged
        );
        server.await.unwrap();
    }
    #[tokio::test]
    async fn cancel_drops_real_network_body_read_and_active_slot() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            assert!(stream.read(&mut request).await.unwrap() > 0);
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1000\r\n\r\n{").await.unwrap();
            ready_tx.send(()).unwrap();
            let mut byte = [0; 1];
            tokio::time::timeout(Duration::from_secs(2), stream.read(&mut byte))
                .await
                .unwrap()
                .unwrap()
        });
        let mut client = TimetableClient::new().unwrap();
        client.client = mock_http_client();
        client.base_url = Some(url::Url::parse(&format!("http://{address}")).unwrap());
        let search = client.lookup_stops(
            "cancel-http".into(),
            StopQuery {
                query: "test".into(),
            },
        );
        let cancel = async {
            ready_rx.await.unwrap();
            client.cancel_request("cancel-http").unwrap();
        };
        let (result, _) = tokio::join!(search, cancel);
        assert_eq!(result, Err(TimetableError::Cancelled));
        assert_eq!(server.await.unwrap(), 0);
        assert!(client.requests.lock().unwrap().active.is_empty());
    }
    #[tokio::test]
    async fn cancellation_precedes_registration_and_aborts_inflight_future() {
        let client = TimetableClient::new().unwrap();
        client.cancel_request("first").unwrap();
        assert_eq!(
            client.run("first".into(), async { Ok(1) }).await,
            Err(TimetableError::Cancelled)
        );
        let task = client.run("second".into(), async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok(1)
        });
        let cancel = async {
            tokio::task::yield_now().await;
            client.cancel_request("second").unwrap();
        };
        let (result, _) = tokio::join!(task, cancel);
        assert_eq!(result, Err(TimetableError::Cancelled));
        assert!(client.requests.lock().unwrap().active.is_empty());
    }
    #[tokio::test(start_paused = true)]
    async fn deadline_and_limits_are_shared() {
        let client = TimetableClient::new().unwrap();
        assert_eq!(
            client
                .run("slow".into(), async {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    Ok(1)
                })
                .await,
            Err(TimetableError::Timeout)
        );
        client.requests.lock().unwrap().backoff = Some(Instant::now() + Duration::from_secs(30));
        assert_eq!(
            client.run("stop".into(), async { Ok(1) }).await,
            Err(TimetableError::RateLimited)
        );
        assert_eq!(
            client.run("journey".into(), async { Ok(1) }).await,
            Err(TimetableError::RateLimited)
        );
        for i in 0..1000 {
            client.cancel_request(&format!("request-{i}")).unwrap();
        }
        assert!(client.requests.lock().unwrap().cancelled.len() <= MAX_TOMBSTONES);
        assert!(client.begin("bad/id".into()).is_err());
    }
    #[test]
    fn backoff_accepts_seconds_dates_and_bounds() {
        let now = DateTime::parse_from_rfc3339("2026-11-17T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(backoff_duration(Some("8"), now), Duration::from_secs(8));
        assert_eq!(backoff_duration(Some("0"), now), Duration::from_secs(2));
        assert_eq!(
            backoff_duration(Some("999999999"), now),
            Duration::from_secs(999999999)
        );
        assert_eq!(
            backoff_duration(Some("Tue, 17 Nov 2026 12:00:12 GMT"), now),
            Duration::from_secs(12)
        );
        assert_eq!(backoff_duration(Some("bad"), now), Duration::from_secs(30));
    }
}
