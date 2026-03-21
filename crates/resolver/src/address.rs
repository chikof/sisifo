use pointer::PointerStore;
use types::{Result, SisiError};

pub async fn parse_and_resolve(addr: &str) -> Result<String> {
    let stripped = addr
        .trim()
        .trim_start_matches("sisi://")
        .trim_end_matches('/');

    // 64-char hex that doesn't start with "bafy" means that it is not a pubkey
    if stripped.len() == 64
        && stripped.chars().all(|c| c.is_ascii_hexdigit())
        && !stripped.starts_with("bafy")
    {
        let store = PointerStore::load()
            .await
            .map_err(|e| SisiError::Iroh(anyhow::anyhow!(e)))?;

        return store.get(stripped).map(|p| p.hash.clone()).ok_or_else(|| {
            SisiError::ManifestNotFound(format!("no pointer found for pubkey {}", &stripped[..8]))
        });
    }

    // Otherwise treat as a raw hash
    Ok(stripped.to_string())
}
