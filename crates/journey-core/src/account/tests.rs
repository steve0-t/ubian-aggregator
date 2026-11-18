use super::*;
use reqwest::cookie::CookieStore as ReqwestCookieStore;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

const LOGIN: &str = r#"<!doctype html><form id="login-form" action="/login" method="POST"><input name="email" type="text"><input name="password" type="password"><input name="rememberme" type="checkbox" checked></form>"#;

// This is a synthetic document using only the selectors verified by the
// authorized probe. It contains no copied account values or private capture.
const PROFILE: &str = r#"<!doctype html><form id="js-profile-form" method="post"><input id="profile-login" name="login" type="text" value="synthetic@example.invalid"><input id="profile-name" name="name" value="Synthetic"><input id="profile-surname" name="surname" value="User"><input id="profile-email" name="email" value="contact@example.invalid"></form>"#;

fn credentials() -> LoginRequest {
    LoginRequest {
        email: "synthetic@example.invalid".into(),
        password: Zeroizing::new("synthetic-test-password".into()),
        remember_session: false,
    }
}
fn profile() -> AccountProfile {
    AccountProfile {
        email: "synthetic@example.invalid".into(),
        display_name: Some("Synthetic User".into()),
    }
}
fn jar() -> Arc<SessionJar> {
    let jar = Arc::new(SessionJar::default());
    let header = reqwest::header::HeaderValue::from_static(
        "session=synthetic-session; Secure; HttpOnly; SameSite=Lax; Path=/",
    );
    jar.set_cookies(
        &mut std::iter::once(&header),
        &Url::parse("https://www.ubian.sk/login").unwrap(),
    );
    jar
}
fn signed_in(client: &AccountClient) {
    let session = Arc::new(client.session(jar()).unwrap());
    let generation = client.generation();
    client.commit(generation, session, profile()).unwrap();
}
fn http(status: &str, headers: &[(&str, &str)], body: &str) -> Vec<u8> {
    let mut response = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
    if !headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("Content-Type"))
    {
        response.push_str("Content-Type: text/html; charset=utf-8\r\n");
    }
    if !headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("Content-Length"))
    {
        response.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    response.push_str(body);
    response.into_bytes()
}
#[derive(Default)]
struct Request {
    method: String,
    path: String,
    headers: String,
    body: Vec<u8>,
}
async fn read_request(stream: &mut tokio::net::TcpStream) -> Request {
    let mut data = Vec::new();
    let mut chunk = [0; 4096];
    let header_end;
    loop {
        let size = stream.read(&mut chunk).await.unwrap();
        assert!(size > 0);
        data.extend_from_slice(&chunk[..size]);
        if let Some(index) = data.windows(4).position(|part| part == b"\r\n\r\n") {
            header_end = index + 4;
            break;
        }
        assert!(data.len() < 128 * 1024);
    }
    let headers = String::from_utf8(data[..header_end].to_vec()).unwrap();
    let length = headers
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map(|(_, v)| v.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    while data.len() - header_end < length {
        let size = stream.read(&mut chunk).await.unwrap();
        assert!(size > 0);
        data.extend_from_slice(&chunk[..size]);
    }
    let mut parts = headers.lines().next().unwrap().split_whitespace();
    Request {
        method: parts.next().unwrap().into(),
        path: parts.next().unwrap().into(),
        headers: headers.clone(),
        body: data[header_end..header_end + length].to_vec(),
    }
}
async fn mock(
    responses: Vec<Vec<u8>>,
) -> (
    AccountClient,
    Arc<Mutex<Vec<Request>>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let capture = requests.clone();
    let server = tokio::spawn(async move {
        for response in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_request(&mut stream).await;
            capture.lock().unwrap().push(request);
            stream.write_all(&response).await.unwrap();
        }
    });
    let mut client = AccountClient::new().unwrap();
    client.base_url = Some(base);
    (client, requests, server)
}

