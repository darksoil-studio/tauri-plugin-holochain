use hc_seed_bundle::SharedLockedArray;
use holochain_client::InstalledAppId;
use holochain_runtime::{vec_to_locked, HolochainRuntime, HolochainRuntimeConfig, NetworkConfig};
use holochain_types::{
    app::{AppBundle, RoleSettingsMap},
    prelude::tokio_helper,
};
use std::{collections::HashMap, path::PathBuf, time::Duration};
use tauri::{http::response, plugin::TauriPlugin, AppHandle, Emitter, Manager, RunEvent, Runtime};

use crate::{
    create_hc_live_file, delete_hc_live_file,
    http_server::{pong_iframe, read_asset},
    launch_holochain_runtime, HolochainExt, HolochainPlugin, HolochainPluginConfig,
};

pub struct Builder {
    mdns_discovery: bool,
    passphrase: SharedLockedArray,
    network_config: NetworkConfig,
    admin_port: Option<u16>,
    data_dir: PathBuf,
    managed_happs: HashMap<InstalledAppId, (AppBundle, Option<RoleSettingsMap>)>,
    licensed: bool,
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            mdns_discovery: true,
            passphrase: vec_to_locked(vec![]),
            network_config: default_network_config(),
            admin_port: None,
            data_dir: default_holochain_dir(),
            managed_happs: HashMap::new(),
            licensed: false,
        }
    }
}

impl Builder {
    pub fn disable_mdns_discovery(mut self) -> Self {
        self.mdns_discovery = false;
        self
    }

    pub fn passphrase(mut self, passphrase: SharedLockedArray) -> Self {
        self.passphrase = passphrase;
        self
    }

    pub fn network_config(mut self, network_config: NetworkConfig) -> Self {
        self.network_config = network_config;
        self
    }

    pub fn data_dir(mut self, data_dir: PathBuf) -> Self {
        self.data_dir = data_dir;
        self
    }

    pub fn admin_port(mut self, admin_port: Option<u16>) -> Self {
        self.admin_port = admin_port;
        self
    }

    pub fn licensed(mut self) -> Self {
        self.licensed = true;
        self
    }

    pub fn install_or_update_app(
        mut self,
        app_id: InstalledAppId,
        app_bundle: AppBundle,
        roles_settings: Option<RoleSettingsMap>,
    ) -> Self {
        self.managed_happs
            .insert(app_id, (app_bundle, roles_settings));

        self
    }

    pub fn build<R: Runtime>(self) -> TauriPlugin<R> {
        tauri::plugin::Builder::new("holochain")
            .invoke_handler(tauri::generate_handler![
                crate::commands::sign_zome_call::sign_zome_call,
                crate::commands::open_app::open_app,
                crate::commands::install::install_web_app,
                crate::commands::install::uninstall_web_app,
                crate::commands::install::list_apps,
            ])
            .register_uri_scheme_protocol("happ", |context, request| {
                log::info!("Received request {}", request.uri().to_string());
                if request.uri().to_string().starts_with("happ://ping") {
                    return response::Builder::new()
                        .status(tauri::http::StatusCode::ACCEPTED)
                        .header("Content-Type", "text/html;charset=utf-8")
                        .body(pong_iframe().as_bytes().to_vec())
                        .expect("Failed to build body of accepted response");
                }
                // prepare our response
                tauri::async_runtime::block_on(async move {
                    // let mutex = app_handle.state::<Mutex<AdminWebsocket>>();
                    // let mut admin_ws = mutex.lock().await;

                    let uri_without_protocol = request
                        .uri()
                        .to_string()
                        .split("://")
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                        .get(1)
                        .expect("Malformed request: not enough items")
                        .clone();
                    let uri_without_querystring: String = uri_without_protocol
                        .split("?")
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                        .get(0)
                        .expect("Malformed request: not enough items 2")
                        .clone();
                    let uri_components: Vec<String> = uri_without_querystring
                        .split("/")
                        .map(|s| s.to_string())
                        .collect();
                    let lowercase_app_id = uri_components
                        .get(0)
                        .expect("Malformed request: not enough items 3");
                    let mut asset_file = PathBuf::new();
                    for i in 1..uri_components.len() {
                        asset_file = asset_file.join(uri_components[i].clone());
                    }

                    let Ok(holochain_plugin) = context.app_handle().holochain() else {
                        return response::Builder::new()
                            .status(tauri::http::StatusCode::INTERNAL_SERVER_ERROR)
                            .body(
                                format!("Called http UI before initializing holochain")
                                    .as_bytes()
                                    .to_vec(),
                            )
                            .expect("Failed to build asset with not internal server error");
                    };

                    let r = match read_asset(
                        &holochain_plugin.holochain_runtime.filesystem,
                        lowercase_app_id,
                        asset_file
                            .as_os_str()
                            .to_str()
                            .expect("Malformed request: not enough items 4")
                            .to_string(),
                    )
                    .await
                    {
                        Ok(Some((asset, mime_type))) => {
                            log::info!("Got asset for app with id: {}", lowercase_app_id);
                            let mut response =
                                response::Builder::new().status(tauri::http::StatusCode::ACCEPTED);
                            if let Some(mime_type) = mime_type {
                                response = response
                                    .header("Content-Type", format!("{};charset=utf-8", mime_type))
                            } else {
                                response = response.header("Content-Type", "charset=utf-8")
                            }

                            return response
                                .body(asset)
                                .expect("Failed to build response with asset");
                        }
                        Ok(None) => response::Builder::new()
                            .status(tauri::http::StatusCode::NOT_FOUND)
                            .body(vec![])
                            .expect("Failed to build asset with not found"),
                        Err(e) => response::Builder::new()
                            .status(500)
                            .body(format!("{:?}", e).into())
                            .expect("Failed to build body of error response"),
                    };
                    r
                })
            })
            .on_event(|app, event| match event {
                RunEvent::Exit => {
                    if tauri::is_dev() {
                        if let Ok(h) = app.holochain() {
                            if let Err(err) = delete_hc_live_file(h.holochain_runtime.admin_port) {
                                log::error!("Failed to delete hc live file: {err:?}");
                            }
                        }
                    }
                }
                RunEvent::ExitRequested { code, api, .. } => {
                    api.prevent_exit();

                    if let Err(err) = shutdown_runtime(app) {
                        log::error!("Error shutting down holochain runtime: {err:?}.");

                        std::process::exit(1);
                    } else {
                        std::process::exit(code.unwrap_or(0));
                    }
                }
                _ => {}
            })
            .setup(move |app, _api| {
                let handle = app.clone();
                let config = HolochainRuntimeConfig {
                    holochain_dir: self.data_dir,
                    network_config: self.network_config,
                    admin_port: self.admin_port.clone(),
                    mdns_discovery: self.mdns_discovery,
                };
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = launch_and_setup_holochain(
                        handle.clone(),
                        self.passphrase,
                        config,
                        self.licensed,
                        self.managed_happs,
                    )
                    .await
                    {
                        log::error!("Failed to launch holochain: {err:?}");
                        if let Err(err) = handle.emit("holochain://setup-failed", ()) {
                            log::error!(
                                "Failed to emit \"holochain://setup-failed\" event: {err:?}"
                            );
                        }
                    }
                });

                Ok(())
            })
            .build()
    }
}

