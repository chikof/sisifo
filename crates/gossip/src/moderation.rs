use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use node::SisiNode;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf};

/// A signed modlist published by a forum owner.
/// Users who trust the owner's pubkey apply this list locally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModList {
    /// Forum owner's pubkey - the authority for this modlist
    pub owner_pubkey: String,
    /// Topic this modlist applies to
    pub topic: String,
    /// Blocked pubkeys
    pub blocked: HashSet<String>,
    /// Pinned message IDs
    pub pinned: Vec<String>,
    pub updated_at: u64,
    pub signature: Vec<u8>,
}

impl ModList {
    pub fn new(signing_key: &SigningKey, topic: &str) -> Result<Self> {
        let owner_pubkey = hex::encode(signing_key.verifying_key().to_bytes());
        let updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let mut list = ModList {
            owner_pubkey,
            topic: topic.to_string(),
            blocked: HashSet::new(),
            pinned: vec![],
            updated_at,
            signature: vec![],
        };

        list.sign(signing_key)?;
        Ok(list)
    }

    pub fn block(&mut self, signing_key: &SigningKey, pubkey: &str) -> Result<()> {
        self.blocked.insert(pubkey.to_string());
        self.updated_at = now();
        self.sign(signing_key)
    }

    pub fn unblock(&mut self, signing_key: &SigningKey, pubkey: &str) -> Result<()> {
        self.blocked.remove(pubkey);
        self.updated_at = now();
        self.sign(signing_key)
    }

    pub fn pin(&mut self, signing_key: &SigningKey, msg_id: &str) -> Result<()> {
        if !self.pinned.contains(&msg_id.to_string()) {
            self.pinned.push(msg_id.to_string());
        }
        self.updated_at = now();
        self.sign(signing_key)
    }

    pub fn is_blocked(&self, pubkey: &str) -> bool {
        self.blocked.contains(pubkey)
    }

    pub fn verify(&self) -> Result<()> {
        let pubkey_bytes: [u8; 32] = hex::decode(&self.owner_pubkey)?
            .try_into()
            .map_err(|_| anyhow!("invalid pubkey"))?;
        let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)?;
        let sig_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid sig"))?;
        let signature = Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify(&self.canonical_payload(), &signature)
            .map_err(|e| anyhow!("invalid modlist signature: {}", e))
    }

    fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        self.signature = vec![];
        let payload = self.canonical_payload();
        let sig: Signature = signing_key.sign(&payload);
        self.signature = sig.to_bytes().to_vec();
        Ok(())
    }

    fn canonical_payload(&self) -> Vec<u8> {
        let mut blocked: Vec<_> = self.blocked.iter().map(String::from).collect();
        blocked.sort();
        format!(
            "{}:{}:{}:{}:{}",
            self.owner_pubkey,
            self.topic,
            blocked.join(","),
            self.pinned.join(","),
            self.updated_at,
        )
        .into_bytes()
    }
}

/// Local per-user block list — users block people they don't want to see
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PersonalBlockList {
    pub blocked: HashSet<String>,
}

impl PersonalBlockList {
    pub async fn load() -> Result<Self> {
        let path = blocklist_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = tokio::fs::read(&path).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn block(&mut self, pubkey: &str) -> Result<()> {
        self.blocked.insert(pubkey.to_string());
        self.save().await
    }

    pub async fn unblock(&mut self, pubkey: &str) -> Result<()> {
        self.blocked.remove(pubkey);
        self.save().await
    }

    pub fn is_blocked(&self, pubkey: &str) -> bool {
        self.blocked.contains(pubkey)
    }

    async fn save(&self) -> Result<()> {
        let path = blocklist_path()?;
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        tokio::fs::write(&path, serde_json::to_vec_pretty(self)?).await?;
        Ok(())
    }
}

fn blocklist_path() -> Result<PathBuf> {
    let handle = SisiNode::get().map_err(|e| anyhow!(e.to_string()))?;
    Ok(handle.data_dir.join("gossip").join("blocklist.json"))
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
