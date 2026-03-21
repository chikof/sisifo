use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum byte-length of the full name string (e.g. "chiko@forum").
pub const MAX_NAME_LEN: usize = 64;

/// A cryptographically-owned human-readable name.
///
/// A name has the form `local` or `local@scope`, e.g. `chiko` or `chiko@forum`.
/// The `scope` component maps to a gossip topic, so name claims propagate
/// naturally to nodes subscribed to that topic.
///
/// Conflict resolution (deterministic, no consensus required):
/// 1. A claim is valid only if `verify()` passes.
/// 2. If two different pubkeys claim the same name, the one with the **lower
///    `claimed_at`** timestamp wins (first-claim semantics).
/// 3. Ties in `claimed_at` are broken by lexicographic ordering of `pubkey`
///    (lower pubkey hex wins) — fully deterministic.
/// 4. The owner of a name (same pubkey) may update it by incrementing
///    `sequence`; updates with equal or lower sequence are silently ignored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NameClaim {
    /// Human-readable name, e.g. `"chiko"` or `"chiko@forum"`.
    pub name: String,
    /// Hex-encoded ed25519 public key — the permanent owner identity.
    pub pubkey: String,
    /// Monotonically increasing counter; higher value supersedes lower for the
    /// *same* pubkey.
    pub sequence: u64,
    /// Unix timestamp (seconds) when the claim was first created.
    /// Never changes across sequence updates.
    pub claimed_at: u64,
    /// ed25519 signature over [`NameClaim::canonical_payload`].
    pub signature: Vec<u8>,
}

impl NameClaim {
    /// Create and sign a brand-new name claim.
    pub fn new(signing_key: &SigningKey, name: &str) -> Result<Self> {
        validate_name(name)?;

        let pubkey = hex::encode(signing_key.verifying_key().to_bytes());
        let claimed_at = now_secs();

        let mut claim = NameClaim {
            name: name.to_string(),
            pubkey,
            sequence: 1,
            claimed_at,
            signature: vec![],
        };

        let sig: Signature = signing_key.sign(&claim.canonical_payload());
        claim.signature = sig.to_bytes().to_vec();

        Ok(claim)
    }

    /// Produce a new claim updating an existing one (increments sequence).
    ///
    /// The `claimed_at` timestamp is preserved so that first-claim precedence
    /// is maintained even after updates.
    pub fn update(mut self, signing_key: &SigningKey) -> Result<Self> {
        let expected_pubkey = hex::encode(signing_key.verifying_key().to_bytes());
        if self.pubkey != expected_pubkey {
            return Err(anyhow!("signing key does not match claim owner"));
        }

        self.sequence += 1;
        self.signature = vec![];
        let sig: Signature = signing_key.sign(&self.canonical_payload());
        self.signature = sig.to_bytes().to_vec();

        Ok(self)
    }

    /// Verify the ed25519 signature.
    pub fn verify(&self) -> Result<()> {
        let pubkey_bytes: [u8; 32] = hex::decode(&self.pubkey)?
            .try_into()
            .map_err(|_| anyhow!("invalid pubkey length"))?;

        let verifying_key =
            VerifyingKey::from_bytes(&pubkey_bytes).map_err(|e| anyhow!("bad pubkey: {e}"))?;

        let sig_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid signature length"))?;

        let signature = Signature::from_bytes(&sig_bytes);

        verifying_key
            .verify(&self.canonical_payload(), &signature)
            .map_err(|e| anyhow!("invalid name claim signature: {e}"))
    }

    /// Deterministic bytes over which the signature is computed.
    ///
    /// The `signature` field itself is excluded; all other fields are included
    /// so a tampered claim will fail verification.
    pub fn canonical_payload(&self) -> Vec<u8> {
        format!(
            "sisi-name:{}:{}:{}:{}",
            self.name, self.pubkey, self.sequence, self.claimed_at,
        )
        .into_bytes()
    }

    /// Returns the `scope` part of the name (the part after `@`), if any.
    ///
    /// `"chiko@forum"` → `Some("forum")`
    /// `"chiko"`       → `None`
    pub fn scope(&self) -> Option<&str> {
        parse_scope(&self.name)
    }

    /// Returns the local part of the name (the part before `@`).
    ///
    /// `"chiko@forum"` → `"chiko"`
    /// `"chiko"`       → `"chiko"`
    pub fn local(&self) -> &str {
        self.name
            .split_once('@')
            .map(|(local, _)| local)
            .unwrap_or(&self.name)
    }

    /// Determine the "winner" between two conflicting claims for the same name
    /// from *different* pubkeys, using the deterministic first-claim rule.
    ///
    /// Returns `true` when `self` should beat `other`.
    pub fn beats(&self, other: &NameClaim) -> bool {
        debug_assert_eq!(self.name, other.name);

        // Earlier timestamp wins.
        match self.claimed_at.cmp(&other.claimed_at) {
            std::cmp::Ordering::Less => return true,
            std::cmp::Ordering::Greater => return false,
            std::cmp::Ordering::Equal => {}
        }

        // Tie-break: lower pubkey hex wins (lexicographic, deterministic).
        self.pubkey < other.pubkey
    }
}

