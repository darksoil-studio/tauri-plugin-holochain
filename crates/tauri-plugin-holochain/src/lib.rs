use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use hc_seed_bundle::dependencies::sodoken::BufRead;
use http_server::{pong_iframe, read_asset};
use tauri::{
    http::response,
    ipc::CapabilityBuilder,
    plugin::{Builder, TauriPlugin},
    AppHandle, Emitter, Manager, RunEvent, Runtime, WebviewUrl, WebviewWindowBuilder,
};

use holochain_types::prelude::*;
use holochain_client::{AdminWebsocket, AgentPubKey, AppInfo, AppWebsocket, InstalledAppId};
use holochain_types::{web_app::WebAppBundle, websocket::AllowedOrigins};

mod commands;
mod error;
mod hc_live_file;
mod http_server;

pub use error::{Error, Result};
use hc_live_file::*;
pub use holochain_runtime::*;

pub const ZOME_CALL_SIGNER_INITIALIZATION_SCRIPT: &'static str = include_str!("../zome-call-signer.js");

/// Access to the holochain APIs.
pub struct HolochainPlugin<R: Runtime> {
    pub app_handle: AppHandle<R>,
    pub holochain_runtime: HolochainRuntime,
}

fn happ_origin(app_id: &String) -> String {
    if cfg!(any(target_os = "windows", target_os = "android")) {
        format!("http://happ.{app_id}")
    } else {
        format!("happ://{app_id}")
    }
}

impl<R: Runtime> HolochainPlugin<R> {
    /// Build a window that opens the UI for the given holochain web-app.
    ///
    /// * `app_id` - the app whose UI will be open. The must have been installed before with `Self::install_web_app()`.
    /// * `url_path` - [url path](https://developer.mozilla.org/en-US/docs/Web/API/URL/pathname) for the window that will be opened.
    pub async fn web_happ_window_builder(
        &self,
        app_id: InstalledAppId,
        url_path: Option<String>,
    ) -> crate::Result<WebviewWindowBuilder<R, AppHandle<R>>> {
        let app_id: String = app_id.into();

        let allowed_origins= self.get_allowed_origins(&app_id, false);
        let app_websocket_auth = self.holochain_runtime.get_app_websocket_auth(&app_id, allowed_origins).await?;

        let token_vector: Vec<String> = app_websocket_auth
            .token
            .iter()
            .map(|n| n.to_string())
            .collect();
        let token = token_vector.join(",");
        let url_origin = happ_origin(&app_id.clone().into());

        let url_path = url_path.unwrap_or_default();

        let webview_url = tauri::WebviewUrl::CustomProtocol(url::Url::parse(
            format!("{url_origin}/{url_path}").as_str(),
        )?);
        let window_builder =
            WebviewWindowBuilder::new(&self.app_handle, app_id.clone(), webview_url)
                .initialization_script(
                    format!(
                        r#"
            if (!window.__HC_LAUNCHER_ENV__) window.__HC_LAUNCHER_ENV__ = {{}};
            window.__HC_LAUNCHER_ENV__.APP_INTERFACE_PORT = {};
            window.__HC_LAUNCHER_ENV__.APP_INTERFACE_TOKEN = [{}];
            window.__HC_LAUNCHER_ENV__.INSTALLED_APP_ID = "{}";
        "#,
                        app_websocket_auth.app_websocket_port, token, app_id
                    )
                    .as_str(),
                )
                .initialization_script(ZOME_CALL_SIGNER_INITIALIZATION_SCRIPT);

        let mut capability_builder =
            CapabilityBuilder::new("sign-zome-call").permission("holochain:allow-sign-zome-call");

        capability_builder = capability_builder.window(app_id);

        self.app_handle.add_capability(capability_builder)?;

        Ok(window_builder)
    }

