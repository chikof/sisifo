use node::{NodeConfig, SisiNode, signing_key};
use sisi_daemon::{ipc, pinset::PinSet};
use std::env::var;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("sisi=debug,iroh=info")
        .init();

    let data_dir = data_dir();
    tokio::fs::create_dir_all(&data_dir).await?;

    let config = if let (Ok(relay), Ok(pkarr), Ok(dns)) = (
        var("SISI_RELAY_URL"),
        var("SISI_PKARR_URL"),
        var("SISI_DNS_ORIGIN"),
    ) {
        NodeConfig::custom(&relay, &pkarr, &dns)
    } else {
        NodeConfig::default()
    };

    tracing::info!("starting sisid at {:?}", data_dir);
    SisiNode::start(&data_dir, config).await?;

    let key = signing_key().await?;
    tracing::info!(
        "node identity: {}",
        hex::encode(key.verifying_key().to_bytes())
    );

    // Load persisted pins and re-announce them on the DHT
    let pin_set = PinSet::load(&data_dir).await?;
    announce_pins(&pin_set).await;
    ipc::serve(pin_set).await?;

    Ok(())
}

async fn announce_pins(pin_set: &PinSet) {
    let hashes = pin_set.list().await;
    if hashes.is_empty() {
        return;
    }
    tracing::info!("re-announcing {} pinned sites", hashes.len());
    for hash in &hashes {
        tracing::debug!("seeding: {}", hash);
    }
}

fn data_dir() -> std::path::PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".local").join("share"));
    base.join("sisi")
}
