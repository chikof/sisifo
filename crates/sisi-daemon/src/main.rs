use node::SisiNode;
use sisi_daemon::{ipc, pinset::PinSet};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("sisi=debug,iroh=info")
        .init();

    let data_dir = data_dir();
    tokio::fs::create_dir_all(&data_dir).await?;

    tracing::info!("starting sisid at {:?}", data_dir);
    SisiNode::start(&data_dir).await?;

    let pin_set = PinSet::load(&data_dir).await?;

    announce_pins(&pin_set).await;
    ipc::serve(pin_set).await?;

    Ok(())
}

async fn announce_pins(pin_set: &PinSet) {
    let hashes = pin_set.list().await;
    tracing::info!("re-announcing {} pinned sites", hashes.len());
    // iroh re-announces blobs it has in store automatically on startup,
    // so this is mostly a no-op — but you could force-provide here
}

fn data_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".local").join("share"));
    base.join("sisi")
}
