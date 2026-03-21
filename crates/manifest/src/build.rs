use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use types::{SiteFile, SiteManifest};

pub struct ManifestBuilder {
    name: String,
    files: HashMap<String, SiteFile>,
    owner_pubkey: Vec<u8>,
    version: u32,
}

impl ManifestBuilder {
    pub fn new(name: impl Into<String>, owner_pubkey: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            files: HashMap::new(),
            owner_pubkey,
            version: 1,
        }
    }

    pub fn add_file(mut self, file: SiteFile) -> Self {
        self.files.insert(file.path.clone(), file);
        self
    }

    pub fn build_unsigned(self) -> SiteManifest {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        SiteManifest {
            name: self.name,
            files: self.files,
            owner_pubkey: self.owner_pubkey,
            signature: vec![],
            created_at: now,
            updated_at: now,
            version: self.version,
        }
    }

    pub fn build_unsigned_with_version(self, version: u32) -> SiteManifest {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        SiteManifest {
            name: self.name,
            files: self.files,
            owner_pubkey: self.owner_pubkey,
            signature: vec![],
            created_at: now,
            updated_at: now,
            version,
        }
    }
}
