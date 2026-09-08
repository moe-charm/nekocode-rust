//! Rust-first, evidence-backed project context.
//!
//! This module deliberately does not reimplement Rust semantic analysis. Cargo
//! is the source of truth for workspace/package metadata; Git is the source of
//! truth for the requested change set. Later backends can enrich this snapshot
//! with rustc, Clippy, and rust-analyzer results without changing the JSON
//! contract established here.

use crate::error::{NekocodeError, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Internal compatibility marker retained while the public artifacts use
/// versioned `snapshot-v1` and `context-v1` envelopes.
pub const SCHEMA_VERSION: u32 = 3;
pub const SNAPSHOT_CONTRACT_VERSION: &str = "snapshot-v1";
pub const CONTEXT_CONTRACT_VERSION: &str = "context-v1";
const MAX_SOURCE_EXCERPT_BYTES: usize = 32 * 1024;
const CARGO_CHECK_TIMEOUT: Duration = Duration::from_secs(180);
const MAX_CARGO_STDOUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_CARGO_STDERR_BYTES: usize = 2 * 1024 * 1024;
const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_GIT_STDOUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_GIT_STDERR_BYTES: usize = 2 * 1024 * 1024;
const CARGO_TARGET_DIR_NAME: &str = "nekocode-rust-first-target";

mod budget;
mod context;
mod diagnostics;
mod execution;
mod git;
mod snapshot;
mod summary;
mod workspace;

pub use context::{
    build_rust_context, build_rust_context_with_config, build_rust_context_with_options,
};
pub use snapshot::{
    build_rust_snapshot, build_rust_snapshot_with_analysis, build_rust_snapshot_with_mode,
    read_rust_snapshot, sanitize_context_for_output, sanitize_snapshot_for_output,
    write_rust_snapshot,
};
pub use summary::format_context_summary;
pub use workspace::index_rust_workspace;

// The facade keeps the old internal test vocabulary while implementations live
// in responsibility-specific modules. These are crate-private, not wire API.
pub(super) use budget::safe_workspace_file;
#[cfg(test)]
pub(super) use budget::truncate_utf8;
#[cfg(test)]
pub(super) use diagnostics::compare_diagnostic_multisets;
#[cfg(test)]
pub(super) use diagnostics::evidence_for_artifact_status;
#[cfg(test)]
pub(super) use execution::parse_cargo_diagnostics;
#[cfg(test)]
pub(super) use execution::run_bounded_command;
#[cfg(test)]
pub(super) use git::{parse_name_status_z, parse_numstat_z, parse_unified_hunks};
#[cfg(test)]
pub(super) use summary::{display_path, single_line, unique_primary_diagnostics};
#[cfg(test)]
pub(super) use workspace::parse_package;

/// Provenance for one external tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolProvenance {
    pub tool: String,
    pub command: String,
    pub cwd: PathBuf,
    pub version: Option<String>,
    pub exit_code: Option<i32>,
}

/// A stable digest of an input file that affects Rust build meaning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustInputDigest {
    pub path: PathBuf,
    pub sha256: String,
}

/// Evidence level for data returned by the Rust-first context layer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceLevel {
    /// Directly reported by Cargo/Git rather than inferred from syntax.
    ToolConfirmed,
    /// Derived from a semantic backend (reserved for later backends).
    SemanticResolved,
    /// Derived from syntax only (reserved for later backends).
    SyntaxOnly,
    /// The requested backend could not provide complete information.
    Incomplete,
}

/// Whether a snapshot performs metadata observation or invokes Cargo.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisMode {
    #[default]
    MetadataOnly,
    CargoCheck,
    Clippy,
}

/// The official diagnostic producer used for one explicit observation.
///
/// Cargo check remains the default. Clippy is opt-in and is never folded into
/// a cargo-check observation or compared with one as if they were equivalent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticProducer {
    #[default]
    CargoCheck,
    Clippy,
}

/// Command profile marker required for exact diagnostic comparability.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticProfile {
    #[default]
    CargoCheckV1,
    ClippyDefaultV1,
}

/// Whether a comparison input was observed, known to be absent, or could not
/// be observed safely.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonObservationStatus {
    #[default]
    Observed,
    Absent,
    Unknown,
}

