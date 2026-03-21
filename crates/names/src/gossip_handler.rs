use crate::{NameClaim, store::NameStore};
use anyhow::Result;
use gossip::{GossipHandle, GossipMessage, MessageKind};
use tracing::{debug, warn};

/// Process an incoming gossip message that may carry a [`crate::claim::NameClaim`] payload.
///
/// Call this from the gossip receive loop in `src-tauri/src/commands.rs` (or
/// wherever you handle incoming gossip events) *before* forwarding the event
/// to the frontend.
///
/// Returns `Ok(true)` when the message was a name-related message and was
/// handled, `Ok(false)` when the caller should handle it normally.
pub fn handle_name_gossip_message(msg: &GossipMessage) -> Result<bool> {
    match msg.kind {
        MessageKind::NameClaim => {
            let claim: NameClaim = serde_json::from_str(&msg.content)
                .map_err(|e| anyhow::anyhow!("malformed NameClaim payload: {e}"))?;

            // Verify that the gossip author matches the claim owner — prevents
            // one peer from broadcasting name claims on behalf of another.
            if claim.pubkey != msg.author {
                warn!(
                    gossip_author = %msg.author,
                    claim_owner   = %claim.pubkey,
                    "NameClaim pubkey mismatch — dropping"
                );
                return Ok(true);
            }

            let store = NameStore::open()?;
            let result = store.upsert(&claim)?;

            debug!(
                name   = %claim.name,
                result = ?result,
                "processed incoming NameClaim"
            );

            Ok(true)
        }

        MessageKind::NameRelease => {
            // content is the name string being released.
            let name = msg.content.trim();

            let store = NameStore::open()?;
            if let Some(existing) = store.get(name)? {
                if existing.pubkey == msg.author {
                    store.delete(name)?;
                    debug!(name = %name, "name released by owner");
                } else {
                    warn!(
                        name   = %name,
                        author = %msg.author,
                        owner  = %existing.pubkey,
                        "NameRelease from non-owner — ignoring"
                    );
                }
            }

            Ok(true)
        }

        _ => Ok(false),
    }
}

/// Broadcast a [`crate::claim::NameClaim`] to all peers subscribed to the relevant topic.
///
/// If the name has a `@scope` (e.g. `chiko@forum`), the claim is broadcast on
/// the `forum` topic so it reaches all forum participants naturally. For
/// unscoped names the `"names"` meta-topic is used.
pub async fn broadcast_name_claim(
    gossip: &GossipHandle,
    signing_key: &ed25519_dalek::SigningKey,
    claim: &NameClaim,
) -> Result<()> {
    let topic = claim.scope().unwrap_or("names");
    let content = serde_json::to_string(claim)?;

    let msg = GossipMessage::new(signing_key, topic, MessageKind::NameClaim, &content, None)?;

    gossip.broadcast(topic, &msg).await?;
    Ok(())
}

/// Broadcast a name release (the owner gives up a name) to peers.
pub async fn broadcast_name_release(
    gossip: &GossipHandle,
    signing_key: &ed25519_dalek::SigningKey,
    name: &str,
) -> Result<()> {
    let topic = crate::claim::parse_scope(name).unwrap_or("names");

    let msg = GossipMessage::new(signing_key, topic, MessageKind::NameRelease, name, None)?;

    gossip.broadcast(topic, &msg).await?;
    Ok(())
}
