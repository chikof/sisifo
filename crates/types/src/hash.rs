use crate::error::SisiError;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SiteHash(pub iroh_blobs::Hash);

impl fmt::Display for SiteHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SiteHash {
    type Err = SisiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<iroh_blobs::Hash>()
            .map(SiteHash)
            .map_err(|e| SisiError::InvalidAddress(e.to_string()))
    }
}
