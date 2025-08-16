//! Error handling for NekoCode

use thiserror::Error;

pub type Result<T> = std::result::Result<T, NekocodeError>;

#[derive(Error, Debug)]
pub enum NekocodeError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    
    #[error("Invalid session ID: {0}")]
    InvalidSessionId(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Language not supported: {0}")]
    LanguageNotSupported(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    
    #[error("Analysis error: {0}")]
    Analysis(String),
    
    #[error("Memory error: {0}")]
    Memory(String),
    
    #[error("Preview error: {0}")]
    Preview(String),
    
    #[error("Refactoring error: {0}")]
    Refactoring(String),
    
    #[error("Impact analysis error: {0}")]
    Impact(String),
    
    #[error("Watch error: {0}")]
    Watch(String),
    
    #[error("Session error: {0}")]
    Session(String),
    
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}