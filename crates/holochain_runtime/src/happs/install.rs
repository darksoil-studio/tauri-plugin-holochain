use std::{collections::HashMap, path::PathBuf};

use holochain_client::{
    AdminWebsocket, AgentPubKey, AppInfo,  InstallAppPayload, 
};
use holochain_types::prelude::*;

pub async fn install_app(
    admin_ws: &AdminWebsocket,
    app_id: String,
    app_bundle_path: PathBuf,
    roles_settings: Option<HashMap<String,RoleSettings>>,
    agent_key: Option<AgentPubKey>,
    network_seed: Option<NetworkSeed>,
) -> crate::Result<AppInfo> {
    log::info!("Installing app {}", app_id);

    let app_info = admin_ws
        .install_app(InstallAppPayload {
            agent_key,
            roles_settings,
            network_seed,
            source: AppBundleSource::Path(app_bundle_path),
            installed_app_id: Some(app_id.clone()),
            ignore_genesis_failure: false,
            allow_throwaway_random_agent_key: false
        })
        .await
        .map_err(|err| crate::Error::ConductorApiError(err))?;
    log::info!("Installed app {app_info:?}");

    Ok(app_info)
}

pub async fn enable_app(
    admin_ws: &AdminWebsocket,
    app_id: String,
) -> crate::Result<AppInfo> {
    log::info!("Enabling app {}", app_id);

    let response = admin_ws
        .enable_app(app_id.clone())
        .await
        .map_err(|err| crate::Error::ConductorApiError(err))?;

    log::info!("Enabled app {app_id:?}");

    Ok(response.app)
}