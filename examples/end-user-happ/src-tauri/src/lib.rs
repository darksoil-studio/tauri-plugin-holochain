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
                .build(),
        )
        .plugin(
            tauri_plugin_holochain::Builder::default()
                // .install_or_update_happ(APP_ID.into(), example_happ())
                .build()
        )
        .setup(|app| {
            let h = app.handle().clone();
            app.listen("holochain://setup-completed", move |_e| {
                let h = h.clone();
                tauri::async_runtime::spawn(async move {
                   setup(h).await.expect("Failed to setup app"); 
                });
            });
            WebviewWindowBuilder::new(app.handle(), "main", WebviewUrl::App(PathBuf::from("")))
                .enable_app_interface(APP_ID.into())
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Very simple setup for now:
// - On app start, check whether the app is already installed:
//   - If it's not installed, install it
//   - If it's installed, check if it's necessary to update the coordinators for our hApp,
//     and do so if it is
//
// You can modify this function to suit your needs if they become more complex
async fn setup(handle: AppHandle) -> anyhow::Result<()> {
    let admin_ws = handle.holochain()?.admin_websocket().await?;

    let installed_apps = admin_ws
        .list_apps(None)
        .await
        .map_err(|err| tauri_plugin_holochain::Error::ConductorApiError(err))?;

    // DeepKey comes preinstalled as the first app
    if installed_apps.iter().find(|app| app.installed_app_id.as_str().eq(APP_ID)).is_none() {
        handle
            .holochain()?
            .install_app(
                String::from(APP_ID),
                example_happ(),
                None,
                None,
                None,
            )
            .await?;

        Ok(())
    } else {
        handle.holochain()?.update_app_if_necessary(
            String::from(APP_ID),
            example_happ()
        ).await?;

        Ok(())
    }
}
