use crate::HolochainExt;
use holochain_client::AppInfo;
use holochain_types::{app::RoleSettingsMap, web_app::WebAppBundle};
use tauri::{command, AppHandle, Runtime};

#[command]
pub(crate) async fn install_web_app<R: Runtime>(
    app: AppHandle<R>,
    app_id: String,
    web_app_bundle: WebAppBundle,
    roles_settings: Option<RoleSettingsMap>,
    network_seed: Option<String>
) -> crate::Result<AppInfo> {
    app.holochain()?.install_web_app(app_id, web_app_bundle, roles_settings, None, network_seed)
        .await
}


#[command]
pub(crate) async fn uninstall_web_app<R: Runtime>(
    app: AppHandle<R>,
    app_id: String,
) -> crate::Result<()> {
    let admin_ws = app.holochain()?.admin_websocket().await?;
     admin_ws.uninstall_app(app_id, false).await?;
    Ok(())
}

#[command]
pub(crate) async fn list_apps<R: Runtime>(
    app: AppHandle<R>,
) -> crate::Result<Vec<AppInfo>> {
    let admin_ws = app.holochain()?.admin_websocket().await?;
    let apps = admin_ws.list_apps(None).await?;

    Ok(apps)
}
