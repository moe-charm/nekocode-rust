//! Error handling for NekoCode

use thiserror::Error;

pub type Result<T> = std::result::Result<T, NekocodeError>;

#[derive(Error, Debug)]
pub enum NekocodeError {
    #[error("Session not found: {0}")]
    #[cfg(feature = "legacy")]
    SessionNotFound(String),

    #[error("Invalid session ID: {0}")]
    #[cfg(feature = "legacy")]
    InvalidSessionId(String),

    #[error("File not found: {0}")]
    #[cfg(feature = "legacy")]
    FileNotFound(String),

    #[error("Language not supported: {0}")]
    #[cfg(feature = "legacy")]
    LanguageNotSupported(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Parse error: {0}")]
    #[cfg(feature = "legacy")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Analysis error: {0}")]
    #[cfg(feature = "legacy")]
    Analysis(String),

    #[error("Memory error: {0}")]
    #[cfg(feature = "legacy")]
    Memory(String),

    #[error("Preview error: {0}")]
    #[cfg(feature = "legacy")]
    Preview(String),

    #[error("Refactoring error: {0}")]
    #[cfg(feature = "legacy")]
    Refactoring(String),

    #[error("Impact analysis error: {0}")]
    #[cfg(feature = "legacy")]
    Impact(String),

    #[error("Watch error: {0}")]
    #[cfg(feature = "legacy")]
    Watch(String),

    #[error("Session error: {0}")]
    #[cfg(feature = "legacy")]
    Session(String),

    #[error("External tool error: {0}")]
    External(String),

    #[error("Not implemented: {0}")]
    #[cfg(feature = "legacy")]
    NotImplemented(String),

    #[error("Analysis error: {0}")]
    #[cfg(feature = "legacy")]
    AnalysisError(String),

    #[error("{0}")]
    #[cfg(feature = "legacy")]
    Other(#[from] anyhow::Error),
}