/// Non-secret observation of compiler-affecting Cargo configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RustCompilerConfigObservation {
    pub status: ComparisonObservationStatus,
    pub sha256: Option<String>,
}

/// Machine-readable basis used to decide whether two diagnostic runs may be
/// compared. This is evidence, not a second semantic analyzer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiagnosticComparisonBasis {
    pub workspace_members_sha256: String,
    pub package_set_sha256: String,
    pub package_targets_sha256: String,
    pub feature_definitions_sha256: String,
    pub workspace_inputs_sha256: String,
    pub compiler_config: RustCompilerConfigObservation,
    pub target_coverage: String,
    pub feature_coverage: String,
    pub toolchain: RustToolchainInfo,
    pub producer: DiagnosticProducer,
    pub profile: DiagnosticProfile,
    pub producer_version: Option<String>,
    pub complete: bool,
}

/// Stable reason codes for a non-comparable or incomplete diagnostic delta.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonReasonCode {
    BaselineMissing,
    BaselineIntegrityUnavailable,
    BaselineIntegrityMismatch,
    ComparisonBasisUnavailable,
    ToolchainMismatch,
    ProducerMismatch,
    ProducerVersionUnknown,
    ProducerVersionMismatch,
    AnalysisProfileMismatch,
    PackageSetMismatch,
    TargetCoverageMismatch,
    FeatureCoverageMismatch,
    FeatureDefinitionMismatch,
    WorkspaceInputsMismatch,
    CompilerConfigMismatch,
    CompilerConfigUnknown,
    ToolProvenanceMismatch,
    BaselineObservationIncomplete,
    CurrentObservationIncomplete,
}

/// One stable comparison reason with a human-readable dimension label.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComparisonReason {
    pub code: ComparisonReasonCode,
    pub dimension: String,
}

impl DiagnosticProducer {
    fn profile(self) -> DiagnosticProfile {
        match self {
            Self::CargoCheck => DiagnosticProfile::CargoCheckV1,
            Self::Clippy => DiagnosticProfile::ClippyDefaultV1,
        }
    }

    fn analysis_mode(self) -> AnalysisMode {
        match self {
            Self::CargoCheck => AnalysisMode::CargoCheck,
            Self::Clippy => AnalysisMode::Clippy,
        }
    }

    fn command_name(self) -> &'static str {
        match self {
            Self::CargoCheck => "cargo check",
            Self::Clippy => "cargo clippy",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::CargoCheck => "cargo_check",
            Self::Clippy => "clippy",
        }
    }
}

/// Safety posture recorded with every public artifact.
///
/// These are descriptive strings rather than a claim that an OS sandbox is
/// present. In particular, `process_network_isolation` remains
/// `not_enforced` until a platform sandbox is implemented.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionPolicy {
    pub mode: AnalysisMode,
    pub workspace_trust: String,
    pub cargo_registry_network: String,
    pub process_network_isolation: String,
    pub environment: String,
    pub compiler_wrappers: String,
    pub target_directory: String,
}

fn metadata_execution_policy() -> ExecutionPolicy {
    ExecutionPolicy {
        mode: AnalysisMode::MetadataOnly,
        workspace_trust: "not_required".to_string(),
        cargo_registry_network: "not_used".to_string(),
        process_network_isolation: "not_applicable".to_string(),
        environment: "not_applicable".to_string(),
        compiler_wrappers: "not_run".to_string(),
        target_directory: "not_used".to_string(),
    }
}

fn diagnostic_execution_policy(producer: DiagnosticProducer) -> ExecutionPolicy {
    ExecutionPolicy {
        mode: producer.analysis_mode(),
        workspace_trust: "required".to_string(),
        cargo_registry_network: "offline".to_string(),
        process_network_isolation: "not_enforced".to_string(),
        environment: "allowlist".to_string(),
        compiler_wrappers: "disabled".to_string(),
        target_directory: "dedicated_temp".to_string(),
    }
}

/// Lifecycle state of a snapshot/context operation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStatus {
    NotRun,
    #[default]
    CompletedClean,
    CompletedWithDiagnostics,
    ToolFailed,
    TimedOut,
    OutputLimited,
    Partial,
}

