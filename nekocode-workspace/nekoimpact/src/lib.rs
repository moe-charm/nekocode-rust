//! Impact analysis library for NekoCode

pub mod impact;
pub mod analyzer;
pub mod cli;

pub use impact::{ImpactAnalyzer, ImpactResult, RiskLevel, ChangeType, ChangedSymbol};
pub use analyzer::AnalysisOptions;