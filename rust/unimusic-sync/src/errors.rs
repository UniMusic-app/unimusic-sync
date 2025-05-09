use crate::types::UNamespaceId;
use uniffi::deps::anyhow;

pub type Result<T> = std::result::Result<T, IrohManagerError>;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum IrohManagerError {
    #[error("Iroh error: {0}")]
    Iroh(String),
    #[error("Iroh Gossip error: {0}")]
    IrohGossip(String),

    #[error("Tried to open replica, which does not exist: {0}")]
    ReplicaMissing(UNamespaceId),
    #[error("Tried to access entry, which does not exist:\n namespace: {0}\n path: {1}")]
    EntryMissing(UNamespaceId, String),
    #[error("Invalid namespace id: {0}")]
    InvalidNamespaceId(String),
}

impl From<anyhow::Error> for IrohManagerError {
    fn from(value: anyhow::Error) -> Self {
        IrohManagerError::Iroh(value.to_string())
    }
}

impl From<iroh_gossip::net::Error> for IrohManagerError {
    fn from(value: iroh_gossip::net::Error) -> Self {
        IrohManagerError::IrohGossip(value.to_string())
    }
}
