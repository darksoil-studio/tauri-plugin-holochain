use std::collections::{HashMap, HashSet};

use hc_seed_bundle::SharedLockedArray;
use holochain_client::{AdminWebsocket, AppInfo, AppWebsocket};
use tauri::{async_runtime::RwLock, AppHandle, Emitter, Manager, Runtime, WebviewWindowBuilder};

pub use holochain_types::{prelude::*, web_app::WebAppBundle, websocket::AllowedOrigins};

mod plugin_builder;
pub use plugin_builder::*;
mod commands;
mod error;
mod hc_live_file;
mod http_server;
mod window_builder;
pub use window_builder::*;

pub use error::{Error, Result};
use hc_live_file::*;
pub use holochain_runtime::*;

/// Access to the holochain APIs.
pub struct HolochainPlugin<R: Runtime> {
    pub app_handle: AppHandle<R>,
    pub holochain_runtime: HolochainRuntime,
    pub licensed: bool,
}

fn happ_origin(app_id: &String) -> String {
    if cfg!(any(target_os = "windows", target_os = "android")) {
        format!("http://happ.{app_id}")
    } else {
        format!("happ://{app_id}")
    }
}

fn main_window_origin() -> String {
    if cfg!(any(target_os = "windows", target_os = "android")) {
        "http://tauri.localhost".into()
    } else {
        "tauri://localhost".into()
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

        let url_origin = happ_origin(&app_id.clone().into());

        let url_path = url_path.unwrap_or_default();

        let webview_url = tauri::WebviewUrl::CustomProtocol(url::Url::parse(
            format!("{url_origin}/{url_path}").as_str(),
        )?);
        let window_builder =
            WebviewWindowBuilder::new(&self.app_handle, app_id.clone(), webview_url)
                .enable_app_interface(app_id);

        Ok(window_builder)
    }

    /// Builds an `AdminWebsocket` ready to use
    pub async fn admin_websocket(&self) -> crate::Result<AdminWebsocket> {
        let admin_ws = self.holochain_runtime.admin_websocket().await?;
        Ok(admin_ws)
    }

    fn get_allowed_origins(&self, app_id: &InstalledAppId, main_window: bool) -> AllowedOrigins {
        // Allow any when the app is build in debug mode to allow normal tauri development pointing to http://localhost:1420
        let allowed_origins = if tauri::is_dev() {
            AllowedOrigins::Any
        } else {
            let mut origins: HashSet<String> = HashSet::new();
            origins.insert(self.get_app_origin(app_id, main_window));

            AllowedOrigins::Origins(origins)
        };
        allowed_origins
    }

    fn get_app_origin(&self, app_id: &InstalledAppId, main_window: bool) -> String {
        if main_window {
            main_window_origin()
        } else {
            happ_origin(&app_id)
        }
    }

    /// Builds an `AppWebsocket` for the given app ready to use
    ///
    /// * `app_id` - the app to build the `AppWebsocket` for
    pub async fn app_websocket(&self, app_id: InstalledAppId) -> crate::Result<AppWebsocket> {
        let app_origin = self.get_app_origin(&app_id, false);

        let mut origins: HashSet<String> = HashSet::new();
        origins.insert(app_origin);

        let app_ws = self
            .holochain_runtime
            .app_websocket(app_id, AllowedOrigins::Origins(origins))
            .await?;
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
        let app_info = self
            .holochain_runtime
            .install_web_app(
                app_id.clone(),
                web_app_bundle,
                roles_settings,
                agent,
                network_seed,
            )
            .await?;

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
        let app_info = self
            .holochain_runtime
            .install_app(
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
        self.holochain_runtime
            .update_web_app(app_id.clone(), web_app_bundle)
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
        self.holochain_runtime
            .update_app(app_id.clone(), app_bundle)
            .await?;

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
        self.holochain_runtime
            .update_app_if_necessary(app_id, current_app_bundle)
            .await?;

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
        self.holochain_runtime
            .update_web_app_if_necessary(app_id, current_web_app_bundle)
            .await?;

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

static RUNNING_HOLOCHAIN_RUNTIME: RwLock<Option<HolochainRuntime>> = RwLock::const_new(None);

pub async fn launch_holochain_runtime(
    passphrase: SharedLockedArray,
    config: HolochainPluginConfig,
) -> crate::Result<HolochainRuntime> {
    let mut lock = RUNNING_HOLOCHAIN_RUNTIME.write().await;

    if let Some(runtime) = lock.to_owned() {
        return Ok(runtime);
    }

    let crypto_provider = rustls::crypto::aws_lc_rs::default_provider().install_default();
    if crypto_provider.is_err() {
        log::error!(
            "could not set crypto provider for tls: {:?}.",
            crypto_provider
        );
    }

    let holochain_runtime = HolochainRuntime::launch(passphrase, config).await?;

    *lock = Some(holochain_runtime.clone());

    Ok(holochain_runtime)
}
