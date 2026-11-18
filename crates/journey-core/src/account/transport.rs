use super::*;
use reqwest::{header, redirect::Policy, Method, StatusCode};
use std::future::Future;
use tokio::sync::Semaphore;
use url::Url;

const ORIGIN: &str = "https://www.ubian.sk";
const DEADLINE: Duration = Duration::from_secs(30);
const MAX_BODY: usize = 1024 * 1024;
const INTERVAL: Duration = Duration::from_millis(750);
const MAX_SAVED_AGE: i64 = 30 * 24 * 60 * 60;

pub struct AccountClient {
    state: Mutex<State>,
    serial: Semaphore,
    next_request: tokio::sync::Mutex<Instant>,
    #[cfg(test)]
    base_url: Option<Url>,
}
impl AccountClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            state: Mutex::new(State::default()),
            serial: Semaphore::new(1),
            next_request: tokio::sync::Mutex::new(Instant::now()),
            #[cfg(test)]
            base_url: None,
        })
    }
    pub fn snapshot(&self) -> Result<AccountSnapshot> {
        self.state
            .lock()
            .map(|state| state.snapshot.clone())
            .map_err(|_| AccountError::ProviderUnavailable)
    }
    pub fn generation(&self) -> u64 {
        self.state
            .lock()
            .map(|state| state.generation)
            .unwrap_or(u64::MAX)
    }
    pub fn is_current(&self, generation: u64) -> bool {
        self.state
            .lock()
            .is_ok_and(|state| state.generation == generation)
    }
    pub async fn login(
        &self,
        request_id: String,
        request: LoginRequest,
    ) -> Result<AccountSnapshot> {
        request.validate()?;
        let mut operation = self.begin(request_id, true, true)?;
        let generation = operation.generation;
        let result = self
            .run(&mut operation, async {
                let session = Arc::new(self.session(Arc::new(SessionJar::default()))?);
                let page = self.get(&session, "/login").await?;
                if page.path != "/login" {
                    return Err(AccountError::ProviderChanged);
                }
                let form = html::login_form(&page.body)?;
                let expected_email = request.email.clone();
                let encoded = html::login_body(form, &request);
                // Release the input record before sending. The bounded encoded body
                // and HTTP transport own the remaining temporary copies.
                drop(request);
                let response = self
                    .request(&session, Method::POST, "/login", Some(encoded))
                    .await?;
                // Read the fixed protected account page independently. The submitted
                // login response and its cookies cannot themselves prove success.
                if response.path == "/login" && html::is_login(&response.body) {
                    return Err(html::login_error(&response.body)?);
                }
                let profile = self.verify_profile(&session).await?;
                if !profile.email.eq_ignore_ascii_case(&expected_email) {
                    return Err(AccountError::ProviderChanged);
                }
                self.commit(generation, session, profile)
            })
            .await;
        if result.is_err() {
            self.clear_failed_login(generation);
        }
        result
    }
    pub async fn refresh(&self, request_id: String) -> Result<AccountSnapshot> {
        let mut operation = self.begin(request_id, false, true)?;
        let generation = operation.generation;
        let session = self
            .state
            .lock()
            .map_err(|_| AccountError::ProviderUnavailable)?
            .session
            .clone();
        let Some(session) = session else {
            self.clear_local(generation, AccountState::SignedOut);
            return Err(AccountError::NotSignedIn);
        };
        let expected_email = self
            .snapshot()?
            .profile
            .map(|profile| profile.email)
            .ok_or(AccountError::NotSignedIn)?;
        let result = self
            .run(&mut operation, async {
                let profile = self.verify_profile(&session).await?;
                if !profile.email.eq_ignore_ascii_case(&expected_email) {
                    return Err(AccountError::ProviderChanged);
                }
                self.commit(generation, session, profile)
            })
            .await;
        if let Err(error) = result {
            self.apply_refresh_failure(generation, error);
            return Err(error);
        }
        result
    }
    pub async fn restore_session(
        &self,
        request_id: String,
        payload: Zeroizing<String>,
    ) -> Result<AccountSnapshot> {
        let saved = decode_saved(&payload)?;
        let expected_email = saved.profile.email.clone();
        let jar = Arc::new(SessionJar::restore(saved.cookies)?);
        let session = Arc::new(self.session(jar)?);
        let mut operation = self.begin(request_id, true, true)?;
        let generation = operation.generation;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AccountError::ProviderUnavailable)?;
            if state.generation != generation {
                return Err(AccountError::Cancelled);
            }
            state.session = Some(session.clone());
            state.snapshot = AccountSnapshot {
                state: AccountState::Cached,
                profile: Some(saved.profile),
                last_updated: Some(saved.verified_at),
            };
        }
        let result = self
            .run(&mut operation, async {
                let profile = self.verify_profile(&session).await?;
                if !profile.email.eq_ignore_ascii_case(&expected_email) {
                    return Err(AccountError::ProviderChanged);
                }
                self.commit(generation, session, profile)
            })
            .await;
        match result {
            Err(error @ (AccountError::Offline | AccountError::Timeout)) => {
                self.apply_refresh_failure(generation, error);
                self.snapshot()
            }
            Err(AccountError::Cancelled) => {
                self.clear_local(generation, AccountState::SignedOut);
                Err(AccountError::Cancelled)
            }
            Err(error) => {
                self.apply_refresh_failure(generation, error);
                Err(error)
            }
            Ok(snapshot) => Ok(snapshot),
        }
    }
    /// Secret serialization is Rust-internal. There is no frontend command to
    /// read this payload; only the native encrypted store receives it.
    pub fn export_session(&self) -> Result<Zeroizing<String>> {
        let state = self
            .state
            .lock()
            .map_err(|_| AccountError::ProviderUnavailable)?;
        if state.snapshot.state != AccountState::SignedIn {
            return Err(AccountError::NotSignedIn);
        }
        let session = state.session.as_ref().ok_or(AccountError::NotSignedIn)?;
        let saved = SavedSession {
            version: 1,
            source: ORIGIN.into(),
            saved_at: now(),
            verified_at: state
                .snapshot
                .last_updated
                .clone()
                .ok_or(AccountError::NotSignedIn)?,
            profile: state
                .snapshot
                .profile
                .clone()
                .ok_or(AccountError::NotSignedIn)?,
            cookies: session.jar.export()?,
        };
        let serialized = Zeroizing::new(
            serde_json::to_string(&saved).map_err(|_| AccountError::PersistenceInvalid)?,
        );
        if serialized.len() > cookies::MAX_SESSION {
            return Err(AccountError::PersistenceInvalid);
        }
        Ok(serialized)
    }
    /// Local logout happens before the optional server request, so offline or
    /// canceled work can never resurrect the cleared account.
    pub async fn logout(&self, request_id: String) -> Result<LogoutReport> {
        let session = self
            .state
            .lock()
            .map_err(|_| AccountError::ProviderUnavailable)?
            .session
            .clone();
        let mut operation = self.begin(request_id, true, false)?;
        self.clear_local(operation.generation, AccountState::SignedOut);
        let Some(session) = session else {
            return Ok(LogoutReport {
                server_confirmed: false,
                snapshot: self.snapshot()?,
            });
        };
        let result = self
            .run(&mut operation, async {
                // Verify revocation using an isolated copy of the pre-logout
                // cookies. A cleared local jar alone cannot prove server logout.
                let verification = session
                    .jar
                    .export()
                    .and_then(SessionJar::restore)
                    .and_then(|jar| self.session(Arc::new(jar)))
                    .ok();
                self.get(&session, "/logout").await?;
                if let Some(verification) = verification {
                    match self.verify_profile(&verification).await {
                        Err(AccountError::SessionExpired) => Ok(true),
                        Ok(_) => Ok(false),
                        Err(error) => Err(error),
                    }
                } else {
                    Ok(false)
                }
            })
            .await;
        match result {
            Err(AccountError::Cancelled) => Err(AccountError::Cancelled),
            result => Ok(LogoutReport {
                server_confirmed: result.unwrap_or(false),
                snapshot: self.snapshot()?,
            }),
        }
    }
    pub fn forget(&self) -> Result<AccountSnapshot> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AccountError::ProviderUnavailable)?;
        state.generation = state.generation.wrapping_add(1);
        cancel_all(&mut state);
        state.session = None;
        state.snapshot = AccountSnapshot::default();
        Ok(state.snapshot.clone())
    }
    pub fn cancel_request(&self, request_id: &str) -> Result<()> {
        validate_id(request_id)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| AccountError::ProviderUnavailable)?;
        prune(&mut state);
        if let Some(sender) = state.active.get(request_id) {
            let _ = sender.send(true);
        }
        if state.cancelled.len() >= 128 {
            if let Some(id) = state
                .cancelled
                .iter()
                .min_by_key(|(_, at)| *at)
                .map(|(id, _)| id.clone())
            {
                state.cancelled.remove(&id);
            }
        }
        state.cancelled.insert(request_id.into(), Instant::now());
        Ok(())
    }
    fn begin(&self, id: String, clear: bool, network: bool) -> Result<Operation<'_>> {
        validate_id(&id)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| AccountError::ProviderUnavailable)?;
        prune(&mut state);
        if state.cancelled.contains_key(&id) {
            return Err(AccountError::Cancelled);
        }
        if state.active.contains_key(&id) {
            return Err(AccountError::InvalidInput);
        }
        if network
            && (state.indefinite_backoff || state.backoff.is_some_and(|at| at > Instant::now()))
        {
            return Err(AccountError::RateLimited);
        }
        state.generation = state.generation.wrapping_add(1);
        cancel_all(&mut state);
        if clear {
            state.session = None;
            state.snapshot = AccountSnapshot {
                state: AccountState::SigningIn,
                ..AccountSnapshot::default()
            };
        }
        let (sender, cancel) = watch::channel(false);
        state.active.insert(id.clone(), sender);
        Ok(Operation {
            id,
            generation: state.generation,
            state: &self.state,
            cancel,
        })
    }
    async fn run<T>(
        &self,
        operation: &mut Operation<'_>,
        work: impl Future<Output = Result<T>>,
    ) -> Result<T> {
        let result = tokio::select! {biased;
            _=operation.cancel.changed()=>Err(AccountError::Cancelled),
            result=tokio::time::timeout(DEADLINE,work)=>result.unwrap_or(Err(AccountError::Timeout)),
        };
        if *operation.cancel.borrow() || !self.is_current(operation.generation) {
            return Err(AccountError::Cancelled);
        }
        result
    }
    fn commit(
        &self,
        generation: u64,
        session: Arc<Session>,
        profile: AccountProfile,
    ) -> Result<AccountSnapshot> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AccountError::ProviderUnavailable)?;
        if state.generation != generation {
            return Err(AccountError::Cancelled);
        }
        state.session = Some(session);
        state.snapshot = AccountSnapshot {
            state: AccountState::SignedIn,
            profile: Some(profile),
            last_updated: Some(now()),
        };
        Ok(state.snapshot.clone())
    }
    fn clear_failed_login(&self, generation: u64) {
        self.clear_local(generation, AccountState::SignedOut);
    }
    fn clear_local(&self, generation: u64, phase: AccountState) {
        if let Ok(mut state) = self.state.lock() {
            if state.generation == generation {
                state.session = None;
                state.snapshot = AccountSnapshot {
                    state: phase,
                    ..AccountSnapshot::default()
                };
            }
        }
    }
    fn apply_refresh_failure(&self, generation: u64, error: AccountError) {
        if matches!(
            error,
            AccountError::SessionExpired | AccountError::InvalidCredentials
        ) {
            self.clear_local(generation, AccountState::Expired);
        } else if let Ok(mut state) = self.state.lock() {
            if state.generation == generation && state.snapshot.profile.is_some() {
                state.snapshot.state = AccountState::Cached;
            }
        }
    }
    fn session(&self, jar: Arc<SessionJar>) -> Result<Session> {
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| AccountError::ProviderUnavailable)?
        .with_root_certificates(roots)
        .with_no_client_auth();
        #[allow(unused_mut)]
        let mut builder = Client::builder()
            .tls_backend_preconfigured(tls)
            .https_only(true)
            .redirect(Policy::none())
            .cookie_provider(jar.clone())
            .connect_timeout(Duration::from_secs(8))
            .timeout(DEADLINE)
            .user_agent("Cestou-personal-client/0.1 (native account session)");
        #[cfg(test)]
        if self.base_url.is_some() {
            builder = builder
                .https_only(false)
                .cookie_provider(Arc::new(TestCookies(jar.clone())));
        }
        let client = builder
            .build()
            .map_err(|_| AccountError::ProviderUnavailable)?;
        Ok(Session { client, jar })
    }
    async fn verify_profile(&self, session: &Session) -> Result<AccountProfile> {
        let response = self.get(session, profile_path()).await?;
        if response.path == "/login" && html::is_login(&response.body) {
            return Err(AccountError::SessionExpired);
        }
        if response.path != profile_path() {
            return Err(AccountError::ProviderChanged);
        }
        html::profile(&response.body)
    }
    async fn get(&self, session: &Session, path: &str) -> Result<Page> {
        self.request(session, Method::GET, path, None).await
    }
    async fn request(
        &self,
        session: &Session,
        mut method: Method,
        path: &str,
        mut body: Option<Zeroizing<String>>,
    ) -> Result<Page> {
        let _permit = self
            .serial
            .acquire()
            .await
            .map_err(|_| AccountError::ProviderUnavailable)?;
        let mut path = path.to_string();
        for hop in 0..=4 {
            if !allowed_path(&path) {
                return Err(AccountError::ProviderChanged);
            }
            {
                let mut next = self.next_request.lock().await;
                tokio::time::sleep_until((*next).into()).await;
                let state = self
                    .state
                    .lock()
                    .map_err(|_| AccountError::ProviderUnavailable)?;
                if state.indefinite_backoff || state.backoff.is_some_and(|at| at > Instant::now()) {
                    return Err(AccountError::RateLimited);
                }
                *next = Instant::now() + INTERVAL;
            }
            let mut policy_url =
                Url::parse(ORIGIN).map_err(|_| AccountError::ProviderUnavailable)?;
            policy_url.set_path(&path);
            #[allow(unused_mut)]
            let mut url = policy_url.clone();
            #[cfg(test)]
            if let Some(base) = &self.base_url {
                url = base.clone();
                url.set_path(&path);
            }
            let mut request = session
                .client
                .request(method.clone(), url.clone())
                .header(header::ACCEPT, "text/html")
                .header(header::CACHE_CONTROL, "no-cache");
            if method == Method::POST {
                request = request
                    .header(header::ORIGIN, ORIGIN)
                    .header(header::REFERER, format!("{ORIGIN}/login"))
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
                if let Some(encoded) = &body {
                    request = request.body(encoded.as_bytes().to_vec());
                }
            }
            let mut response = request.send().await.map_err(network_error)?;
            session.jar.valid()?;
            let retry_after = response
                .headers()
                .get(header::RETRY_AFTER)
                .and_then(|h| h.to_str().ok());
            if response.status() == StatusCode::TOO_MANY_REQUESTS
                || (response.status() == StatusCode::SERVICE_UNAVAILABLE && retry_after.is_some())
            {
                let seconds = retry_after_seconds(retry_after);
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| AccountError::ProviderUnavailable)?;
                state.backoff = Instant::now().checked_add(Duration::from_secs(seconds));
                state.indefinite_backoff = state.backoff.is_none();
                return Err(AccountError::RateLimited);
            }
            if response.status().is_redirection() {
                if hop == 4 {
                    return Err(AccountError::ProviderChanged);
                }
                let location = response
                    .headers()
                    .get(header::LOCATION)
                    .and_then(|h| h.to_str().ok())
                    .ok_or(AccountError::ProviderChanged)?;
                // Ubian's email login redirects to /eshop with a query. The
                // query is unnecessary and may be sensitive: never log, retain
                // or fetch it. This is only a login-success candidate; the fixed
                // protected profile is still the sole authentication proof.
                if method == Method::POST
                    && matches!(response.status(), StatusCode::FOUND | StatusCode::SEE_OTHER)
                {
                    if let Some(path) = login_success_redirect_path(&policy_url, location)? {
                        return Ok(Page {
                            path,
                            body: Zeroizing::new(Vec::new()),
                        });
                    }
                }
                let next = redirect_path(&policy_url, location)?;
                if matches!(
                    response.status(),
                    StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND | StatusCode::SEE_OTHER
                ) {
                    method = Method::GET;
                    body = None;
                } else if method == Method::POST {
                    return Err(AccountError::ProviderChanged);
                }
                path = next;
                continue;
            }
            if response.status() == StatusCode::UNAUTHORIZED {
                return Err(AccountError::SessionExpired);
            }
            if matches!(
                response.status(),
                StatusCode::FORBIDDEN | StatusCode::SERVICE_UNAVAILABLE
            ) {
                return Err(AccountError::ProviderUnavailable);
            }
            if !response.status().is_success() {
                return Err(AccountError::ProviderChanged);
            }
            if !response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|h| h.to_str().ok())
                .is_some_and(|s| {
                    s.split(';')
                        .next()
                        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("text/html"))
                })
            {
                return Err(AccountError::ProviderChanged);
            }
            if response
                .content_length()
                .is_some_and(|size| size > MAX_BODY as u64)
            {
                return Err(AccountError::ProviderChanged);
            }
            let mut bytes = Zeroizing::new(Vec::new());
            while let Some(chunk) = response.chunk().await.map_err(network_error)? {
                if bytes.len().saturating_add(chunk.len()) > MAX_BODY {
                    return Err(AccountError::ProviderChanged);
                }
                bytes.extend_from_slice(&chunk);
            }
            return Ok(Page { path, body: bytes });
        }
        Err(AccountError::ProviderChanged)
    }
}
struct Page {
    path: String,
    body: Zeroizing<Vec<u8>>,
}
fn profile_path() -> &'static str {
    "/user/profile"
}
fn allowed_path(path: &str) -> bool {
    matches!(
        path,
        "/" | "/login" | "/login_form" | "/logout" | "/card/new"
    ) || path == profile_path()
}
fn login_success_redirect_path(base: &Url, location: &str) -> Result<Option<String>> {
    if location.len() > 2048
        || location
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
    {
        return Err(AccountError::ProviderChanged);
    }
    let url = base
        .join(location)
        .map_err(|_| AccountError::ProviderChanged)?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.ubian.sk")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(AccountError::ProviderChanged);
    }
    Ok(matches!(url.path(), "/" | "/card/new" | "/eshop").then(|| url.path().to_owned()))
}
fn redirect_path(base: &Url, location: &str) -> Result<String> {
    if location.len() > 512
        || location
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
    {
        return Err(AccountError::ProviderChanged);
    }
    let url = base
        .join(location)
        .map_err(|_| AccountError::ProviderChanged)?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.ubian.sk")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !allowed_path(url.path())
    {
        return Err(AccountError::ProviderChanged);
    }
    Ok(url.path().into())
}
fn retry_after_seconds(value: Option<&str>) -> u64 {
    value
        .and_then(|raw| raw.parse::<u64>().ok())
        .or_else(|| {
            let date = chrono::DateTime::parse_from_rfc2822(value?).ok()?;
            Some((date.with_timezone(&Utc) - Utc::now()).num_seconds().max(0) as u64)
        })
        .unwrap_or(30)
        .max(2)
}
fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(AccountError::InvalidInput);
    }
    Ok(())
}
fn prune(state: &mut State) {
    state
        .cancelled
        .retain(|_, at| at.elapsed() < Duration::from_secs(60));
}
fn cancel_all(state: &mut State) {
    for sender in state.active.values() {
        let _ = sender.send(true);
    }
    state.active.clear();
}
fn network_error(error: reqwest::Error) -> AccountError {
    if error.is_timeout() {
        AccountError::Timeout
    } else if error.is_connect() || error.is_body() || error.is_request() {
        AccountError::Offline
    } else {
        AccountError::ProviderChanged
    }
}
fn decode_saved(payload: &str) -> Result<SavedSession> {
    if payload.len() > cookies::MAX_SESSION {
        return Err(AccountError::PersistenceInvalid);
    }
    let saved: SavedSession =
        serde_json::from_str(payload).map_err(|_| AccountError::PersistenceInvalid)?;
    let saved_at = chrono::DateTime::parse_from_rfc3339(&saved.saved_at)
        .map_err(|_| AccountError::PersistenceInvalid)?;
    let verified_at = chrono::DateTime::parse_from_rfc3339(&saved.verified_at)
        .map_err(|_| AccountError::PersistenceInvalid)?;
    let age = (Utc::now() - saved_at.with_timezone(&Utc)).num_seconds();
    if saved.version != 1
        || saved.source != ORIGIN
        || !(-300..=MAX_SAVED_AGE).contains(&age)
        || verified_at > saved_at
        || !(0..=MAX_SAVED_AGE + 300)
            .contains(&(Utc::now() - verified_at.with_timezone(&Utc)).num_seconds())
        || !valid_text(&saved.profile.email, 254)
        || !saved.profile.email.contains('@')
        || saved
            .profile
            .display_name
            .as_ref()
            .is_some_and(|value| !valid_text(value, 256))
    {
        return Err(AccountError::PersistenceInvalid);
    }
    Ok(saved)
}

// Local HTTP tests preserve the production cookie origin even though the
// socket endpoint is loopback. This adapter does not exist in production.
#[cfg(test)]
struct TestCookies(Arc<SessionJar>);
#[cfg(test)]
impl reqwest::cookie::CookieStore for TestCookies {
    fn set_cookies(&self, headers: &mut dyn Iterator<Item = &header::HeaderValue>, url: &Url) {
        let mut origin = Url::parse(ORIGIN).unwrap();
        origin.set_path(url.path());
        self.0.set_cookies(headers, &origin);
    }
    fn cookies(&self, url: &Url) -> Option<header::HeaderValue> {
        let mut origin = Url::parse(ORIGIN).unwrap();
        origin.set_path(url.path());
        self.0.cookies(&origin)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