#[test]
fn public_login_form_uses_observed_fields_and_preserves_csrf() {
    let source = LOGIN.replace(
        "<input name=\"email\"",
        "<input type=\"hidden\" name=\"csrf\" value=\"synthetic&amp;token\"><input name=\"email\"",
    );
    let form = html::login_form(source.as_bytes()).unwrap();
    let body = html::login_body(form, &credentials());
    let fields: HashMap<_, _> = url::form_urlencoded::parse(body.as_bytes())
        .into_owned()
        .collect();
    assert_eq!(fields["email"], "synthetic@example.invalid");
    assert_eq!(fields["password"], "synthetic-test-password");
    assert_eq!(fields["csrf"], "synthetic&token");
    assert!(!fields.contains_key("rememberme"));
    let mut request = credentials();
    request.remember_session = true;
    assert!(
        html::login_body(html::login_form(LOGIN.as_bytes()).unwrap(), &request)
            .contains("rememberme=on")
    );
    for source in [
        LOGIN.replace("/login", "https://evil.invalid/login"),
        LOGIN.replace("method=\"POST\"", "method=\"GET\""),
        LOGIN.replace("name=\"password\"", "name=\"changed\""),
        LOGIN.repeat(2),
        LOGIN.replace("<form", "<div class=\"g-recaptcha\"></div><form"),
    ] {
        assert!(html::login_form(source.as_bytes()).is_err());
    }
}
#[test]
fn login_inputs_and_response_markers_fail_closed() {
    for email in [
        "",
        "missing-at.invalid",
        " user@example.invalid",
        "user@example.invalid\n",
    ] {
        let mut request = credentials();
        request.email = email.into();
        assert!(request.validate().is_err());
    }
    let mut request = credentials();
    request.password = Zeroizing::new("".into());
    assert!(request.validate().is_err());
    assert_eq!(
        html::profile(LOGIN.as_bytes()),
        Err(AccountError::ProviderChanged)
    );
    assert_eq!(
        html::profile(b"<html>HTTP 200 and no login form</html>"),
        Err(AccountError::ProviderChanged)
    );
    assert_eq!(
        html::login_error(b"<div class='h-captcha'></div>"),
        Ok(AccountError::ChallengeRequired)
    );
    assert_eq!(
        html::login_error(LOGIN.as_bytes()),
        Ok(AccountError::ProviderChanged)
    );
    let failure=format!("{LOGIN}<script>ubian.wrongPassword({{\"email\":\"synthetic@example.invalid\",\"password\":\"synthetic-test-password\"}});</script>");
    assert_eq!(
        html::login_error(failure.as_bytes()),
        Ok(AccountError::InvalidCredentials)
    );
    assert_eq!(
        serde_json::to_string(&html::login_error(failure.as_bytes()).unwrap()).unwrap(),
        "\"invalid_credentials\""
    );
}
#[test]
fn cookies_keep_provider_scope_flags_expiry_and_session_semantics() {
    let jar = jar();
    let origin = Url::parse("https://www.ubian.sk/").unwrap();
    let exported = jar.export().unwrap();
    let encoded = serde_json::to_string(&exported).unwrap();
    assert!(
        encoded.contains("HttpOnly")
            && encoded.contains("Secure")
            && encoded.contains("SameSite=Lax")
    );
    let restored = SessionJar::restore(exported).unwrap();
    assert_eq!(
        restored.cookies(&origin).unwrap(),
        "session=synthetic-session"
    );
    for url in [
        "https://evil.invalid/",
        "http://www.ubian.sk/",
        "https://ubian.sk/",
        "https://www.ubian.sk.evil.invalid/",
    ] {
        assert!(restored.cookies(&Url::parse(url).unwrap()).is_none());
    }
    let erase = reqwest::header::HeaderValue::from_static("unused=; Max-Age=0; Path=/; Secure");
    restored.set_cookies(&mut std::iter::once(&erase), &origin);
    assert!(restored.valid().is_ok());
    let invalid =
        reqwest::header::HeaderValue::from_static("bad=value; Domain=evil.invalid; Path=/");
    restored.set_cookies(&mut std::iter::once(&invalid), &origin);
    assert_eq!(restored.valid(), Err(AccountError::ProviderChanged));
    assert!(SessionJar::restore(json!([])).is_err());
    let mut foreign = jar.export().unwrap();
    foreign[0]["domain"] = json!({"HostOnly":"evil.invalid"});
    assert!(SessionJar::restore(foreign).is_err());
}
#[test]
fn exported_session_has_version_source_age_and_cache_but_no_password() {
    let client = AccountClient::new().unwrap();
    assert_eq!(
        client.export_session().err(),
        Some(AccountError::NotSignedIn)
    );
    signed_in(&client);
    let payload = client.export_session().unwrap();
    assert!(!payload.contains("synthetic-test-password"));
    assert!(!payload.contains("password"));
    let saved: SavedSession = serde_json::from_str(&payload).unwrap();
    assert_eq!(saved.version, 1);
    assert_eq!(saved.source, ORIGIN);
    assert!(decode_saved(&payload).is_ok());
    let mut altered: serde_json::Value = serde_json::from_str(&payload).unwrap();
    altered["source"] = json!("https://evil.invalid");
    assert!(decode_saved(&altered.to_string()).is_err());
    altered["source"] = json!(ORIGIN);
    altered["savedAt"] = json!("2000-01-01T00:00:00Z");
    assert!(decode_saved(&altered.to_string()).is_err());
    assert!(decode_saved(&"x".repeat(cookies::MAX_SESSION + 1)).is_err());
}

