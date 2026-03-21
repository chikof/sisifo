use gossip::{GossipMessage, MessageKind, MessageStore, PersonalBlockList, get_gossip};
use index::{IndexStore, SiteRecord, crawl_site};
use names::gossip_handler::{broadcast_name_claim, broadcast_name_release};
use names::{NameClaim, NameStore, handle_name_gossip_message};
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
    /// The scope the site was published under, if any.
    pub scope: Option<String>,
    /// The human-readable name that was claimed, if any.
    pub claimed_name: Option<String>,
}

/// Publish a new site from a local folder.
#[tauri::command]
pub async fn publish_site(
    path: String,
    name: String,
    scope: Option<String>,
    human_name: Option<String>,
) -> Result<PublishResponse, String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    let result = publish_dir(
        &PathBuf::from(path),
        &name,
        &key,
        scope.as_deref(),
        human_name.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    let hash = result.site_hash.to_string();

    if let Some(ref claim) = result.name_claim
        && let Ok(gossip) = get_gossip()
    {
        broadcast_name_claim(&gossip, &key, claim)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(mut client) = DaemonClient::connect().await {
        client
            .send(DaemonCommand::Pin { hash: hash.clone() })
            .await
            .ok();
    }

    Ok(PublishResponse {
        hash,
        permanent_address: result.permanent_address,
        version: result.meta.version,
        scope: result.scope,
        claimed_name: result.name_claim.map(|c| c.name),
    })
}

/// Re-publish from a folder - increments version, same permanent address.
#[tauri::command]
pub async fn update_site(
    path: String,
    name: String,
    scope: Option<String>,
    human_name: Option<String>,
) -> Result<PublishResponse, String> {
    publish_site(path, name, scope, human_name).await
}

/// Claim a human-readable name and broadcast it to peers.
///
/// `name` must be of the form `local` or `local@scope`, e.g. `"chiko@forum"`.
#[tauri::command]
pub async fn claim_name(name: String) -> Result<NameClaim, String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    let claim = NameClaim::new(&key, &name).map_err(|e| e.to_string())?;

    let store = NameStore::open().map_err(|e| e.to_string())?;
    let result = store.upsert(&claim).map_err(|e| e.to_string())?;

    use names::UpsertResult;
    match result {
        UpsertResult::Inserted | UpsertResult::Updated => {}
        UpsertResult::Rejected { owner } => {
            return Err(format!("name '{name}' is already owned by {owner}"));
        }
        UpsertResult::Stale => {
            return Err(format!("your claim for '{name}' is already up to date"));
        }
    }

    // Broadcast so peers learn about the claim.
    if let Ok(gossip) = get_gossip() {
        broadcast_name_claim(&gossip, &key, &claim)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(claim)
}

/// Relinquish ownership of a name and tell peers.
#[tauri::command]
pub async fn release_name(name: String) -> Result<(), String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    let store = NameStore::open().map_err(|e| e.to_string())?;
    store.delete(&name).map_err(|e| e.to_string())?;

    if let Ok(gossip) = get_gossip() {
        broadcast_name_release(&gossip, &key, &name)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Resolve a human-readable name to a pubkey (or `null` if unknown).
#[tauri::command]
pub async fn resolve_name(name: String) -> Result<Option<String>, String> {
    let store = NameStore::open().map_err(|e| e.to_string())?;
    store.resolve(&name).map_err(|e| e.to_string())
}

/// All names claimed by the current node identity.
#[tauri::command]
pub async fn my_names() -> Result<Vec<NameClaim>, String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
    let pubkey = hex::encode(key.verifying_key().to_bytes());
    let store = NameStore::open().map_err(|e| e.to_string())?;
    store.names_for_pubkey(&pubkey).map_err(|e| e.to_string())
}

/// All name claims whose scope matches the given topic (e.g. `"forum"`).
#[tauri::command]
pub async fn names_in_scope(scope: String) -> Result<Vec<NameClaim>, String> {
    let store = NameStore::open().map_err(|e| e.to_string())?;
    store.claims_in_scope(&scope).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resolve_address(addr: String) -> Result<String, String> {
    resolve_to_gateway_url(&addr)
        .await
        .map_err(|e| e.to_string())
}

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

#[tauri::command]
pub async fn list_replies(parent_id: String) -> Result<Vec<GossipMessage>, String> {
    let store = MessageStore::open().map_err(|e| e.to_string())?;
    store.list_replies(&parent_id).map_err(|e| e.to_string())
}

/// Subscribe to live messages — emits a Tauri event for each new message.
///
/// Name-related messages (NameClaim, NameRelease) are also persisted to the
/// local name store before being forwarded to the frontend.
#[tauri::command]
pub async fn subscribe_topic(topic: String, app: tauri::AppHandle) -> Result<(), String> {
    let gossip = get_gossip().map_err(|e| e.to_string())?;
    let mut rx = gossip.subscribe(&topic).await.map_err(|e| e.to_string())?;
    let event_name = format!("gossip:{}", topic);

    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // Handle name-related messages transparently.
            let is_name_msg = handle_name_gossip_message(&msg).unwrap_or(false);

            // Always forward to the frontend so it can react to name events
            // (e.g. display a resolved name badge on posts).
            app.emit(&event_name, &msg).ok();

            // For pure name messages we don't insert into the message store.
            if !is_name_msg && let Ok(store) = MessageStore::open() {
                store.insert(&msg).ok();
            }
        }
    });

    Ok(())
}

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

#[tauri::command]
pub async fn mod_block_user(topic: String, pubkey: String) -> Result<(), String> {
    let key = signing_key()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
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

#[tauri::command]
pub async fn search_sites(query: String, limit: usize) -> Result<Vec<SiteRecord>, String> {
    let store = IndexStore::open().map_err(|e| e.to_string())?;
    store.search(&query, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn recent_sites(limit: usize) -> Result<Vec<SiteRecord>, String> {
    let store = IndexStore::open().map_err(|e| e.to_string())?;
    store.recent(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn index_count() -> Result<usize, String> {
    let store = IndexStore::open().map_err(|e| e.to_string())?;
    store.count().map_err(|e| e.to_string())
}