    /// Build a window that opens the main UI for your Tauri app.
    /// This is equivalent to creating a window with `WebviewUrl::App(PathBuf::from("index.html"))`.
    ///
    /// * `label` - the identifier of the window.
    /// * `enable_admin_websocket` - whether the window should have direct access to the `AdminWebsocket`'s API.
    /// * `enabled_app` - an optional `app_id` for the app whose `AppWebsocket` should be enabled in the window.
    /// * `url_path` - [url path](https://developer.mozilla.org/en-US/docs/Web/API/URL/pathname) for the window that will be opened.
    pub async fn main_window_builder(
        &self,
        label: String,
        enable_admin_websocket: bool,
        enabled_app: Option<InstalledAppId>,
        url_path: Option<String>,
    ) -> crate::Result<WebviewWindowBuilder<R, AppHandle<R>>> {
        let url_path = url_path.unwrap_or_default();

        let mut window_builder = WebviewWindowBuilder::new(
            &self.app_handle,
            label.clone(),
            // Pointing to index.html
            WebviewUrl::App(format!("{url_path}").into()),
        );

        if enable_admin_websocket {
            window_builder = window_builder.initialization_script(
                format!(
                    r#"
            if (!window.__HC_LAUNCHER_ENV__) window.__HC_LAUNCHER_ENV__ = {{}};
            window.__HC_LAUNCHER_ENV__.ADMIN_INTERFACE_PORT = {};
                        
                    "#,
                    self.holochain_runtime.admin_port
                )
                .as_str(),
            )
        }

        if let Some(enabled_app) = enabled_app {
            let allowed_origins= self.get_allowed_origins(&enabled_app, true);
            let app_websocket_auth = self
                .holochain_runtime
                .get_app_websocket_auth(&enabled_app, allowed_origins).await?;

            let token_vector: Vec<String> = app_websocket_auth
                .token
                .iter()
                .map(|n| n.to_string())
                .collect();
            let token = token_vector.join(",");
            window_builder = window_builder
                .initialization_script(
                    format!(
                        r#"
            if (!window.__HC_LAUNCHER_ENV__) window.__HC_LAUNCHER_ENV__ = {{}};
            window.__HC_LAUNCHER_ENV__.APP_INTERFACE_PORT = {};
            window.__HC_LAUNCHER_ENV__.APP_INTERFACE_TOKEN = [{}];
            window.__HC_LAUNCHER_ENV__.INSTALLED_APP_ID = "{}";
        "#,
                        app_websocket_auth.app_websocket_port, token, enabled_app
                    )
                    .as_str(),
                )
                .initialization_script(ZOME_CALL_SIGNER_INITIALIZATION_SCRIPT);

            let mut capability_builder = CapabilityBuilder::new("sign-zome-call")
                .permission("holochain:allow-sign-zome-call");

            capability_builder = capability_builder.window(label);

            self.app_handle.add_capability(capability_builder)?;
        }

        Ok(window_builder)
    }

    /// Builds an `AdminWebsocket` ready to use
    pub async fn admin_websocket(&self) -> crate::Result<AdminWebsocket> {
        let admin_ws = self.holochain_runtime.admin_websocket().await?;
        Ok(admin_ws)
    }

    pub fn get_allowed_origins(&self,
        app_id: &InstalledAppId,
        main_window: bool
    ) -> AllowedOrigins {
        // Allow any when the app is build in debug mode to allow normal tauri development pointing to http://localhost:1420
        let allowed_origins = if tauri::is_dev() {
            AllowedOrigins::Any
        } else {
            let mut origins: HashSet<String> = HashSet::new();
            origins.insert(happ_origin(&app_id));

            if main_window {
                origins.insert("http://tauri.localhost".into());
                origins.insert("tauri://localhost".into());
            }

            AllowedOrigins::Origins(origins)
        };
        allowed_origins
    }
    
    /// Builds an `AppWebsocket` for the given app ready to use
    ///
    /// * `app_id` - the app to build the `AppWebsocket` for
    pub async fn app_websocket(&self, app_id: InstalledAppId) -> crate::Result<AppWebsocket> {
        let allowed_origins= self.get_allowed_origins(&app_id, false);
        let app_ws = self.holochain_runtime.app_websocket(app_id, allowed_origins).await?;
        Ok(app_ws)
    }

