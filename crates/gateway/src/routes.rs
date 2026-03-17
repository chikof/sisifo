use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use iroh_blobs::Hash;
use node::SisiNode;
use types::SiteManifest;

pub async fn health() -> &'static str {
    "ok"
}

pub async fn serve_blob(Path(hash): Path<String>) -> Response {
    let Ok(hash) = hash.parse::<Hash>() else {
        return (StatusCode::BAD_REQUEST, "invalid hash").into_response();
    };

    let Ok(handle) = SisiNode::get() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "node not ready").into_response();
    };

    match handle.client.blobs().read_to_bytes(hash).await {
        Ok(bytes) => {
            let mime = tree_magic_mini::from_u8(&bytes);
            ([(header::CONTENT_TYPE, mime)], bytes.to_vec()).into_response()
        }
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

pub async fn serve_site_file(Path((site_hash, file_path)): Path<(String, String)>) -> Response {
    let Ok(manifest_hash) = site_hash.parse::<Hash>() else {
        return (StatusCode::BAD_REQUEST, "invalid site hash").into_response();
    };

    let Ok(handle) = SisiNode::get() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "node not ready").into_response();
    };

    let manifest_bytes = match handle.client.blobs().read_to_bytes(manifest_hash).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::NOT_FOUND, "manifest not found").into_response(),
    };

    let manifest: SiteManifest = match serde_json::from_slice(&manifest_bytes) {
        Ok(m) => m,
        Err(_) => return (StatusCode::BAD_REQUEST, "corrupt manifest").into_response(),
    };

    let lookup = format!("/{}", file_path.trim_start_matches('/'));

    let Some(file) = manifest.files.get(&lookup) else {
        return (StatusCode::NOT_FOUND, "file not in manifest").into_response();
    };

    match handle.client.blobs().read_to_bytes(file.hash).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, file.mime.clone())], bytes.to_vec()).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "blob not found").into_response(),
    }
}
