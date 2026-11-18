//! Local account commands. Session plaintext crosses only the internal mobile
//! bridge; the bundled frontend receives profile/status metadata, never cookies.
use journey_core::account::{AccountClient, AccountError, AccountSnapshot, LoginRequest};
use journey_core::timetable::Language;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};
use tauri::{Manager, Runtime, Webview};
#[cfg(target_os = "android")]
use zeroize::Zeroizing;

pub(super) struct Accounts {
    client: AccountClient,
    gate: tauri::async_runtime::Mutex<()>,
    epoch: AtomicU64,
    control: Mutex<Control>,
}

#[derive(Default)]
struct Control {
    active: Option<(String, OperationKind)>,
    cancelled: HashMap<String, Instant>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OperationKind {
    Authentication,
    Save,
    Cleanup,
    Other,
}

impl Control {
    fn prune(&mut self) {
        self.cancelled
            .retain(|_, at| at.elapsed() < Duration::from_secs(300));
        if self.cancelled.len() >= 128 {
            if let Some(oldest) = self
                .cancelled
                .iter()
                .min_by_key(|(_, at)| *at)
                .map(|(id, _)| id.clone())
            {
                self.cancelled.remove(&oldest);
            }
        }
    }
}

impl Accounts {
    pub(super) fn new() -> Result<Self, AccountError> {
        Ok(Self {
            client: AccountClient::new()?,
            gate: tauri::async_runtime::Mutex::new(()),
            epoch: AtomicU64::new(0),
            control: Mutex::new(Control::default()),
        })
    }

    async fn lock(&self, cleanup: bool) -> Result<tokio::sync::MutexGuard<'_, ()>, String> {
        // A queued invocation must expire before its cancellation tombstone.
        // Even an unreachable biometric bridge cannot leave it waiting forever.
        tokio::time::timeout(Duration::from_secs(90), self.gate.lock())
            .await
            .map_err(|_| {
                if cleanup {
                    "cleanup_failed".to_owned()
                } else {
                    "timeout".to_owned()
                }
            })
    }

    fn supersede(&self, id: &str, kind: OperationKind) -> Result<u64, String> {
        validate_id(id)?;
        let mut control = self.control.lock().map_err(|_| "provider_unavailable")?;
        control.prune();
        if control.cancelled.contains_key(id) && kind != OperationKind::Cleanup {
            return Err("cancelled".into());
        }
        if let Some((previous, _)) = control.active.take() {
            self.client.cancel_request(&previous).map_err(code)?;
        }
        let epoch = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        control.active = Some((id.to_owned(), kind));
        Ok(epoch)
    }

    fn cancel(&self, id: &str) -> Result<Option<u64>, String> {
        validate_id(id)?;
        self.client.cancel_request(id).map_err(code)?;
        let mut control = self.control.lock().map_err(|_| "provider_unavailable")?;
        control.prune();
        control.cancelled.insert(id.to_owned(), Instant::now());
        if let Some((active_id, kind)) = &control.active {
            if active_id != id {
                return Ok(None);
            }
            if *kind == OperationKind::Cleanup {
                // Cancel the optional server request, while local destruction
                // completes under the gate even after navigation away.
                return Ok(None);
            }
            if *kind == OperationKind::Authentication {
                // Covers cancel after verified login committed, while the
                // command is still awaiting storage-status metadata.
                self.client.forget().map_err(code)?;
            }
            control.active = None;
            Ok(Some(self.epoch.fetch_add(1, Ordering::SeqCst) + 1))
        } else {
            // A delayed cancel for an older invoke must not cancel a new one.
            Ok(None)
        }
    }

    fn current(&self, epoch: u64) -> Result<(), String> {
        if self.epoch.load(Ordering::SeqCst) == epoch {
            Ok(())
        } else {
            Err("cancelled".into())
        }
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Err("invalid_input".into())
    } else {
        Ok(())
    }
}

