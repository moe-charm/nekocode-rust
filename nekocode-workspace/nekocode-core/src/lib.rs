//! NekoCode core library.
//!
//! The Rust-first SSOT for explicit workspace snapshots and bounded context
//! packs. Legacy session/configuration modules remain available for recovery,
//! but they are not part of the canonical snapshot/context contract.

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
    build_context, build_rust_context, build_rust_context_with_config,
    build_rust_context_with_options, build_rust_snapshot, build_rust_snapshot_with_mode,
    build_snapshot, index_rust_workspace, read_rust_snapshot, sanitize_context_for_output,
    sanitize_snapshot_for_output, write_rust_snapshot,
    AnalysisMode, ArtifactStatus, BudgetReport, ChangedRustFile, ComparisonStatus, ContextRequest,
    ContextV1, EvidenceLevel, Omission, RustContextOptions, RustContextPack, RustContextSnapshot,
    RustDiagnostic, RustDiagnosticDelta, RustDiagnosticRun, RustDiagnosticSpan, RustDiffHunk,
    RustDiffSummary, RustInputDigest, RustPackage, RustSourceExcerpt, RustTarget,
    RustToolchainInfo, RustWorkspaceSnapshot, SnapshotRequest, SnapshotV1, ToolProvenance,
    CONTEXT_CONTRACT_VERSION, SCHEMA_VERSION, SNAPSHOT_CONTRACT_VERSION,
};

/// NekoCode Core version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default session directory name
pub const SESSION_DIR: &str = ".nekocode_sessions";

/// Default configuration file name
pub const CONFIG_FILE: &str = "nekocode_config.json";
