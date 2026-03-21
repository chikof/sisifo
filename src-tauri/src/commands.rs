use gossip::{GossipMessage, MessageKind, MessageStore, PersonalBlockList, get_gossip};
use index::{IndexStore, SiteRecord, crawl_site};
use node::signing_key;
use publisher::{list_local_sites as publisher_list_local_sites, publish_dir};
use resolver::resolve_to_gateway_url;
use serde::Serialize;
use sisi_daemon::DaemonClient;
use sisi_daemon::ipc::DaemonCommand;
use std::path::PathBuf;
use tauri::Emitter;
use types::{NodeStats, SiteMeta};

const GATEWAY_BASE: &str = "http://127.0.0.1:7777";

#[derive(Serialize)]
pub struct PublishResponse {
    pub hash: String,
    pub permanent_address: String,
    pub version: u32,
}

#[tauri::command]
pub async fn publish_site(path: String, name: String) -> Result<PublishResponse, String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    let result = publish_dir(&PathBuf::from(path), &name, &key)
        .await
        .map_err(|e| e.to_string())?;

    let hash = result.site_hash.to_string();
    let permanent_address = hex::encode(key.verifying_key().to_bytes());
    let version = result.meta.version;

    if let Some(mut client) = DaemonClient::connect().await {
        client
            .send(DaemonCommand::Pin { hash: hash.clone() })
            .await
            .ok();
    }

    Ok(PublishResponse {
        hash,
        permanent_address,
        version,
    })
}

/// Republish an existing site from a folder — increments version, same permanent address
#[tauri::command]
pub async fn update_site(path: String, name: String) -> Result<PublishResponse, String> {
    publish_site(path, name).await
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
    publisher::remove_site(&hash)
        .await
        .map_err(|e| e.to_string())?;

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
pub async fn post_message(
    app: tauri::AppHandle,
    topic: String,
    content: String,
) -> Result<GossipMessage, String> {
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

    app.emit(&format!("gossip:{}", topic), &msg)
        .map_err(|e| e.to_string())?;

    Ok(msg)
}

/// Reply to a message
#[tauri::command]
pub async fn reply_message(
    app: tauri::AppHandle,
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

    app.emit(&format!("gossip:{}", topic), &msg)
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
pub async fn delete_message(
    app: tauri::AppHandle,
    topic: String,
    message_id: String,
) -> Result<(), String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    let tombstone = GossipMessage::new(&key, &topic, MessageKind::Delete, &message_id, None)
        .map_err(|e| e.to_string())?;

    get_gossip()
        .map_err(|e| e.to_string())?
        .broadcast(&topic, &tombstone)
        .await
        .map_err(|e| e.to_string())?;

    MessageStore::open()
        .map_err(|e| e.to_string())?
        .delete(&message_id)
        .map_err(|e| e.to_string())?;

    app.emit(&format!("gossip:{}", topic), &tombstone)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn block_user(pubkey: String) -> Result<(), String> {
    let mut list = PersonalBlockList::load().await.map_err(|e| e.to_string())?;
    list.block(&pubkey).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unblock_user(pubkey: String) -> Result<(), String> {
    let mut list = PersonalBlockList::load().await.map_err(|e| e.to_string())?;
    list.unblock(&pubkey).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_blocked_users() -> Result<Vec<String>, String> {
    let list = PersonalBlockList::load().await.map_err(|e| e.to_string())?;
    Ok(list.blocked.into_iter().collect())
}

/// Forum owner: block a user from a topic (broadcasts signed modlist)
#[tauri::command]
pub async fn mod_block_user(topic: String, pubkey: String) -> Result<(), String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
    // Load or create modlist for this topic
    // In production: store modlist in gossip store, broadcast update
    // For now: local modlist broadcast as a ModBlock message
    let msg = GossipMessage::new(&key, &topic, MessageKind::ModBlock, &pubkey, None)
        .map_err(|e| e.to_string())?;
    get_gossip()
        .map_err(|e| e.to_string())?
        .broadcast(&topic, &msg)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn index_site(hash: String) -> Result<Option<SiteRecord>, String> {
    crawl_site(&hash, GATEWAY_BASE)
        .await
        .map_err(|e| e.to_string())
}

/// Full-text search across indexed sites
#[tauri::command]
pub async fn search_sites(query: String, limit: usize) -> Result<Vec<SiteRecord>, String> {
    let store = IndexStore::open().map_err(|e| e.to_string())?;
    store.search(&query, limit).map_err(|e| e.to_string())
}

/// Recently visited / most popular sites
#[tauri::command]
pub async fn recent_sites(limit: usize) -> Result<Vec<SiteRecord>, String> {
    let store = IndexStore::open().map_err(|e| e.to_string())?;
    store.recent(limit).map_err(|e| e.to_string())
}

/// Count indexed sites
#[tauri::command]
pub async fn index_count() -> Result<usize, String> {
    let store = IndexStore::open().map_err(|e| e.to_string())?;
    store.count().map_err(|e| e.to_string())
}