/// Status of a requested baseline comparison.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonStatus {
    Comparable,
    #[default]
    BaselineMissing,
    NotComparable,
    Partial,
}

/// A machine-readable record of content omitted by a hard budget.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Omission {
    pub kind: String,
    pub reason: String,
    pub omitted_count: usize,
    pub priority: String,
}

/// Hard serialized-byte budget plus the caller's advisory token request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BudgetReport {
    pub requested_tokens: usize,
    pub max_bytes: usize,
    pub serialized_bytes: usize,
    pub exceeded: bool,
}

/// Tool versions used to produce a workspace snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustToolchainInfo {
    pub rustc_version: Option<String>,
    pub cargo_version: Option<String>,
    pub host: Option<String>,
}

/// One Cargo target with enough information to explain workspace shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustTarget {
    pub name: String,
    pub kind: Vec<String>,
    pub src_path: Option<PathBuf>,
    pub edition: Option<String>,
    pub required_features: Vec<String>,
}

/// One Cargo feature and the feature/dependency values it enables.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustFeatureDefinition {
    pub name: String,
    pub enables: Vec<String>,
}

/// A Cargo package in the indexed workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustPackage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub manifest_path: PathBuf,
    pub targets: Vec<String>,
    pub target_details: Vec<RustTarget>,
    pub features: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feature_definitions: Vec<RustFeatureDefinition>,
    pub dependencies: Vec<String>,
    pub edition: Option<String>,
}

/// A stable, serializable snapshot of Cargo workspace structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustWorkspaceSnapshot {
    pub schema_version: u32,
    pub evidence: EvidenceLevel,
    pub root: PathBuf,
    pub workspace_root: PathBuf,
    pub toolchain: RustToolchainInfo,
    pub packages: Vec<RustPackage>,
    pub workspace_members: Vec<String>,
    pub inputs: Vec<RustInputDigest>,
    pub provenance: ToolProvenance,
}

fn default_snapshot_contract_version() -> String {
    SNAPSHOT_CONTRACT_VERSION.to_string()
}

fn default_context_contract_version() -> String {
    CONTEXT_CONTRACT_VERSION.to_string()
}

fn default_snapshot_artifact_kind() -> String {
    "snapshot".to_string()
}

fn default_context_artifact_kind() -> String {
    "context".to_string()
}

fn default_evidence_level() -> EvidenceLevel {
    EvidenceLevel::ToolConfirmed
}

fn default_execution_policy() -> ExecutionPolicy {
    metadata_execution_policy()
}

/// A complete, explicit JSON snapshot that can be used as a later baseline.
///
/// The snapshot is deliberately a file supplied by the caller. NekoCode does
/// not maintain a hidden database or silently create history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustContextSnapshot {
    #[serde(default = "default_snapshot_contract_version")]
    pub contract_version: String,
    #[serde(default = "default_snapshot_artifact_kind")]
    pub artifact_kind: String,
    #[serde(default)]
    pub status: ArtifactStatus,
    #[serde(default)]
    pub analysis_mode: AnalysisMode,
    pub schema_version: u32,
    #[serde(default = "default_evidence_level")]
    pub evidence: EvidenceLevel,
    #[serde(default = "default_execution_policy")]
    pub execution_policy: ExecutionPolicy,
    pub generated_at: String,
    pub workspace: RustWorkspaceSnapshot,
    pub diagnostics: Option<RustDiagnosticRun>,
    #[serde(default)]
    pub canonical_hash: Option<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
    #[serde(default)]
    pub omissions: Vec<Omission>,
}

/// A disjoint Git observation used by Change Scope v1.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum GitChangeScope {
    Revision,
    Staged,
    Unstaged,
    Untracked,
}

/// Why additions/deletions are present or unknown for one scoped file change.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineCountStatus {
    Counted,
    Binary,
    NotRead,
}

/// One scope-specific observation for a possibly multi-scope changed path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustFileScopeChange {
    pub scope: GitChangeScope,
    pub status: String,
    pub additions: Option<usize>,
    pub deletions: Option<usize>,
    pub line_count_status: LineCountStatus,
}

