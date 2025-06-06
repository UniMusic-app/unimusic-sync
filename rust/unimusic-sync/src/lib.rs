pub mod errors;
use errors::{Result, SharedError};

pub mod types;
use types::{UAuthorId, UDocTicket, UEntry, UHash, UNamespaceId, UNodeId};

pub mod node_storage;
use node_storage::NodeStorage;

use log::{info, warn};

use std::{borrow::Cow, fmt::Debug, path::PathBuf, sync::Arc};

use iroh::{Endpoint, NodeAddr, node_info::NodeData, protocol::Router};
use iroh_blobs::{
    ALPN as BLOBS_ALPN, Hash,
    net_protocol::Blobs,
    store::{ExportFormat, ExportMode},
};
use iroh_docs::{
    ALPN as DOCS_ALPN, DocTicket,
    engine::LiveEvent,
    protocol::Docs,
    rpc::{AddrInfoOptions, client::docs::ShareMode},
    store::Query,
};
use iroh_gossip::{ALPN as GOSSIP_ALPN, net::Gossip};

use tokio::{fs, sync::RwLock};
use tokio_stream::StreamExt;

#[cfg(feature = "default")]
use {std::sync::LazyLock, tokio::runtime::Runtime};

#[cfg(feature = "default")]
uniffi::setup_scaffolding!();

#[cfg(feature = "default")]
pub static TOKIO_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();
    let runtime = builder.build().unwrap();
    runtime.block_on(uniffi::deps::async_compat::Compat::new(async {}));
    runtime
});

#[cfg_attr(feature = "default", derive(uniffi::Object))]
#[derive(Debug)]
pub struct IrohFactory;

#[cfg_attr(feature = "default", uniffi::export(async_runtime = "tokio"))]
impl IrohFactory {
    #[cfg_attr(feature = "default", uniffi::constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn iroh_manager(&self, path: &str) -> Result<IrohManager> {
        let path = PathBuf::from(path);

        // Load or generate secret key to preserve the NodeId
        let secret_key = iroh_blobs::util::fs::load_secret_key(path.join("secret.key")).await?;

        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .discovery_local_network()
            .discovery_dht()
            .bind()
            .await?;

        let blobs = Blobs::persistent(&path).await?.build(&endpoint);
        let gossip = Gossip::builder().spawn(endpoint.clone()).await?;
        let docs = Docs::persistent(path.clone())
            .spawn(&blobs, &gossip)
            .await?;

        let router = Router::builder(endpoint.clone())
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

const TOMBSTONE: &[u8] = b"\00";

#[cfg_attr(feature = "default", derive(uniffi::Object))]
#[derive(Debug)]
pub struct IrohManager {
    pub path: PathBuf,
    pub router: Router,
    pub node_storage: Arc<RwLock<NodeStorage>>,

