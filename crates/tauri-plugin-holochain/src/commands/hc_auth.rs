use serde::Serialize;
use tauri::{command, AppHandle, Manager, Runtime};

use crate::HolochainPlugin;

#[derive(Serialize)]
pub struct HcAuthStatusResponse {
    pub status: String,
    pub agent_key: Option<String>,
    pub raw_ed25519_b64url: Option<String>,
    pub detail: Option<String>,
}

#[command]
pub(crate) fn get_hc_auth_status<R: Runtime>(
    app_handle: AppHandle<R>,
) -> Result<HcAuthStatusResponse, String> {
    let plugin = app_handle
        .try_state::<HolochainPlugin<R>>()
        .ok_or_else(|| "Holochain not initialized".to_string())?;

    let runtime = plugin.runtime();

    if !runtime.is_hc_auth_configured() {
        return Ok(HcAuthStatusResponse {
            status: "not_configured".to_string(),
            agent_key: None,
            raw_ed25519_b64url: None,
            detail: None,
        });
    }

    let status = runtime.hc_auth_status();

    use holochain_runtime::hc_auth::HcAuthStatus;
    let (status_str, detail) = match &status {
        HcAuthStatus::Authorized => ("authorized".to_string(), None),
        HcAuthStatus::Pending => ("pending".to_string(), None),
        HcAuthStatus::NotRegistered => ("not_registered".to_string(), None),
        HcAuthStatus::Blocked => ("blocked".to_string(), None),
        HcAuthStatus::Failed(msg) => ("failed".to_string(), Some(msg.clone())),
    };

    Ok(HcAuthStatusResponse {
        status: status_str,
        agent_key: runtime.hc_auth_agent_key().map(|k| format!("{}", k)),
        raw_ed25519_b64url: runtime.hc_auth_raw_ed25519_b64url(),
        detail,
    })
}
