use std::path::PathBuf;
use tauri_plugin_holochain::*;
use tauri::{AppHandle, Listener, WebviewUrl, WebviewWindowBuilder};

const APP_ID: &'static str = "example";

pub fn example_happ() -> AppBundle {
    let bytes = include_bytes!("../../workdir/forum.happ");
    AppBundle::decode(bytes).expect("Failed to decode example happ")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .level_for("tracing::span", log::LevelFilter::Error)
                .level_for("holochain_sqlite", log::LevelFilter::Error)
                .level_for("iroh", log::LevelFilter::Warn)
                .build(),
        )
        .plugin(
            tauri_plugin_holochain::Builder::default()
                .licensed()
                .install_or_update_app(APP_ID.into(), example_happ(), None)
                .build()
        )
        .setup(|app| {
            WebviewWindowBuilder::new(app.handle(), "main", WebviewUrl::App(PathBuf::new()))
                .enable_app_interface(APP_ID.into())
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

