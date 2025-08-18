use holochain_client::ConductorApiError;
use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),

    #[error(transparent)]
    TauriError(#[from] tauri::Error),

    #[error("Lock error: {0}")]
    LockError(String),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error("ConductorApiError: `{0:?}`")]
    ConductorApiError(ConductorApiError),

    #[error(transparent)]
    HolochainRuntimeError(#[from] holochain_runtime::Error),

    #[error(transparent)]
    UpdateHappError(#[from] holochain_runtime::UpdateHappError),

    #[error("Http server error: {0}")]
    HttpServerError(String),

    #[error("Sign zome call error: {0}")]
    SignZomeCallError(String),

    #[error("Sign zome call error: {0}")]
    ShutdownError(String),

    #[error("Error opening app: {0}")]
    OpenAppError(String),

    #[error("Holochain has not been initialized yet")]
    HolochainNotInitializedError,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
