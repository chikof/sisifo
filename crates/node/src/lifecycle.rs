use crate::config::NodeConfig;
use anyhow::anyhow;
use iroh::{
    Endpoint, RelayMap, RelayUrl, RelayUrlParseError,
    address_lookup::{DnsAddressLookup, PkarrPublisher, PkarrResolver},
    endpoint::{RelayMode, presets},
    protocol::Router,
};
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
    pub async fn start(data_dir: &Path, config: NodeConfig) -> Result<()> {
        if NODE.initialized() {
            return Ok(());
        }

        info!("starting iroh node at {:?}", data_dir);
        tokio::fs::create_dir_all(data_dir).await?;

        let endpoint = Self::build_endpoint(&config).await?;

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

    async fn build_endpoint(config: &NodeConfig) -> Result<Endpoint> {
        if config.relay_url.is_some() || config.pkarr_url.is_some() {
            let relay_mode = match &config.relay_url {
                Some(url) => {
                    let relay_url: RelayUrl = url
                        .parse()
                        .map_err(|e: RelayUrlParseError| SisiError::Iroh(anyhow::anyhow!(e)))?;
                    RelayMode::Custom(RelayMap::from(relay_url))
                }
                None => RelayMode::Default,
            };

            let mut builder = Endpoint::empty_builder().relay_mode(relay_mode);

            if let Some(pkarr_url) = &config.pkarr_url {
                let url: url::Url = pkarr_url
                    .parse()
                    .map_err(|e: url::ParseError| SisiError::Iroh(anyhow::anyhow!(e)))?;
                builder = builder.address_lookup(PkarrPublisher::builder(url.clone()));
                builder = builder.address_lookup(PkarrResolver::builder(url));
            }

            if let Some(dns_origin) = &config.dns_origin {
                builder = builder.address_lookup(DnsAddressLookup::builder(dns_origin.clone()));
            }

            builder
                .bind()
                .await
                .map_err(|e| SisiError::Iroh(anyhow!(e)))
        } else {
            Endpoint::builder(presets::N0)
                .bind()
                .await
                .map_err(|e| SisiError::Iroh(anyhow!(e)))
        }
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
