use crate::canonical_payload;
use ed25519_dalek::{Signer, SigningKey};
use types::{Result, SiteManifest};

/// Signs the canonical JSON of the files map (excluding the sig field itself)
pub fn sign_manifest(mut manifest: SiteManifest, signing_key: &SigningKey) -> Result<SiteManifest> {
    let payload = canonical_payload(&manifest)?;
    let sig = signing_key.sign(&payload);
    manifest.signature = sig.to_bytes().to_vec();
    manifest.owner_pubkey = signing_key.verifying_key().to_bytes().to_vec();
    Ok(manifest)
}
