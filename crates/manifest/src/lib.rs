mod build;
mod sign;
mod verify;

pub use build::ManifestBuilder;
pub use sign::sign_manifest;
pub use verify::verify_manifest;

use types::{Result, SisiError, SiteManifest};

pub fn canonical_payload(manifest: &SiteManifest) -> Result<Vec<u8>> {
    let mut sorted: Vec<_> = manifest.files.iter().collect();
    sorted.sort_by_key(|(k, _)| k.as_str());

    serde_json::to_vec(&sorted).map_err(SisiError::Serde)
}
