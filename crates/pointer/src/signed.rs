use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// A signed pointer: pubkey -> current manifest hash + version
/// The pubkey IS the permanent site address - it never changes.
/// The hash changes on every republish.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPointer {
    /// hex ed25519 pubkey, which is going to be the permanent address
    pub pubkey: String,
    /// current manifest hash
    pub hash: String,
    /// monotomically increasing, higher version always wins
    pub version: u32,
    pub updated_at: u64,
    /// signs over canonical_payload()
    pub signature: Vec<u8>,
}

impl SignedPointer {
    pub fn canonical_payload(&self) -> Vec<u8> {
        format!(
            "sisi-pointer:{}:{}:{}:{}",
            self.pubkey, self.hash, self.version, self.updated_at
        )
        .into_bytes()
    }
}

pub fn create_pointer(signin_key: &SigningKey, hash: &str, version: u32) -> Result<SignedPointer> {
    let pubkey = hex::encode(signin_key.verifying_key().to_bytes());
    let updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let mut pointer = SignedPointer {
        pubkey,
        hash: hash.to_string(),
        version,
        updated_at,
        signature: vec![],
    };

    let sig: Signature = signin_key.sign(&pointer.canonical_payload());
    pointer.signature = sig.to_bytes().to_vec();

    Ok(pointer)
}

pub fn verify_pointer(pointer: &SignedPointer) -> Result<()> {
    let pubkey_bytes: [u8; 32] = hex::decode(&pointer.pubkey)?
        .try_into()
        .map_err(|_| anyhow!("invalid pubkey length"))?;

    let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)?;
    let sig_bytes: [u8; 64] = pointer
        .signature
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("invalid signature length"))?;

    let signature = Signature::from_bytes(&sig_bytes);
    verifying_key
        .verify(&pointer.canonical_payload(), &signature)
        .map_err(|e| anyhow!("invalid pointer signature: {}", e))
}
