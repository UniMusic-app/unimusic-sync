mod errors;
use errors::{Result, SharedError};

mod types;
use types::{UAuthorId, UDocTicket, UHash, UNamespaceId, UNodeId};

mod node_storage;
use node_storage::NodeStorage;

use log::{info, warn};

use std::{
    borrow::Cow,
    fmt::Debug,
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use iroh::{Endpoint, NodeAddr, protocol::Router};
use iroh_blobs::{ALPN as BLOBS_ALPN, net_protocol::Blobs};
use iroh_docs::{
    ALPN as DOCS_ALPN,
    engine::LiveEvent,
    protocol::Docs,
    rpc::{AddrInfoOptions, client::docs::ShareMode},
    store::Query,
};
use iroh_gossip::{ALPN as GOSSIP_ALPN, net::Gossip};

use tokio::{runtime::Runtime, sync::RwLock};
use tokio_stream::StreamExt;

uniffi::setup_scaffolding!();

pub static TOKIO_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();
    let rt = builder.build().unwrap();
    rt.block_on(uniffi::deps::async_compat::Compat::new(async {}));
    rt
});

#[derive(uniffi::Object)]
pub struct IrohFactory;

#[uniffi::export(async_runtime = "tokio")]
impl IrohFactory {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {}
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn iroh_manager(&self, path: String) -> Result<IrohManager> {
        let path = PathBuf::from(path);

        // Load or generate secret key to preserve the NodeId
        let secret_key = iroh_blobs::util::fs::load_secret_key(path.join("secret.key")).await?;

        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .discovery_local_network()
            .bind()
            .await?;

        let builder = Router::builder(endpoint.clone());

        let blobs = Blobs::persistent(&path).await?.build(&endpoint);
        let gossip = Gossip::builder().spawn(endpoint.clone()).await?;
        let docs = Docs::persistent(path.clone())
            .spawn(&blobs, &gossip)
            .await?;

        let router = builder
            .accept(BLOBS_ALPN, blobs.clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .accept(DOCS_ALPN, docs.clone())
            .spawn();

        let node_storage = NodeStorage::load(path.join("nodes.json")).await?;

        let node_storage = Arc::new(RwLock::new(node_storage));

        {
            let node_storage = node_storage.clone();
            tokio::spawn(async move {
                while let Some(item) = endpoint.discovery_stream().next().await {
                    let discovery_item = match item {
                        Ok(item) => item,
                        Err(error) => {
                            warn!("Lagging behind: {error}");
                            continue;
                        }
                    };

                    let node_info = discovery_item.node_info();
                    node_storage
                        .write()
                        .await
                        .upsert_node(node_info.node_id, Cow::Borrowed(&node_info.data))
                }
            });
        }

        Ok(IrohManager {
            path,
            router,
            node_storage,

            blobs,
            gossip,
            docs,
        })
    }
}

type PersistentStore = iroh_blobs::store::fs::Store;

#[derive(Debug, uniffi::Object)]
pub struct IrohManager {
    pub path: PathBuf,
    pub router: Router,
    pub node_storage: Arc<RwLock<NodeStorage>>,

    pub blobs: Blobs<PersistentStore>,
    pub gossip: Gossip,
    pub docs: Docs<PersistentStore>,
}

#[uniffi::export(async_runtime = "tokio")]
impl IrohManager {
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn shutdown(&self) -> Result<()> {
        let node_storage = self.node_storage.read().await;
        let (shutdown, save) = tokio::join!(
            self.router.shutdown(),
            node_storage.save(self.path.join("nodes.json"))
        );
        shutdown?;
        save?;
        Ok(())
    }

