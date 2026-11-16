fn main() {
    tauri_plugin::Builder::new(&["status", "open_provider", "keystore_probe"])
        .android_path("android")
        .build();
}
