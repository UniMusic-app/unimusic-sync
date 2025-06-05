// Iroh types passable via UniFFI
use iroh::{NodeId, node_info::NodeData};
use iroh_blobs::Hash;
use iroh_docs::{AuthorId, DocTicket, Entry, NamespaceId};
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

        impl FromStr for $out {
            type Err = <$in as FromStr>::Err;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self($in::from_str(s)?))
            }
        }

        impl From<$out> for String {
            fn from(value: $out) -> Self {
                value.0.to_string()
            }
        }

        #[cfg(feature = "default")]
        uniffi::custom_type!($out, String, {
            lower: |item| item.to_string(),
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

#[cfg_attr(feature = "default", derive(uniffi::Object))]
#[derive(Debug)]
pub struct UEntry(Entry);

impl From<Entry> for UEntry {
    fn from(value: Entry) -> Self {
        Self(value)
    }
}

#[cfg_attr(feature = "default", uniffi::export)]
impl UEntry {
    pub fn key(&self) -> String {
        let key = self.0.key();
        let path = std::str::from_utf8(key).expect("Key to be UTF-8 encoded path");
        path.to_string()
    }

    pub fn content_hash(&self) -> UHash {
        self.0.content_hash().into()
    }

    pub fn content_len(&self) -> u64 {
        self.0.content_len()
    }

    pub fn timestamp(&self) -> u64 {
        self.0.timestamp()
    }

    pub fn namespace(&self) -> UNamespaceId {
        self.0.namespace().into()
    }

    pub fn author(&self) -> UAuthorId {
        self.0.author().into()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