/// Result of attempting to store a [`NameClaim`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpsertResult {
    /// The name was not previously claimed; claim inserted.
    Inserted,
    /// The same pubkey provided a higher-sequence update; claim updated.
    Updated,
    /// A different pubkey already owns this name and their claim is older
    /// (or tied and lexicographically prior); claim rejected.
    Rejected { owner: String },
    /// The incoming claim had a lower-or-equal sequence than what is stored
    /// for the same pubkey; treated as a duplicate / replay, silently ignored.
    Stale,
}

/// Extract the scope component from a name string.
pub fn parse_scope(name: &str) -> Option<&str> {
    name.split_once('@').map(|(_, scope)| scope)
}

/// Validate a name string, returning `Err` with a descriptive message on
/// failure.
///
/// Rules:
/// - Total length ≤ `MAX_NAME_LEN` bytes.
/// - Must not be empty.
/// - Contains at most one `@`.
/// - Both the local part and scope (if present) must be non-empty and consist
///   only of `[a-z0-9_-]`.
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("name must not be empty"));
    }
    if name.len() > MAX_NAME_LEN {
        return Err(anyhow!(
            "name too long ({} bytes, max {MAX_NAME_LEN})",
            name.len()
        ));
    }

    let (local, scope) = match name.splitn(3, '@').collect::<Vec<_>>().as_slice() {
        [local] => (*local, None),
        [local, scope] => (*local, Some(*scope)),
        _ => return Err(anyhow!("name must contain at most one '@'")),
    };

    validate_segment(local, "local")?;
    if let Some(s) = scope {
        validate_segment(s, "scope")?;
    }

    Ok(())
}

fn validate_segment(s: &str, label: &str) -> Result<()> {
    if s.is_empty() {
        return Err(anyhow!("{label} part must not be empty"));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(anyhow!(
            "{label} part '{s}' contains invalid characters (only [a-z0-9_-] allowed)"
        ));
    }
    Ok(())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_secs()
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
    fn validate_name_should_accept_simple_local() {
        assert!(validate_name("chiko").is_ok());
    }

    #[test]
    fn validate_name_should_accept_scoped_name() {
        assert!(validate_name("chiko@forum").is_ok());
    }

    #[test]
    fn validate_name_should_reject_uppercase() {
        assert!(validate_name("Chiko@Forum").is_err());
    }

    #[test]
    fn validate_name_should_reject_double_at() {
        assert!(validate_name("a@b@c").is_err());
    }

    #[test]
    fn validate_name_should_reject_empty_local() {
        assert!(validate_name("@forum").is_err());
    }

    #[test]
    fn validate_name_should_reject_empty_scope() {
        assert!(validate_name("chiko@").is_err());
    }

    #[test]
    fn claim_new_should_produce_valid_signature() {
        let key = random_key();
        let claim = NameClaim::new(&key, "chiko@forum").unwrap();
        assert!(claim.verify().is_ok());
    }

    #[test]
    fn claim_update_should_increment_sequence() {
        let key = random_key();
        let claim = NameClaim::new(&key, "chiko").unwrap();
        let updated = claim.clone().update(&key).unwrap();
        assert_eq!(updated.sequence, claim.sequence + 1);
        assert!(updated.verify().is_ok());
    }

    #[test]
    fn beats_should_prefer_earlier_timestamp() {
        let key_a = random_key();
        let key_b = random_key();

        let mut old_claim = NameClaim::new(&key_a, "bob@dev").unwrap();
        old_claim.claimed_at = 1000;
        // Re-sign with updated claimed_at.
        old_claim.signature = vec![];
        let sig = key_a.sign(&old_claim.canonical_payload());
        old_claim.signature = sig.to_bytes().to_vec();

        let mut new_claim = NameClaim::new(&key_b, "bob@dev").unwrap();
        new_claim.claimed_at = 2000;
        new_claim.signature = vec![];
        let sig = key_b.sign(&new_claim.canonical_payload());
        new_claim.signature = sig.to_bytes().to_vec();

        assert!(old_claim.beats(&new_claim));
        assert!(!new_claim.beats(&old_claim));
    }

    #[test]
    fn scope_and_local_helpers_work() {
        let key = random_key();
        let claim = NameClaim::new(&key, "chiko@forum").unwrap();
        assert_eq!(claim.local(), "chiko");
        assert_eq!(claim.scope(), Some("forum"));

        let claim2 = NameClaim::new(&key, "chiko").unwrap();
        assert_eq!(claim2.local(), "chiko");
        assert_eq!(claim2.scope(), None);
    }
}
