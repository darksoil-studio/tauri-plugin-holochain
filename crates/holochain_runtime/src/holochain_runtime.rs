use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use async_std::sync::Mutex;
use holochain::{
    conductor::ConductorHandle,
    prelude::{AppStatus, DisabledAppReason, NetworkSeed, ZomeCallParams},
};
use holochain_client::{
    AdminWebsocket, AgentPubKey, AppInfo, AppWebsocket, ConnectRequest, InstalledAppId,
    WebsocketConfig,
};
use holochain_conductor_api::ZomeCallParamsSigned;
use holochain_keystore::MetaLairClient;
use holochain_types::{
    app::{AppBundle, RoleSettings},
    web_app::WebAppBundle,
    websocket::AllowedOrigins,
};
use lair_keystore::dependencies::futures::future::join_all;
use lair_keystore_api::{in_proc_keystore::InProcKeystore, types::SharedLockedArray};

use crate::{
    filesystem::{AppBundleStore, BundleStore, FileSystem},
    happs::{
        install::install_app,
        update::{update_app, UpdateHappError},
    },
    lair_signer::LairAgentSignerWithProvenance,
    launch::launch_holochain_runtime,
    sign_zome_call_with_client, HolochainRuntimeConfig,
};

const NETWORK_SHUTDOWN_DISABLED_APP_REASON: &'static str = "holochain_runtime/network_shutdown";

#[derive(Clone)]
pub struct AppWebsocketAuth {
    pub app_id: String,
    pub app_websocket_port: u16,
    pub allowed_origins: AllowedOrigins,
    pub token: Vec<u8>,
}

#[derive(Clone)]
pub struct HolochainRuntime {
    pub filesystem: FileSystem,
    pub apps_websockets_auths: Arc<Mutex<Vec<AppWebsocketAuth>>>,
    pub admin_port: u16,
    pub conductor_handle: ConductorHandle,

    pub(crate) lair_client: MetaLairClient,
    pub(crate) passphrase: SharedLockedArray,
    pub(crate) in_proc_keystore: InProcKeystore,

    #[cfg(feature = "hc-auth")]
    pub(crate) hc_auth_config: Option<crate::hc_auth::HcAuthConfig>,
    #[cfg(feature = "hc-auth")]
    pub(crate) hc_auth_status: Arc<std::sync::RwLock<crate::hc_auth::HcAuthStatus>>,
    #[cfg(feature = "hc-auth")]
    pub(crate) hc_auth_agent_key: Arc<std::sync::RwLock<Option<AgentPubKey>>>,
    #[cfg(feature = "hc-auth")]
    pub(crate) hc_auth_raw_ed25519_b64url: Arc<std::sync::RwLock<Option<String>>>,
}

impl HolochainRuntime {
    pub async fn launch(
        passphrase: SharedLockedArray,
        config: HolochainRuntimeConfig,
    ) -> crate::Result<Self> {
        let runtime = launch_holochain_runtime(passphrase, config).await?;

        let admin_ws = runtime.admin_websocket().await?;

        let apps = admin_ws
            .list_apps(Some(holochain_client::AppStatusFilter::Disabled))
            .await?;

        if !apps.is_empty() {
            log::info!("Re-enabling all apps disabled in shutdown.");

            join_all(apps.into_iter().map(async |app| {
                let AppStatus::Disabled(DisabledAppReason::Error(reason)) = app.status else {
                    return ();
                };

                if reason.ne(&NETWORK_SHUTDOWN_DISABLED_APP_REASON.to_string()) {
                    return ();
                }

                if let Err(err) = runtime
                    .conductor_handle
                    .clone()
                    .enable_app(app.installed_app_id)
                    .await
                {
                    log::error!("Error re-enabling the app: {err:?}.");
                }
            }))
            .await;

            log::info!("Re-enabled all apps disabled in shutdown.");
        }

        Ok(runtime)
    }