fn code(error: AccountError) -> String {
    error.to_string()
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StorageStatus {
    availability: String,
    record_present: bool,
}

#[derive(Serialize)]
pub(super) struct AccountView {
    #[serde(flatten)]
    account: AccountSnapshot,
    storage: StorageStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct LogoutView {
    account: AccountView,
    server_confirmed: bool,
}

#[cfg(target_os = "android")]
fn storage_error(error: tauri::plugin::mobile::PluginInvokeError) -> String {
    if let tauri::plugin::mobile::PluginInvokeError::InvokeRejected(response) = error {
        if let Some(message) = response.message {
            if matches!(
                message.as_str(),
                "authentication_unavailable"
                    | "authentication_cancelled"
                    | "key_missing"
                    | "key_invalidated"
                    | "tampered"
                    | "unsupported_version"
                    | "invalid_record"
                    | "io_failure"
                    | "busy"
                    | "nonce_reused"
            ) {
                return message;
            }
        }
    }
    "secure_storage_unavailable".into()
}

#[cfg(target_os = "android")]
async fn mobile_call<R: Runtime, T: serde::de::DeserializeOwned>(
    webview: &Webview<R>,
    command: &str,
    payload: impl Serialize,
    deadline: Duration,
) -> Result<T, String> {
    let platform = webview.state::<super::Platform<R>>();
    tokio::time::timeout(
        deadline,
        platform.inner().0.run_mobile_plugin_async(command, payload),
    )
    .await
    .map_err(|_| "timeout".to_owned())?
    .map_err(storage_error)
}

#[cfg(target_os = "android")]
#[derive(Serialize)]
struct Generation {
    generation: u64,
}

async fn native_cancel<R: Runtime>(webview: &Webview<R>, generation: u64) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        mobile_call(
            webview,
            "sessionCancelNative",
            Generation { generation },
            Duration::from_secs(3),
        )
        .await
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (webview, generation);
        Ok(())
    }
}

async fn native_delete<R: Runtime>(webview: &Webview<R>, generation: u64) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        mobile_call(
            webview,
            "sessionDeleteNative",
            Generation { generation },
            Duration::from_secs(5),
        )
        .await
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (webview, generation);
        Ok(())
    }
}

async fn view<R: Runtime>(webview: &Webview<R>) -> Result<AccountView, String> {
    #[cfg(target_os = "android")]
    let storage = mobile_call(
        webview,
        "sessionStorageStatusNative",
        (),
        Duration::from_secs(3),
    )
    .await
    .unwrap_or(StorageStatus {
        availability: "io_failure".into(),
        record_present: true,
    });
    #[cfg(not(target_os = "android"))]
    let storage = StorageStatus {
        availability: "unsupported_platform".into(),
        record_present: false,
    };
    let account = webview
        .state::<Accounts>()
        .client
        .snapshot()
        .map_err(code)?;
    Ok(AccountView { account, storage })
}

async fn current_view<R: Runtime>(webview: &Webview<R>, epoch: u64) -> Result<AccountView, String> {
    let result = view(webview).await?;
    webview.state::<Accounts>().current(epoch)?;
    Ok(result)
}

#[tauri::command]
pub(super) async fn account_status<R: Runtime>(webview: Webview<R>) -> Result<AccountView, String> {
    super::trusted(&webview)?;
    view(&webview).await
}

#[tauri::command]
pub(super) async fn account_login<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
    request: LoginRequest,
) -> Result<AccountView, String> {
    super::trusted(&webview)?;
    let state = webview.state::<Accounts>();
    let _guard = state.lock(false).await?;
    let epoch = state.supersede(&request_id, OperationKind::Authentication)?;
    let _ = native_cancel(&webview, epoch).await;
    state.current(epoch)?;
    // A replacement sign-in never leaves another account's saved session behind.
    state.client.forget().map_err(code)?;
    native_delete(&webview, epoch)
        .await
        .map_err(|_| "cleanup_failed".to_owned())?;
    state.current(epoch)?;
    state
        .client
        .login(request_id, request)
        .await
        .map_err(code)?;
    state.current(epoch)?;
    current_view(&webview, epoch).await
}

