use crate::HolochainExt;
use tauri::{command, AppHandle, Runtime};

#[command]
pub(crate) async fn open_app<R: Runtime>(
    app: AppHandle<R>,
    app_id: String,
    title: String,
    url_path: Option<String>,
) -> crate::Result<()> {
    #[cfg(mobile)]
    {
        app.holochain()?
            .web_happ_window_builder(app_id, url_path)
            .await?
            .build()?;
    }

    #[cfg(desktop)]
    {
        app.holochain()?
            .web_happ_window_builder(app_id, url_path)
            .await?
            .title(title)
            .build()?;
    }

    Ok(())
}
