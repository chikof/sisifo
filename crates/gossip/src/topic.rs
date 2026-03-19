use iroh_gossip::proto::TopicId;
use sha2::{Digest, Sha256};

/// Derive a TopicId from a human-readable topic name.
/// "general", "rust-help", "announcements" all become valid topic IDs.
pub fn topic_id(name: &str) -> TopicId {
    let hash = Sha256::digest(format!("sisi-gossip:{}", name).as_bytes());
    TopicId::from_bytes(*hash.as_ref())
}

/// Subscription handle - keeps track of active topics
pub struct TopicHandle {
    pub name: String,
    pub id: TopicId,
}

impl TopicHandle {
    pub fn new(name: &str) -> Self {
        TopicHandle {
            name: name.to_string(),
            id: topic_id(name),
        }
    }
}
