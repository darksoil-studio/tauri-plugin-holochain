use std::{collections::HashMap, sync::Arc};

use async_std::sync::Mutex;
use holochain::{conductor::{self, ConductorHandle}, core::ActionHash, prelude::{ builder::{CloseChain, OpenChain}, CellId, ChainTopOrdering, MigrationTarget, NetworkSeed, RoleSettingsMap, ZomeCallUnsigned} };
use holochain_client::{AdminWebsocket, AgentPubKey, AppInfo, AppWebsocket, InstalledAppId, WebsocketConfig, ZomeCall};
use holochain_state::prelude::SourceChain;
use holochain_types::{app::{AppBundle, RoleSettings}, web_app::WebAppBundle, websocket::AllowedOrigins};
use lair_keystore::dependencies::sodoken::BufRead;
use sbd_server::SbdServer;

use crate::{filesystem::{AppBundleStore, BundleStore, FileSystem}, happs::{install::install_app, update::{update_app, UpdateHappError}}, lair_signer::LairAgentSignerWithProvenance, launch::launch_holochain_runtime, sign_zome_call_with_client, HolochainRuntimeConfig};

#[derive(Clone)]
pub struct AppWebsocketAuth {
    pub app_id: String,
    pub app_websocket_port: u16,
    pub allowed_origins: AllowedOrigins,
    pub token: Vec<u8>,
}

pub struct HolochainRuntime {
    pub filesystem: FileSystem,
    pub apps_websockets_auths: Arc<Mutex<Vec<AppWebsocketAuth>>>,
    pub admin_port: u16,
    pub conductor_handle: ConductorHandle,
    pub(crate) _local_sbd_server: Option<SbdServer>,
}

impl HolochainRuntime {
    pub async fn launch(passphrase: BufRead, config: HolochainRuntimeConfig) -> crate::Result<Self> {
        launch_holochain_runtime(passphrase, config).await
    }
    
    /// Builds an `AdminWebsocket` ready to use
    pub async fn admin_websocket(&self) -> crate::Result<AdminWebsocket> {
        let mut config = WebsocketConfig::CLIENT_DEFAULT;
        config.default_request_timeout = std::time::Duration::new(60 * 5, 0);

        let admin_ws = AdminWebsocket::connect_with_config(
            format!("localhost:{}", self.admin_port),
            Arc::new(config),
        )
        .await
        .map_err(|err| crate::Error::WebsocketConnectionError(format!("{err:?}")))?;

        Ok(admin_ws)
    }

