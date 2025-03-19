use holochain::{conductor::error::ConductorError, core::SourceChainError, prelude::SerializedBytesError};
use holochain_client::ConductorApiError;
use holochain_state::prelude::{DatabaseError, DatabaseResult};
use mr_bundle::error::MrBundleError;
use one_err::OneErr;
use serde::{ser::Serializer, Serialize};

use crate::{filesystem::FileSystemError, happs::update::UpdateHappError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    LairError(OneErr),

    #[error(transparent)]
    ConductorError(#[from] ConductorError),

    #[error(transparent)]
    SerializedBytesError(#[from] SerializedBytesError),

    #[error(transparent)]
    MrBundleError(#[from] MrBundleError),

    #[error(transparent)]
    FileSystemError(#[from] FileSystemError),

    #[error("JSON serialization error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Lock error: {0}")]
    LockError(String),

    #[error("ConductorApiError: `{0:?}`")]
    ConductorApiError(ConductorApiError),

    #[error("Filesystem error: {0}")]
    FilesystemError(String),

    #[error("Sign zome call error: {0}")]
    SignZomeCallError(String),

    #[error("Admin websocket error: {0}")]
    AdminWebsocketError(String),

    #[error("Error connecting websocket: {0}")]
    WebsocketConnectionError(String),

    #[error("App \"{0}\" does not exist ")]
    AppDoesNotExist(String),

    #[error("App \"{0}\" does not have any UI")]
    AppDoesNotHaveUIError(String),

    #[error(transparent)]
    UpdateAppError(#[from] UpdateHappError),

    #[error(transparent)]
    DatabaseError(#[from] DatabaseError),

    #[error(transparent)]
    SourceChainError(#[from] SourceChainError),

    #[error("Error shutting down holochain: {0}")]
    HolochainShutdownError(String),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