/// Fixed-size line/count aggregate retained independently from patch bodies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustChangeScopeSummary {
    pub scope: GitChangeScope,
    pub file_count: usize,
    pub rust_file_count: usize,
    pub additions: usize,
    pub deletions: usize,
    pub counted_files: usize,
    pub binary_files: usize,
    pub not_read_files: usize,
}

/// A file reported by Git, flattened for v1 compatibility while preserving
/// every scope-specific observation in `scope_changes`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedRustFile {
    pub status: String,
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub is_rust: bool,
    pub package: Option<String>,
    pub hunks: Vec<RustDiffHunk>,
    #[serde(default)]
    pub scope_changes: Vec<RustFileScopeChange>,
}

/// One unified-diff hunk on the old and new file sides.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiffHunk {
    /// The observation whose new-side coordinates this hunk uses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<GitChangeScope>,
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub header: Option<String>,
}

/// Git provenance and bounded patch content for a context request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiffSummary {
    pub compare_ref: Option<String>,
    pub resolved_base: Option<String>,
    pub resolved_head: Option<String>,
    pub include_working_tree: bool,
    #[serde(default)]
    pub include_untracked_content: bool,
    pub patch: String,
    pub patch_truncated: bool,
    pub omitted_patch_bytes: usize,
    #[serde(default)]
    pub change_scopes: Vec<RustChangeScopeSummary>,
    pub provenance: Option<ToolProvenance>,
}

/// One compiler diagnostic extracted from Cargo's JSON message stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiagnostic {
    pub level: String,
    pub message: String,
    pub code: Option<String>,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub rendered: Option<String>,
    pub package_id: Option<String>,
    pub target: Option<String>,
    pub spans: Vec<RustDiagnosticSpan>,
    #[serde(default)]
    pub fingerprint: String,
}

/// A source span from rustc's structured diagnostic payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiagnosticSpan {
    pub file: Option<PathBuf>,
    pub line_start: Option<u32>,
    pub column_start: Option<u32>,
    pub line_end: Option<u32>,
    pub column_end: Option<u32>,
    pub is_primary: bool,
    pub label: Option<String>,
}

/// Result of one Cargo diagnostic invocation, including failed checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiagnosticRun {
    #[serde(default)]
    pub producer: DiagnosticProducer,
    #[serde(default)]
    pub profile: DiagnosticProfile,
    #[serde(default)]
    pub producer_version: Option<String>,
    pub command: String,
    pub status: String,
    pub messages: Vec<RustDiagnostic>,
    pub stderr: Option<String>,
    pub all_targets: bool,
    pub all_features: bool,
    pub provenance: ToolProvenance,
    /// The normalized conditions under which this observation was produced.
    #[serde(default)]
    pub comparison_basis: Option<RustDiagnosticComparisonBasis>,
}

/// Comparison of diagnostics from a saved snapshot and the current run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiagnosticDelta {
    pub baseline_path: PathBuf,
    #[serde(default)]
    pub status: ComparisonStatus,
    pub compatible: bool,
    pub added: Vec<RustDiagnostic>,
    pub resolved: Vec<RustDiagnostic>,
    pub persisting: Vec<RustDiagnostic>,
    #[serde(default)]
    pub reasons: Vec<ComparisonReason>,
    pub limitations: Vec<String>,
}

/// A bounded source excerpt adjacent to a Git diff hunk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustSourceExcerpt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<GitChangeScope>,
    pub path: PathBuf,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
    pub source: String,
    pub truncated: bool,
}