    /// Builds an `AdminWebsocket` ready to use
    pub async fn admin_websocket(&self) -> crate::Result<AdminWebsocket> {
        let mut config = WebsocketConfig::CLIENT_DEFAULT;
        config.default_request_timeout = std::time::Duration::new(60 * 5, 0);

        let admin_ws = AdminWebsocket::connect_with_config(
            format!("localhost:{}", self.admin_port),
            Arc::new(config),
            None,
        )
        .await
        .map_err(|err| crate::Error::WebsocketConnectionError(format!("{err:?}")))?;

        Ok(admin_ws)
    }

    pub async fn get_app_websocket_auth(
        &self,
        app_id: &InstalledAppId,
        allowed_origins: AllowedOrigins,
    ) -> crate::Result<AppWebsocketAuth> {
        let mut apps_websockets_auths = self.apps_websockets_auths.lock().await;
        let existing_auth = apps_websockets_auths
            .iter()
            .find(|auth| auth.allowed_origins.eq(&allowed_origins) && auth.app_id.eq(app_id));
        if let Some(app_websocket_auth) = existing_auth {
            return Ok(app_websocket_auth.clone());
        }

        let admin_ws = self.admin_websocket().await?;

        let app_port = admin_ws
            .attach_app_interface(0, None, allowed_origins.clone(), Some(app_id.clone()))
            .await?;

        let response = admin_ws
            .issue_app_auth_token(
                holochain_conductor_api::IssueAppAuthenticationTokenPayload {
                    installed_app_id: app_id.clone(),
                    expiry_seconds: 999999999,
                    single_use: false,
                },
            )
            .await?;

        let token = response.token;

        let app_websocket_auth = AppWebsocketAuth {
            app_id: app_id.clone(),
            allowed_origins,
            app_websocket_port: app_port,
            token,
        };

        apps_websockets_auths.push(app_websocket_auth.clone());
        Ok(app_websocket_auth)
    }

    /// Builds an `AppWebsocket` for the given app ready to use
    ///
    /// * `app_id` - the app to build the `AppWebsocket` for
    pub async fn app_websocket(
        &self,
        app_id: InstalledAppId,
        allowed_origins: AllowedOrigins,
    ) -> crate::Result<AppWebsocket> {
        let app_websocket_auth = self
            .get_app_websocket_auth(&app_id, allowed_origins.clone())
            .await?;

        let config = Arc::new(WebsocketConfig::CLIENT_DEFAULT);
        let mut request = ConnectRequest::new(SocketAddr::new(
            Ipv4Addr::LOCALHOST.into(),
            app_websocket_auth.app_websocket_port,
        ));

        if let AllowedOrigins::Origins(origins) = allowed_origins {
            if let Some(origin) = origins.into_iter().collect::<Vec<String>>().first() {
                request = request.try_set_header("Origin", origin.as_str())?;
            }
        }

        let app_ws = AppWebsocket::connect_with_request_and_config(
            request,
            config,
            app_websocket_auth.token,
            Arc::new(LairAgentSignerWithProvenance::new(Arc::new(
                self.conductor_handle.keystore().lair_client().clone(),
            ))),
        )
        .await
        .map_err(|err| crate::Error::WebsocketConnectionError(format!("{err:?}")))?;

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
        self.filesystem
            .bundle_store
            .store_web_happ_bundle(app_id.clone(), &web_app_bundle)
            .await?;

        let app_bundle = web_app_bundle.happ_bundle().await?;
        let app_bundle_path = self
            .filesystem
            .bundle_store
            .happ_bundle_store()
            .app_bundle_path(&app_bundle)?;

        let admin_ws = self.admin_websocket().await?;
        let app_info = install_app(
            &admin_ws,
            app_id.clone(),
            app_bundle_path,
            roles_settings,
            agent,
            network_seed,
        )
        .await?;

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
        let admin_ws = self.admin_websocket().await?;

        self.filesystem
            .bundle_store
            .store_happ_bundle(app_id.clone(), &app_bundle)?;

        let app_bundle_path = self
            .filesystem
            .bundle_store
            .happ_bundle_store()
            .app_bundle_path(&app_bundle)?;

        let app_info = install_app(
            &admin_ws,
            app_id.clone(),
            app_bundle_path,
            roles_settings,
            agent,
            network_seed,
        )
        .await?;

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
        self.filesystem
            .bundle_store
            .store_web_happ_bundle(app_id.clone(), &web_app_bundle)
            .await?;

        let admin_ws = self
            .admin_websocket()
            .await
            .map_err(|_err| UpdateHappError::WebsocketError)?;
        update_app(
            &admin_ws,
            app_id.clone(),
            web_app_bundle.happ_bundle().await?,
        )
        .await?;

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
    ) -> std::result::Result<(), UpdateHappError> {
        let mut admin_ws = self
            .admin_websocket()
            .await
            .map_err(|_err| UpdateHappError::WebsocketError)?;
        let app_info = update_app(&mut admin_ws, app_id.clone(), app_bundle).await?;

        Ok(app_info)
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
        let hash = AppBundleStore::app_bundle_hash(&current_app_bundle)?;

        let installed_apps = self.filesystem.bundle_store.installed_apps_store.get()?;
        let Some(installed_app_info) = installed_apps.get(&app_id) else {
            return Err(UpdateHappError::AppNotFound(app_id))?;
        };

        if !installed_app_info.happ_bundle_hash.eq(&hash) {
            self.update_app(app_id, current_app_bundle).await?;
        }

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
        let hash = BundleStore::web_app_bundle_hash(&current_web_app_bundle)?;

        let installed_apps = self.filesystem.bundle_store.installed_apps_store.get()?;
        let Some(installed_app_info) = installed_apps.get(&app_id) else {
            return Err(UpdateHappError::AppNotFound(app_id))?;
        };

        if !installed_app_info.happ_bundle_hash.eq(&hash) {
            self.update_web_app(app_id, current_web_app_bundle).await?;
        }

        Ok(())
    }

