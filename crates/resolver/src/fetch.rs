use super::address::{SisiAddress, parse_address};
use node::SisiNode;
use types::{Result, SiteManifest};

const GATEWAY_BASE: &str = "http://127.0.0.1:7777";

pub async fn resolve_to_gateway_url(addr: &str) -> Result<String> {
    let handle = SisiNode::get()?;

    match parse_address(addr)? {
        SisiAddress::Hash(site_hash) => {
            let bytes = handle.blobs.get_bytes(site_hash.0).await?;

            let _manifest: SiteManifest = serde_json::from_slice(&bytes)?;

            Ok(format!("{}/site/{}/index.html", GATEWAY_BASE, site_hash))
        }
    }
}
