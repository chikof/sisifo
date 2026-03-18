use node::signing_key;
use publisher::{list_local_sites as publisher_list_local_sites, publish_dir};
use resolver::resolve_to_gateway_url;
use sisi_daemon::DaemonClient;
use sisi_daemon::ipc::DaemonCommand;
use std::path::PathBuf;
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
