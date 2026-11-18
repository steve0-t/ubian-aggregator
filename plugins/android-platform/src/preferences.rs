use serde::{Deserialize, Serialize};
use std::{fs, io::ErrorKind, process, sync::Mutex};
use tauri::{Manager, Runtime, Webview};

const STORAGE_ERROR: &str = "theme_storage_unavailable";
const THEME_FILE: &str = "theme.json";
static THEME_WRITE: Mutex<()> = Mutex::new(());

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Theme {
    System,
    Light,
    Dark,
    Black,
}

#[tauri::command]
pub(crate) async fn load_theme<R: Runtime>(
    webview: Webview<R>,
) -> Result<Option<Theme>, &'static str> {
    super::trusted(&webview)?;
    let app = webview.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let directory = app.path().app_config_dir().map_err(|_| STORAGE_ERROR)?;
        match fs::read(directory.join(THEME_FILE)) {
            Ok(contents) => Ok(serde_json::from_slice(&contents).ok()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(_) => Err(STORAGE_ERROR),
        }
    })
    .await
    .map_err(|_| STORAGE_ERROR)?
}

#[tauri::command]
pub(crate) async fn save_theme<R: Runtime>(
    webview: Webview<R>,
    theme: Theme,
) -> Result<(), &'static str> {
    super::trusted(&webview)?;
    let app = webview.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let directory = app.path().app_config_dir().map_err(|_| STORAGE_ERROR)?;
        let contents = serde_json::to_vec(&theme).map_err(|_| STORAGE_ERROR)?;
        let _guard = THEME_WRITE.lock().map_err(|_| STORAGE_ERROR)?;
        fs::create_dir_all(&directory).map_err(|_| STORAGE_ERROR)?;
        let temporary = directory.join(format!("theme.{}.tmp", process::id()));
        // Replace the previous selection only after the new value is written.
        fs::write(&temporary, contents)
            .and_then(|_| fs::rename(&temporary, directory.join(THEME_FILE)))
            .inspect_err(|_| {
                let _ = fs::remove_file(&temporary);
            })
            .map_err(|_| STORAGE_ERROR)
    })
    .await
    .map_err(|_| STORAGE_ERROR)?
}
