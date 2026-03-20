use crate::store::{IndexStore, SiteRecord};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// The manifest.json schema
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct SiteManifestJson {
    pub sisifo: Option<String>,
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub icon: Option<String>, // relative path e.g. "/icon.png"
    pub accent_color: Option<String>,
    pub author_pubkey: Option<String>,
    pub lang: Option<String>,
    pub category: Option<String>,
}

/// Called after successfully navigating to a site.
/// Tries to fetch /manifest.json from the gateway and index it.
pub async fn crawl_site(site_hash: &str, gateway_base: &str) -> Result<Option<SiteRecord>> {
    let manifest_url = format!("{}/site/{}/manifest.json", gateway_base, site_hash);

    // Fetch manifest via reqwest from the local gateway
    let resp = reqwest::get(&manifest_url).await?;
    if !resp.status().is_success() {
        return Ok(None); // No manifest — site isn't indexed
    }

    let manifest: SiteManifestJson = resp.json().await?;

    // Must have at least a name to be worth indexing
    let name = match manifest.name {
        Some(n) if !n.is_empty() => n,
        _ => return Ok(None),
    };

    let icon_url = manifest
        .icon
        .map(|path| format!("{}/site/{}{}", gateway_base, site_hash, path));

    let indexed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let record = SiteRecord {
        hash: site_hash.to_string(),
        name,
        short_name: manifest.short_name,
        description: manifest.description,
        keywords: manifest.keywords.unwrap_or_default(),
        icon_url,
        author_pubkey: manifest.author_pubkey,
        category: manifest.category,
        lang: manifest.lang,
        accent_color: manifest.accent_color,
        indexed_at,
        visit_count: 1,
    };

    let store = IndexStore::open()?;
    store.upsert(&record)?;

    tracing::info!("indexed site '{}' ({})", record.name, &site_hash[..12]);
    Ok(Some(record))
}
