use crate::{HolochainExt, HolochainPlugin};
use futures::lock::Mutex;
use holochain_client::InstalledAppId;
use std::sync::Arc;
use tauri::{
    ipc::CapabilityBuilder, webview::PageLoadEvent, AppHandle, Listener, Manager, Runtime,
    WebviewWindow, WebviewWindowBuilder,
};

const ZOME_CALL_SIGNER_INITIALIZATION_SCRIPT: &'static str = include_str!("../zome-call-signer.js");

pub trait HappWindowBuilder: Sized {
    fn enable_admin_interface(self) -> Self;
    fn enable_app_interface(self, app_id: InstalledAppId) -> Self;
}

impl<'a, R: Runtime> HappWindowBuilder for WebviewWindowBuilder<'a, R, AppHandle<R>> {
    fn enable_admin_interface(self) -> Self {
        let done = Arc::new(Mutex::new(false));
        self.on_page_load(move |window, payload| {
            let PageLoadEvent::Started = payload.event() else {
                return ();
            };

            let done = done.clone();
            tauri::async_runtime::spawn(async move {
                let done = done.lock().await;
                if *done {
                    return;
                }

                if let Ok(holochain_plugin) = window.holochain() {
                    if let Err(err) = enable_admin_interface(&window, holochain_plugin).await
                    {
                        log::error!("Failed to enable admin interface: {err:?}.");
                    }
                } else {
                    let w = window.clone();
                    window
                        .app_handle()
                        .listen("holochain://setup-completed", move |_e| {
                            let w = w.clone();
                            tauri::async_runtime::spawn(async move {
                                let Ok(holochain_plugin) = w.holochain() else {
                                    log::error!("Could not get holochain plugin after holochain setup completed");
                                    return;
                                };
                                if let Err(err) =
                                    enable_admin_interface(&w, holochain_plugin).await
                                {
                                    log::error!("Failed to enable admin interface: {err:?}.");
                                }
                            });
                        });
                }
            });
        })
    }

    fn enable_app_interface(self, app_id: InstalledAppId) -> Self {
        let done = Arc::new(Mutex::new(false));
        self.on_page_load(move |window, payload| {
            println!("aaaa");
            let PageLoadEvent::Started = payload.event() else {
                return ();
            };
            println!("aaa1");

            let app_id = app_id.clone();
            let done = done.clone();
            tauri::async_runtime::spawn(async move {
            println!("aaa2");
                let done = done.lock().await;
                if *done {
                    return;
                }
            println!("aaa2");

                if let Ok(holochain_plugin) = window.holochain() {
            println!("aaa3");
                    if let Err(err) = enable_app_interface(&window, holochain_plugin, &app_id).await
                    {
                        println!("nooo");
                        log::error!("Failed to enable app interface: {err:?}.");
                    }
                } else {
            println!("aaa4");
                    let w = window.clone();
                    window
                        .app_handle()
                        .listen("holochain://setup-completed", move |_e| {
            println!("aaa5");
                            let app_id = app_id.clone();
                            let w = w.clone();
                            tauri::async_runtime::spawn(async move {
                                let Ok(holochain_plugin) = w.holochain() else {
                                    log::error!("Could not get holochain plugin after holochain setup completed");
                                    return;
                                };
                                if let Err(err) =
                                    enable_app_interface(&w, holochain_plugin, &app_id).await
                                {
                                    log::error!("Failed to enable app interface: {err:?}.");
                                }
                            });
                        });
                }
            });
        })
    }
}

pub async fn enable_admin_interface<R: Runtime>(
    window: &WebviewWindow<R>,
    holochain_plugin: &HolochainPlugin<R>,
) -> crate::Result<()> {
    let admin_port = holochain_plugin.holochain_runtime.admin_port;

    window.eval(format!(
        r#"
if (!window.__HC_LAUNCHER_ENV__) window.__HC_LAUNCHER_ENV__ = {{}};
window.__HC_LAUNCHER_ENV__.ADMIN_INTERFACE_PORT = {};
                    "#,
        admin_port
    ))?;

    Ok(())
}

pub async fn enable_app_interface<R: Runtime>(
    window: &WebviewWindow<R>,
    holochain_plugin: &HolochainPlugin<R>,
    app_id: &InstalledAppId,
) -> crate::Result<()> {
    let allowed_origins = holochain_plugin.get_allowed_origins(app_id, true);
    let app_websocket_auth = holochain_plugin
        .holochain_runtime
        .get_app_websocket_auth(&app_id, allowed_origins)
        .await?;

    let token_vector: Vec<String> = app_websocket_auth
        .token
        .iter()
        .map(|n| n.to_string())
        .collect();
    let token = token_vector.join(",");

    window.eval(format!(
        r#"
if (!window.__HC_LAUNCHER_ENV__) window.__HC_LAUNCHER_ENV__ = {{}};
window.__HC_LAUNCHER_ENV__.APP_INTERFACE_PORT = {};
window.__HC_LAUNCHER_ENV__.APP_INTERFACE_TOKEN = [{}];
window.__HC_LAUNCHER_ENV__.INSTALLED_APP_ID = "{}";
    "#,
        app_websocket_auth.app_websocket_port, token, app_id
    ))?;
    window.eval(ZOME_CALL_SIGNER_INITIALIZATION_SCRIPT)?;

    let mut capability_builder =
        CapabilityBuilder::new("sign-zome-call").permission("holochain:allow-sign-zome-call");

    capability_builder = capability_builder.window(window.label());

    window.add_capability(capability_builder)?;

    Ok(())
}
