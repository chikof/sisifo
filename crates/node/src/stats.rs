use super::lifecycle::SisiNode;
use types::{NodeStats, Result};

pub async fn collect_stats() -> Result<NodeStats> {
    let handle = SisiNode::get()?;
    let node_id = handle.endpoint.id().to_string();

    Ok(NodeStats {
        node_id,
        peer_count: 0, // TODO: handle.client.connections()
        bytes_sent: 0,
        bytes_recv: 0,
        hosted_sites: 0,
    })
}
