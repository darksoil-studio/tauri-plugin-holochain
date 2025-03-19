const COMMANDS: &[&str] = &[
    "sign_zome_call",
    "install_web_app",
    "uninstall_web_app",
    "open_app",
    "list_apps",
    "is_holochain_ready",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        // .android_path("android")
        // .ios_path("ios")
        .build();
}