#[tokio::test]
async fn unknown_login_response_cannot_claim_authentication_and_cookie_jar_is_owned() {
    let (client, requests, server) = mock(vec![
        http(
            "200 OK",
            &[("Set-Cookie", "anonymous=synthetic; Secure; Path=/")],
            LOGIN,
        ),
        http("200 OK", &[], LOGIN),
    ])
    .await;
    assert_eq!(
        client.login("login".into(), credentials()).await,
        Err(AccountError::ProviderChanged)
    );
    server.await.unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(
        (requests[0].method.as_str(), requests[0].path.as_str()),
        ("GET", "/login")
    );
    assert_eq!(
        (requests[1].method.as_str(), requests[1].path.as_str()),
        ("POST", "/login")
    );
    assert!(requests[1]
        .headers
        .to_ascii_lowercase()
        .contains("cookie: anonymous=synthetic"));
    let body = String::from_utf8(requests[1].body.clone()).unwrap();
    assert!(body.contains("password=synthetic-test-password"));
    assert_eq!(client.snapshot().unwrap().state, AccountState::SignedOut);
}
#[tokio::test]
async fn network_failures_schema_redirects_and_rate_limit_are_explicit() {
    for (response, error) in [
        (
            http("403 Forbidden", &[], ""),
            AccountError::ProviderUnavailable,
        ),
        (
            http("500 Internal Server Error", &[], ""),
            AccountError::ProviderChanged,
        ),
        (
            http(
                "302 Found",
                &[("Location", "https://evil.invalid/login")],
                "",
            ),
            AccountError::ProviderChanged,
        ),
        (
            http("307 Temporary Redirect", &[("Location", "/unknown")], ""),
            AccountError::ProviderChanged,
        ),
        (
            http("200 OK", &[("Content-Type", "application/json")], "{}"),
            AccountError::ProviderChanged,
        ),
        (
            http("200 OK", &[("Content-Length", "99999999")], ""),
            AccountError::ProviderChanged,
        ),
    ] {
        let (client, _, server) = mock(vec![response]).await;
        assert_eq!(
            client.login("request".into(), credentials()).await,
            Err(error)
        );
        server.await.unwrap();
    }
    let (client, requests, server) = mock(vec![http(
        "429 Too Many Requests",
        &[("Retry-After", "3600")],
        "",
    )])
    .await;
    assert_eq!(
        client.login("first".into(), credentials()).await,
        Err(AccountError::RateLimited)
    );
    server.await.unwrap();
    assert_eq!(
        client.login("second".into(), credentials()).await,
        Err(AccountError::RateLimited)
    );
    assert_eq!(requests.lock().unwrap().len(), 1);
    let report = client.logout("logout".into()).await.unwrap();
    assert!(!report.server_confirmed);
    assert_eq!(report.snapshot.state, AccountState::SignedOut);
}
#[tokio::test]
async fn cancellation_stops_body_network_and_tombstones_cover_ipc_reordering() {
    let client = AccountClient::new().unwrap();
    client.cancel_request("early").unwrap();
    assert_eq!(
        client.login("early".into(), credentials()).await,
        Err(AccountError::Cancelled)
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut client = AccountClient::new().unwrap();
    client.base_url =
        Some(Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap());
    let (ready_send, ready) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        read_request(&mut stream).await;
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 9999\r\n\r\n",
            )
            .await
            .unwrap();
        ready_send.send(()).unwrap();
        let mut byte = [0];
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(3), stream.read(&mut byte))
                .await
                .unwrap()
                .unwrap(),
            0
        );
    });
    let request = client.login("stream".into(), credentials());
    let cancel = async {
        ready.await.unwrap();
        client.cancel_request("stream").unwrap();
    };
    let (result, _) = tokio::join!(request, cancel);
    assert_eq!(result, Err(AccountError::Cancelled));
    server.await.unwrap();
    assert_eq!(client.snapshot().unwrap().state, AccountState::SignedOut);
}
#[tokio::test]
async fn restore_offline_exposes_only_cached_data_and_forget_fences_late_commit() {
    let source = AccountClient::new().unwrap();
    signed_in(&source);
    let payload = source.export_session().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let mut target = AccountClient::new().unwrap();
    target.base_url = Some(Url::parse(&format!("http://{address}/")).unwrap());
    let snapshot = target
        .restore_session("restore".into(), payload)
        .await
        .unwrap();
    assert_eq!(snapshot.state, AccountState::Cached);
    assert_eq!(snapshot.profile, Some(profile()));
    assert_eq!(
        target.export_session().err(),
        Some(AccountError::NotSignedIn)
    );
    let generation = target.generation();
    let session = target.state.lock().unwrap().session.clone().unwrap();
    target.forget().unwrap();
    assert_eq!(
        target.commit(generation, session, profile()),
        Err(AccountError::Cancelled)
    );
    assert_eq!(target.snapshot().unwrap(), AccountSnapshot::default());
}

