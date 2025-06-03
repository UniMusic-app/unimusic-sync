use crate::types::UNamespaceId;
use uniffi::deps::anyhow;

pub type Result<T> = std::result::Result<T, SharedError>;

#[derive(Debug, thiserror::Error, uniffi::Error, PartialEq, Eq)]
pub enum SharedError {
    #[error("Iroh error: {0}")]
    Iroh(String),
    #[error("Iroh Gossip error: {0}")]
    IrohGossip(String),

    #[error("Serde error: {0}")]
    Serde(String),

    #[error("I/O Error: {0}")]
    IO(String),

    #[error("Tried to open replica, which does not exist: {0}")]
    ReplicaMissing(UNamespaceId),
    #[error("Tried to access entry, which does not exist:\nnamespace: {0}\npath: {1}")]
    EntryMissing(UNamespaceId, String),
    #[error(
        "Tried to access entry, which has been tombstoned (deleted):\nnamespace: {0}\npath: {1}"
    )]
    EntryTombstoned(UNamespaceId, String),
    #[error("Invalid namespace id: {0}")]
    InvalidNamespaceId(String),
    #[error("Sync failed: {0}")]
    SyncFailed(String),
}

impl From<anyhow::Error> for SharedError {
    fn from(value: anyhow::Error) -> Self {
        SharedError::Iroh(value.to_string())
    }
}

impl From<iroh_gossip::net::Error> for SharedError {
    fn from(value: iroh_gossip::net::Error) -> Self {
        SharedError::IrohGossip(value.to_string())
    }
}

impl From<std::io::Error> for SharedError {
    fn from(value: std::io::Error) -> Self {
        SharedError::IO(value.to_string())
    }
}
