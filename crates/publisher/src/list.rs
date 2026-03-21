use node::SisiNode;
use std::path::PathBuf;
use types::{Result, SisiError, SiteManifest, SiteMeta};

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct PublishedIndex {
    sites: Vec<PublishedEntry>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PublishedEntry {
    hash: String,
    manifest: SiteManifest,
}

fn index_path(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("published").join("index.json")
}

async fn load_index(data_dir: &std::path::Path) -> Result<PublishedIndex> {
    let path = index_path(data_dir);
    if !path.exists() {
        return Ok(PublishedIndex::default());
    }
    let bytes = tokio::fs::read(&path).await?;
    serde_json::from_slice(&bytes).map_err(SisiError::Serde)
}

async fn save_index(data_dir: &std::path::Path, index: &PublishedIndex) -> Result<()> {
    let path = index_path(data_dir);
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    let bytes = serde_json::to_vec_pretty(index)?;
    tokio::fs::write(&path, bytes).await?;
    Ok(())
}

/// Called by publish_dir after a successful publish to register the site
pub async fn register_site(hash: &str, manifest: &SiteManifest) -> Result<()> {
    let handle = SisiNode::get()?;
    let mut index = load_index(&handle.data_dir).await?;

    let owner = hex::encode(&manifest.owner_pubkey);
    if let Some(entry) = index
        .sites
        .iter_mut()
        .find(|e| hex::encode(&e.manifest.owner_pubkey) == owner)
    {
        entry.hash = hash.to_string();
        entry.manifest = manifest.clone();
    } else {
        index.sites.push(PublishedEntry {
            hash: hash.to_string(),
            manifest: manifest.clone(),
        });
    }

    save_index(&handle.data_dir, &index).await
}

pub async fn list_local_sites() -> Result<Vec<SiteMeta>> {
    let handle = SisiNode::get()?;
    let index = load_index(&handle.data_dir).await?;

    let sites = index
        .sites
        .iter()
        .map(|e| SiteMeta::from((&e.manifest, e.hash.as_str())))
        .collect();

    Ok(sites)
}

pub async fn remove_site(hash: &str) -> Result<()> {
    let handle = SisiNode::get()?;
    let mut index = load_index(&handle.data_dir).await?;

    let before = index.sites.len();
    index.sites.retain(|e| e.hash != hash);

    if index.sites.len() == before {
        return Err(SisiError::ManifestNotFound(hash.to_string()));
    }

    save_index(&handle.data_dir, &index).await
}