    pub async fn reconnect(&self) {
        for (node_id, node_data) in self.node_storage.read().await.nodes.iter() {
            let node_addr = NodeAddr::from_parts(
                (*node_id).into(),
                node_data.relay_url.clone(),
                node_data.direct_addresses.clone(),
            );

            match self.router.endpoint().connect(node_addr, DOCS_ALPN).await {
                Ok(_) => {
                    info!("[reconnect] Connected to {node_id}");
                }
                Err(error) => {
                    warn!("[reconnect] Failed to establish a connection with {node_id}: {error}")
                }
            }
        }
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn get_known_nodes(&self) -> Vec<UNodeId> {
        self.node_storage
            .read()
            .await
            .nodes
            .keys()
            .copied()
            .collect()
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn get_author(&self) -> Result<UAuthorId> {
        let docs_client = self.docs.client();
        let authors = docs_client.authors();
        let author = authors.default().await?;
        Ok(author.into())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn create_namespace(&self) -> Result<UNamespaceId> {
        let docs_client = self.docs.client();
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
            .ok_or(SharedError::ReplicaMissing(namespace))?;

        let hash = replica.set_bytes(author, path, data).await?;

        Ok(hash.into())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn read_file(&self, namespace: UNamespaceId, path: &str) -> Result<Vec<u8>> {
        let docs_client = self.docs.client();

        let replica = docs_client
            .open(namespace.into())
            .await?
            .ok_or(SharedError::ReplicaMissing(namespace))?;

        let entry = replica
            .get_one(Query::key_exact(&path))
            .await?
            .ok_or_else(|| SharedError::EntryMissing(namespace, path.to_string()))?;

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
            .ok_or(SharedError::ReplicaMissing(namespace))?;

        let ticket = replica
            .share(ShareMode::Write, AddrInfoOptions::RelayAndAddresses)
            .await?;

        Ok(ticket.into())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn sync(&self, namespace: UNamespaceId) -> Result<()> {
        let docs_client = self.docs.client();
        let replica = docs_client
            .open(namespace.into())
            .await?
            .ok_or(SharedError::ReplicaMissing(namespace))?;

        let node_addrs = {
            let node_storage = self.node_storage.read().await;
            node_storage
                .nodes
                .iter()
                .map(|(node_id, node_data)| {
                    NodeAddr::from_parts(
                        (*node_id).into(),
                        node_data.relay_url.clone(),
                        node_data.direct_addresses.clone(),
                    )
                })
                .collect()
        };

        replica.start_sync(node_addrs).await?;

        let mut event_stream = replica.subscribe().await?;
        while let Some(event) = event_stream.try_next().await? {
            match event {
                LiveEvent::SyncFinished(event) => {
                    if let Err(err_message) = event.result {
                        return Err(SharedError::SyncFailed(err_message));
                    }
                    info!("[namespace {namespace}] sync finished");
                }
                LiveEvent::ContentReady { hash } => {
                    info!("[namespace {namespace}] Downloaded: {hash}")
                }
                LiveEvent::InsertLocal { entry } => {
                    info!("[namespace {namespace}] Locally inserted: {entry:?}");
                }
                LiveEvent::InsertRemote {
                    from,
                    entry,
                    content_status,
                } => {
                    info!(
                        "[namespace {namespace}] {} inserted: {} (available: {content_status:?})",
                        from.fmt_short(),
                        entry.content_hash().fmt_short()
                    );
                }
                LiveEvent::PendingContentReady => {
                    info!("[namespace {namespace}] content ready");
                    break;
                }
                LiveEvent::NeighborDown(key) => {
                    info!("[namespace {namespace}] {key} disconnected");
                }
                LiveEvent::NeighborUp(key) => info!("[namespace {namespace}] {key} connected"),
            }
        }

        Ok(())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn import(&self, ticket: UDocTicket) -> Result<UNamespaceId> {
        let ticket = ticket.into();

        let docs_client = self.docs.client();

        info!("[ticket] importing {ticket}");
        let (replica, mut event_stream) = docs_client.import_and_subscribe(ticket).await?;
        let namespace = replica.id();
        info!("[ticket] syncing namespace {namespace}");
        while let Some(event) = event_stream.try_next().await? {
            match event {
                LiveEvent::SyncFinished(event) => {
                    if let Err(err_message) = event.result {
                        return Err(SharedError::SyncFailed(err_message));
                    }
                    info!("[namespace {namespace}] sync finished");
                }
                LiveEvent::ContentReady { hash } => {
                    info!("[namespace {namespace}] Downloaded: {hash}")
                }
                LiveEvent::InsertLocal { entry } => {
                    info!("[namespace {namespace}] Locally inserted: {entry:?}");
                }
                LiveEvent::InsertRemote {
                    from,
                    entry,
                    content_status,
                } => {
                    info!(
                        "[namespace {namespace}] {} inserted: {} (available: {content_status:?})",
                        from.fmt_short(),
                        entry.content_hash().fmt_short()
                    );
                }
                LiveEvent::PendingContentReady => {
                    info!("[namespace {namespace}] content ready");
                    break;
                }
                LiveEvent::NeighborDown(key) => {
                    info!("[namespace {namespace}] {key} disconnected");
                }
                LiveEvent::NeighborUp(key) => info!("[namespace {namespace}] {key} connected"),
            }
        }
        info!("[ticket] imported namespace {namespace}");

        Ok(replica.id().into())
    }
}
