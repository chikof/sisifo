use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStats {
    pub node_id: String,
    pub peer_count: usize,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub hosted_sites: usize,
    pub relay_url: Option<String>,
    pub is_online: bool,
}
