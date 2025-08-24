//! NekoCode Core Library
//! 
//! Provides shared functionality for all NekoCode tools:
//! - Session management
//! - Configuration handling
//! - Common types and traits
//! - File I/O utilities
//! - Memory management

pub mod session;
pub mod sqlite_session;
pub mod cli_session;
pub mod config;
pub mod types;
pub mod io;
pub mod memory;
pub mod traits;
pub mod error;
pub mod compact;

// Re-exports for easy access
pub use session::{SessionManager, SessionInfo, Session, SessionProvider};
pub use sqlite_session::{SqliteSession, FileRecord, ChangedFile, ChangeType, SessionStats};
pub use cli_session::{CliSessionConfig, CliSessionHelper, SessionHistoryEntry, CliSettings};
pub use config::{Config, AnalysisConfig, GeneralConfig, MemoryConfig};
pub use types::{Language, SymbolInfo, FunctionInfo, ClassInfo, FileInfo, AnalysisResult};
pub use traits::{AnalysisProvider, LanguageSupport};
pub use error::{NekocodeError, Result};
pub use io::{FileProcessor, PathUtils};
pub use memory::{MemoryManager, MemoryType};
pub use compact::{OutputMode, CompactSerializer, HumanFormatter};

/// NekoCode Core version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default session directory name
pub const SESSION_DIR: &str = ".nekocode_sessions";

/// Default configuration file name
pub const CONFIG_FILE: &str = "nekocode_config.json";