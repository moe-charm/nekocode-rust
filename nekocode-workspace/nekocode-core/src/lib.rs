//! NekoCode core library.
//!
//! The Rust-first SSOT for explicit workspace snapshots and bounded context
//! packs. Frozen session/configuration modules compile only with the explicit
//! `legacy` feature and are not part of the canonical dependency graph.

#[cfg(feature = "legacy")]
pub mod cli_session;
#[cfg(feature = "legacy")]
pub mod compact;
#[cfg(feature = "legacy")]
pub mod config;
pub mod error;
#[cfg(feature = "legacy")]
pub mod io;
#[cfg(feature = "legacy")]
pub mod memory;
pub mod rust_context;
#[cfg(feature = "legacy")]
pub mod session;
#[cfg(feature = "legacy")]
pub mod sqlite_session;
#[cfg(feature = "legacy")]
pub mod traits;
#[cfg(feature = "legacy")]
pub mod types;

// Recovery-only exports for the frozen multi-binary implementation.
#[cfg(feature = "legacy")]
pub use cli_session::{CliSessionConfig, CliSessionHelper, CliSettings, SessionHistoryEntry};
#[cfg(feature = "legacy")]
pub use compact::{CompactSerializer, HumanFormatter, OutputMode};
#[cfg(feature = "legacy")]
pub use config::{AnalysisConfig, Config, ConfigManager, GeneralConfig, MemoryConfig};
pub use error::{NekocodeError, Result};
#[cfg(feature = "legacy")]
pub use io::{FileProcessor, PathUtils};
#[cfg(feature = "legacy")]
pub use memory::{MemoryEntry, MemoryManager, MemoryType};
pub use rust_context::{
    build_context, build_rust_context, build_rust_context_with_config,
    build_rust_context_with_options, build_rust_snapshot, build_rust_snapshot_with_mode,
    build_snapshot, index_rust_workspace, read_rust_snapshot, sanitize_context_for_output,
    sanitize_snapshot_for_output, write_rust_snapshot, AnalysisMode, ArtifactStatus, BudgetReport,
    ChangedRustFile, ComparisonStatus, ContextRequest, ContextV1, EvidenceLevel, ExecutionPolicy,
    Omission, RustContextOptions, RustContextPack, RustContextSnapshot, RustDiagnostic,
    RustDiagnosticDelta, RustDiagnosticRun, RustDiagnosticSpan, RustDiffHunk, RustDiffSummary,
    RustInputDigest, RustPackage, RustSourceExcerpt, RustTarget, RustToolchainInfo,
    RustWorkspaceSnapshot, SnapshotRequest, SnapshotV1, ToolProvenance, CONTEXT_CONTRACT_VERSION,
    SCHEMA_VERSION, SNAPSHOT_CONTRACT_VERSION,
};
#[cfg(feature = "legacy")]
pub use session::{Session, SessionInfo, SessionManager, SessionProvider};
#[cfg(feature = "legacy")]
pub use sqlite_session::{ChangeType, ChangedFile, FileRecord, SessionStats, SqliteSession};
#[cfg(feature = "legacy")]
pub use traits::{AnalysisProvider, LanguageSupport};
#[cfg(feature = "legacy")]
pub use types::{AnalysisResult, ClassInfo, FileInfo, FunctionInfo, Language, SymbolInfo};

/// NekoCode Core version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default session directory name
#[cfg(feature = "legacy")]
pub const SESSION_DIR: &str = ".nekocode_sessions";

/// Default configuration file name
#[cfg(feature = "legacy")]
pub const CONFIG_FILE: &str = "nekocode_config.json";
