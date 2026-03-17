use iroh_blobs::Hash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteFile {
    pub path: String, // e.g. "/index.html"
    pub hash: Hash,
    pub mime: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteManifest {
    pub name: String,
    pub files: HashMap<String, SiteFile>,
    pub owner_pubkey: Vec<u8>, // ed25519 pubkey bytes
    pub signature: Vec<u8>,    // signs over canonical hash of files map
    pub created_at: u64,
    pub updated_at: u64,
    pub version: u32,
}

/// Lightweight metadata for listing sites (no file map)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteMeta {
    pub name: String,
    pub hash: String,
    pub file_count: usize,
    pub total_size: u64,
    pub updated_at: u64,
}

impl<T> From<(&SiteManifest, T)> for SiteMeta
where
    T: AsRef<str>,
{
    fn from((m, hash): (&SiteManifest, T)) -> Self {
        SiteMeta {
            name: m.name.clone(),
            hash: hash.as_ref().to_string(),
            file_count: m.files.len(),
            total_size: m.files.values().map(|f| f.size).sum(),
            updated_at: m.updated_at,
        }
    }
}
