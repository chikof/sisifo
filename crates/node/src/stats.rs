use iroh::Endpoint;
use iroh_metrics::{MetricValue, MetricsGroupSet};
use std::{path::Path, sync::atomic::Ordering};

use super::lifecycle::SisiNode;
use types::{NodeStats, Result};

pub async fn collect_stats() -> Result<NodeStats> {
    let handle = SisiNode::get()?;
    let node_id = handle.endpoint.id().to_string();

    let peer_count = handle.peer_count.load(Ordering::Relaxed);
    let (bytes_sent, bytes_recv) = collect_byte_metric(&handle.endpoint);
    let (relay_url, is_online) = collect_connectivity(&handle.endpoint).await;
    let hosted_sites = count_hosted_sites(&handle.data_dir).await;

    Ok(NodeStats {
        node_id,
        peer_count,
        bytes_sent,
        bytes_recv,
        hosted_sites,
        relay_url,
        is_online,
    })
}

fn collect_byte_metric(endpoint: &Endpoint) -> (u64, u64) {
    let mut sent = 0u64;
    let mut recv = 0u64;

    for (group, metric) in endpoint.metrics().iter() {
        let name = format!("{group}:{}", metric.name());
        let value = match metric.value() {
            MetricValue::Counter(v) => v,
            MetricValue::Gauge(v) => v as u64,
            _ => todo!(),
        };

        match name.as_str() {
            "socket:send_bytes" => sent = value,
            "socket:recv_bytes" => recv = value,

            "socket:send_datagrams" if sent == 0 => sent = value,
            "socket:recv_datagrams" if recv == 0 => recv = value,

            _ => {}
        }
    }

    (sent, recv)
}

async fn collect_connectivity(endpoint: &Endpoint) -> (Option<String>, bool) {
    let addr = endpoint.addr();

    let relay_url = addr.relay_urls().next().map(|u| u.to_string());
    let has_direct = addr.ip_addrs().next().is_some();

    let is_online = relay_url.is_some() || has_direct;

    (relay_url, is_online)
}

async fn count_hosted_sites(data_dir: &Path) -> usize {
    let index_path = data_dir.join("published").join("index.json");
    if !index_path.exists() {
        return 0;
    }

    #[derive(serde::Deserialize)]
    struct Index {
        sites: Vec<serde_json::Value>,
    }

    tokio::fs::read(&index_path)
        .await
        .ok()
        .and_then(|b| serde_json::from_slice::<Index>(&b).ok())
        .map(|i| i.sites.len())
        .unwrap_or(0)
}
