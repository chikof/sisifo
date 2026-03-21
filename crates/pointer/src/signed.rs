use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// A signed pointer: pubkey → current manifest hash + version.
///
/// The pubkey IS the permanent site address — it never changes.
/// The hash changes on every republish.
///
/// The optional `scope` allows one keypair to own multiple independent sites:
/// - `scope = None`              → the owner's default / primary site.
/// - `scope = Some("forum")`    → the site published in the `forum` context.
///
/// This maps cleanly to the `local@scope` name system: resolving `chiko@forum`
/// finds the pointer with `pubkey = chiko_pubkey, scope = Some("forum")`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPointer {
    /// Hex ed25519 pubkey — the permanent address.
    pub pubkey: String,
    /// Optional topic scope, e.g. `"forum"`, `"blog"`.  `None` = default site.
    #[serde(default)]
    pub scope: Option<String>,
    /// Current manifest hash.
    pub hash: String,
    /// Monotonically increasing; higher version always wins.
    pub version: u32,
    pub updated_at: u64,
    /// Signs over `canonical_payload()`.
    pub signature: Vec<u8>,
}

impl SignedPointer {
    pub fn canonical_payload(&self) -> Vec<u8> {
        // Backward-compatible format: if scope is None or empty, use the
        // original 4-field format so existing signed pointers still verify.
        match self.scope.as_deref() {
            None | Some("") => format!(
                "sisi-pointer:{}:{}:{}:{}",
                self.pubkey, self.hash, self.version, self.updated_at
            ),
            Some(scope) => format!(
                "sisi-pointer:{}:{}:{}:{}:{}",
                self.pubkey, scope, self.hash, self.version, self.updated_at
            ),
        }
        .into_bytes()
    }
}

/// Create a new pointer, signing it with `signing_key`.
pub fn create_pointer(
    signing_key: &SigningKey,
    hash: &str,
    version: u32,
    scope: Option<&str>,
) -> Result<SignedPointer> {
    let pubkey = hex::encode(signing_key.verifying_key().to_bytes());
    let updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let mut pointer = SignedPointer {
        pubkey,
        scope: scope.map(str::to_string),
        hash: hash.to_string(),
        version,
        updated_at,
        signature: vec![],
    };

    let sig: Signature = signing_key.sign(&pointer.canonical_payload());
    pointer.signature = sig.to_bytes().to_vec();

    Ok(pointer)
}

/// Verify a pointer's signature.
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

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn random_key() -> SigningKey {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).unwrap();
        SigningKey::from_bytes(&seed)
    }

    #[test]
    fn create_pointer_should_produce_valid_signature() {
        let key = random_key();
        let ptr = create_pointer(&key, "bafyreic3n7fa4qx", 1, None).unwrap();
        assert!(verify_pointer(&ptr).is_ok());
    }

    #[test]
    fn create_pointer_with_scope_should_produce_valid_signature() {
        let key = random_key();
        let ptr = create_pointer(&key, "bafyreic3n7fa4qx", 1, Some("forum")).unwrap();
        assert!(verify_pointer(&ptr).is_ok());
        assert_eq!(ptr.scope.as_deref(), Some("forum"));
    }

    #[test]
    fn tampered_hash_should_fail_verification() {
        let key = random_key();
        let mut ptr = create_pointer(&key, "bafyreic3n7fa4qx", 1, None).unwrap();
        ptr.hash = "bafyreic000tampered".to_string();
        assert!(verify_pointer(&ptr).is_err());
    }

    #[test]
    fn different_scopes_produce_different_payloads() {
        let key = random_key();
        // unscoped uses 4-field format, scoped uses 5-field format
        let p1 = create_pointer(&key, "hash1", 1, None).unwrap();
        let p2 = create_pointer(&key, "hash1", 1, Some("forum")).unwrap();
        assert_ne!(p1.canonical_payload(), p2.canonical_payload());
    }

    #[test]
    fn unscoped_pointer_canonical_payload_is_backward_compatible() {
        let key = random_key();
        let ptr = create_pointer(&key, "somehash", 1, None).unwrap();
        let payload = String::from_utf8(ptr.canonical_payload()).unwrap();
        // Must NOT contain a scope segment (backward compat with old signatures)
        assert!(!payload.contains("sisi-pointer:abcd::"));
        assert_eq!(payload.matches(':').count(), 4); // sisi-pointer:pubkey:hash:version:ts
    }
}