#[test]
fn verified_profile_schema_uses_login_identity_instead_of_contact_email() {
    assert_eq!(html::profile(PROFILE.as_bytes()), Ok(profile()));
    for source in [
        PROFILE.replace("js-profile-form", "other-form"),
        PROFILE.replace("name=\"login\"", "name=\"changed\""),
        PROFILE.replace("method=\"post\"", "method=\"get\""),
        PROFILE.replace("<form", "<form action=\"/changed\""),
        PROFILE.replace("synthetic@example.invalid", "invalid-email"),
        PROFILE.repeat(2),
    ] {
        assert_eq!(
            html::profile(source.as_bytes()),
            Err(AccountError::ProviderChanged)
        );
    }
}
#[tokio::test]
async fn verified_login_refresh_restore_and_logout_require_the_protected_profile() {
    let (client, requests, server) = mock(vec![
        http(
            "200 OK",
            &[("Set-Cookie", "anonymous=synthetic; Secure; Path=/")],
            LOGIN,
        ),
        http(
            "302 Found",
            &[
                ("Location", "/card/new"),
                (
                    "Set-Cookie",
                    "session=synthetic-session; Secure; HttpOnly; SameSite=Lax; Path=/",
                ),
            ],
            "",
        ),
        http("200 OK", &[], PROFILE),
        http("200 OK", &[], PROFILE),
        http("200 OK", &[], PROFILE),
        http(
            "302 Found",
            &[
                ("Location", "/login"),
                (
                    "Set-Cookie",
                    "session=; Secure; HttpOnly; Max-Age=0; Path=/",
                ),
            ],
            "",
        ),
        http("200 OK", &[], LOGIN),
        http("302 Found", &[("Location", "/login")], ""),
        http("200 OK", &[], LOGIN),
        http("302 Found", &[("Location", "/login_form")], ""),
        http("302 Found", &[("Location", "/login")], ""),
        http("200 OK", &[], LOGIN),
    ])
    .await;
    let signed = client.login("login".into(), credentials()).await.unwrap();
    assert_eq!(signed.state, AccountState::SignedIn);
    assert_eq!(signed.profile, Some(profile()));
    assert_eq!(
        client.refresh("refresh".into()).await.unwrap().state,
        AccountState::SignedIn
    );
    let saved = client.export_session().unwrap();
    let mut restored = AccountClient::new().unwrap();
    restored.base_url = client.base_url.clone();
    assert_eq!(
        restored
            .restore_session("restore".into(), saved.clone())
            .await
            .unwrap()
            .state,
        AccountState::SignedIn
    );
    let logout = restored.logout("logout".into()).await.unwrap();
    assert!(logout.server_confirmed);
    assert_eq!(logout.snapshot, AccountSnapshot::default());
    let mut expired = AccountClient::new().unwrap();
    expired.base_url = client.base_url.clone();
    assert_eq!(
        expired.restore_session("expired".into(), saved).await,
        Err(AccountError::SessionExpired)
    );
    assert_eq!(expired.snapshot().unwrap().state, AccountState::Expired);
    assert!(expired.snapshot().unwrap().profile.is_none());
    server.await.unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 12);
    assert!(!requests.iter().any(|r| r.path == "/card/new"));
    assert_eq!(requests[2].path, "/user/profile");
    assert!(requests[2].headers.contains("session=synthetic-session"));
    assert!(requests[7].headers.contains("session=synthetic-session"));
}
#[tokio::test]
async fn a_different_returned_identity_cannot_become_the_submitted_account() {
    let wrong = PROFILE.replace("synthetic@example.invalid", "different@example.invalid");
    let (client, _, server) = mock(vec![
        http("200 OK", &[], LOGIN),
        http("302 Found", &[("Location", "/")], ""),
        http("200 OK", &[], &wrong),
    ])
    .await;
    assert_eq!(
        client.login("wrong-identity".into(), credentials()).await,
        Err(AccountError::ProviderChanged)
    );
    server.await.unwrap();
    assert_eq!(client.snapshot().unwrap().state, AccountState::SignedOut);
}

#[tokio::test(start_paused = true)]
async fn account_deadline_and_duplicate_generations_fence_obsolete_completions() {
    let client = AccountClient::new().unwrap();
    let mut slow = client.begin("slow".into(), false, true).unwrap();
    assert_eq!(
        client
            .run(&mut slow, async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(())
            })
            .await,
        Err(AccountError::Timeout)
    );
    drop(slow);
    let old = client.begin("reused".into(), false, true).unwrap();
    assert_eq!(
        client.begin("reused".into(), false, true).err(),
        Some(AccountError::InvalidInput)
    );
    let newer = client.begin("newer".into(), false, true).unwrap();
    let reused = client.begin("reused".into(), false, true).unwrap();
    drop(old);
    drop(newer);
    assert!(client.state.lock().unwrap().active.contains_key("reused"));
    drop(reused);
    assert!(client.state.lock().unwrap().active.is_empty());
    signed_in(&client);
    client.state.lock().unwrap().backoff = Some(Instant::now() + Duration::from_secs(300));
    let report = client.logout("rate-limited-logout".into()).await.unwrap();
    assert!(!report.server_confirmed);
    assert_eq!(report.snapshot, AccountSnapshot::default());
}
