mod errors;
use errors::{IrohManagerError, Result};

mod types;
use types::{UAuthorId, UDocTicket, UHash, UNamespaceId};

use std::sync::LazyLock;

use iroh::{Endpoint, protocol::Router};
use iroh_blobs::{ALPN as BLOBS_ALPN, net_protocol::Blobs};
use iroh_docs::{
    ALPN as DOCS_ALPN,
    engine::LiveEvent,
    protocol::Docs,
    rpc::{AddrInfoOptions, client::docs::ShareMode},
    store::Query,
};
use iroh_gossip::{ALPN as GOSSIP_ALPN, net::Gossip};

use tokio::runtime::Runtime;
use tokio_stream::StreamExt;

uniffi::setup_scaffolding!();

pub static TOKIO_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all(); // and others
    let rt = builder.build().unwrap();
    rt.block_on(uniffi::deps::async_compat::Compat::new(async {}));
    rt
});

#[derive(uniffi::Object)]
pub struct IrohFactory {}

#[uniffi::export(async_runtime = "tokio")]
impl IrohFactory {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {}
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn iroh_manager(&self, path: String) -> Result<IrohManager> {
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
            .spawn();

        Ok(IrohManager {
            router,

            blobs,
            gossip,
            docs,
        })
    }
}

type PersistentStore = iroh_blobs::store::fs::Store;

#[derive(Debug, uniffi::Object)]
pub struct IrohManager {
    pub router: Router,

    pub blobs: Blobs<PersistentStore>,
    pub gossip: Gossip,
    pub docs: Docs<PersistentStore>,
}

#[uniffi::export(async_runtime = "tokio")]
impl IrohManager {
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn shutdown(&self) -> Result<()> {
        self.router.shutdown().await?;
        Ok(())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn get_author(&self) -> Result<UAuthorId> {
        let docs_client = self.docs.client();
        let authors = docs_client.authors();
        let author = authors.default().await?;
        Ok(author.into())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn get_or_create_namespace(&self) -> Result<UNamespaceId> {
        let docs_client = self.docs.client();

        if let Some(doc) = docs_client.list().await?.next().await {
            return Ok(doc?.0.into());
        }

        let doc = docs_client.create().await?;
        Ok(doc.id().into())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn write_file(
        &self,
        namespace: UNamespaceId,
        path: String,
        data: Vec<u8>,
    ) -> Result<UHash> {
        let docs_client = self.docs.client();

        let authors = docs_client.authors();
        let author = authors.default().await?;

        let replica = docs_client
            .open(namespace.into())
            .await?
            .ok_or(IrohManagerError::ReplicaMissing(namespace))?;

        let hash = replica.set_bytes(author, path, data).await?;

        Ok(hash.into())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn read_file(&self, namespace: UNamespaceId, path: &str) -> Result<Vec<u8>> {
        let docs_client = self.docs.client();

        let replica = docs_client
            .open(namespace.into())
            .await?
            .ok_or(IrohManagerError::ReplicaMissing(namespace))?;

        let entry = replica
            .get_one(Query::key_exact(&path))
            .await?
            .ok_or_else(|| IrohManagerError::EntryMissing(namespace, path.to_string()))?;

        self.read_file_hash(entry.content_hash().into()).await
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn read_file_hash(&self, hash: UHash) -> Result<Vec<u8>> {
        let blobs_client = self.blobs.client();
        let bytes = blobs_client.read_to_bytes(hash.into()).await?;
        Ok(bytes.to_vec())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn share(&self, namespace: UNamespaceId) -> Result<UDocTicket> {
        let docs_client = self.docs.client();
        let replica = docs_client
            .open(namespace.into())
            .await?
            .ok_or(IrohManagerError::ReplicaMissing(namespace))?;

        let ticket = replica
            .share(ShareMode::Write, AddrInfoOptions::RelayAndAddresses)
            .await?;

        Ok(ticket.into())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn import(&self, ticket: UDocTicket) -> Result<UNamespaceId> {
        let ticket = ticket.into();

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

        Ok(replica.id().into())
    }
}
