//! Error handling for NekoCode

use thiserror::Error;

pub type Result<T> = std::result::Result<T, NekocodeError>;

#[derive(Error, Debug)]
pub enum NekocodeError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("External tool error: {0}")]
    External(String),
}
