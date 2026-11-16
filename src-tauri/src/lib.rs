#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_android_platform::init())
        .setup(|app| {
            let config = &app.config().app.windows[0];
            let dev_origin = if cfg!(debug_assertions) {
                app.config().build.dev_url.as_ref().map(|u| u.origin())
            } else {
                None
            };
            tauri::WebviewWindowBuilder::from_config(app, config)?
                .on_navigation(move |url| {
                    let bundled = url.username().is_empty()
                        && url.password().is_none()
                        && url.port().is_none()
                        && matches!(
                            (url.scheme(), url.host_str()),
                            ("tauri", Some("localhost"))
                                | ("http" | "https", Some("tauri.localhost"))
                        );
                    bundled
                        || dev_origin
                            .as_ref()
                            .is_some_and(|origin| *origin == url.origin())
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("ubian-aggregator could not start");
}
