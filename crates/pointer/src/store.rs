use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, anyhow};
use node::SisiNode;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{SignedPointer, verify_pointer};

/// Composite key used internally to index pointers.
///
/// Serialised as a JSON object so it round-trips through
/// `serde_json::to_vec_pretty` without ambiguity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct PointerKey {
    pubkey: String,
    /// Topic scope, e.g. `"forum"` or `"blog"`.  Empty string = default site.
    scope: String,
}

impl PointerKey {
    fn new(pubkey: &str, scope: Option<&str>) -> Self {
        PointerKey {
            pubkey: pubkey.to_string(),
            scope: scope.unwrap_or("").to_string(),
        }
    }
}

/// Current on-disk format.
#[derive(Serialize, Deserialize, Default)]
struct StoreDisk {
    pointers: HashMap<String, SignedPointer>,
}

// serde_json requires map keys to be strings, so we encode PointerKey as
// `"<pubkey>:<scope>"` for the JSON map key.
fn key_to_str(k: &PointerKey) -> String {
    format!("{}:{}", k.pubkey, k.scope)
}

fn str_to_key(s: &str) -> PointerKey {
    if s.len() >= 65 && s.as_bytes()[64] == b':' {
        return PointerKey {
            pubkey: s[..64].to_string(),
            scope: s[65..].to_string(), // empty string when unscoped
        };
    }

    warn!(
        "unrecognised pointer key format (len {}), treating as unscoped",
        s.len()
    );
    PointerKey {
        pubkey: s.to_string(),
        scope: String::new(),
    }
}

pub struct PointerStore {
    pointers: HashMap<PointerKey, SignedPointer>,
    path: PathBuf,
}

impl PointerStore {
    pub async fn load() -> Result<Self> {
        let path = store_path()?;
        if !path.exists() {
            tracing::info!("pointer store not found at {:?}, starting fresh", path);
            return Ok(PointerStore {
                pointers: HashMap::new(),
                path,
            });
        }

        let bytes = tokio::fs::read(&path).await?;

        let disk: StoreDisk = serde_json::from_slice(&bytes).map_err(|e| {
            anyhow!(
                "failed to parse pointers.json — \
                 delete it to reset: {e}"
            )
        })?;

        let pointers: HashMap<PointerKey, SignedPointer> = disk
            .pointers
            .into_iter()
            .map(|(k, v)| (str_to_key(&k), v))
            .collect();

        tracing::info!(
            "pointer store loaded: {} entries from {:?}",
            pointers.len(),
            path
        );
        for (k, v) in &pointers {
            tracing::debug!(
                "  pointer: pubkey={} scope={:?} hash={}",
                &k.pubkey[..8.min(k.pubkey.len())],
                k.scope,
                &v.hash[..12.min(v.hash.len())]
            );
        }

        Ok(PointerStore { pointers, path })
    }

    /// Insert or update a pointer, accepting only strictly higher versions.
    ///
    /// Verifies the signature — use this for pointers received over the network.
    pub async fn upsert(&mut self, pointer: SignedPointer) -> Result<()> {
        verify_pointer(&pointer)?;
        self.upsert_trusted(pointer).await
    }

    /// Insert or update a pointer without re-verifying the signature.
    ///
    /// Use this only for pointers we created ourselves or loaded from the
    /// trusted local file (which was already verified when it was first stored).
    pub async fn upsert_trusted(&mut self, pointer: SignedPointer) -> Result<()> {
        let key = PointerKey::new(&pointer.pubkey, pointer.scope.as_deref());

        let should_update = self
            .pointers
            .get(&key)
            .map(|ex| pointer.version > ex.version)
            .unwrap_or(true);

        if should_update {
            info!(
                pubkey = %&pointer.pubkey[..8],
                scope  = %pointer.scope.as_deref().unwrap_or("<default>"),
                version = pointer.version,
                hash = %&pointer.hash[..12],
                "pointer updated"
            );
            self.pointers.insert(key, pointer);
            self.save().await?;
        }

        Ok(())
    }

    /// Get the pointer for a pubkey + optional scope.
    ///
    /// `scope = None` returns the default (unscoped) site pointer.
    pub fn get(&self, pubkey: &str) -> Option<&SignedPointer> {
        self.get_scoped(pubkey, None)
    }

    pub fn get_scoped(&self, pubkey: &str, scope: Option<&str>) -> Option<&SignedPointer> {
        let key = PointerKey::new(pubkey, scope);
        let result = self.pointers.get(&key);
        if result.is_none() {
            tracing::debug!(
                "pointer lookup miss: pubkey={} scope={:?} (store has {} entries)",
                &pubkey[..8.min(pubkey.len())],
                scope,
                self.pointers.len()
            );
        }
        result
    }

    /// All pointers owned by a pubkey (across all scopes).
    pub fn all_for_pubkey(&self, pubkey: &str) -> Vec<&SignedPointer> {
        self.pointers
            .iter()
            .filter(|(k, _)| k.pubkey == pubkey)
            .map(|(_, v)| v)
            .collect()
    }

    /// Next version number for a (pubkey, scope) pair.
    pub fn next_version(&self, pubkey: &str, scope: Option<&str>) -> u32 {
        self.pointers
            .get(&PointerKey::new(pubkey, scope))
            .map(|p| p.version + 1)
            .unwrap_or(1)
    }

    pub async fn remove_by_hash(&mut self, hash: &str) -> Result<()> {
        let before = self.pointers.len();
        self.pointers.retain(|_, v| v.hash != hash);
        if self.pointers.len() < before {
            self.save().await?;
        }

        Ok(())
    }

    async fn save(&self) -> Result<()> {
        tokio::fs::create_dir_all(self.path.parent().unwrap()).await?;
        let disk = StoreDisk {
            pointers: self
                .pointers
                .iter()
                .map(|(k, v)| (key_to_str(k), v.clone()))
                .collect(),
        };
        tokio::fs::write(&self.path, serde_json::to_vec_pretty(&disk)?).await?;
        Ok(())
    }
}

fn store_path() -> Result<PathBuf> {
    let handle = SisiNode::get().map_err(|e| anyhow!(e.to_string()))?;
    Ok(handle.data_dir.join("pointers.json"))
}