    /// Sign a zome call
    ///
    /// * `zome_call_unsigned` - the unsigned zome call
    pub async fn sign_zome_call(
        &self,
        zome_call_unsigned: ZomeCallParams,
    ) -> crate::Result<ZomeCallParamsSigned> {
        let signed_zome_call = sign_zome_call_with_client(
            zome_call_unsigned,
            &self.conductor_handle.keystore().lair_client().clone(),
        )
        .await?;
        Ok(signed_zome_call)
    }

    /// Check if an app with a given app_id installed on the holochain conductor
    ///
    /// * `app_id` - the app id to check
    pub async fn is_app_installed(&self, app_id: InstalledAppId) -> crate::Result<bool> {
        let admin_ws = self.admin_websocket().await?;
        let apps = admin_ws.list_apps(None).await?;

        let matching_app = apps
            .into_iter()
            .find(|app_info| app_info.installed_app_id == app_id);

        Ok(matching_app.is_some())
    }

    /// Uninstall the app with the given `app_id` from the holochain conductor
    ///
    /// * `app_id` - the app id of the app to uninstall
    pub async fn uninstall_app(&self, app_id: InstalledAppId) -> crate::Result<()> {
        let admin_ws = self.admin_websocket().await?;
        admin_ws.uninstall_app(app_id, false).await?;

        Ok(())
    }

    /// Enable the app with the given `app_id` from the holochain conductor
    ///
    /// * `app_id` - the app id of the app to enable
    pub async fn enable_app(&self, app_id: InstalledAppId) -> crate::Result<()> {
        let admin_ws = self.admin_websocket().await?;
        admin_ws.enable_app(app_id).await?;

        Ok(())
    }

    /// Disable the app with the given `app_id` from the holochain conductor
    ///
    /// * `app_id` - the app id of the app to disable
    pub async fn disable_app(&self, app_id: InstalledAppId) -> crate::Result<()> {
        let admin_ws = self.admin_websocket().await?;
        admin_ws.disable_app(app_id).await?;

        Ok(())
    }

