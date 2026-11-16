fn main() {
    tauri_plugin::Builder::new(&[
        "status",
        "open_provider",
        "keystore_probe",
        "lookup_stops",
        "vault_status",
        "set_appearance",
    ])
    .android_path("android")
    .build();
}
