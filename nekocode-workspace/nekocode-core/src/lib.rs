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
pub mod rust_context;

// Re-exports for easy access
pub use session::{SessionManager, SessionInfo, Session, SessionProvider};
pub use sqlite_session::{SqliteSession, FileRecord, ChangedFile, ChangeType, SessionStats};
pub use cli_session::{CliSessionConfig, CliSessionHelper, SessionHistoryEntry, CliSettings};
pub use config::{Config, AnalysisConfig, GeneralConfig, MemoryConfig, ConfigManager};
pub use types::{Language, SymbolInfo, FunctionInfo, ClassInfo, FileInfo, AnalysisResult};
pub use traits::{AnalysisProvider, LanguageSupport};
pub use error::{NekocodeError, Result};
pub use io::{FileProcessor, PathUtils};
pub use memory::{MemoryManager, MemoryType, MemoryEntry};
pub use compact::{OutputMode, CompactSerializer, HumanFormatter};
pub use rust_context::{
    build_rust_context, build_rust_context_with_config, build_rust_context_with_options,
    index_rust_workspace, ChangedRustFile, EvidenceLevel, RustContextOptions, RustContextPack,
    RustDiagnostic, RustDiagnosticRun, RustDiagnosticSpan, RustDiffHunk, RustDiffSummary,
    RustInputDigest, RustPackage, RustTarget, RustToolchainInfo, RustWorkspaceSnapshot,
    ToolProvenance,
};

/// NekoCode Core version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default session directory name
pub const SESSION_DIR: &str = ".nekocode_sessions";

/// Default configuration file name
pub const CONFIG_FILE: &str = "nekocode_config.json";
