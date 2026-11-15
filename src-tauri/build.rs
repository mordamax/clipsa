fn main() {
    // The updater plugin (and therefore its permissions) only exists in
    // direct-download builds; App Store builds must not reference it.
    let capabilities = if std::env::var_os("CARGO_FEATURE_SELF_UPDATE").is_some() {
        "./capabilities/**/*"
    } else {
        "./capabilities/default.json"
    };
    tauri_build::try_build(tauri_build::Attributes::new().capabilities_path_pattern(capabilities))
        .expect("failed to run tauri-build");
}