    pub async fn get_app_websocket_auth(
        &self,
        app_id: &InstalledAppId,
        allowed_origins: AllowedOrigins
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
            .attach_app_interface(0, allowed_origins.clone(), Some(app_id.clone()))
            .await
            .map_err(|err| crate::Error::ConductorApiError(err))?;

        let response = admin_ws
            .issue_app_auth_token(
                holochain_conductor_api::IssueAppAuthenticationTokenPayload {
                    installed_app_id: app_id.clone(),
                    expiry_seconds: 999999999,
                    single_use: false,
                },
            )
            .await
            .map_err(|err| crate::Error::ConductorApiError(err))?;

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
    pub async fn app_websocket(&self, app_id: InstalledAppId, allowed_origins: AllowedOrigins) -> crate::Result<AppWebsocket> {
        let app_websocket_auth = self.get_app_websocket_auth(&app_id, allowed_origins).await?;
        let app_ws = AppWebsocket::connect(
            format!("localhost:{}", app_websocket_auth.app_websocket_port),
            app_websocket_auth.token,
            Arc::new(LairAgentSignerWithProvenance::new(Arc::new(
                self
                    .conductor_handle
                    .keystore()
                    .lair_client()
                    .clone(),
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
        self
            .filesystem
            .bundle_store
            .store_web_happ_bundle(app_id.clone(), &web_app_bundle)
            .await?;

        let app_bundle = web_app_bundle.happ_bundle().await?;
        let app_bundle_path = self.filesystem.bundle_store.happ_bundle_store().app_bundle_path(&app_bundle)?;

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

        self
            .filesystem
            .bundle_store
            .store_happ_bundle(app_id.clone(), &app_bundle)?;

        let app_bundle_path = self.filesystem.bundle_store.happ_bundle_store().app_bundle_path(&app_bundle)?;

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
        self
            .filesystem
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

        let installed_apps = self
            .filesystem
            .bundle_store
            .installed_apps_store
            .get()?;
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

        let installed_apps = self
            .filesystem
            .bundle_store
            .installed_apps_store
            .get()?;
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
    pub async fn sign_zome_call(&self, zome_call_unsigned: ZomeCallUnsigned) -> crate::Result<ZomeCall> {
        let signed_zome_call = sign_zome_call_with_client(
            zome_call_unsigned,
            &self
                .conductor_handle
                .keystore()
                .lair_client()
                .clone(),
        )
        .await?;
        Ok(signed_zome_call)
    }

    /// Check if an app with a given app_id installed on the holochain conductor
    /// 
    /// * `app_id` - the app id to check
    pub async fn is_app_installed(
        &self,
        app_id: InstalledAppId
    ) -> crate::Result<bool> {
        let admin_ws = self.admin_websocket().await?;
        let apps = admin_ws.list_apps(None).await
            .map_err(|e| crate::Error::ConductorApiError(e))?;
        let matching_app = apps.into_iter().find(|app_info| app_info.installed_app_id == app_id);

        Ok(matching_app.is_some())
    }

    /// Uninstall the app with the given `app_id` from the holochain conductor
    ///
    /// * `app_id` - the app id of the app to uninstall
    pub async fn uninstall_app(
        &self,
        app_id: InstalledAppId
    ) -> crate::Result<()> {
        let admin_ws = self.admin_websocket().await?;
        admin_ws.uninstall_app(app_id, false)
            .await
            .map_err(|e| crate::Error::ConductorApiError(e))?;

        Ok(())
    }

    /// Enable the app with the given `app_id` from the holochain conductor
    ///
    /// * `app_id` - the app id of the app to enable
    pub async fn enable_app(
        &self,
        app_id: InstalledAppId
    ) -> crate::Result<()> {
        let admin_ws = self.admin_websocket().await?;
        admin_ws.enable_app(app_id)
            .await
            .map_err(|e| crate::Error::ConductorApiError(e))?;

        Ok(())
    }

    /// Disable the app with the given `app_id` from the holochain conductor
    ///
    /// * `app_id` - the app id of the app to disable
    pub async fn disable_app(
        &self,
        app_id: InstalledAppId
    ) -> crate::Result<()> {
        let admin_ws = self.admin_websocket().await?;
        admin_ws.disable_app(app_id)
            .await
            .map_err(|e| crate::Error::ConductorApiError(e))?;

        Ok(())
    }

    pub async fn migrate_app(&self,
        old_app_id: InstalledAppId,
        new_app_id: InstalledAppId,
        new_web_app_bundle: WebAppBundle,
    ) -> crate::Result<AppInfo> {
        let old_app = self.app_websocket(old_app_id.clone(), AllowedOrigins::Any).await?;
        let Some(app_info) = old_app.app_info().await.map_err(|err| crate::Error::ConductorApiError(err))? else {
            return Err(crate::Error::AppDoesNotExist(old_app_id));
        };
        let mut roles_settings: RoleSettingsMap = RoleSettingsMap::new();

        // for 

        
        let new_app = self.install_web_app(new_app_id, new_web_app_bundle, Some(roles_settings), Some(app_info.agent_pub_key), None).await?;

        // TODO: clone all clone cells again
        // TODO: fix duplication of cell ids

        Ok(new_app)
    }
 
    async fn get_source_chain(&self, cell_id: &CellId) -> crate::Result<SourceChain> {
        let (dna_hash, agent_key) = cell_id.clone().into_dna_and_agent();
        let spaces = self.conductor_handle.get_spaces();
        Ok(SourceChain::new(
            spaces.get_or_create_authored_db(&dna_hash, agent_key.clone())?,
            spaces.dht_db(&dna_hash)?,
            spaces.get_or_create_space(&dna_hash)?.dht_query_cache,
            self.conductor_handle.keystore().clone(),
            agent_key.clone(),
        )
        .await?)
    }

    /// Given two existing cells, write a CloseChain on the old one and an OpenChain on the new one
    pub(crate) async fn migrate_cell(
        &self,
        old_cell_id: &CellId,
        new_cell_id: &CellId,
    ) -> crate::Result<(ActionHash, ActionHash)> {
        let (backward, forward) = match (old_cell_id.dna_hash(),old_cell_id.agent_pubkey(), new_cell_id.dna_hash(),new_cell_id.agent_pubkey()) {
            (dna1, agent1, dna2, agent2)
                if dna1 == dna2 && agent1 != agent2 =>
            {
                (
                    MigrationTarget::Agent(agent1.clone()),
                    MigrationTarget::Agent(agent2.clone()),
                )
            }
            (dna1, agent1, dna2, agent2)
                if dna1 != dna2 && agent1 == agent2 =>
            {
                (
                    MigrationTarget::Dna(dna1.clone()),
                    MigrationTarget::Dna(dna2.clone()),
                )
            }
            _ => todo!(
                "not valid migration targets: {:?} -> {:?}",
                old_cell_id,
                new_cell_id
            ),
        };
        let old_chain = self.get_source_chain(old_cell_id).await?;
        let close_hash = old_chain
            .put_weightless(
                CloseChain::new(Some(forward)),
                None,
                ChainTopOrdering::Strict,
            )
            .await?;
        {
            let old_cell = self.conductor_handle.cell_by_id(old_cell_id).await?;
            let old_network = old_cell.holochain_p2p_dna();
            old_chain.flush(old_network).await?;
        }

        let new_chain = self.get_source_chain(new_cell_id).await?;
        let open_hash = new_chain
            .put_weightless(
                OpenChain::new(backward, close_hash.clone()),
                None,
                ChainTopOrdering::Strict,
            )
            .await?;
        {
            let new_cell = self.conductor_handle.cell_by_id(new_cell_id).await?;
            let new_network = new_cell.holochain_p2p_dna();
            new_chain.flush(new_network).await?;
        }

        Ok((close_hash, open_hash))
    }

    /// Shutdown the running conductor
    /// Note that this is *NOT* fully implemented by Holochain,
    /// so kitsune tasks will continue to run.
    pub async fn shutdown(&self) -> crate::Result<()> {
        self.conductor_handle
            .shutdown()
            .await
            .map_err(|e| crate::Error::HolochainShutdownError(e.to_string()))?
            .map_err(|e| crate::Error::HolochainShutdownError(e.to_string()))?;
        Ok(())
    }
}