    #[cfg(feature = "hc-auth")]
    pub fn is_hc_auth_configured(&self) -> bool {
        self.hc_auth_config.is_some()
    }

    /// Get the hc-auth status for this runtime.
    #[cfg(feature = "hc-auth")]
    pub fn hc_auth_status(&self) -> crate::hc_auth::HcAuthStatus {
        self.hc_auth_status.read().unwrap().clone()
    }

    /// Get the hc-auth agent key (Holochain format) if available.
    #[cfg(feature = "hc-auth")]
    pub fn hc_auth_agent_key(&self) -> Option<AgentPubKey> {
        self.hc_auth_agent_key.read().unwrap().clone()
    }

    /// Get the raw Ed25519 base64url public key for hc-auth.
    #[cfg(feature = "hc-auth")]
    pub fn hc_auth_raw_ed25519_b64url(&self) -> Option<String> {
        self.hc_auth_raw_ed25519_b64url.read().unwrap().clone()
    }

    /// Export the raw 32-byte seed for the hc-auth agent key.
    /// Reads the persisted key file from disk (works regardless of auth status),
    /// looks up the seed in the Lair store, decrypts it, and returns the plaintext bytes.
    #[cfg(feature = "hc-auth")]
    pub async fn export_agent_seed(&self) -> crate::Result<Vec<u8>> {
        use lair_keystore_api::lair_store::LairEntryInner;

        let key_path = self.filesystem.app_data_dir.join("hc-auth-agent-key");
        let key_str = std::fs::read_to_string(&key_path)
            .map_err(|e| crate::Error::AgentSeedError(format!("No agent key file found: {e}")))?;
        let agent_key = holochain_client::AgentPubKey::try_from(key_str.trim()).map_err(|e| {
            crate::Error::AgentSeedError(format!("Invalid agent key in file: {e:?}"))
        })?;

        let mut pub_key_32 = [0u8; 32];
        pub_key_32.copy_from_slice(agent_key.get_raw_32());

        let store =
            self.in_proc_keystore.store().await.map_err(|e| {
                crate::Error::AgentSeedError(format!("Failed to get Lair store: {e}"))
            })?;

        let entry = store
            .get_entry_by_ed25519_pub_key(pub_key_32.into())
            .await
            .map_err(|e| crate::Error::AgentSeedError(format!("Failed to find seed entry: {e}")))?;

        match &*entry {
            LairEntryInner::Seed { seed, .. } => {
                let ctx_key = store.get_bidi_ctx_key();
                let mut decrypted = seed.decrypt(ctx_key).await.map_err(|e| {
                    crate::Error::AgentSeedError(format!("Failed to decrypt seed: {e}"))
                })?;
                let bytes = decrypted.lock().to_vec();
                Ok(bytes)
            }
            _ => Err(crate::Error::AgentSeedError(
                "Agent key entry is not a standard Seed".into(),
            )),
        }
    }