#[tauri::command]
pub(super) async fn account_refresh<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
) -> Result<AccountView, String> {
    super::trusted(&webview)?;
    let state = webview.state::<Accounts>();
    let _guard = state.lock(false).await?;
    let epoch = state.supersede(&request_id, OperationKind::Other)?;
    let _ = native_cancel(&webview, epoch).await;
    state.current(epoch)?;
    state.client.refresh(request_id).await.map_err(code)?;
    state.current(epoch)?;
    current_view(&webview, epoch).await
}

#[tauri::command]
pub(super) async fn account_logout<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
) -> Result<LogoutView, String> {
    super::trusted(&webview)?;
    let state = webview.state::<Accounts>();
    let _guard = state.lock(true).await?;
    let epoch = state.supersede(&request_id, OperationKind::Cleanup)?;
    let _ = native_cancel(&webview, epoch).await;
    state.current(epoch)?;
    let result = state.client.logout(request_id).await;
    // A cancel may precede registration of the core logout operation. Local
    // logout still completes even when that server operation never starts.
    state.client.forget().map_err(code)?;
    // Always remove the persisted session, including offline/canceled logout.
    // The operation gate prevents this cleanup deleting a newer account's data.
    let cleanup_epoch = state.epoch.load(Ordering::SeqCst);
    native_delete(&webview, cleanup_epoch)
        .await
        .map_err(|_| "cleanup_failed".to_owned())?;
    state.current(epoch)?;
    let report = result.map_err(code)?;
    Ok(LogoutView {
        account: current_view(&webview, epoch).await?,
        server_confirmed: report.server_confirmed,
    })
}

#[tauri::command]
pub(super) async fn account_forget<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
) -> Result<AccountView, String> {
    super::trusted(&webview)?;
    let state = webview.state::<Accounts>();
    let _guard = state.lock(true).await?;
    let epoch = state.supersede(&request_id, OperationKind::Cleanup)?;
    let _ = native_cancel(&webview, epoch).await;
    state.current(epoch)?;
    state.client.forget().map_err(code)?;
    native_delete(&webview, epoch)
        .await
        .map_err(|_| "cleanup_failed".to_owned())?;
    state.current(epoch)?;
    current_view(&webview, epoch).await
}

#[tauri::command]
pub(super) async fn account_cancel<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
) -> Result<(), String> {
    super::trusted(&webview)?;
    let state = webview.state::<Accounts>();
    if let Some(epoch) = state.cancel(&request_id)? {
        native_cancel(&webview, epoch).await
    } else {
        Ok(())
    }
}

#[tauri::command]
pub(super) async fn account_save_session<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
    language: Language,
) -> Result<AccountView, String> {
    super::trusted(&webview)?;
    let state = webview.state::<Accounts>();
    let _guard = state.lock(false).await?;
    let epoch = state.supersede(&request_id, OperationKind::Save)?;
    let _ = native_cancel(&webview, epoch).await;
    state.current(epoch)?;
    #[cfg(target_os = "android")]
    {
        #[derive(Serialize)]
        struct Payload<'a> {
            snapshot: &'a str,
            language: Language,
            generation: u64,
        }
        let secret = state.client.export_session().map_err(code)?;
        let result: Result<(), String> = mobile_call(
            &webview,
            "sessionStoreNative",
            Payload {
                snapshot: &secret,
                language,
                generation: epoch,
            },
            Duration::from_secs(70),
        )
        .await;
        if let Err(error) = result {
            if error == "timeout" {
                if let Some(cancel_epoch) = state.cancel(&request_id)? {
                    let _ = native_cancel(&webview, cancel_epoch).await;
                }
                native_delete(&webview, state.epoch.load(Ordering::SeqCst))
                    .await
                    .map_err(|_| "cleanup_failed".to_owned())?;
            }
            return Err(error);
        }
        let result = current_view(&webview, epoch).await;
        if result.is_err() {
            native_delete(&webview, state.epoch.load(Ordering::SeqCst))
                .await
                .map_err(|_| "cleanup_failed".to_owned())?;
        }
        result
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = language;
        Err("unsupported_platform".into())
    }
}

