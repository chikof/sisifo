use crate::claim::{NameClaim, UpsertResult};
use anyhow::{Result, anyhow};
use node::SisiNode;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Persistent store for [`NameClaim`] records, backed by SQLite.
///
/// The store enforces the first-claim ownership rule:
/// - A new name is inserted on first valid claim.
/// - The same pubkey may update by providing a higher `sequence`.
/// - A different pubkey whose claim is *older* (or ties and wins the tiebreak)
///   is rejected.
pub struct NameStore {
    conn: Connection,
}

impl NameStore {
    /// Open (or create) the name store at the node's data directory.
    pub fn open() -> Result<Self> {
        let path = db_path()?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        let conn = Connection::open(&path)?;
        let store = NameStore { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS name_claims (
                -- The full name string, e.g. 'chiko' or 'chiko@forum'.
                name        TEXT PRIMARY KEY,
                pubkey      TEXT NOT NULL,
                sequence    INTEGER NOT NULL,
                claimed_at  INTEGER NOT NULL,
                signature   BLOB NOT NULL,
                raw_json    TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_pubkey
                ON name_claims(pubkey);
            CREATE INDEX IF NOT EXISTS idx_scope
                ON name_claims(name);
            ",
        )?;
        Ok(())
    }

    /// Attempt to insert or update a name claim, enforcing ownership rules.
    ///
    /// Returns [`UpsertResult`] describing the outcome; never returns `Err`
    /// for a business-logic rejection — only for I/O or parse failures.
    pub fn upsert(&self, incoming: &NameClaim) -> Result<UpsertResult> {
        // Signature must be valid before we touch the DB.
        incoming
            .verify()
            .map_err(|e| anyhow!("invalid claim signature: {e}"))?;

        match self.get(&incoming.name)? {
            None => {
                // Name is unclaimed — insert.
                self.insert_raw(incoming)?;
                info!(
                    name = %incoming.name,
                    pubkey = %&incoming.pubkey[..8],
                    "new name claimed"
                );
                Ok(UpsertResult::Inserted)
            }

            Some(existing) if existing.pubkey == incoming.pubkey => {
                // Same owner — allow update only if sequence is strictly higher.
                if incoming.sequence > existing.sequence {
                    self.update_raw(incoming)?;
                    debug!(
                        name = %incoming.name,
                        seq = incoming.sequence,
                        "name claim updated"
                    );
                    Ok(UpsertResult::Updated)
                } else {
                    debug!(
                        name = %incoming.name,
                        "stale name claim ignored (seq {} ≤ {})",
                        incoming.sequence,
                        existing.sequence
                    );
                    Ok(UpsertResult::Stale)
                }
            }

            Some(existing) => {
                // Different pubkey — apply first-claim rule.
                if incoming.beats(&existing) {
                    // Incoming claim is older; it wins and overwrites.
                    self.update_raw(incoming)?;
                    warn!(
                        name = %incoming.name,
                        winner = %&incoming.pubkey[..8],
                        loser  = %&existing.pubkey[..8],
                        "name ownership transferred via first-claim rule"
                    );
                    Ok(UpsertResult::Updated)
                } else {
                    debug!(
                        name = %incoming.name,
                        owner = %&existing.pubkey[..8],
                        "name claim rejected — already owned"
                    );
                    Ok(UpsertResult::Rejected {
                        owner: existing.pubkey.clone(),
                    })
                }
            }
        }
    }

    /// Delete a name claim. Only useful for local cleanup; the release is
    /// propagated by broadcasting a signed `NameRelease` gossip message.
    pub fn delete(&self, name: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM name_claims WHERE name = ?1", params![name])?;
        Ok(())
    }

    /// Resolve a name to a pubkey, or `None` if not locally known.
    pub fn resolve(&self, name: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT pubkey FROM name_claims WHERE name = ?1")?;
        let mut rows = stmt.query(params![name])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Look up the full [`NameClaim`] for a name, or `None`.
    pub fn get(&self, name: &str) -> Result<Option<NameClaim>> {
        let mut stmt = self
            .conn
            .prepare("SELECT raw_json FROM name_claims WHERE name = ?1")?;
        let mut rows = stmt.query(params![name])?;
        if let Some(row) = rows.next()? {
            let raw: String = row.get(0)?;
            Ok(Some(serde_json::from_str(&raw)?))
        } else {
            Ok(None)
        }
    }

    /// All names owned by a given pubkey.
    pub fn names_for_pubkey(&self, pubkey: &str) -> Result<Vec<NameClaim>> {
        self.query_claims(
            "SELECT raw_json FROM name_claims WHERE pubkey = ?1 ORDER BY name",
            params![pubkey],
        )
    }

    /// All name claims whose scope matches `scope` (e.g. `"forum"`).
    ///
    /// Matches names of the form `<local>@<scope>`.
    pub fn claims_in_scope(&self, scope: &str) -> Result<Vec<NameClaim>> {
        let pattern = format!("%@{scope}");
        self.query_claims(
            "SELECT raw_json FROM name_claims WHERE name LIKE ?1 ORDER BY name",
            params![pattern],
        )
    }

    /// Total number of stored claims.
    pub fn count(&self) -> Result<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM name_claims", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    /// All claims, ordered by name.
    pub fn all(&self) -> Result<Vec<NameClaim>> {
        self.query_claims("SELECT raw_json FROM name_claims ORDER BY name", [])
    }

    fn insert_raw(&self, claim: &NameClaim) -> Result<()> {
        let raw = serde_json::to_string(claim)?;
        self.conn.execute(
            "INSERT INTO name_claims
             (name, pubkey, sequence, claimed_at, signature, raw_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                claim.name,
                claim.pubkey,
                claim.sequence as i64,
                claim.claimed_at as i64,
                claim.signature,
                raw,
            ],
        )?;
        Ok(())
    }

    fn update_raw(&self, claim: &NameClaim) -> Result<()> {
        let raw = serde_json::to_string(claim)?;
        self.conn.execute(
            "UPDATE name_claims
             SET pubkey=?2, sequence=?3, claimed_at=?4, signature=?5, raw_json=?6
             WHERE name=?1",
            params![
                claim.name,
                claim.pubkey,
                claim.sequence as i64,
                claim.claimed_at as i64,
                claim.signature,
                raw,
            ],
        )?;
        Ok(())
    }

    fn query_claims(&self, sql: &str, params: impl rusqlite::Params) -> Result<Vec<NameClaim>> {
        let mut stmt = self.conn.prepare(sql)?;
        let claims = stmt
            .query_map(params, |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .filter_map(|raw| serde_json::from_str::<NameClaim>(&raw).ok())
            .collect();
        Ok(claims)
    }
}

fn db_path() -> Result<PathBuf> {
    let handle = SisiNode::get().map_err(|e| anyhow!(e.to_string()))?;
    Ok(handle.data_dir.join("names").join("claims.db"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::NameClaim;
    use ed25519_dalek::{Signer, SigningKey};
    use rusqlite::Connection;

    fn make_store() -> NameStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = NameStore { conn };
        store.init_schema().unwrap();
        store
    }

    fn random_key() -> SigningKey {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).unwrap();
        SigningKey::from_bytes(&seed)
    }

    #[test]
    fn upsert_should_insert_new_claim() {
        let store = make_store();
        let key = random_key();
        let claim = NameClaim::new(&key, "alice@dev").unwrap();
        assert_eq!(store.upsert(&claim).unwrap(), UpsertResult::Inserted);
    }

    #[test]
    fn upsert_should_update_same_owner_with_higher_sequence() {
        let store = make_store();
        let key = random_key();
        let claim = NameClaim::new(&key, "alice@dev").unwrap();
        store.upsert(&claim).unwrap();

        let updated = claim.update(&key).unwrap();
        assert_eq!(store.upsert(&updated).unwrap(), UpsertResult::Updated);
    }

    #[test]
    fn upsert_should_return_stale_for_same_sequence() {
        let store = make_store();
        let key = random_key();
        let claim = NameClaim::new(&key, "alice@dev").unwrap();
        store.upsert(&claim).unwrap();

        // Send exact same claim again.
        assert_eq!(store.upsert(&claim).unwrap(), UpsertResult::Stale);
    }

    #[test]
    fn upsert_should_reject_later_claim_by_different_pubkey() {
        let store = make_store();

        let key_a = random_key();
        let mut claim_a = NameClaim::new(&key_a, "bob@dev").unwrap();
        claim_a.claimed_at = 1000;
        claim_a.signature = vec![];
        let sig = key_a.sign(&claim_a.canonical_payload());
        claim_a.signature = sig.to_bytes().to_vec();
        store.upsert(&claim_a).unwrap();

        let key_b = random_key();
        let mut claim_b = NameClaim::new(&key_b, "bob@dev").unwrap();
        claim_b.claimed_at = 2000;
        claim_b.signature = vec![];
        let sig = key_b.sign(&claim_b.canonical_payload());
        claim_b.signature = sig.to_bytes().to_vec();

        let result = store.upsert(&claim_b).unwrap();
        assert!(matches!(result, UpsertResult::Rejected { .. }));
    }

    #[test]
    fn resolve_should_return_pubkey_after_insert() {
        let store = make_store();
        let key = random_key();
        let claim = NameClaim::new(&key, "carol@blog").unwrap();
        store.upsert(&claim).unwrap();

        let pubkey = store.resolve("carol@blog").unwrap();
        assert_eq!(pubkey, Some(claim.pubkey));
    }

    #[test]
    fn resolve_should_return_none_for_unknown_name() {
        let store = make_store();
        assert_eq!(store.resolve("nobody@anywhere").unwrap(), None);
    }

    #[test]
    fn claims_in_scope_should_filter_correctly() {
        let store = make_store();
        let key = random_key();

        store
            .upsert(&NameClaim::new(&key, "a@forum").unwrap())
            .unwrap();
        store
            .upsert(&NameClaim::new(&key, "b@forum").unwrap())
            .unwrap();
        store
            .upsert(&NameClaim::new(&key, "c@blog").unwrap())
            .unwrap();

        let forum_claims = store.claims_in_scope("forum").unwrap();
        assert_eq!(forum_claims.len(), 2);
        assert!(forum_claims.iter().all(|c| c.scope() == Some("forum")));
    }

    #[test]
    fn names_for_pubkey_should_return_all_owned_names() {
        let store = make_store();
        let key = random_key();

        store
            .upsert(&NameClaim::new(&key, "x@forum").unwrap())
            .unwrap();
        store
            .upsert(&NameClaim::new(&key, "x@blog").unwrap())
            .unwrap();
        store.upsert(&NameClaim::new(&key, "x").unwrap()).unwrap();

        let names = store
            .names_for_pubkey(&hex::encode(key.verifying_key().to_bytes()))
            .unwrap();
        assert_eq!(names.len(), 3);
    }
}