    pub blobs: Blobs<PersistentStore>,
    pub gossip: Gossip,
    pub docs: Docs<PersistentStore>,
}

#[cfg_attr(feature = "default", uniffi::export(async_runtime = "tokio"))]
impl IrohManager {
    // TODO: Add channel/lock which notifies storage to stop locking so shutdown doesn't get starved
    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
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

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn get_known_nodes(&self) -> Vec<UNodeId> {
        self.node_storage
            .read()
            .await
            .nodes
            .keys()
            .copied()
            .collect()
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn get_author(&self) -> Result<UAuthorId> {
        let docs_client = self.docs.client();
        let authors = docs_client.authors();
        let author = authors.default().await?;
        Ok(author.into())
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn get_node_id(&self) -> UNodeId {
        let node_id = self.router.endpoint().node_id();
        node_id.into()
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn create_namespace(&self) -> Result<UNamespaceId> {
        let docs_client = self.docs.client();
        let doc = docs_client.create().await?;
        Ok(doc.id().into())
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn delete_namespace(&self, namespace: UNamespaceId) -> Result<()> {
        let docs_client = self.docs.client();
        docs_client.drop_doc(namespace.into()).await?;
        Ok(())
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn get_files(&self, namespace: UNamespaceId) -> Result<Vec<Arc<UEntry>>> {
        let docs_client = self.docs.client();

        let replica = docs_client
            .open(namespace.into())
            .await?
            .ok_or(SharedError::ReplicaMissing(namespace))?;

        let mut entries = replica.get_many(Query::single_latest_per_key()).await?;

        let mut files = Vec::new();
        let tombstone_hash = Hash::new(TOMBSTONE);
        while let Some(file) = entries.try_next().await? {
            if file.content_hash() == tombstone_hash {
                continue;
            }

            files.push(Arc::new(file.into()))
        }

        Ok(files)
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn delete_file(&self, namespace: UNamespaceId, path: String) -> Result<UHash> {
        let tombstone_hash = self.write_file(namespace, path, TOMBSTONE.to_vec()).await;
        return tombstone_hash;
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
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

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn read_file(&self, namespace: UNamespaceId, path: &str) -> Result<Vec<u8>> {
        let docs_client = self.docs.client();

        let replica = docs_client
            .open(namespace.into())
            .await?
            .ok_or(SharedError::ReplicaMissing(namespace))?;

        let entry = replica
            .get_one(Query::key_exact(path))
            .await?
            .ok_or_else(|| SharedError::EntryMissing(namespace, path.to_string()))?;

        let content_hash = entry.content_hash();

        let tombstone_hash = Hash::new(TOMBSTONE);
        if content_hash == tombstone_hash {
            return Err(SharedError::EntryTombstoned(namespace, path.to_string()));
        }

        self.read_file_hash(content_hash.into()).await
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn read_file_hash(&self, hash: UHash) -> Result<Vec<u8>> {
        let blobs_client = self.blobs.client();
        let bytes = blobs_client.read_to_bytes(hash.into()).await?;
        Ok(bytes.to_vec())
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn export(
        &self,
        namespace: UNamespaceId,
        path: &str,
        destination: &str,
    ) -> Result<()> {
        let docs_client = self.docs.client();

        let replica = docs_client
            .open(namespace.into())
            .await?
            .ok_or(SharedError::ReplicaMissing(namespace))?;

        let entry = replica
            .get_one(Query::key_exact(path))
            .await?
            .ok_or_else(|| SharedError::EntryMissing(namespace, path.to_string()))?;

        self.export_hash(entry.content_hash().into(), destination)
            .await?;

        Ok(())
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn export_hash(&self, hash: UHash, destination: &str) -> Result<()> {
        let blobs_client = self.blobs.client();

        let destination = PathBuf::from(destination);
        if destination.exists() {
            fs::remove_file(&destination).await?;
        }

        blobs_client
            .export(
                hash.into(),
                destination,
                ExportFormat::Blob,
                ExportMode::Copy,
            )
            .await?
            .await?;

        Ok(())
    }

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
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

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn sync(&self, namespace: UNamespaceId) -> Result<()> {
        let docs_client = self.docs.client();
        let replica = docs_client
            .open(namespace.into())
            .await?
            .ok_or(SharedError::ReplicaMissing(namespace))?;

        let node_addrs: Vec<NodeAddr> = {
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

        if node_addrs.is_empty() {
            return Ok(());
        }

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

    #[cfg_attr(feature = "default", uniffi::method(async_runtime = "tokio"))]
    pub async fn import(&self, ticket: UDocTicket) -> Result<UNamespaceId> {
        let ticket: DocTicket = ticket.into();

        let docs_client = self.docs.client();

        {
            info!("[ticket] upserting nodes from ticket {ticket}");
            let mut node_storage = self.node_storage.write().await;
            for node in &ticket.nodes {
                let node_data =
                    NodeData::new(node.relay_url.clone(), node.direct_addresses.clone());
                node_storage.upsert_node(node.node_id, Cow::Owned(node_data));
            }
        }

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
                LiveEvent::NeighborUp(key) => {
                    info!("[namespace {namespace}] {key} connected");
                }
            }
        }
        info!("[ticket] imported namespace {namespace}");

        Ok(replica.id().into())
    }
}

#[cfg(test)]
mod test {
    use crate::errors::SharedError;

    use super::{IrohFactory, IrohManager};
    use log::{error, info, warn};
    use std::{
        error::Error,
        path::{Path, PathBuf},
        sync::Arc,
        time::{Duration, SystemTime},
    };
    use tokio::task::{JoinHandle, JoinSet};

    type Result<T> = std::result::Result<T, Box<dyn Error + Sync + Send>>;

    static MODIFIED_FILE: (&str, &[u8]) = (
        TEST_FILES[0].0,
        "German Shephard, Husky, Pomeranian".as_bytes(),
    );
    static DELETED_FILE: &str = TEST_FILES[1].0;
    static TEST_FILES: [(&str, &[u8]); 3] = [
        (
            "dog_breeds.txt",
            "American Eskimo Dog, Husky, Cocker Spaniel, Pomeranian".as_bytes(),
        ),
        ("i_will_disappear.txt", "i am a ghost".as_bytes()),
        (
            "bing chilling.txt",
            r#"現在我有冰淇淋
        我很喜歡冰淇淋
        但是
        《速度與激情9》
        比冰淇淋
        《速度與激-》
        《速度與激情9》
        我最喜歡
        所以現在是
        音樂時間
        準備

        一
        二
        三

        兩個禮拜以後
        《速度與激情9》
        兩個禮拜以後
        《速度與激情9》
        兩個禮拜以後
        《速度與激情9》

        不要忘記
        不要錯過
        去電影院
        看《速度與激情9》
        因為非常好電影
        動作非常好
        差不多一樣「冰激淋」
        再見"#
                .as_bytes(),
        ),
    ];

    struct TempDir(PathBuf);
    impl TempDir {
        pub fn new() -> Self {
            let sys_time = std::time::SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            Self(std::env::temp_dir().join(sys_time.to_string()))
        }

        pub fn subpath<P: AsRef<Path>>(&self, path: P) -> PathBuf {
            self.0.join(path)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    async fn mock_client(dir: PathBuf) -> Result<IrohManager> {
        let iroh_manager = IrohFactory::new()
            .iroh_manager(&dir.to_string_lossy())
            .await?;
        Ok(iroh_manager)
    }

    #[tokio::test]
    async fn test_connection() -> Result<()> {
        env_logger::init();
        let temp_dir = TempDir::new();

        let provider = mock_client(temp_dir.subpath("provider")).await?;

        info!("[provider] create namespace");
        let namespace = provider.create_namespace().await?;

        info!("[provider] write files");
        let mut file_hashes = Vec::new();
        for (path, contents) in TEST_FILES {
            assert!(provider.read_file(namespace, path).await.is_err());

            let file_hash = provider
                .write_file(namespace, path.to_string(), contents.to_vec())
                .await?;
            assert_eq!(provider.read_file_hash(file_hash).await?, contents);
            assert_eq!(provider.read_file(namespace, path).await?, contents);

            file_hashes.push(file_hash);
        }

        info!("[provider] share ticket");
        let ticket = provider.share(namespace).await?;

        let mut set = JoinSet::new();
        let ticket = Arc::new(ticket);
        let file_hashes = Arc::new(file_hashes);

        info!("[receivers] test 5 concurrent connections");
        for i in 0..5 {
            let ticket = ticket.clone();
            let file_hashes = file_hashes.clone();
            let receiver_path = temp_dir.subpath(format!("receiver_{i}"));
            let task: JoinHandle<Result<()>> = tokio::spawn(async move {
                let receiver = mock_client(receiver_path).await?;

                info!("[receiver {i}]: make sure files are empty before import:");
                info!("[receiver {i}]: via hash");
                for file_hash in file_hashes.iter() {
                    assert!(receiver.read_file_hash(*file_hash).await.is_err());
                }
                info!("[receiver {i}]: via path");
                for (file_path, _) in TEST_FILES {
                    assert!(receiver.read_file(namespace, file_path).await.is_err());
                }

                info!("[receiver {i}]: import ticket");
                let imported_namespace = receiver.import((*ticket).clone()).await?;
                assert_eq!(namespace, imported_namespace);

                info!("[receiver {i}]: imported ticket, waiting 5 seconds for it to propagate...");
                tokio::time::sleep(Duration::from_secs(5)).await;

                info!("[receiver {i}]: make sure files are got properly imported");
                for (j, (path, contents)) in TEST_FILES.into_iter().enumerate() {
                    info!("[receiver {i}]: make sure {path} gets properly imported");
                    assert_eq!(&receiver.read_file_hash(file_hashes[j]).await?, contents);
                    assert_eq!(&receiver.read_file(namespace, path).await?, contents);
                }

                info!("[receiver {i}]: shutdown");
                receiver.shutdown().await?;
                Ok(())
            });
            set.spawn(task);
        }

        for result in set.join_all().await {
            result.unwrap()?;
        }

        info!("[provider]: modify {}", MODIFIED_FILE.0);
        provider
            .write_file(
                namespace,
                MODIFIED_FILE.0.to_string(),
                MODIFIED_FILE.1.to_vec(),
            )
            .await?;
        assert_eq!(
            provider.read_file(namespace, MODIFIED_FILE.0).await?,
            MODIFIED_FILE.1
        );

        info!("[provider]: delete {}", DELETED_FILE);
        provider
            .delete_file(namespace, DELETED_FILE.to_string())
            .await?;
        assert!(matches!(
            provider.read_file(namespace, DELETED_FILE).await,
            Err(SharedError::EntryTombstoned(..))
        ));

        info!(
            "[provider]: modified {} and deleted {}, waiting 5 seconds to propagate...",
            MODIFIED_FILE.0, DELETED_FILE
        );
        tokio::time::sleep(Duration::from_secs(5)).await;

        let mut set = JoinSet::new();
        for i in 0..5 {
            let file_hashes = file_hashes.clone();
            let provider_id = provider.router.endpoint().node_id().into();
            let receiver_path = temp_dir.subpath(format!("receiver_{i}"));
            let task: JoinHandle<Result<IrohManager>> = tokio::spawn(async move {
                info!("[receiver {i}]: recreate");
                let receiver = mock_client(receiver_path).await?;

                info!("[receiver {i}]: make sure files are still there");
                for (j, (path, contents)) in TEST_FILES.iter().enumerate() {
                    assert_eq!(&receiver.read_file_hash(file_hashes[j]).await?, contents);
                    assert_eq!(&receiver.read_file(namespace, path).await?, contents);
                }

                info!("[receiver {i}]: reconnect");
                receiver.reconnect().await;
                let mut retries = 0u32;
                while !receiver.get_known_nodes().await.contains(&provider_id) {
                    if retries > 15 {
                        error!("[receiver {i}]: didn't find provider after 15 tries, giving up");
                        break;
                    }

                    warn!("[receiver {i}]: didn't find provider, retrying in 2s...");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    receiver.reconnect().await;

                    retries += 1;
                }

                info!("[receiver {i}]: sync");
                retries = 0;
                while let Err(_) = receiver.sync(namespace).await {
                    if retries > 15 {
                        error!("[receiver {i}]: failed sync 15 times, giving up");
                        break;
                    }

                    warn!("[receiver {i}]: sync failed, retrying in 2s...");
                    tokio::time::sleep(Duration::from_secs(2)).await;

                    retries += 1
                }

                info!("[receiver {i}]: synced, wait 5 seconds for it to propagate...");
                tokio::time::sleep(Duration::from_secs(5)).await;

                info!("[receiver {i}]: make sure file got properly synced");
                assert_eq!(
                    &receiver.read_file(namespace, MODIFIED_FILE.0).await?,
                    MODIFIED_FILE.1
                );

                info!("[receiver {i}]: make sure file got properly deleted");
                assert!(matches!(
                    receiver.read_file(namespace, DELETED_FILE).await,
                    Err(SharedError::EntryTombstoned(..))
                ));

                Ok(receiver)
            });
            set.spawn(task);
        }

        let mut receivers = Vec::new();
        for result in set.join_all().await {
            receivers.push(result.unwrap()?);
        }

        info!("[receivers]: shutdown");
        for receiver in receivers {
            receiver.shutdown().await?;
        }
        info!("[provider]: shutdown");
        provider.shutdown().await?;

        Ok(())
    }
}
