//! Explicit authorized-account smoke check, excluded from the test suite.
//! Feed a bounded JSON LoginRequest through stdin; never pass secrets as args.
//! Output contains only state/validation flags, never profile or session values.
use journey_core::account::{AccountClient, AccountError, AccountState, LoginRequest};
use std::io::Read;
use zeroize::Zeroizing;

async fn check() -> Result<(), AccountError> {
    let mut input = Zeroizing::new(String::new());
    std::io::stdin()
        .take(16 * 1024 + 1)
        .read_to_string(&mut input)
        .map_err(|_| AccountError::InvalidInput)?;
    if input.len() > 16 * 1024 {
        return Err(AccountError::InvalidInput);
    }
    let request: LoginRequest =
        serde_json::from_str(&input).map_err(|_| AccountError::InvalidInput)?;
    drop(input);
    let identity = Zeroizing::new(request.email.clone());
    // This opt-in check retains one narrow-lived zeroizing copy only to verify
    // that persistence does not include the submitted password.
    let password = Zeroizing::new(request.password.as_str().to_owned());
    let client = AccountClient::new()?;
    let initial = client.login("live-account-login".into(), request).await?;
    let verified = initial.state == AccountState::SignedIn
        && initial
            .profile
            .as_ref()
            .is_some_and(|profile| profile.email.eq_ignore_ascii_case(&identity));
    println!("login: verified_identity={verified}");
    if !verified {
        return Err(AccountError::ProviderChanged);
    }
    let refreshed = client.refresh("live-account-refresh".into()).await?;
    println!(
        "refresh: signed_in={}",
        refreshed.state == AccountState::SignedIn
    );
    let saved = client.export_session()?;
    let password_absent = !saved.contains(password.as_str());
    drop(password);
    println!("session_export: password_absent={password_absent}");
    if !password_absent {
        return Err(AccountError::PersistenceInvalid);
    }
    let restored = AccountClient::new()?;
    let snapshot = restored
        .restore_session("live-account-restore".into(), saved.clone())
        .await?;
    let verified_restore = snapshot.state == AccountState::SignedIn
        && snapshot
            .profile
            .as_ref()
            .is_some_and(|profile| profile.email.eq_ignore_ascii_case(&identity));
    println!("restore: verified_identity={verified_restore}");
    if !verified_restore {
        return Err(AccountError::ProviderChanged);
    }
    let report = restored.logout("live-account-logout".into()).await?;
    println!(
        "logout: server_confirmed={}, local_cleared={}",
        report.server_confirmed,
        report.snapshot.state == AccountState::SignedOut && report.snapshot.profile.is_none()
    );
    // The exported session is deliberately reused once only to check revocation.
    let after_logout = AccountClient::new()?;
    match after_logout
        .restore_session("live-account-revocation".into(), saved)
        .await
    {
        Err(AccountError::SessionExpired) => println!("revocation: previous_session_rejected=true"),
        Ok(snapshot) => println!(
            "revocation: previous_session_rejected={}",
            snapshot.state == AccountState::Expired
        ),
        Err(error) => return Err(error),
    }
    client.forget()?;
    Ok(())
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = check().await {
        eprintln!("account_check: {error}");
        std::process::exit(1);
    }
}
