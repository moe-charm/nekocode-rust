use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One investigation or a replay of a previously captured investigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SymbolContextRequest {
    pub path: Option<PathBuf>,
    pub at: Option<String>,
    pub symbol: Option<String>,
    pub packet: Option<PathBuf>,
    pub save_packet: Option<PathBuf>,
    pub item: Option<String>,
    pub cursor: Option<String>,
    pub budget: usize,
    pub max_items: usize,
    pub timeout_seconds: u64,
    pub all_features: bool,
    pub allow_build_scripts: bool,
    pub text_candidates: bool,
    pub scan_profile: Option<String>,
}

impl Default for SymbolContextRequest {
    fn default() -> Self {
        Self {
            path: None,
            at: None,
            symbol: None,
            packet: None,
            save_packet: None,
            item: None,
            cursor: None,
            budget: 8_000,
            max_items: 8,
            timeout_seconds: 60,
            all_features: false,
            allow_build_scripts: false,
            text_candidates: false,
            scan_profile: None,
        }
    }
}

/// Positions are one-based Unicode scalar columns in the captured source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolLocation {
    pub path: PathBuf,
    pub start: SymbolPosition,
    pub end: SymbolPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolCandidate {
    pub name: String,
    pub kind: Option<u32>,
    pub location: SymbolLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolTarget {
    pub requested_at: Option<String>,
    pub requested_symbol: Option<String>,
    pub resolution: String,
    pub location: Option<SymbolLocation>,
    pub candidates: Vec<SymbolCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolScope {
    pub workspace_root: PathBuf,
    pub features: String,
    pub cfg_test: String,
    pub build_scripts: bool,
    pub proc_macros: bool,
    pub tests_executed: bool,
    pub hop_limit: usize,
    pub external_configuration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolBackend {
    pub name: String,
    pub version: Option<String>,
    pub health: Option<String>,
    pub message: Option<String>,
    pub quiescent: bool,
    pub readiness_observed: bool,
    pub startup_ms: u64,
    pub observation_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolQuery {
    pub method: String,
    pub status: String,
    pub result_count: Option<usize>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolFreshness {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scans: Option<ScanComparison>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<InputVerification>,
    pub state: String,
    pub source_state: String,
    pub backend_synchronization: String,
    pub checked_inputs: usize,
    pub changed_inputs: Vec<PathBuf>,
    pub input_scan_complete: bool,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainingSymbol {
    pub name: String,
    pub kind: Option<u32>,
    pub location: SymbolLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolItem {
    pub id: String,
    pub relation: String,
    pub backend: String,
    pub source: String,
    pub location: SymbolLocation,
    pub reason: String,
    /// Verbatim captured source, never path-sanitized or terminal-rendered.
    pub code: String,
    pub excerpt_start_line: u32,
    pub excerpt_end_line: u32,
    pub containing_symbol: Option<ContainingSymbol>,
    pub detail: Option<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolTotals {
    pub observed: usize,
    pub retained: usize,
    pub displayed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolOmission {
    pub kind: String,
    pub reason: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolContinuation {
    pub packet: Option<PathBuf>,
    pub next_cursor: Option<String>,
    pub expandable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolBudget {
    pub requested_tokens: usize,
    pub max_bytes: usize,
    pub serialized_bytes: usize,
    pub exceeded: bool,
}

/// Public response. Exact source text is deliberately not sanitized as paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolContextV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<Vec<CoverageExplanation>>,
    pub contract_version: String,
    pub artifact_kind: String,
    pub packet_id: String,
    pub status: String,
    pub target: SymbolTarget,
    pub scope: SymbolScope,
    pub backend: SymbolBackend,
    pub queries: Vec<SymbolQuery>,
    pub freshness: SymbolFreshness,
    pub items: Vec<SymbolItem>,
    pub totals: SymbolTotals,
    pub omissions: Vec<SymbolOmission>,
    pub continuation: SymbolContinuation,
    pub budget: SymbolBudget,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageExplanation {
    pub area: String,
    pub requested: String,
    pub observed: String,
    pub verification: String,
    pub limitation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputIssue {
    pub path: PathBuf,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputVerification {
    pub basis: String,
    pub verdict: String,
    pub matched: usize,
    pub modified: usize,
    pub missing: usize,
    pub unreadable: usize,
    pub unobserved: usize,
    pub newly_observed: usize,
    pub captured_mismatches: usize,
    pub baseline_scan_complete: bool,
    pub current_scan_complete: bool,
    pub issues: Vec<InputIssue>,
    pub issues_omitted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanComparison {
    pub baseline: Option<ScanReport>,
    pub current: Option<ScanReport>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub profile: String,
    pub max_files: usize,
    pub max_entries: usize,
    pub max_bytes: u64,
    pub max_file_bytes: u64,
    pub examined_entries: usize,
    pub hashed_files: usize,
    pub hashed_bytes: u64,
    pub complete: bool,
    pub issues: Vec<InputIssue>,
    pub issues_omitted: usize,
}
