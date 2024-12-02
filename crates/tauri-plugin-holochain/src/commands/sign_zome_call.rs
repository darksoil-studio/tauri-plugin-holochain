use holochain_client::ZomeCall;
use holochain_types::prelude::ZomeCallUnsigned;
use tauri::{command, AppHandle, Runtime};

use crate::HolochainExt;

#[command]
pub(crate) async fn sign_zome_call<R: Runtime>(
    app_handle: AppHandle<R>,
    zome_call_unsigned: ZomeCallUnsigned,
) -> crate::Result<ZomeCall> {
    let signed_zome_call = app_handle.holochain()?.holochain_runtime.sign_zome_call(zome_call_unsigned).await?;

    Ok(signed_zome_call)
}
