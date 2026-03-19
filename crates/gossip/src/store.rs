use crate::message::GossipMessage;
use anyhow::{Result, anyhow};
use node::SisiNode;
use rusqlite::{Connection, params};
use std::path::PathBuf;

pub struct MessageStore {
    conn: Connection,
}

impl MessageStore {
    pub fn open() -> Result<Self> {
        let path = db_path()?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        let conn = Connection::open(&path)?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS messages (
                id          TEXT PRIMARY KEY,
                author      TEXT NOT NULL,
                topic       TEXT NOT NULL,
                kind        TEXT NOT NULL,
                content     TEXT NOT NULL,
                parent_id   TEXT,
                created_at  INTEGER NOT NULL,
                signature   BLOB NOT NULL,
                raw_json    TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_topic ON messages(topic, created_at);
            CREATE INDEX IF NOT EXISTS idx_parent ON messages(parent_id);
        ",
        )?;

        Ok(MessageStore { conn })
    }

    pub fn insert(&self, msg: &GossipMessage) -> Result<()> {
        let raw = serde_json::to_string(msg)?;
        let kind = format!("{:?}", msg.kind);

        self.conn.execute(
            "INSERT OR IGNORE INTO messages
             (id, author, topic, kind, content, parent_id, created_at, signature, raw_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                msg.id,
                msg.author,
                msg.topic,
                kind,
                msg.content,
                msg.parent_id,
                msg.created_at as i64,
                msg.signature,
                raw,
            ],
        )?;

        Ok(())
    }

    pub fn list_topic(
        &self,
        topic: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<GossipMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT raw_json FROM messages
             WHERE topic = ?1
             ORDER BY created_at DESC
             LIMIT ?2 OFFSET ?3",
        )?;

        let msgs = stmt
            .query_map(params![topic, limit as i64, offset as i64], |row| {
                row.get::<_, String>(0)
            })?
            .filter_map(|r| r.ok())
            .filter_map(|raw| serde_json::from_str::<GossipMessage>(&raw).ok())
            .collect();

        Ok(msgs)
    }

    pub fn list_replies(&self, parent_id: &str) -> Result<Vec<GossipMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT raw_json FROM messages
             WHERE parent_id = ?1
             ORDER BY created_at ASC",
        )?;

        let msgs = stmt
            .query_map(params![parent_id], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .filter_map(|raw| serde_json::from_str::<GossipMessage>(&raw).ok())
            .collect();

        Ok(msgs)
    }

    pub fn message_count(&self, topic: &str) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE topic = ?1",
            params![topic],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM messages WHERE id = ?1", params![id])?;
        Ok(())
    }
}

fn db_path() -> Result<PathBuf> {
    let handle = SisiNode::get().map_err(|e| anyhow!(e.to_string()))?;
    Ok(handle.data_dir.join("gossip").join("messages.db"))
}
