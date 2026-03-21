use crate::parse_and_resolve;
use types::Result;

const GATEWAY_BASE: &str = "http://127.0.0.1:7777";

pub async fn resolve_to_gateway_url(addr: &str) -> Result<String> {
    let hash = parse_and_resolve(addr).await?;

    Ok(format!("{}/site/{}/index.html", GATEWAY_BASE, hash))
}