fn default_network_config() -> NetworkConfig {
    let mut network_config = NetworkConfig::default();

    // Only use mDNS discovery on tauri dev
    if tauri::is_dev() {
        network_config.bootstrap_url = url2::Url2::parse("http://bad.bad");
    }

    // Don't hold any slice of the DHT in mobile
    if cfg!(mobile) {
        network_config.target_arc_factor = 0;
    }

    network_config
}

fn default_holochain_dir() -> PathBuf {
    if tauri::is_dev() {
        let tmp_dir =
            tempdir::TempDir::new("holochain").expect("Could not create temporary directory");

        // Convert `tmp_dir` into a `Path`, destroying the `TempDir`
        // without deleting the directory.
        let tmp_path = tmp_dir.into_path();
        tmp_path
    } else {
        app_dirs2::app_root(
            app_dirs2::AppDataType::UserData,
            &app_dirs2::AppInfo {
                name: "holochain",
                author: std::env!("CARGO_PKG_AUTHORS"),
            },
        )
        .expect("Could not get app root")
        .join("holochain")
    }
}

async fn launch_and_setup_holochain<R: Runtime>(
    app_handle: AppHandle<R>,
    passphrase: SharedLockedArray,
    config: HolochainPluginConfig,
    licensed: bool,
    managed_happs: HashMap<InstalledAppId, (AppBundle, Option<RoleSettingsMap>)>,
) -> crate::Result<HolochainRuntime> {
    let holochain_runtime = launch_holochain_runtime(passphrase, config).await?;

    #[cfg(desktop)]
    if tauri::is_dev() {
        create_hc_live_file(holochain_runtime.admin_port)?;
    }
    let h = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .unwrap_or_else(|e| log::error!("Could not handle termination signal: {:?}", e));

        #[cfg(desktop)]
        if tauri::is_dev() {
            if let Err(err) = delete_hc_live_file(holochain_runtime.admin_port) {
                log::error!("Failed to delete hc live file: {err:?}");
            }
        }
        if let Err(err) = shutdown_runtime(&h) {
            log::error!("Failed to shutdown holochain runtime: {err:?}");
        }
        std::process::exit(0);
    });

    let p = HolochainPlugin::<R> {
        app_handle: app_handle.clone(),
        holochain_runtime: holochain_runtime.clone(),
        licensed,
    };

    for (app_id, (app_bundle, roles_settings)) in managed_happs {
        let versioned_app = holochain_runtime.versioned_app(app_id);
        versioned_app
            .install_or_update(app_bundle, roles_settings)
            .await?;
    }

    // manage state so it is accessible by the commands
    app_handle.manage(p);
    app_handle.emit("holochain://setup-completed", ())?;

    Ok(holochain_runtime)
}

fn shutdown_runtime<R: Runtime>(app: &AppHandle<R>) -> crate::Result<()> {
    let result: std::result::Result<crate::Result<()>, tokio::time::error::Elapsed> =
        tokio_helper::block_on(
            async move {
                let holochain = app
                    .holochain()
                    .map_err(|_err| crate::Error::HolochainNotInitializedError)?;

                holochain.holochain_runtime.shutdown().await?;

                Ok(())
            },
            Duration::from_secs(3),
        );
    result.map_err(|err| crate::Error::ShutdownError(format!("{err:?}")))??;
    Ok(())
}