/// Compact context pack intended for MCP/AI consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustContextPack {
    #[serde(default = "default_context_contract_version")]
    pub contract_version: String,
    #[serde(default = "default_context_artifact_kind")]
    pub artifact_kind: String,
    #[serde(default)]
    pub status: ArtifactStatus,
    #[serde(default)]
    pub comparison_status: ComparisonStatus,
    pub schema_version: u32,
    pub evidence: EvidenceLevel,
    #[serde(default = "default_execution_policy")]
    pub execution_policy: ExecutionPolicy,
    pub root: PathBuf,
    pub workspace: RustWorkspaceSnapshot,
    pub compare_ref: Option<String>,
    pub changed_files: Vec<ChangedRustFile>,
    pub diff: Option<RustDiffSummary>,
    pub source_excerpts: Vec<RustSourceExcerpt>,
    pub diagnostics: Option<RustDiagnosticRun>,
    /// Requested producer/profile remain visible if a budget removes the
    /// diagnostic body itself.
    #[serde(default)]
    pub diagnostic_producer: Option<DiagnosticProducer>,
    #[serde(default)]
    pub diagnostic_profile: Option<DiagnosticProfile>,
    pub baseline: Option<PathBuf>,
    pub diagnostic_delta: Option<RustDiagnosticDelta>,
    #[serde(default)]
    pub budget: BudgetReport,
    pub budget_tokens: usize,
    pub estimated_tokens: usize,
    pub serialized_bytes: usize,
    pub budget_exceeded: bool,
    pub include_working_tree: bool,
    #[serde(default)]
    pub include_untracked_content: bool,
    pub all_features: bool,
    pub omitted_changed_files: usize,
    pub omitted_excerpts: usize,
    pub omitted_diagnostics: usize,
    pub omitted_delta_items: usize,
    pub omitted_diff_bytes: usize,
    pub truncation_order: Vec<String>,
    pub limitations: Vec<String>,
    #[serde(default)]
    pub omissions: Vec<Omission>,
}

const SUMMARY_FILE_LIMIT: usize = 40;
const SUMMARY_DIAGNOSTIC_LIMIT: usize = 10;
const SUMMARY_LIMITATION_LIMIT: usize = 8;

/// Render a deterministic, plain-text view of a context artifact.
///
/// This is a presentation of evidence already present in `ContextV1`; it does
/// not infer symbols, references, or semantic impact. JSON remains the public
/// machine contract used by MCP and other adapters.
pub struct RustContextOptions {
    pub compare_ref: Option<String>,
    pub budget_tokens: usize,
    pub include_diagnostics: bool,
    pub diagnostic_producer: DiagnosticProducer,
    pub include_working_tree: bool,
    pub include_untracked_content: bool,
    pub all_features: bool,
    pub include_diff: bool,
    pub excerpt_lines: usize,
    pub baseline: Option<PathBuf>,
}

impl RustContextOptions {
    pub fn new(compare_ref: Option<String>, budget_tokens: usize) -> Self {
        Self {
            include_diff: compare_ref.is_some(),
            compare_ref,
            budget_tokens,
            include_diagnostics: false,
            diagnostic_producer: DiagnosticProducer::CargoCheck,
            include_working_tree: false,
            include_untracked_content: false,
            all_features: false,
            excerpt_lines: 8,
            baseline: None,
        }
    }
}

/// Shared request consumed by CLI and adapters for the snapshot use case.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotRequest {
    pub path: PathBuf,
    pub analysis: AnalysisMode,
    pub all_features: bool,
}

impl SnapshotRequest {
    pub fn metadata_only(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            analysis: AnalysisMode::MetadataOnly,
            all_features: false,
        }
    }
}

/// Shared request consumed by CLI and adapters for the context use case.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextRequest {
    pub path: PathBuf,
    pub compare_ref: Option<String>,
    pub budget: usize,
    pub diagnostics: bool,
    #[serde(default)]
    pub diagnostic_producer: DiagnosticProducer,
    pub working_tree: bool,
    pub include_untracked_content: bool,
    pub all_features: bool,
    pub excerpt_lines: usize,
    pub baseline: Option<PathBuf>,
}

impl ContextRequest {
    pub fn new(path: impl Into<PathBuf>, budget: usize) -> Self {
        Self {
            path: path.into(),
            compare_ref: None,
            budget,
            diagnostics: false,
            diagnostic_producer: DiagnosticProducer::CargoCheck,
            working_tree: false,
            include_untracked_content: false,
            all_features: false,
            excerpt_lines: 8,
            baseline: None,
        }
    }
}

/// Public contract aliases used by CLI/MCP parity tests.
pub type SnapshotV1 = RustContextSnapshot;
pub type ContextV1 = RustContextPack;

