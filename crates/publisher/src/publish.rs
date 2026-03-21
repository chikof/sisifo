use anyhow::anyhow;
use ed25519_dalek::SigningKey;
use iroh_blobs::HashAndFormat;
use manifest::{ManifestBuilder, sign_manifest};
use names::{NameClaim, NameStore};
use node::SisiNode;
use pointer::{PointerStore, create_pointer};
use std::path::Path;
use tracing::{debug, info};
use types::{Result, SisiError, SiteFile, SiteHash, SiteMeta};
use walkdir::WalkDir;

use crate::register_site;

pub struct PublishResult {
    pub site_hash: SiteHash,
    pub meta: SiteMeta,
    pub permanent_address: String,
    /// The scope this site was published under, if any.
    pub scope: Option<String>,
    /// The name claim that was registered locally, if any.
    pub name_claim: Option<NameClaim>,
}

/// Publish a directory as a Sísifo site.
///
/// - `scope` maps to a gossip topic scope, e.g. `"forum"`.  Use `None` for
///   the owner's default / primary site.
/// - `human_name` is an optional `local[@scope]` string (e.g. `"chiko@forum"`)
///   that will be claimed in the local [`NameStore`] and returned in the result
///   so the caller can broadcast it via gossip.
pub async fn publish_dir(
    dir: &Path,
    name: &str,
    signing_key: &SigningKey,
    scope: Option<&str>,
    human_name: Option<&str>,
) -> Result<PublishResult> {
    let handle = SisiNode::get()?;
    let blobs = &handle.blobs;
    let pubkey = hex::encode(signing_key.verifying_key().to_bytes());

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

    let mut pointer_store = PointerStore::load()
        .await
        .map_err(|e| SisiError::Iroh(anyhow!(e)))?;

    let version = pointer_store.next_version(&pubkey, scope.or(Some("default")));
    let manifest = sign_manifest(builder.build_unsigned_with_version(version), signing_key)?;

    let manifest_bytes = serde_json::to_vec(&manifest)?;
    let manifest_outcome = blobs.add_bytes(manifest_bytes).await?;

    let site_hash = SiteHash(manifest_outcome.hash);
    let meta = SiteMeta::from((&manifest, &site_hash.to_string()));

    register_site(&site_hash.to_string(), &manifest, scope).await?;

    let effective_scope = scope.or(Some("default"));
    let pointer = create_pointer(
        signing_key,
        &site_hash.to_string(),
        version,
        effective_scope,
    )
    .map_err(|e| SisiError::Iroh(anyhow!(e)))?;

    pointer_store
        .upsert_trusted(pointer)
        .await
        .map_err(|e| SisiError::Iroh(anyhow!(e)))?;

    let site_hash = SiteHash(manifest_outcome.hash);
    let tag = format!("site:{site_hash}");

    handle
        .blobs
        .tags()
        .set(tag.clone(), HashAndFormat::raw(manifest_outcome.hash))
        .await
        .map_err(|e| SisiError::Iroh(anyhow!(e)))?;

    // register a human-readable name claim locally.
    let name_claim = if let Some(n) = human_name {
        let claim = NameClaim::new(signing_key, n).map_err(|e| SisiError::Iroh(anyhow!(e)))?;
        let name_store = NameStore::open().map_err(|e| SisiError::Iroh(anyhow!(e)))?;
        name_store
            .upsert(&claim)
            .map_err(|e| SisiError::Iroh(anyhow!(e)))?;
        Some(claim)
    } else {
        None
    };

    info!(
        "published '{name}' v{version} → {}{}",
        &site_hash.to_string()[..12],
        scope.map(|s| format!(" (scope: {s})")).unwrap_or_default(),
    );

    Ok(PublishResult {
        site_hash,
        meta,
        permanent_address: pubkey,
        scope: scope.map(str::to_string),
        name_claim,
    })
}
