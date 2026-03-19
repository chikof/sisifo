use gossip::{GossipMessage, MessageKind, MessageStore, get_gossip};
use node::signing_key;
use publisher::{list_local_sites as publisher_list_local_sites, publish_dir};
use resolver::resolve_to_gateway_url;
use sisi_daemon::DaemonClient;
use sisi_daemon::ipc::DaemonCommand;
use std::path::PathBuf;
use tauri::Emitter;
use tracing::{info, warn};
use types::{NodeStats, SiteMeta};

#[tauri::command]
pub async fn publish_site(path: String, name: String) -> Result<String, String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    let result = publish_dir(&PathBuf::from(path), &name, &key)
        .await
        .map_err(|e| e.to_string())?;

    let hash = result.site_hash.to_string();

    match DaemonClient::connect().await {
        Some(mut client) => {
            client
                .send(DaemonCommand::Pin { hash: hash.clone() })
                .await
                .map_err(|e| e.to_string())?;
            info!("handed seeding of {} to sisid", hash);
        }
        None => {
            warn!("sisid not running, seeding only while app is open");
        }
    }

    Ok(hash)
}

#[tauri::command]
pub async fn resolve_address(addr: String) -> Result<String, String> {
    resolve_to_gateway_url(&addr)
        .await
        .map_err(|e| e.to_string())
}

// Renamed to avoid recursion — publisher::list_local_sites is aliased above
#[tauri::command]
pub async fn list_local_sites() -> Result<Vec<SiteMeta>, String> {
    publisher_list_local_sites()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn node_stats() -> Result<NodeStats, String> {
    node::collect_stats().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn node_identity() -> Result<String, String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
    Ok(hex::encode(key.verifying_key().to_bytes()))
}

#[tauri::command]
pub async fn daemon_running() -> bool {
    DaemonClient::is_running()
}

#[tauri::command]
pub async fn unpin_site(hash: String) -> Result<(), String> {
    match DaemonClient::connect().await {
        Some(mut client) => client
            .send(DaemonCommand::Unpin { hash })
            .await
            .map(|_| ())
            .map_err(|e| e.to_string()),
        None => Err("sisid not running".into()),
    }
}

#[tauri::command]
pub async fn pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let folder = app
        .dialog()
        .file()
        .set_title("Select site folder")
        .blocking_pick_folder();

    Ok(folder.map(|p| p.to_string()))
}

/// Post a new top-level message to a topic
#[tauri::command]
pub async fn post_message(topic: String, content: String) -> Result<GossipMessage, String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
    let msg = GossipMessage::new(&key, &topic, MessageKind::Post, &content, None)
        .map_err(|e| e.to_string())?;

    get_gossip()
        .map_err(|e| e.to_string())?
        .broadcast(&topic, &msg)
        .await
        .map_err(|e| e.to_string())?;

    Ok(msg)
}

/// Reply to a message
#[tauri::command]
pub async fn reply_message(
    topic: String,
    content: String,
    parent_id: String,
) -> Result<GossipMessage, String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
    let msg = GossipMessage::new(&key, &topic, MessageKind::Reply, &content, Some(parent_id))
        .map_err(|e| e.to_string())?;

    get_gossip()
        .map_err(|e| e.to_string())?
        .broadcast(&topic, &msg)
        .await
        .map_err(|e| e.to_string())?;

    Ok(msg)
}

/// List cached messages for a topic
#[tauri::command]
pub async fn list_messages(
    topic: String,
    limit: usize,
    offset: usize,
) -> Result<Vec<GossipMessage>, String> {
    let store = MessageStore::open().map_err(|e| e.to_string())?;
    store
        .list_topic(&topic, limit, offset)
        .map_err(|e| e.to_string())
}

/// List replies to a specific message
#[tauri::command]
pub async fn list_replies(parent_id: String) -> Result<Vec<GossipMessage>, String> {
    let store = MessageStore::open().map_err(|e| e.to_string())?;
    store.list_replies(&parent_id).map_err(|e| e.to_string())
}

/// Subscribe to live messages — emits a Tauri event for each new message
#[tauri::command]
pub async fn subscribe_topic(topic: String, app: tauri::AppHandle) -> Result<(), String> {
    let gossip = get_gossip().map_err(|e| e.to_string())?;
    let mut rx = gossip.subscribe(&topic).await.map_err(|e| e.to_string())?;
    let event_name = format!("gossip:{}", topic);

    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            app.emit(&event_name, &msg).ok();
        }
    });

    Ok(())
}

/// Delete your own message (broadcasts a tombstone)
#[tauri::command]
pub async fn delete_message(topic: String, message_id: String) -> Result<(), String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    // Only the author can delete — verified by signature on the tombstone
    let tombstone = GossipMessage::new(&key, &topic, MessageKind::Delete, &message_id, None)
        .map_err(|e| e.to_string())?;

    get_gossip()
        .map_err(|e| e.to_string())?
        .broadcast(&topic, &tombstone)
        .await
        .map_err(|e| e.to_string())?;

    // Remove from local store
    MessageStore::open()
        .map_err(|e| e.to_string())?
        .delete(&message_id)
        .map_err(|e| e.to_string())?;

    Ok(())
}