/// Build the shared snapshot response for a request.
pub fn build_snapshot(request: &SnapshotRequest) -> Result<SnapshotV1> {
    build_rust_snapshot_with_analysis(&request.path, request.analysis, request.all_features)
}

/// Build the shared context response for a request.
pub fn build_context(request: &ContextRequest) -> Result<ContextV1> {
    let mut options = RustContextOptions::new(request.compare_ref.clone(), request.budget);
    options.include_diagnostics = request.diagnostics;
    options.diagnostic_producer = request.diagnostic_producer;
    options.include_working_tree = request.working_tree;
    options.include_untracked_content = request.include_untracked_content;
    options.all_features = request.all_features;
    options.excerpt_lines = request.excerpt_lines;
    options.baseline = request.baseline.clone();
    build_rust_context_with_config(&request.path, options)
}

#[cfg(test)]
fn git_changed_files(root: &Path, compare_ref: &str) -> Result<Vec<ChangedRustFile>> {
    let (files, _) = git::git_context(root, Some(compare_ref), false, false, false)?;
    Ok(files)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cargo_package_metadata() {
        let value = serde_json::json!({
            "name": "demo",
            "version": "0.1.0",
            "manifest_path": "/tmp/demo/Cargo.toml",
            "targets": [{"name": "demo", "kind": ["bin"]}],
            "features": {"default": [], "full": ["dep:full"]}
        });

        let package = parse_package(&value).expect("package should parse");
        assert_eq!(package.name, "demo");
        assert_eq!(package.targets, vec!["demo"]);
        assert_eq!(package.features, vec!["default", "full"]);
        assert_eq!(package.feature_definitions[1].name, "full");
        assert_eq!(package.feature_definitions[1].enables, vec!["dep:full"]);
    }

    #[test]
    fn parses_git_name_status_and_uses_new_path_for_rename() {
        let files = parse_name_status_z(
            b"M\0src/lib.rs\0R100\0src/old.rs\0src/new.rs\0",
            GitChangeScope::Staged,
        )
        .expect("name-status should parse");

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].status, "M");
        assert_eq!(files[1].status, "R100");
        assert_eq!(files[1].path, PathBuf::from("src/new.rs"));
        assert_eq!(files[1].old_path, Some(PathBuf::from("src/old.rs")));
        assert!(files[1].is_rust);
        assert_eq!(files[1].scope_changes[0].scope, GitChangeScope::Staged);
    }

    #[test]
    fn parses_git_numstat_for_text_rename_binary_and_utf8_paths() {
        let records = parse_numstat_z(
            b"3\t1\tsrc/lib.rs\0\
              0\t0\t\0src/old.rs\0src/new.rs\0\
              -\t-\tassets/blob.bin\0\
              2\t0\tsrc/\xe5\xa4\x89\xe6\x9b\xb4.rs\0",
        )
        .expect("numstat should parse");

        assert_eq!(records.len(), 4);
        assert_eq!(records[0].additions, Some(3));
        assert_eq!(records[0].deletions, Some(1));
        assert_eq!(records[0].line_count_status, LineCountStatus::Counted);
        assert_eq!(records[1].old_path, Some(PathBuf::from("src/old.rs")));
        assert_eq!(records[1].path, PathBuf::from("src/new.rs"));
        assert_eq!(records[2].line_count_status, LineCountStatus::Binary);
        assert_eq!(records[2].additions, None);
        assert_eq!(records[3].path, PathBuf::from("src/\u{5909}\u{66f4}.rs"));
    }

    #[test]
    fn snapshot_evidence_is_incomplete_when_the_tool_did_not_finish() {
        assert_eq!(
            evidence_for_artifact_status(ArtifactStatus::ToolFailed),
            EvidenceLevel::Incomplete
        );
        assert_eq!(
            evidence_for_artifact_status(ArtifactStatus::TimedOut),
            EvidenceLevel::Incomplete
        );
        assert_eq!(
            evidence_for_artifact_status(ArtifactStatus::CompletedWithDiagnostics),
            EvidenceLevel::ToolConfirmed
        );
    }

    #[test]
    fn parses_unified_diff_hunks() {
        let hunks = parse_unified_hunks(
            "diff --git a/src/lib.rs b/src/lib.rs\n+++ b/src/lib.rs\n@@ -12,2 +13,4 @@ fn demo\n",
        );
        let hunk = &hunks[&PathBuf::from("src/lib.rs")][0];
        assert_eq!(hunk.old_start, 12);
        assert_eq!(hunk.old_count, 2);
        assert_eq!(hunk.new_start, 13);
        assert_eq!(hunk.new_count, 4);
        assert_eq!(hunk.header.as_deref(), Some("fn demo"));
    }

    #[test]
    fn unified_diff_content_cannot_impersonate_file_headers() {
        let hunks = parse_unified_hunks(
            "diff --git a/src/lib.rs b/src/lib.rs\n\
             --- a/src/lib.rs\n\
             +++ b/src/lib.rs\n\
             @@ -1 +1 @@ first\n\
             --- a/not-a-header.rs\n\
             +++ b/not-a-header.rs\n\
             @@ -8 +8 @@ second\n",
        );

        assert_eq!(hunks[&PathBuf::from("src/lib.rs")].len(), 2);
        assert!(!hunks.contains_key(&PathBuf::from("not-a-header.rs")));
    }

    #[test]
    fn truncates_utf8_without_splitting_a_character() {
        let original = "変更されたRustコード".to_string();
        for max_bytes in 0..=original.len() + 2 {
            let mut text = original.clone();
            let omitted = truncate_utf8(&mut text, max_bytes);
            assert!(text.is_char_boundary(text.len()));
            assert!(text.len() <= max_bytes);
            assert_eq!(omitted, original.len() - text.len());
        }
    }

    #[test]
    fn summary_text_neutralizes_terminal_control_characters() {
        assert_eq!(
            display_path(Path::new("src/line\nbreak.rs")),
            "src/line\\nbreak.rs"
        );
        assert_eq!(
            single_line("message\n\u{1b}[31mred"),
            "message \\u{1b}[31mred"
        );
    }

    #[test]
    fn parses_target_and_dependency_metadata() {
        let value = serde_json::json!({
            "id": "demo 0.1.0 (path+file:///tmp/demo)",
            "name": "demo",
            "version": "0.1.0",
            "manifest_path": "/tmp/demo/Cargo.toml",
            "targets": [{
                "name": "demo",
                "kind": ["lib"],
                "src_path": "/tmp/demo/src/lib.rs",
                "edition": "2021",
                "required-features": ["full"]
            }],
            "features": {"full": ["dep:full"]},
            "dependencies": [{"name": "serde"}, {"name": "serde"}]
        });

        let package = parse_package(&value).expect("package should parse");
        assert_eq!(package.id, "demo 0.1.0 (path+file:///tmp/demo)");
        assert_eq!(package.edition.as_deref(), Some("2021"));
        assert_eq!(package.dependencies, vec!["serde"]);
        assert_eq!(package.target_details[0].kind, vec!["lib"]);
        assert_eq!(package.target_details[0].required_features, vec!["full"]);
    }

    #[test]
    fn rejects_empty_compare_ref() {
        let error = git_changed_files(Path::new("."), " ").expect_err("empty ref must fail");
        assert!(error.to_string().contains("compare ref"));
    }

    #[test]
    fn rejects_option_like_compare_ref() {
        let error = git_changed_files(Path::new("."), "--output=/tmp/unsafe")
            .expect_err("option-like revisions must be rejected before invoking Git");
        assert!(error.to_string().contains("simple Git revision"));
    }

    #[test]
    fn rejects_zero_budget() {
        let error = build_rust_context(".", None, 0).expect_err("zero budget must fail");
        assert!(error.to_string().contains("budget"));
    }

    #[test]
    fn rejects_all_features_without_diagnostics() {
        let mut options = RustContextOptions::new(None, 8_000);
        options.all_features = true;
        let error = build_rust_context_with_config(".", options)
            .expect_err("feature selection without compiler diagnostics must fail");
        assert!(error
            .to_string()
            .contains("--all-features requires --diagnostics"));
    }

    #[test]
    fn parses_primary_cargo_diagnostic_span() {
        let text = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused function","code":{"code":"dead_code"},"spans":[{"file_name":"src/lib.rs","line_start":3,"column_start":5,"is_primary":true}]}}"#;
        let diagnostics = parse_cargo_diagnostics(text);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_deref(), Some("dead_code"));
        assert_eq!(diagnostics[0].file, Some(PathBuf::from("src/lib.rs")));
        assert_eq!(diagnostics[0].line, Some(3));
        assert_eq!(diagnostics[0].spans.len(), 1);
        assert!(diagnostics[0].spans[0].is_primary);
    }

    #[test]
    fn diagnostic_delta_preserves_multiplicity_and_excludes_auxiliary_notes() {
        let error = r#"{"reason":"compiler-message","message":{"level":"error","message":"mismatched types","code":{"code":"E0308"},"spans":[{"file_name":"src/lib.rs","line_start":3,"column_start":5,"is_primary":true}]}}"#;
        let note = r#"{"reason":"compiler-message","message":{"level":"failure-note","message":"try rustc --explain E0308","code":null,"spans":[]}}"#;
        let baseline = parse_cargo_diagnostics(&format!("{error}\n{error}\n{note}\n{note}"));
        let current = parse_cargo_diagnostics(error);

        let partial = compare_diagnostic_multisets(&baseline, &current);
        assert!(partial.added.is_empty());
        assert_eq!(partial.persisting.len(), 1);
        assert_eq!(partial.resolved.len(), 1);
        assert!(partial
            .resolved
            .iter()
            .all(|diagnostic| diagnostic.code.as_deref() == Some("E0308")));

        let resolved = compare_diagnostic_multisets(&baseline, &[]);
        assert_eq!(resolved.resolved.len(), 2);
        assert!(resolved
            .resolved
            .iter()
            .all(|diagnostic| diagnostic.level == "error"));
        assert_eq!(unique_primary_diagnostics(&baseline).len(), 1);
    }

    #[test]
    fn metadata_policy_does_not_claim_execution() {
        let policy = metadata_execution_policy();
        assert_eq!(policy.mode, AnalysisMode::MetadataOnly);
        assert_eq!(policy.workspace_trust, "not_required");
        assert_eq!(policy.process_network_isolation, "not_applicable");
    }

    #[test]
    fn cargo_policy_reports_unenforced_network_isolation() {
        let policy = diagnostic_execution_policy(DiagnosticProducer::CargoCheck);
        assert_eq!(policy.mode, AnalysisMode::CargoCheck);
        assert_eq!(policy.workspace_trust, "required");
        assert_eq!(policy.cargo_registry_network, "offline");
        assert_eq!(policy.process_network_isolation, "not_enforced");
        assert_eq!(policy.compiler_wrappers, "disabled");
    }

    #[cfg(unix)]
    #[test]
    fn bounded_runner_caps_output() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf 'abcdef'"]);
        let output = run_bounded_command(command, Duration::from_secs(2), 3, 3)
            .expect("bounded command should run");
        assert!(output.output_limited);
        assert_eq!(output.stdout.bytes, b"abc");
        assert_eq!(output.status.expect("status").code(), Some(0));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_runner_terminates_process_group_on_timeout() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 2"]);
        let started = Instant::now();
        let output = run_bounded_command(command, Duration::from_millis(50), 1024, 1024)
            .expect("bounded command should return after timeout");
        assert!(output.timed_out);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_runner_does_not_wait_for_orphaned_pipe_holder() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 5 & exit 0"]);
        let started = Instant::now();
        let result = run_bounded_command(command, Duration::from_millis(100), 1024, 1024);
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[cfg(unix)]
    #[test]
    fn safe_workspace_file_rejects_symlink_escape() {
        let root = tempfile::tempdir().expect("workspace tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        std::fs::write(outside.path().join("secret.rs"), "secret").expect("outside file");
        std::os::unix::fs::symlink(outside.path(), root.path().join("link"))
            .expect("symlink should be created");
        assert!(safe_workspace_file(root.path(), Path::new("link/secret.rs")).is_none());
    }
}
