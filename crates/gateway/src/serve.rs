use crate::routes::{health, serve_blob, serve_site_file};
use axum::Router;
use tracing::info;

pub async fn start_gateway(port: u16) {
    let app = Router::new()
        .route("/health", axum::routing::get(health))
        .route("/blob/:hash", axum::routing::get(serve_blob))
        .route("/site/:hash/", axum::routing::get(serve_site_file))
        .route("/site/:hash/*path", axum::routing::get(serve_site_file));

    let addr = format!("127.0.0.1:{}", port);
    info!("gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