    /// Install the given `WebAppBundle` in the holochain runtime
    /// It installs the hApp in the holochain conductor, and extracts the UI for it to be opened using `Self::web_happ_window_builder()`
    ///
    /// * `app_id` - the app id to give to the installed app
    /// * `web_app_bundle` - the web-app bundle to install
    /// * `membrane_proofs` - the input membrane proofs for the app
    /// * `agent` - the agent to install the app for
    /// * `network_seed` - the network seed for the app
    pub async fn install_web_app(
        &self,
        app_id: InstalledAppId,
        web_app_bundle: WebAppBundle,
        roles_settings: Option<HashMap<String, RoleSettings>>,
        agent: Option<AgentPubKey>,
        network_seed: Option<NetworkSeed>,
    ) -> crate::Result<AppInfo> {
        let app_info= self
            .holochain_runtime
            .install_web_app(app_id.clone(), web_app_bundle,roles_settings, agent, network_seed).await?;

        self.app_handle.emit("holochain://app-installed", app_id)?;

        Ok(app_info)
    }

    /// Install the given `AppBundle` in the holochain conductor
    ///
    /// * `app_id` - the app id to give to the installed app
    /// * `app_bundle` - the web-app bundle to install
    /// * `membrane_proofs` - the input membrane proofs for the app
    /// * `agent` - the agent to install the app for
    /// * `network_seed` - the network seed for the app
    pub async fn install_app(
        &self,
        app_id: InstalledAppId,
        app_bundle: AppBundle,
        roles_settings: Option<HashMap<String, RoleSettings>>,
        agent: Option<AgentPubKey>,
        network_seed: Option<NetworkSeed>,
    ) -> crate::Result<AppInfo> {
        let app_info = self.holochain_runtime.install_app(
            app_id.clone(),
            app_bundle,
            roles_settings,
            agent,
            network_seed,
        )
        .await?;

        self.app_handle.emit("holochain://app-installed", app_id)?;
        Ok(app_info)
    }

    /// Updates the coordinator zomes and UI for the given app with an updated `WebAppBundle`
    ///
    /// * `app_id` - the app to update
    /// * `web_app_bundle` - the new version of the web-hApp bundle
    pub async fn update_web_app(
        &self,
        app_id: InstalledAppId,
        web_app_bundle: WebAppBundle,
    ) -> crate::Result<()> {
        self.holochain_runtime.update_web_app(
            app_id.clone(),
            web_app_bundle
        )
        .await?;

        self.app_handle.emit("holochain://app-updated", app_id)?;

        Ok(())
    }

    /// Updates the coordinator zomes for the given app with an updated `AppBundle`
    ///
    /// * `app_id` - the app to update
    /// * `app_bundle` - the new version of the hApp bundle
    pub async fn update_app(
        &self,
        app_id: InstalledAppId,
        app_bundle: AppBundle,
    ) -> crate::Result<()> {
        self.holochain_runtime.update_app(app_id.clone(), app_bundle).await?;

        self.app_handle.emit("holochain://app-updated", app_id)?;
        Ok(())
    }

    /// Checks whether it is necessary to update the hApp, and if so,
    /// updates the coordinator zomes for the given app with an updated `AppBundle`
    ///
    /// To do the check it compares the hash of the `AppBundle` that was installed for the given `app_id`
    /// with the hash of the `current_app_bundle`, and proceeds to update the coordinator zomes for the app if they are different
    ///
    /// * `app_id` - the app to update
    /// * `current_app_bundle` - the new version of the hApp bundle
    pub async fn update_app_if_necessary(
        &self,
        app_id: InstalledAppId,
        current_app_bundle: AppBundle,
    ) -> crate::Result<()> {
        self.holochain_runtime.update_app_if_necessary(app_id, current_app_bundle).await?;

        Ok(())
    }

