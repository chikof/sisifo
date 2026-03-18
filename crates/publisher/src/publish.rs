use ed25519_dalek::SigningKey;
use manifest::{ManifestBuilder, sign_manifest};
use node::SisiNode;
use std::path::Path;
use tracing::debug;
use types::{Result, SisiError, SiteFile, SiteHash, SiteMeta};
use walkdir::WalkDir;

use crate::register_site;

pub struct PublishResult {
    pub site_hash: SiteHash,
    pub meta: SiteMeta,
}

pub async fn publish_dir(
    dir: &Path,
    name: &str,
    signing_key: &SigningKey,
) -> Result<PublishResult> {
    let handle = SisiNode::get()?;
    let blobs = &handle.blobs;

    let mut builder = ManifestBuilder::new(name, signing_key.verifying_key().to_bytes().to_vec());

    for entry in WalkDir::new(dir).follow_links(true) {
        let entry = entry.map_err(|e| SisiError::Io(e.into()))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let abs_path = entry.path();
        let rel_path = abs_path
            .strip_prefix(dir)
            .unwrap()
            .to_string_lossy()
            .to_string();
        let site_path = format!("/{}", rel_path.replace('\\', "/"));

        debug!("adding blob: {}", &site_path);

        let outcome = blobs.add_path(abs_path).await?;

        let mime = mime_guess::from_path(abs_path)
            .first_or_octet_stream()
            .to_string();

        let size = tokio::fs::metadata(abs_path).await?.len();

        builder = builder.add_file(SiteFile {
            path: site_path,
            hash: outcome.hash,
            mime,
            size,
        });
    }

    let manifest = sign_manifest(builder.build_unsigned(), signing_key)?;

    let manifest_bytes = serde_json::to_vec(&manifest)?;
    let manifest_outcome = blobs.add_bytes(manifest_bytes).await?;

    let site_hash = SiteHash(manifest_outcome.hash);
    let meta = SiteMeta::from((&manifest, &site_hash.to_string()));

    register_site(&site_hash.to_string(), &manifest).await?;

    Ok(PublishResult { site_hash, meta })
}
