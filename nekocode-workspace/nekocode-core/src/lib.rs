//! NekoCode core library.
//!
//! The Rust-first SSOT for explicit workspace snapshots and bounded context
//! packs. Rust meaning remains delegated to Cargo and rustc.

pub mod error;
pub mod rust_context;

pub use error::{NekocodeError, Result};
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

/// NekoCode Core version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
