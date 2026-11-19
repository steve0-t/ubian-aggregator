use serde::{Deserialize, Serialize};
mod account;
mod preferences;
#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime, Webview,
};

use journey_core::timetable::{
    ConnectionDetail, Departure, DepartureQuery, DetailQuery, Journey, JourneyQuery, Language,
    Stop, StopQuery, TimetableClient, TimetableError,
};
#[cfg(not(target_os = "android"))]
use journey_core::vault::VaultAvailability;
use journey_core::vault::VaultStatus;
use journey_core::{Destination, NetworkPolicy, POLICY_JSON};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Appearance {
    Light,
    Dark,
    Black,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProviderState {
    Closed,
    Opening,
    Open,
    Cleaning,
    CleanupFailed,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    android: bool,
    policy_version: u32,
    optional_rule_count: usize,
    provider_state: ProviderState,
    last_error: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProbeReport {
    round_trip: bool,
    tamper_rejected: bool,
    unique_nonces: bool,
    missing_key_rejected: bool,
    hardware_backed: bool,
}

#[cfg(target_os = "android")]
struct Platform<R: Runtime>(PluginHandle<R>);

// Capabilities grant only local main assets access. Check the label and origin
// again at the Rust boundary; Android's provider Activity is never a Tauri view.
fn trusted<R: Runtime>(webview: &Webview<R>) -> Result<(), &'static str> {
    if webview.label() != "main" {
        return Err("forbidden");
    }
    let url = webview.url().map_err(|_| "forbidden")?;
    let bundled = url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(
            (url.scheme(), url.host_str()),
            ("tauri", Some("localhost")) | ("http" | "https", Some("tauri.localhost"))
        );
    let development = cfg!(debug_assertions)
        && webview
            .app_handle()
            .config()
            .build
            .dev_url
            .as_ref()
            .is_some_and(|dev| dev.origin() == url.origin());
    if bundled || development {
        Ok(())
    } else {
        Err("forbidden")
    }
}

#[tauri::command]
async fn status<R: Runtime>(webview: Webview<R>) -> Result<Status, &'static str> {
    trusted(&webview)?;
    #[cfg(target_os = "android")]
    {
        let app = webview.app_handle().clone();
        return tauri::async_runtime::spawn_blocking(move || {
            app.state::<Platform<R>>()
                .inner()
                .0
                .run_mobile_plugin("statusNative", ())
                .map_err(|_| "platform_unavailable")
        })
        .await
        .map_err(|_| "platform_unavailable")?;
    }
    #[cfg(not(target_os = "android"))]
    {
        let policy = NetworkPolicy::parse(POLICY_JSON)?;
        Ok(Status {
            android: false,
            policy_version: policy.version,
            optional_rule_count: policy.optional_rule_count(),
            provider_state: ProviderState::Closed,
            last_error: None,
        })
    }
}

#[tauri::command]
async fn open_provider<R: Runtime>(
    webview: Webview<R>,
    destination: Destination,
    language: Language,
) -> Result<(), &'static str> {
    trusted(&webview)?;
    let policy = NetworkPolicy::parse(POLICY_JSON)?;
    if policy.navigation(destination.url()) != journey_core::Navigation::Provider {
        return Err("invalid_destination");
    }
    #[cfg(target_os = "android")]
    {
        #[derive(Serialize)]
        struct Payload {
            destination: Destination,
            language: Language,
        }
        let app = webview.app_handle().clone();
        return tauri::async_runtime::spawn_blocking(move || {
            app.state::<Platform<R>>()
                .inner()
                .0
                .run_mobile_plugin::<()>(
                    "openProviderNative",
                    Payload {
                        destination,
                        language,
                    },
                )
                .map_err(|_| "provider_unavailable")
        })
        .await
        .map_err(|_| "provider_unavailable")?;
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = language;
        Err("android_required")
    }
}

#[tauri::command]
async fn keystore_probe<R: Runtime>(webview: Webview<R>) -> Result<ProbeReport, &'static str> {
    trusted(&webview)?;
    #[cfg(target_os = "android")]
    {
        let app = webview.app_handle().clone();
        return tauri::async_runtime::spawn_blocking(move || {
            app.state::<Platform<R>>()
                .inner()
                .0
                .run_mobile_plugin("keystoreProbeNative", ())
                .map_err(|_| "secure_storage_unavailable")
        })
        .await
        .map_err(|_| "secure_storage_unavailable")?;
    }
    #[cfg(not(target_os = "android"))]
    Err("android_required")
}

