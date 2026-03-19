use crate::{MessageKind, message::GossipMessage, store::MessageStore, topic::topic_id};
use anyhow::{Result, anyhow};
use iroh_gossip::Gossip;
use node::SisiNode;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::{OnceCell, broadcast};
use tokio_stream::StreamExt;

static GOSSIP: OnceCell<Arc<GossipHandle>> = OnceCell::const_new();

pub struct GossipHandle {
    gossip: Gossip,
    /// Per-topic broadcast channels - frontend subscribes to these
    senders: Mutex<HashMap<String, broadcast::Sender<GossipMessage>>>,
}

pub async fn init_gossip() -> Result<()> {
    if GOSSIP.initialized() {
        return Ok(());
    }

    let handle = SisiNode::get().map_err(|e| anyhow!(e.to_string()))?;
    let gossip = Gossip::builder().spawn(handle.endpoint.clone());

    GOSSIP
        .set(Arc::new(GossipHandle {
            gossip,
            senders: Mutex::new(HashMap::new()),
        }))
        .map_err(|_| anyhow!("gossip already initialized"))?;

    tracing::info!("sisi-gossip initialized");
    Ok(())
}

pub fn get_gossip() -> Result<Arc<GossipHandle>> {
    GOSSIP
        .get()
        .cloned()
        .ok_or_else(|| anyhow!("gossip not initialized — call init_gossip() first"))
}

impl GossipHandle {
    pub async fn subscribe(&self, topic_name: &str) -> Result<broadcast::Receiver<GossipMessage>> {
        let tid = topic_id(topic_name);

        let rx = {
            let mut senders = self.senders.lock().unwrap();
            let tx = senders
                .entry(topic_name.to_string())
                .or_insert_with(|| broadcast::channel(256).0);
            tx.subscribe()
        };

        let gossip = self.gossip.clone();
        let topic_name = topic_name.to_string();
        let senders = Arc::new(self.senders.lock().unwrap().clone());

        tokio::spawn(async move {
            if let Ok(mut topic) = gossip.subscribe(tid, vec![]).await {
                tracing::info!("subscribed to topic '{}'", topic_name);

                while let Some(Ok(event)) = topic.next().await {
                    use iroh_gossip::api::Event;

                    if let Event::Received(recv) = event {
                        match serde_json::from_slice::<GossipMessage>(&recv.content) {
                            Ok(msg) => {
                                // Verify signature before accepting
                                if msg.verify().is_err() {
                                    tracing::warn!("dropping message with invalid signature");
                                    continue;
                                }

                                if let Ok(store) = MessageStore::open() {
                                    store.insert(&msg).ok();
                                }

                                if let Some(tx) = senders.get(&topic_name) {
                                    tx.send(msg).ok();
                                }
                            }
                            Err(e) => tracing::debug!("malformed gossip message: {}", e),
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Broadcast a message to all peers in the topic
    pub async fn broadcast(&self, topic_name: &str, msg: &GossipMessage) -> Result<()> {
        let tid = topic_id(topic_name);
        let bytes = serde_json::to_vec(msg)?;

        // Store locally first
        let store = MessageStore::open()?;
        store.insert(msg)?;

        let mut sender = self.gossip.subscribe(tid, vec![]).await?;
        sender.broadcast(bytes.into()).await?;

        let senders = self.senders.lock().unwrap();
        if let Some(tx) = senders.get(topic_name) {
            tx.send(msg.clone()).ok();
        }

        tracing::debug!("broadcast to topic '{}': {}", topic_name, msg.id);
        Ok(())
    }
}
