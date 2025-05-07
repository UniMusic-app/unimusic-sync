use std::error::Error;

use iroh_blobs::store::ExportMode;
use iroh_docs::{
    Author,
    rpc::{AddrInfoOptions, client::docs::ShareMode},
    store::Query,
};

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use iroh::{protocol::Router, Endpoint};
        use iroh_blobs::{net_protocol::Blobs, ALPN as BLOBS_ALPN};
        use iroh_docs::{protocol::Docs, ALPN as DOCS_ALPN};
        use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN};
    } else {
        use wasm_bindgen::prelude::*;
    }
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

type BlobsStore = iroh_blobs::store::mem::Store;
pub struct IrohManager {
    pub router: Router,

    pub blobs: Blobs<BlobsStore>,
    pub gossip: Gossip,
    pub docs: Docs<BlobsStore>,
}

pub async fn manager() -> Result<IrohManager> {
    cfg_if::cfg_if! {
        if #[cfg(not(target_arch = "wasm32"))] {
            let endpoint = Endpoint::builder()
                .discovery_n0()
                .discovery_local_network()
                .bind()
                .await?;

            let builder = Router::builder(endpoint.clone());

            let blobs = Blobs::memory().build(&endpoint);
            let gossip = Gossip::builder().spawn(endpoint.clone()).await?;
            let docs = Docs::memory().spawn(&blobs, &gossip).await?;

            let router = builder
                .accept(BLOBS_ALPN, blobs.clone())
                .accept(GOSSIP_ALPN, gossip.clone())
                .accept(DOCS_ALPN, docs.clone())
                .spawn()
                .await?;

            Ok(IrohManager {
                router,

                blobs,
                gossip,
                docs
            })
        } else {
            todo!()
        }
    }
}

pub async fn start() -> Result<()> {
    cfg_if::cfg_if! {
        if #[cfg(not(target_arch = "wasm32"))] {
            let iroh_manager = manager().await?;

            let docs_client =  iroh_manager.docs.client();
            let blobs_client = iroh_manager.blobs.client();

            let authors = docs_client.authors();
            let author = authors.create().await?;

            let doc = docs_client.create().await?;
            let doc_namespace = doc.id();
            let doc_ticket = doc.share(ShareMode::Read, AddrInfoOptions::Id).await?;

            doc.set_bytes(author, "dog.txt", "woof").await?;
            doc.set_bytes(author, "cat.txt", "meow").await?;

            for path in ["dog.txt", "cat.txt"] {
                let dog_entry = doc.get_one(Query::key_exact(path)).await?;
                if let Some(entry) = dog_entry {
                    let content_bytes = blobs_client.read_to_bytes(entry.content_hash()).await?;
                    let content = String::from_utf8_lossy(&content_bytes);

                    println!("{path}: {content}");
                } else {
                    println!("no {path}");
                }
            }

            println!("Namespace: {doc_namespace}");
            println!("Ticket: {doc_ticket}");
        } else {
            todo!()
        }
    }

    Ok(())
}
