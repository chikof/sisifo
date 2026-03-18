use crate::routes::{health, serve_blob, serve_site_file};
use axum::Router;
use tracing::{error, info};

pub async fn start_gateway(port: u16) {
    let addr = format!("127.0.0.1:{}", port);

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("gateway failed to bind on {} — {}", addr, e);
            error!(
                "is port {} already in use? check with: ss -tlnp | grep {}",
                port, port
            );
            return;
        }
    };

    info!("gateway listening on {}", addr);
    let app = Router::new()
        .route("/health", axum::routing::get(health))
        .route("/blob/{hash}", axum::routing::get(serve_blob))
        .route("/site/{hash}/", axum::routing::get(serve_site_file))
        .route("/site/{hash}/{*path}", axum::routing::get(serve_site_file));

    axum::serve(listener, app).await.unwrap();
}
