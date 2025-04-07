use holochain_client::InstalledAppId;
use tauri::{Manager, Runtime, WebviewWindowBuilder};
use crate::HolochainExt;


pub trait HappWindowBuilder {
	fn enable_admin_interface(self) -> Self;
	fn enable_app_interface(self, app_id: InstalledAppId) -> Self;
}

impl <'a, R: Runtime, M: Manager<R>> HappWindowBuilder for WebviewWindowBuilder<'a, R, M> {
	fn enable_admin_interface(self) -> Self {

			self
	}
	fn enable_app_interface(self, app_id: InstalledAppId) -> crate::Result<Self> {
		let holochain_plugin = self.manager().holochain::<R, M>()?;
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

    let mut capability_builder =
        CapabilityBuilder::new("sign-zome-call")
            .permission("holochain:allow-sign-zome-call");

    capability_builder = capability_builder.window(label);

    self.app_handle.add_capability(capability_builder)?;
	
		self
	}
}

// fn a()