    /// Restart the conductor with fresh hc-auth material.
    /// Lair stays running; only the conductor is shut down and rebuilt.
    /// Returns a new `HolochainRuntime` with the updated conductor.
    #[cfg(feature = "hc-auth")]
    pub async fn restart_with_hc_auth(
        &self,
        mut network_config: crate::NetworkConfig,
    ) -> crate::Result<HolochainRuntime> {
        use crate::hc_auth;

        let hc_auth_config = self
            .hc_auth_config
            .as_ref()
            .ok_or_else(|| crate::Error::HcAuthError("hc-auth not configured".into()))?;

        log::info!("hc-auth restart: Shutting down conductor (Lair stays running)...");
        self.shutdown_conductor_only().await?;

        log::info!("hc-auth restart: Generating fresh auth material...");
        let result = hc_auth::perform_auth_flow(
            &self.lair_client,
            hc_auth_config,
            &self.filesystem.app_data_dir,
        )
        .await?;

        if let Some(ref material) = result.auth_material {
            network_config.base64_auth_material_bootstrap = Some(material.clone());
            network_config.base64_auth_material_relay = Some(material.clone());
        }

        let admin_port = portpicker::pick_unused_port().expect("No ports free");

        let conductor_config = crate::launch::config::conductor_config(
            &self.filesystem,
            admin_port,
            self.filesystem.keystore_dir().into(),
            network_config,
        );

        if let Err(err) =
            crate::launch::write_conductor_config(&self.filesystem.app_data_dir, &conductor_config)
        {
            log::error!("Failed to write conductor config to disk: {}", err);
        }

        let conductor_handle = holochain::conductor::Conductor::builder()
            .config(conductor_config)
            .passphrase(Some(self.passphrase.clone()))
            .with_keystore(self.lair_client.clone())
            .build()
            .await?;

        log::info!(
            "hc-auth restart: Conductor restarted on port {}",
            admin_port
        );

        Ok(HolochainRuntime {
            filesystem: self.filesystem.clone(),
            apps_websockets_auths: Arc::new(Mutex::new(Vec::new())),
            admin_port,
            conductor_handle,
            lair_client: self.lair_client.clone(),
            passphrase: self.passphrase.clone(),
            in_proc_keystore: self.in_proc_keystore.clone(),
            hc_auth_config: self.hc_auth_config.clone(),
            hc_auth_status: Arc::new(std::sync::RwLock::new(result.status)),
            hc_auth_agent_key: Arc::new(std::sync::RwLock::new(Some(result.agent_key))),
            hc_auth_raw_ed25519_b64url: Arc::new(std::sync::RwLock::new(Some(
                result.raw_ed25519_b64url,
            ))),
        })
    }

    /// Shut down only the conductor, leaving Lair running.
    async fn shutdown_conductor_only(&self) -> crate::Result<()> {
        let admin_ws = self.admin_websocket().await?;
        let apps = admin_ws
            .list_apps(Some(holochain_client::AppStatusFilter::Enabled))
            .await?;

        join_all(apps.into_iter().map(async |app| {
            if let Err(err) = self
                .conductor_handle
                .clone()
                .disable_app(
                    app.installed_app_id,
                    holochain::prelude::DisabledAppReason::Error(
                        NETWORK_SHUTDOWN_DISABLED_APP_REASON.into(),
                    ),
                )
                .await
            {
                log::error!("Error disabling app: {err:?}.");
            }
        }))
        .await;

        self.conductor_handle
            .shutdown()
            .await
            .map_err(|e| crate::Error::HolochainShutdownError(e.to_string()))?
            .map_err(|e| crate::Error::HolochainShutdownError(e.to_string()))?;

        log::info!("Conductor shut down (Lair still running).");
        Ok(())
    }

    /// Shutdown the running conductor
    /// Note that this is *NOT* fully implemented by Holochain,
    /// so kitsune tasks will continue to run.
    pub async fn shutdown(&self) -> crate::Result<()> {
        // Leave all networks using `disable_app()`, which will make the cells leave the network
        // and notify the bootstrap server and the peers about it

        let admin_ws = self.admin_websocket().await?;

        let apps = admin_ws
            .list_apps(Some(holochain_client::AppStatusFilter::Enabled))
            .await?;

        join_all(apps.into_iter().map(async |app| {
            if let Err(err) = self
                .conductor_handle
                .clone()
                .disable_app(
                    app.installed_app_id,
                    holochain::prelude::DisabledAppReason::Error(
                        NETWORK_SHUTDOWN_DISABLED_APP_REASON.into(),
                    ),
                )
                .await
            {
                log::error!("Error disabling app: {err:?}.");
            }
        }))
        .await;

        log::info!("Disabled all running apps to leave their networks.");

        self.conductor_handle
            .shutdown()
            .await
            .map_err(|e| crate::Error::HolochainShutdownError(e.to_string()))?
            .map_err(|e| crate::Error::HolochainShutdownError(e.to_string()))?;
        Ok(())
    }
}
