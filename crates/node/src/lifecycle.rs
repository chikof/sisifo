use iroh::{Endpoint, endpoint::presets, protocol::Router};
use iroh_blobs::{BlobsProtocol, store::fs::FsStore};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicUsize},
};
use tokio::sync::OnceCell;
use tracing::info;
use types::{Result, SisiError};

static NODE: OnceCell<NodeHandle> = OnceCell::const_new();

pub struct NodeHandle {
    pub router: Router,
    pub blobs: BlobsProtocol,
    pub endpoint: Endpoint,
    pub data_dir: PathBuf,
    pub peer_count: Arc<AtomicUsize>,
}

pub struct SisiNode;

impl SisiNode {
    pub async fn start(data_dir: &Path) -> Result<()> {
        if NODE.initialized() {
            return Ok(());
        }

        info!("starting iroh node at {:?}", data_dir);
        tokio::fs::create_dir_all(data_dir).await?;

        let endpoint = Endpoint::bind(presets::N0)
            .await
            .map_err(|e| SisiError::Iroh(anyhow::anyhow!(e)))?;

        let blobs_store = FsStore::load(data_dir.join("blobs"))
            .await
            .map_err(|e| SisiError::Iroh(anyhow::anyhow!(e)))?;

        let blobs = BlobsProtocol::new(&blobs_store, None);
        let peer_count = Arc::new(AtomicUsize::new(0));

        let router = Router::builder(endpoint.clone())
            .accept(iroh_blobs::ALPN, blobs.clone())
            .spawn();

        NODE.set(NodeHandle {
            router,
            blobs,
            endpoint,
            data_dir: data_dir.to_path_buf(),
            peer_count,
        })
        .map_err(|_| SisiError::Iroh(anyhow::anyhow!("node already started")))?;

        info!("sisi node started");
        Ok(())
    }

    pub fn get() -> Result<&'static NodeHandle> {
        NODE.get().ok_or_else(|| {
            SisiError::Iroh(anyhow::anyhow!(
                "node not started — call SisiNode::start first"
            ))
        })
    }

    pub async fn shutdown() {
        if let Some(handle) = NODE.get() {
            handle.router.shutdown().await.ok();
            info!("sisi node shut down");
        }
    }
}