#[tauri::command]
pub(super) async fn account_unlock_session<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
    language: Language,
) -> Result<AccountView, String> {
    super::trusted(&webview)?;
    let state = webview.state::<Accounts>();
    let _guard = state.lock(false).await?;
    let epoch = state.supersede(&request_id, OperationKind::Authentication)?;
    let _ = native_cancel(&webview, epoch).await;
    state.current(epoch)?;
    #[cfg(target_os = "android")]
    {
        #[derive(Serialize)]
        struct Payload {
            language: Language,
            generation: u64,
        }
        #[derive(Deserialize)]
        struct Restored {
            snapshot: Zeroizing<String>,
        }
        state.client.forget().map_err(code)?;
        let result: Result<Restored, String> = mobile_call(
            &webview,
            "sessionRestoreNative",
            Payload {
                language,
                generation: epoch,
            },
            Duration::from_secs(70),
        )
        .await;
        let secret = match result {
            Ok(secret) => secret,
            Err(error) => {
                if error == "timeout" {
                    if let Some(cancel_epoch) = state.cancel(&request_id)? {
                        let _ = native_cancel(&webview, cancel_epoch).await;
                    }
                }
                return Err(error);
            }
        };
        state.current(epoch)?;
        state
            .client
            .restore_session(request_id, secret.snapshot)
            .await
            .map_err(code)?;
        state.current(epoch)?;
        current_view(&webview, epoch).await
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = language;
        Err("unsupported_platform".into())
    }
}

#[tauri::command]
pub(super) async fn account_protection<R: Runtime>(
    webview: Webview<R>,
    enabled: bool,
) -> Result<(), String> {
    super::trusted(&webview)?;
    #[cfg(target_os = "android")]
    {
        #[derive(Serialize)]
        struct Payload {
            enabled: bool,
        }
        mobile_call(
            &webview,
            "setAccountProtectionNative",
            Payload { enabled },
            Duration::from_secs(3),
        )
        .await
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = enabled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_before_registration_and_old_cancel_cannot_cancel_new_work() {
        let accounts = Accounts::new().unwrap();
        assert_eq!(accounts.cancel("never-started").unwrap(), None);
        assert_eq!(
            accounts.supersede("never-started", OperationKind::Other),
            Err("cancelled".into())
        );
        let first = accounts.supersede("first", OperationKind::Other).unwrap();
        let second = accounts.supersede("second", OperationKind::Other).unwrap();
        assert_eq!(accounts.current(first), Err("cancelled".into()));
        assert_eq!(accounts.cancel("first").unwrap(), None);
        assert_eq!(accounts.current(second), Ok(()));
        assert!(accounts.cancel("second").unwrap().is_some());
        assert_eq!(accounts.current(second), Err("cancelled".into()));
    }

    #[test]
    fn cancellation_metadata_is_bounded_and_ids_are_validated() {
        let accounts = Accounts::new().unwrap();
        for number in 0..1000 {
            accounts.cancel(&format!("cancel-{number}")).unwrap();
        }
        assert!(accounts.control.lock().unwrap().cancelled.len() <= 128);
        assert_eq!(
            accounts.supersede("bad/id", OperationKind::Other),
            Err("invalid_input".into())
        );
        assert_eq!(accounts.cancel(""), Err("invalid_input".into()));
    }

    #[test]
    fn cleanup_survives_ui_cancel_and_authentication_cancel_clears_session_generation() {
        let accounts = Accounts::new().unwrap();
        let cleanup = accounts
            .supersede("cleanup", OperationKind::Cleanup)
            .unwrap();
        assert_eq!(accounts.cancel("cleanup").unwrap(), None);
        assert_eq!(accounts.current(cleanup), Ok(()));
        accounts.cancel("queued-forget").unwrap();
        assert!(accounts
            .supersede("queued-forget", OperationKind::Cleanup)
            .is_ok());

        let before = accounts.client.generation();
        let authentication = accounts
            .supersede("login", OperationKind::Authentication)
            .unwrap();
        assert!(accounts.cancel("login").unwrap().is_some());
        assert!(accounts.client.generation() > before);
        assert_eq!(accounts.current(authentication), Err("cancelled".into()));
    }
}
