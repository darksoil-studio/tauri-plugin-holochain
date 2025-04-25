use holochain_types::prelude::AppBundle;
use std::path::PathBuf;
use tauri_plugin_holochain::{HolochainExt, HolochainPluginConfig, NetworkConfig, vec_to_locked};
use tauri::{AppHandle, Listener};

const APP_ID: &'static str = "example";

pub fn example_happ() -> AppBundle {
    let bytes = include_bytes!("../../workdir/forum.happ");
    AppBundle::decode(bytes).expect("Failed to decode example happ")
}

fn network_config() -> NetworkConfig {
    let mut network_config = NetworkConfig::default();

    // Don't use the bootstrap service on tauri dev mode
    if tauri::is_dev() {
        network_config.bootstrap_url = url2::Url2::parse("http://0.0.0.0:8888");
    }

    // Don't hold any slice of the DHT in mobile
    if cfg!(mobile) {
        network_config.target_arc_factor = 0;
    }

    network_config
}

fn holochain_dir() -> PathBuf {
    if tauri::is_dev() {
        // if cfg!(target_os = "android") {
        //     app_dirs2::app_root(
        //         app_dirs2::AppDataType::UserData,
        //         &app_dirs2::AppInfo {
        //             name: "example-forum",
        //             author: std::env!("CARGO_PKG_AUTHORS"),
        //         },
        //     )
        //     .expect("Could not get app root")
        //     .join("holochain")
        // } else {
            let tmp_dir =
                tempdir::TempDir::new("forum").expect("Could not create temporary directory");

            // Convert `tmp_dir` into a `Path`, destroying the `TempDir`
            // without deleting the directory.
            let tmp_path = tmp_dir.into_path();
            tmp_path
        // }
    } else {
        app_dirs2::app_root(
            app_dirs2::AppDataType::UserData,
            &app_dirs2::AppInfo {
                name: "example-forum",
                author: std::env!("CARGO_PKG_AUTHORS"),
            },
        )
        .expect("Could not get app root")
        .join("holochain")
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Warn)
                .build(),
        )
        .plugin(tauri_plugin_holochain::async_init(
            vec_to_locked(vec![]),
            HolochainPluginConfig::new(holochain_dir(), network_config())
        ))
        .setup(|app| {
            let handle = app.handle().clone();
            app.handle().listen("holochain://setup-completed", move |_event| {
                let handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    setup(handle.clone()).await.expect("Failed to setup");

                    handle
                        .holochain()
                        .expect("Failed to get holochain")
                        .main_window_builder(String::from("main"), false, Some(APP_ID.into()), None).await
                        .expect("Failed to build window")
                        .build()
                        .expect("Failed to open main window");
                });
            });

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
