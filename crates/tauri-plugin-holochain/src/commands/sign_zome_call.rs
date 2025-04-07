use holochain_runtime::ZomeCallParamsSigned;
use holochain_types::prelude::ZomeCallParams;
use tauri::{command, AppHandle, Runtime};

use crate::HolochainExt;

#[command]
pub(crate) async fn sign_zome_call<R: Runtime>(
    app_handle: AppHandle<R>,
    zome_call_unsigned: ZomeCallParams,
) -> crate::Result<ZomeCallParamsSigned> {
    let signed_zome_call = app_handle.holochain()?.holochain_runtime.sign_zome_call(zome_call_unsigned).await?;

    Ok(signed_zome_call)
}
