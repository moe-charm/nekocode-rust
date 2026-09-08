use super::model::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SymbolDeltaRequest {
    pub before: PathBuf,
    pub after: PathBuf,
    pub path: Option<PathBuf>,
    pub cursor: Option<String>,
    pub budget: usize,
    pub max_items: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaObservation {
    pub packet: PathBuf,
    pub packet_id: String,
    pub status_at_capture: String,
    pub scope: SymbolScope,
    pub backend: SymbolBackend,
    pub freshness_at_capture: SymbolFreshness,
    pub reference_query: Option<SymbolQuery>,
    pub captured_references: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaEvidence {
    pub item_id: String,
    pub location: SymbolLocation,
    pub containing_symbol: Option<ContainingSymbol>,
    pub source_line: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceChange {
    pub change: String,
    pub reason: String,
    pub before: Option<DeltaEvidence>,
    pub after: Option<DeltaEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaTotals {
    pub added: Option<usize>,
    pub removed: Option<usize>,
    pub matched: Option<usize>,
    pub unresolved: Option<usize>,
    pub displayed: usize,
    pub omitted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDeltaV1 {
    pub contract_version: String,
    pub artifact_kind: String,
    pub comparison_id: String,
    pub status: String,
    pub comparison_status: String,
    pub matching_basis: String,
    pub before: DeltaObservation,
    pub after: DeltaObservation,
    pub reasons: Vec<String>,
    pub totals: DeltaTotals,
    pub changes: Vec<ReferenceChange>,
    pub next_cursor: Option<String>,
    pub budget: SymbolBudget,
    pub limitations: Vec<String>,
}
