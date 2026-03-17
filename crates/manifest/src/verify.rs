use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use types::{Result, SisiError, SiteManifest};

use crate::canonical_payload;

pub fn verify_manifest(manifest: &SiteManifest) -> Result<()> {
    let pubkey_bytes: [u8; 32] = manifest
        .owner_pubkey
        .as_slice()
        .try_into()
        .map_err(|_| SisiError::Signing("invalid pubkey length".into()))?;

    let verifying_key =
        VerifyingKey::from_bytes(&pubkey_bytes).map_err(|e| SisiError::Signing(e.to_string()))?;

    let sig_bytes: [u8; 64] = manifest
        .signature
        .as_slice()
        .try_into()
        .map_err(|_| SisiError::Signing("invalid signature length".into()))?;

    let signature = Signature::from_bytes(&sig_bytes);

    let payload = canonical_payload(manifest)?;

    verifying_key
        .verify(&payload, &signature)
        .map_err(|e| SisiError::Signing(e.to_string()))
}
