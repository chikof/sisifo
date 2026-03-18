use crate::lifecycle::SisiNode;
use ed25519_dalek::SigningKey;
use std::path::Path;

const KEY_FILE: &str = "identity.key";

pub async fn load_or_create_signing_key(data_dir: &Path) -> anyhow::Result<SigningKey> {
    let key_path = data_dir.join(KEY_FILE);

    if key_path.exists() {
        let bytes = tokio::fs::read(&key_path).await?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("corrupt identity.key — delete it to regenerate"))?;
        Ok(SigningKey::from_bytes(&bytes))
    } else {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).map_err(|e| anyhow::anyhow!("getrandom failed: {}", e))?;

        let key = SigningKey::from_bytes(&seed);
        tokio::fs::create_dir_all(data_dir).await?;
        tokio::fs::write(&key_path, key.to_bytes()).await?;
        tracing::info!("generated new node identity at {:?}", key_path);
        Ok(key)
    }
}

pub async fn signing_key() -> anyhow::Result<SigningKey> {
    let handle = SisiNode::get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    load_or_create_signing_key(&handle.data_dir).await
}
