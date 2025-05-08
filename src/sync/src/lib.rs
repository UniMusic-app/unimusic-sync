use std::{
    error::Error,
    path::{Path, PathBuf},
    str::FromStr,
};
use thiserror::Error;

use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{net_protocol::Blobs, Hash, ALPN as BLOBS_ALPN};
use iroh_docs::{
    engine::LiveEvent,
    protocol::Docs,
    rpc::{client::docs::ShareMode, AddrInfoOptions},
    store::Query,
    AuthorId, DocTicket, NamespaceId, ALPN as DOCS_ALPN,
};
use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN};

use tokio_stream::StreamExt;

#[derive(Debug, Error)]
pub enum IrohManagerError {
    #[error(transparent)]
    Iroh(#[from] Box<dyn Error>),
    #[error("Tried to open replica, which does not exist: {0}")]
    ReplicaMissing(NamespaceId),
    #[error("Tried to access entry, which does not exist:\n namespace: {0}\n path: {1}")]
    EntryMissing(NamespaceId, String),
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

type PersistentStore = iroh_blobs::store::fs::Store;
// type MemoryStore = iroh_blobs::store::mem::Store;

pub struct IrohManager {
    pub endpoint: Endpoint,
    pub router: Router,

    pub blobs: Blobs<PersistentStore>,
    pub gossip: Gossip,
    pub docs: Docs<PersistentStore>,
}

impl IrohManager {
    pub async fn new<P: AsRef<Path> + Into<PathBuf>>(path: P) -> Result<IrohManager> {
        let endpoint = Endpoint::builder()
            .discovery_n0()
            .discovery_local_network()
            .bind()
            .await?;

        let builder = Router::builder(endpoint.clone());

        let blobs = Blobs::persistent(&path).await?.build(&endpoint);
        let gossip = Gossip::builder().spawn(endpoint.clone()).await?;
        let docs = Docs::persistent(path.into()).spawn(&blobs, &gossip).await?;

        let router = builder
            .accept(BLOBS_ALPN, blobs.clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .accept(DOCS_ALPN, docs.clone())
            .spawn()
            .await?;

        Ok(IrohManager {
            endpoint,
            router,

            blobs,
            gossip,
            docs,
        })
    }

    pub async fn close(self) -> Result<()> {
        self.router.shutdown().await?;
        self.endpoint.close().await;
        Ok(())
    }

    pub async fn get_author(&self) -> Result<AuthorId> {
        let docs_client = self.docs.client();
        let authors = docs_client.authors();
        let author = authors.default().await?;
        Ok(author)
    }

    pub async fn get_or_create_namespace(&mut self) -> Result<NamespaceId> {
        let docs_client = self.docs.client();

        if let Some(doc) = docs_client.list().await?.next().await {
            return Ok(doc?.0);
        }

        let doc = docs_client.create().await?;
        Ok(doc.id())
    }

    pub async fn write_file(
        &mut self,
        namespace: NamespaceId,
        path: String,
        data: Vec<u8>,
    ) -> Result<Hash> {
        let docs_client = self.docs.client();

        let authors = docs_client.authors();
        let author = authors.default().await?;

        let replica = docs_client
            .open(namespace)
            .await?
            .ok_or(IrohManagerError::ReplicaMissing(namespace))?;

        let hash = replica.set_bytes(author, path, data).await?;

        Ok(hash)
    }

    pub async fn read_file(&self, namespace: NamespaceId, path: &str) -> Result<Vec<u8>> {
        let docs_client = self.docs.client();

        let replica = docs_client
            .open(namespace)
            .await?
            .ok_or(IrohManagerError::ReplicaMissing(namespace))?;

        let entry = replica
            .get_one(Query::key_exact(&path))
            .await?
            .ok_or_else(|| IrohManagerError::EntryMissing(namespace, path.to_string()))?;

        self.read_file_hash(entry.content_hash()).await
    }

    pub async fn read_file_hash(&self, hash: Hash) -> Result<Vec<u8>> {
        let blobs_client = self.blobs.client();
        let bytes = blobs_client.read_to_bytes(hash).await?;
        Ok(bytes.to_vec())
    }

    pub async fn share(&self, namespace: NamespaceId) -> Result<DocTicket> {
        let docs_client = self.docs.client();
        let replica = docs_client
            .open(namespace)
            .await?
            .ok_or(IrohManagerError::ReplicaMissing(namespace))?;

        let ticket = replica
            .share(ShareMode::Write, AddrInfoOptions::RelayAndAddresses)
            .await?;

        Ok(ticket)
    }

    pub async fn import(&self, ticket: &str) -> Result<NamespaceId> {
        let ticket = DocTicket::from_str(ticket)?;
        let docs_client = self.docs.client();

        println!("Trying to import ticket: {ticket:?}");

        let (replica, mut event_stream) = docs_client.import_and_subscribe(ticket).await?;
        while let Some(event) = event_stream.next().await {
            match event? {
                LiveEvent::PendingContentReady => break,
                event => println!("EVENT: {event:?}"),
            }
        }

        println!("Imported ticket");

        Ok(replica.id())
    }
}