#[tauri::command]
async fn lookup_stops<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
    request: StopQuery,
) -> Result<Vec<Stop>, TimetableError> {
    trusted(&webview).map_err(|_| TimetableError::SourceUnavailable)?;
    webview
        .state::<TimetableClient>()
        .lookup_stops(request_id, request)
        .await
}

#[tauri::command]
async fn search_journeys<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
    request: JourneyQuery,
) -> Result<Vec<Journey>, TimetableError> {
    trusted(&webview).map_err(|_| TimetableError::SourceUnavailable)?;
    webview
        .state::<TimetableClient>()
        .search_journeys(request_id, request)
        .await
}

#[tauri::command]
async fn departures<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
    request: DepartureQuery,
) -> Result<Vec<Departure>, TimetableError> {
    trusted(&webview).map_err(|_| TimetableError::SourceUnavailable)?;
    webview
        .state::<TimetableClient>()
        .departures(request_id, request)
        .await
}

#[tauri::command]
async fn connection_detail<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
    request: DetailQuery,
) -> Result<ConnectionDetail, TimetableError> {
    trusted(&webview).map_err(|_| TimetableError::SourceUnavailable)?;
    webview
        .state::<TimetableClient>()
        .connection_detail(request_id, request)
        .await
}

#[tauri::command]
async fn cancel_request<R: Runtime>(
    webview: Webview<R>,
    request_id: String,
) -> Result<(), TimetableError> {
    trusted(&webview).map_err(|_| TimetableError::SourceUnavailable)?;
    webview
        .state::<TimetableClient>()
        .cancel_request(&request_id)
}

#[tauri::command]
async fn vault_status<R: Runtime>(webview: Webview<R>) -> Result<VaultStatus, &'static str> {
    trusted(&webview)?;
    #[cfg(target_os = "android")]
    {
        let app = webview.app_handle().clone();
        return tauri::async_runtime::spawn_blocking(move || {
            let mut status: VaultStatus = app
                .state::<Platform<R>>()
                .inner()
                .0
                .run_mobile_plugin("vaultStatusNative", ())
                .map_err(|_| "secure_storage_unavailable")?;
            // Native preparation can never enable real credential handling.
            status.real_credentials_enabled = false;
            Ok(status)
        })
        .await
        .map_err(|_| "secure_storage_unavailable")?;
    }
    #[cfg(not(target_os = "android"))]
    Ok(VaultStatus {
        availability: VaultAvailability::UnsupportedPlatform,
        synthetic_record_present: false,
        real_credentials_enabled: false,
    })
}

#[tauri::command]
async fn set_appearance<R: Runtime>(
    webview: Webview<R>,
    theme: Appearance,
) -> Result<(), &'static str> {
    trusted(&webview)?;
    #[cfg(target_os = "android")]
    {
        #[derive(Serialize)]
        struct Payload {
            theme: Appearance,
        }
        let app = webview.app_handle().clone();
        return tauri::async_runtime::spawn_blocking(move || {
            app.state::<Platform<R>>()
                .inner()
                .0
                .run_mobile_plugin::<()>("setAppearanceNative", Payload { theme })
                .map_err(|_| "appearance_unavailable")
        })
        .await
        .map_err(|_| "appearance_unavailable")?;
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = theme;
        Ok(())
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("android-platform")
        .invoke_handler(tauri::generate_handler![
            status,
            open_provider,
            keystore_probe,
            lookup_stops,
            search_journeys,
            departures,
            connection_detail,
            cancel_request,
            account::account_status,
            account::account_login,
            account::account_refresh,
            account::account_logout,
            account::account_forget,
            account::account_cancel,
            account::account_save_session,
            account::account_unlock_session,
            account::account_protection,
            vault_status,
            set_appearance,
            preferences::load_theme,
            preferences::save_theme
        ])
        .setup(|app, api| {
            app.manage(TimetableClient::new()?);
            app.manage(account::Accounts::new()?);
            #[cfg(target_os = "android")]
            app.manage(Platform(api.register_android_plugin(
                "sk.cestou.platform",
                "AndroidPlatformPlugin",
            )?));
            #[cfg(not(target_os = "android"))]
            let _ = api;
            Ok(())
        })
        .build()
}
