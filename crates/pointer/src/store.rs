use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, anyhow};
use node::SisiNode;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{SignedPointer, verify_pointer};

#[derive(Serialize, Deserialize, Default)]
pub struct PointerStore {
    pointers: HashMap<String, SignedPointer>,
    #[serde(skip)]
    path: PathBuf,
}

impl PointerStore {
    pub async fn load() -> Result<Self> {
        let path = store_path()?;
        if !path.exists() {
            return Ok(PointerStore {
                pointers: HashMap::new(),
                path,
            });
        }
        let bytes = tokio::fs::read(&path).await?;
        let mut store: PointerStore = serde_json::from_slice(&bytes)?;
        store.path = path;

        Ok(store)
    }

    /// Only accepted if version is strictly higher than existing one
    pub async fn upsert(&mut self, pointer: SignedPointer) -> Result<()> {
        verify_pointer(&pointer)?;

        let should_update = self
            .pointers
            .get(&pointer.pubkey)
            .map(|ex| pointer.version > ex.version)
            .unwrap_or(true);

        if should_update {
            info!(
                "pointer updated: {} → v{} → {}",
                &pointer.pubkey[..8],
                pointer.version,
                &pointer.hash[..12]
            );
            self.pointers.insert(pointer.pubkey.clone(), pointer);
            self.save().await?;
        }

        Ok(())
    }

    /// Get a SignedPointer by its pubkey
    pub fn get(&self, pubkey: &str) -> Option<&SignedPointer> {
        self.pointers.get(pubkey)
    }

    #[allow(unused)]
    pub fn find_by_name(&self, name: &str, owner_pubkey: &str) -> Option<&SignedPointer> {
        self.pointers.get(owner_pubkey)
    }

    pub fn last_mine(&self, owner_pubkey: &str) -> Vec<&SignedPointer> {
        // Each pubkey has one pointer (one site per id)
        // TODO: support multiple sites pey key via name-scope pointers
        self.pointers
            .values()
            .filter(|p| p.pubkey == owner_pubkey)
            .collect()
    }

    pub fn next_version(&self, pubkey: &str) -> u32 {
        self.pointers
            .get(pubkey)
            .map(|p| p.version + 1)
            .unwrap_or(1)
    }

    async fn save(&self) -> Result<()> {
        tokio::fs::create_dir_all(self.path.parent().unwrap()).await?;
        tokio::fs::write(&self.path, serde_json::to_vec_pretty(self)?).await?;

        Ok(())
    }
}

fn store_path() -> Result<PathBuf> {
    let handle = SisiNode::get().map_err(|e| anyhow!(e.to_string()))?;
    Ok(handle.data_dir.join("pointers.json"))
}
