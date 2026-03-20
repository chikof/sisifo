use anyhow::Result;
use node::SisiNode;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteRecord {
    pub hash: String,
    pub name: String,
    pub short_name: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub icon_url: Option<String>, // gateway URL to the icon
    pub author_pubkey: Option<String>,
    pub category: Option<String>,
    pub lang: Option<String>,
    pub accent_color: Option<String>,
    pub indexed_at: u64,
    pub visit_count: u64,
}

pub struct IndexStore {
    conn: Connection,
}

impl IndexStore {
    pub fn open() -> Result<Self> {
        let path = db_path()?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        let conn = Connection::open(&path)?;

        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS sites (
                hash            TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                short_name      TEXT,
                description     TEXT,
                keywords        TEXT,       -- JSON array
                icon_url        TEXT,
                author_pubkey   TEXT,
                category        TEXT,
                lang            TEXT,
                accent_color    TEXT,
                indexed_at      INTEGER NOT NULL,
                visit_count     INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_name ON sites(name);
            CREATE INDEX IF NOT EXISTS idx_category ON sites(category);
            CREATE VIRTUAL TABLE IF NOT EXISTS sites_fts
                USING fts5(hash UNINDEXED, name, description, keywords, content=sites, content_rowid=rowid);
            CREATE TRIGGER IF NOT EXISTS sites_ai AFTER INSERT ON sites BEGIN
                INSERT INTO sites_fts(rowid, hash, name, description, keywords)
                VALUES (new.rowid, new.hash, new.name, new.description, new.keywords);
            END;
            CREATE TRIGGER IF NOT EXISTS sites_au AFTER UPDATE ON sites BEGIN
                INSERT INTO sites_fts(sites_fts, rowid, hash, name, description, keywords)
                VALUES ('delete', old.rowid, old.hash, old.name, old.description, old.keywords);
                INSERT INTO sites_fts(rowid, hash, name, description, keywords)
                VALUES (new.rowid, new.hash, new.name, new.description, new.keywords);
            END;
        ")?;

        Ok(IndexStore { conn })
    }

    pub fn upsert(&self, record: &SiteRecord) -> Result<()> {
        let keywords = serde_json::to_string(&record.keywords)?;
        self.conn.execute(
            "INSERT INTO sites
             (hash, name, short_name, description, keywords, icon_url,
              author_pubkey, category, lang, accent_color, indexed_at, visit_count)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(hash) DO UPDATE SET
                name=excluded.name, description=excluded.description,
                keywords=excluded.keywords, icon_url=excluded.icon_url,
                indexed_at=excluded.indexed_at,
                visit_count=visit_count+1",
            params![
                record.hash,
                record.name,
                record.short_name,
                record.description,
                keywords,
                record.icon_url,
                record.author_pubkey,
                record.category,
                record.lang,
                record.accent_color,
                record.indexed_at as i64,
                record.visit_count as i64,
            ],
        )?;
        Ok(())
    }

    pub fn increment_visits(&self, hash: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE sites SET visit_count = visit_count + 1 WHERE hash = ?1",
            params![hash],
        )?;
        Ok(())
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SiteRecord>> {
        if query.trim().is_empty() {
            return self.recent(limit);
        }

        // FTS5 full-text search with ranking
        let mut stmt = self.conn.prepare(
            "SELECT s.hash, s.name, s.short_name, s.description, s.keywords,
                    s.icon_url, s.author_pubkey, s.category, s.lang,
                    s.accent_color, s.indexed_at, s.visit_count
             FROM sites s
             JOIN sites_fts f ON s.rowid = f.rowid
             WHERE sites_fts MATCH ?1
             ORDER BY rank, s.visit_count DESC
             LIMIT ?2",
        )?;

        self.collect_records(&mut stmt, params![query, limit as i64])
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<SiteRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT hash, name, short_name, description, keywords, icon_url,
                    author_pubkey, category, lang, accent_color, indexed_at, visit_count
             FROM sites
             ORDER BY visit_count DESC, indexed_at DESC
             LIMIT ?1",
        )?;
        self.collect_records(&mut stmt, params![limit as i64])
    }

    pub fn by_category(&self, category: &str) -> Result<Vec<SiteRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT hash, name, short_name, description, keywords, icon_url,
                    author_pubkey, category, lang, accent_color, indexed_at, visit_count
             FROM sites WHERE category = ?1
             ORDER BY visit_count DESC",
        )?;
        self.collect_records(&mut stmt, params![category])
    }

    pub fn count(&self) -> Result<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM sites", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    fn collect_records(
        &self,
        stmt: &mut rusqlite::Statement,
        params: impl rusqlite::Params,
    ) -> Result<Vec<SiteRecord>> {
        let records = stmt
            .query_map(params, |row| {
                let keywords_json: String = row.get(4)?;
                Ok((
                    row.get::<_, String>(0)?, // hash
                    row.get::<_, String>(1)?, // name
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    keywords_json,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(
                |(
                    hash,
                    name,
                    short_name,
                    description,
                    kw_json,
                    icon_url,
                    author_pubkey,
                    category,
                    lang,
                    accent_color,
                    indexed_at,
                    visit_count,
                )| {
                    let keywords = serde_json::from_str(&kw_json).unwrap_or_default();
                    SiteRecord {
                        hash,
                        name,
                        short_name,
                        description,
                        keywords,
                        icon_url,
                        author_pubkey,
                        category,
                        lang,
                        accent_color,
                        indexed_at: indexed_at as u64,
                        visit_count: visit_count as u64,
                    }
                },
            )
            .collect();
        Ok(records)
    }
}

fn db_path() -> Result<PathBuf> {
    let handle = SisiNode::get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(handle.data_dir.join("index").join("sites.db"))
}
