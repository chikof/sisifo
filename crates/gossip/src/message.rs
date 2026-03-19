use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageKind {
    Post,
    Reply,
    Reaction,
    Delete,

    ModBlock,
    ModUnblock,
    ModPin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    /// Unique sortable ID (ULID) - also encodes creation time
    pub id: String,
    /// ed25519 pubkey hex of the author
    pub author: String,
    /// Topic this message belongs to
    pub topic: String,
    pub kind: MessageKind,
    pub content: String,
    /// ULID of parent message (for replies)
    pub parent_id: Option<String>,
    pub created_at: u64,
    /// Signs over canonical_payload()
    pub signature: Vec<u8>,
}

impl GossipMessage {
    pub fn new(
        signing_key: &SigningKey,
        topic: &str,
        kind: MessageKind,
        content: &str,
        parent_id: Option<String>,
    ) -> Result<Self> {
        let id = ulid::Ulid::new().to_string();
        let author = hex::encode(signing_key.verifying_key().to_bytes());
        let created_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let mut msg = GossipMessage {
            id,
            author,
            topic: topic.to_string(),
            kind,
            content: content.to_string(),
            parent_id,
            created_at,
            signature: vec![],
        };

        let payload = msg.canonical_payload();
        let sig: Signature = signing_key.sign(&payload);
        msg.signature = sig.to_bytes().to_vec();

        Ok(msg)
    }

    pub fn verify(&self) -> Result<()> {
        let pubkey_bytes: [u8; 32] = hex::decode(&self.author)?
            .try_into()
            .map_err(|_| anyhow!("invalid pubkey"))?;

        let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)?;

        let sig_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid signature length"))?;

        let signature = Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify(&self.canonical_payload(), &signature)
            .map_err(|e| anyhow!("invalid message signature: {}", e))
    }

    /// Deterministic bytes to sign - excludes signature field
    fn canonical_payload(&self) -> Vec<u8> {
        format!(
            "{}:{}:{}:{:?}:{}:{}",
            self.id, self.author, self.topic, self.kind, self.content, self.created_at,
        )
        .into_bytes()
    }
}
