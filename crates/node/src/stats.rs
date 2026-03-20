use super::lifecycle::SisiNode;
use iroh_metrics::{MetricValue, MetricsGroupSet};
use types::{NodeStats, Result};

pub async fn collect_stats() -> Result<NodeStats> {
    let handle = SisiNode::get()?;
    let node_id = handle.endpoint.id().to_string();

    let (bytes_sent, bytes_recv, peer_count) = read_metrics(&handle.endpoint);

    let node_addr = handle.endpoint.addr();
    let relay_url = node_addr.relay_urls().next().map(|u| u.to_string());
    let is_online = relay_url.is_some();

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

fn read_metrics(endpoint: &iroh::Endpoint) -> (u64, u64, usize) {
    let mut sent = 0u64;
    let mut recv = 0u64;
    let mut conns_opened = 0u64;
    let mut conns_closed = 0u64;

    for (group, metric) in endpoint.metrics().iter() {
        let name = format!("{}/{}", group, metric.name());
        let value = match metric.value() {
            MetricValue::Counter(v) => v,
            MetricValue::Gauge(v) => v as u64,
            _ => todo!(),
        };

        match name.as_str() {
            // Bytes sent - sum all transports
            "socket/send_ipv4" => sent = sent.saturating_add(value),
            "socket/send_ipv6" => sent = sent.saturating_add(value),
            "socket/send_relay" => sent = sent.saturating_add(value),
            // Bytes received - sum all transports
            "socket/recv_data_ipv4" => recv = recv.saturating_add(value),
            "socket/recv_data_ipv6" => recv = recv.saturating_add(value),
            "socket/recv_data_relay" => recv = recv.saturating_add(value),
            // Peer count - live connections
            "socket/num_conns_opened" => conns_opened = value,
            "socket/num_conns_closed" => conns_closed = value,

            _ => {}
        }
    }

    let peer_count = conns_opened.saturating_sub(conns_closed) as usize;
    (sent, recv, peer_count)
}

async fn count_hosted_sites(data_dir: &std::path::Path) -> usize {
    let path = data_dir.join("published").join("index.json");
    if !path.exists() {
        return 0;
    }

    #[derive(serde::Deserialize)]
    struct Index {
        sites: Vec<serde_json::Value>,
    }

    tokio::fs::read(&path)
        .await
        .ok()
        .and_then(|b| serde_json::from_slice::<Index>(&b).ok())
        .map(|i| i.sites.len())
        .unwrap_or(0)
}
