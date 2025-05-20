// Iroh types passable via UniFFI
use iroh::{NodeId, node_info::NodeData};
use iroh_blobs::Hash;
use iroh_docs::{AuthorId, DocTicket, NamespaceId};
use serde::{Deserialize, Serialize};

use std::fmt::Display;
use std::str::FromStr;

macro_rules! uniffiable_wrapper {
    ($in:ident, $out:ident) => {
        impl From<$in> for $out {
            fn from(value: $in) -> Self {
                Self(value)
            }
        }

        impl From<$out> for $in {
            fn from(value: $out) -> Self {
                value.0
            }
        }

        impl Display for $out {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                Display::fmt(&self.0, f)
            }
        }

        uniffi::custom_type!($out, String, {
            lower: |author_id| author_id.to_string(),
            try_lift: |string| Ok($out($in::from_str(&string)?))
        });
    };
}

#[derive(Debug, Clone, Copy)]
pub struct UAuthorId(AuthorId);
uniffiable_wrapper!(AuthorId, UAuthorId);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UNamespaceId(NamespaceId);
uniffiable_wrapper!(NamespaceId, UNamespaceId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UNodeId(pub NodeId);
uniffiable_wrapper!(NodeId, UNodeId);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UNodeData {
    pub relay_url: Option<iroh::RelayUrl>,
    pub direct_addresses: std::collections::BTreeSet<std::net::SocketAddr>,
}

impl From<UNodeData> for NodeData {
    fn from(value: UNodeData) -> Self {
        NodeData::new(value.relay_url, value.direct_addresses)
    }
}

impl From<NodeData> for UNodeData {
    fn from(value: NodeData) -> Self {
        UNodeData {
            relay_url: value.relay_url().cloned(),
            direct_addresses: value.direct_addresses().clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UHash(Hash);
uniffiable_wrapper!(Hash, UHash);

#[derive(Debug, Clone)]
pub struct UDocTicket(DocTicket);
uniffiable_wrapper!(DocTicket, UDocTicket);