    /// Checks whether it is necessary to update the web-hApp, and if so,
    /// updates the coordinator zomes and the UI for the given app with an updated `WebAppBundle`
    ///
    /// To do the check it compares the hash of the `WebAppBundle` that was installed for the given `app_id`
    /// with the hash of the `current_web_app_bundle`, and proceeds to update the coordinator zomes and the UI for the app if they are different
    ///
    /// * `app_id` - the app to update
    /// * `current_web_app_bundle` - the new version of the hApp bundle
    pub async fn update_web_app_if_necessary(
        &self,
        app_id: InstalledAppId,
        current_web_app_bundle: WebAppBundle,
    ) -> crate::Result<()> {
        self.holochain_runtime.update_web_app_if_necessary(app_id, current_web_app_bundle).await?;

        Ok(())
    }
}

// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the holochain APIs.
pub trait HolochainExt<R: Runtime> {
    fn holochain(&self) -> crate::Result<&HolochainPlugin<R>>;
}

impl<R: Runtime, T: Manager<R>> crate::HolochainExt<R> for T {
    /// Access the holochain runtime for this Tauri app
    fn holochain(&self) -> crate::Result<&HolochainPlugin<R>> {
        let s = self
            .try_state::<HolochainPlugin<R>>()
            .ok_or(crate::Error::HolochainNotInitializedError)?;

        Ok(s.inner())
    }
}

pub type HolochainPluginConfig = HolochainRuntimeConfig;

fn plugin_builder<R: Runtime>() -> Builder<R> {
    Builder::new("holochain")
        .invoke_handler(tauri::generate_handler![
            commands::sign_zome_call::sign_zome_call,
            commands::open_app::open_app,
            commands::get_runtime_info::is_holochain_ready
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
            _ => {}
        })
}

/// Initializes the plugin, waiting for holochain to launch before finishing the app's setup.
pub fn init<R: Runtime>(passphrase: BufRead, config: HolochainPluginConfig) -> TauriPlugin<R> {
    plugin_builder()
        .setup(|app, _api| {
            let handle = app.clone();
            let result = tauri::async_runtime::block_on(async move {
                launch_and_setup_holochain(handle, passphrase, config).await
            });

            Ok(result?)
        })
        .build()
}

/// Initializes the plugin without waiting for holochain to launch to continue the setup of the app
/// If you use this version of init, you should listen to the `holochain://setup-completed` event in your `setup()` hook
pub fn async_init<R: Runtime>(
    passphrase: BufRead,
    config: HolochainPluginConfig,
) -> TauriPlugin<R> {
    plugin_builder()
        .setup(|app, _api| {
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) =
                    launch_and_setup_holochain(handle.clone(), passphrase, config).await
                {
                    log::error!("Failed to launch holochain: {err:?}");
                    if let Err(err) = handle.emit("holochain://setup-failed", ()) {
                        log::error!("Failed to emit \"holochain://setup-failed\" event: {err:?}");
                    }
                }
            });

            Ok(())
        })
        .build()
}

async fn launch_and_setup_holochain<R: Runtime>(
    app_handle: AppHandle<R>,
    passphrase: BufRead,
    config: HolochainPluginConfig,
) -> crate::Result<()> {
    // let http_server_port = portpicker::pick_unused_port().expect("No ports free");
    // http_server::start_http_server(app_handle.clone(), http_server_port).await?;
    // log::info!("Starting http server at port {http_server_port:?}");

    let holochain_runtime = HolochainRuntime::launch(passphrase, config).await?;

    #[cfg(desktop)]
    if tauri::is_dev() {
        create_hc_live_file(holochain_runtime.admin_port)?;

        ctrlc::set_handler(move || {
            if let Err(err) = delete_hc_live_file(holochain_runtime.admin_port) {
                log::error!("Failed to delete hc live file: {err:?}");
            }
            std::process::exit(0);
        })?;
    }

    let p = HolochainPlugin::<R> {
        app_handle: app_handle.clone(),
        holochain_runtime,
    };

    // manage state so it is accessible by the commands
    app_handle.manage(p);

    app_handle.emit("holochain://setup-completed", ())?;

    Ok(())
}
