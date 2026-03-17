use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
use types::Result;

#[derive(Clone)]
pub struct PinSet(Arc<Inner>);

struct Inner {
    hashes: RwLock<HashSet<String>>,
    path: PathBuf,
}

impl PinSet {
    pub async fn load(data_dir: &Path) -> Result<Self> {
        let path = data_dir.join("daemon-pins.json");
        let hashes = if path.exists() {
            let bytes = tokio::fs::read(&path).await?;
            serde_json::from_slice::<HashSet<String>>(&bytes)?
        } else {
            HashSet::new()
        };

        tracing::info!("loaded {} pinned sites", hashes.len());

        Ok(PinSet(Arc::new(Inner {
            hashes: RwLock::new(hashes),
            path,
        })))
    }

    pub async fn add(&self, hash: &str) -> Result<()> {
        self.0.hashes.write().await.insert(hash.to_string());
        self.persist().await
    }

    pub async fn remove(&self, hash: &str) -> Result<()> {
        self.0.hashes.write().await.remove(hash);
        self.persist().await
    }

    pub async fn list(&self) -> Vec<String> {
        self.0.hashes.read().await.iter().cloned().collect()
    }

    async fn persist(&self) -> Result<()> {
        let hashes = self.0.hashes.read().await;
        let bytes = serde_json::to_vec_pretty(&*hashes)?;
        tokio::fs::write(&self.0.path, bytes).await?;
        Ok(())
    }
}
