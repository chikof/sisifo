use crate::pinset::PinSet;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use types::Result;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "cmd")]
pub enum DaemonCommand {
    Pin { hash: String },
    Unpin { hash: String },
    List,
    Status,
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonResponse {
    Ok,
    Pinned {
        hash: String,
    },
    Unpinned {
        hash: String,
    },
    List {
        hashes: Vec<String>,
    },
    Status {
        peer_count: usize,
        pinned: usize,
        node_id: String,
    },
    Error {
        message: String,
    },
}

pub fn socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));
    PathBuf::from(runtime_dir).join("sisi").join("daemon.sock")
}

pub async fn serve(pin_set: PinSet) -> Result<()> {
    let path = socket_path();
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;

    // Clean up stale socket from a previous crash
    let _ = tokio::fs::remove_file(&path).await;

    let listener = UnixListener::bind(&path)?;
    tracing::info!("IPC socket at {:?}", path);

    loop {
        let (stream, _) = listener.accept().await?;
        let pin_set = pin_set.clone();
        tokio::spawn(handle_connection(stream, pin_set));
    }
}

async fn handle_connection(stream: UnixStream, pin_set: PinSet) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let response = match serde_json::from_str::<DaemonCommand>(&line) {
            Ok(cmd) => dispatch(cmd, &pin_set).await,
            Err(e) => DaemonResponse::Error {
                message: e.to_string(),
            },
        };

        let mut out = serde_json::to_string(&response).unwrap();
        out.push('\n');
        let _ = writer.write_all(out.as_bytes()).await;
    }
}

async fn dispatch(cmd: DaemonCommand, pin_set: &PinSet) -> DaemonResponse {
    match cmd {
        DaemonCommand::Pin { hash } => match pin_set.add(&hash).await {
            Ok(_) => DaemonResponse::Pinned { hash },
            Err(e) => DaemonResponse::Error {
                message: e.to_string(),
            },
        },
        DaemonCommand::Unpin { hash } => match pin_set.remove(&hash).await {
            Ok(_) => DaemonResponse::Unpinned { hash },
            Err(e) => DaemonResponse::Error {
                message: e.to_string(),
            },
        },
        DaemonCommand::List => DaemonResponse::List {
            hashes: pin_set.list().await,
        },
        DaemonCommand::Status => {
            let node = node::SisiNode::get().unwrap();
            let node_id = node.endpoint.id().to_string();
            DaemonResponse::Status {
                peer_count: 0, // TODO: wire up iroh connection count
                pinned: pin_set.list().await.len(),
                node_id,
            }
        }
        DaemonCommand::Shutdown => {
            tracing::info!("shutdown requested via IPC");
            std::process::exit(0);
        }
    }
}
