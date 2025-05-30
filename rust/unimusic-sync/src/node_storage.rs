use iroh::{NodeId, node_info::NodeData};
use log::info;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap, path::Path};
use tokio::fs;

use crate::errors::{Result, SharedError};
use crate::types::{UNodeData, UNodeId};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeStorage {
    pub nodes: HashMap<UNodeId, UNodeData>,
}

impl NodeStorage {
    /// Load NodeStorage from given path, or create a new instance if it doesn't exist
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        if let Ok(file) = fs::read(path).await {
            let deserialized: Self =
                serde_json::from_slice(&file).map_err(|e| SharedError::Serde(e.to_string()))?;
            info!("[node_storage]: loaded {} nodes", deserialized.nodes.len());
            Ok(deserialized)
        } else {
            Ok(Self::default())
        }
    }

    /// Saves NodeStorage to given path
    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let serialized =
            serde_json::to_vec_pretty(self).map_err(|e| SharedError::Serde(e.to_string()))?;
        info!("[node_storage]: saving {} nodes", self.nodes.len());
        fs::write(path, serialized).await?;
        info!("[node_storage]: saved {} nodes", self.nodes.len());
        Ok(())
    }

    /// Update or insert a new data
    /// Updated info merges with the previous one to hold more information about the node
    pub fn upsert_node(&mut self, id: NodeId, new_data: Cow<NodeData>) {
        info!("[node_storage]: upserted new node: {}", id.fmt_short());
        self.nodes
            .entry(id.into())
            .and_modify(|data| {
                if let Some(relay_url) = new_data.relay_url() {
                    data.relay_url = Some(relay_url.clone());
                }
                data.direct_addresses.extend(new_data.direct_addresses());
            })
            .or_insert_with(|| new_data.into_owned().into());
    }

    pub fn get_node_data(&self, id: NodeId) -> Option<&UNodeData> {
        self.nodes.get(&id.into())
    }

    pub fn get_unode_data(&self, id: &UNodeId) -> Option<&UNodeData> {
        self.nodes.get(id)
    }
}
