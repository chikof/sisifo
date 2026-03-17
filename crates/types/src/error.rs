use thiserror::Error;

#[derive(Debug, Error)]
pub enum SisiError {
    #[error("iroh error: {0}")]
    Iroh(#[from] anyhow::Error),

    #[error("iroh node error: {0}")]
    IrohNode(#[from] n0_error::AnyError),

    #[error("invalid address: {0}")]
    InvalidAddress(String),

    #[error("manifest not found: {0}")]
    ManifestNotFound(String),

    #[error("signing error: {0}")]
    Signing(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SisiError>;
