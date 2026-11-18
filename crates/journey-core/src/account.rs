//! Dedicated native Ubian account session. Passwords are accepted once for the
//! fixed login form, never exposed, logged or included in saved sessions. The
//! provider WebView and anonymous timetable adapter do not share this cookie jar.

mod cookies;
mod html;
mod transport;

use chrono::Utc;
use cookies::SessionJar;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::watch;
use zeroize::Zeroizing;

pub use transport::AccountClient;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountError {
    InvalidCredentials,
    InvalidInput,
    Offline,
    Timeout,
    RateLimited,
    ChallengeRequired,
    ProviderChanged,
    SessionExpired,
    Cancelled,
    ProviderUnavailable,
    NotSignedIn,
    PersistenceInvalid,
}
impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Self::InvalidCredentials => "invalid_credentials",
            Self::InvalidInput => "invalid_input",
            Self::Offline => "offline",
            Self::Timeout => "timeout",
            Self::RateLimited => "rate_limited",
            Self::ChallengeRequired => "challenge_required",
            Self::ProviderChanged => "provider_changed",
            Self::SessionExpired => "session_expired",
            Self::Cancelled => "cancelled",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::NotSignedIn => "not_signed_in",
            Self::PersistenceInvalid => "persistence_invalid",
        };
        f.write_str(code)
    }
}
impl std::error::Error for AccountError {}
type Result<T> = std::result::Result<T, AccountError>;

/// Deliberately neither Debug nor Serialize: a password can only enter Rust.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginRequest {
    pub email: String,
    pub password: Zeroizing<String>,
    #[serde(default)]
    pub remember_session: bool,
}
impl LoginRequest {
    fn validate(&self) -> Result<()> {
        let email = self.email.trim();
        if email.is_empty()
            || email.len() > 254
            || email != self.email
            || email.chars().any(|c| c.is_whitespace() || c.is_control())
            || !email.contains('@')
            || email.starts_with('@')
            || email.ends_with('@')
            || self.password.is_empty()
            || self.password.len() > 4096
            || self.password.chars().any(char::is_control)
        {
            return Err(AccountError::InvalidInput);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountState {
    SignedOut,
    SigningIn,
    SignedIn,
    Cached,
    Expired,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountProfile {
    pub email: String,
    pub display_name: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountSnapshot {
    pub state: AccountState,
    pub profile: Option<AccountProfile>,
    pub last_updated: Option<String>,
}
impl Default for AccountSnapshot {
    fn default() -> Self {
        Self {
            state: AccountState::SignedOut,
            profile: None,
            last_updated: None,
        }
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutReport {
    pub server_confirmed: bool,
    pub snapshot: AccountSnapshot,
}

struct Session {
    client: Client,
    jar: Arc<SessionJar>,
}
#[derive(Default)]
struct State {
    generation: u64,
    snapshot: AccountSnapshot,
    session: Option<Arc<Session>>,
    active: HashMap<String, watch::Sender<bool>>,
    cancelled: HashMap<String, Instant>,
    backoff: Option<Instant>,
    indefinite_backoff: bool,
}

struct Operation<'a> {
    id: String,
    generation: u64,
    state: &'a Mutex<State>,
    cancel: watch::Receiver<bool>,
}
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            if state.generation == self.generation {
                state.active.remove(&self.id);
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedSession {
    version: u32,
    source: String,
    saved_at: String,
    verified_at: String,
    profile: AccountProfile,
    cookies: serde_json::Value,
}
fn valid_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max && !value.chars().any(char::is_control)
}
fn now() -> String {
    Utc::now().to_rfc3339()
}
