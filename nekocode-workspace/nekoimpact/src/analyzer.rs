//! Analysis options and configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Options for impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOptions {
    /// Session ID to analyze
    pub session_id: Option<String>,
    
    /// Base session for comparison
    pub base_session_id: Option<String>,
    
    /// Head session for comparison
    pub head_session_id: Option<String>,
    
    /// Files to specifically analyze
    pub target_files: Vec<PathBuf>,
    
    /// Output format
    pub output_format: OutputFormat,
    
    /// Include detailed references
    pub include_references: bool,
    
    /// Include risk assessment
    pub include_risk_assessment: bool,
    
    /// Verbose output
    pub verbose: bool,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            session_id: None,
            base_session_id: None,
            head_session_id: None,
            target_files: Vec::new(),
            output_format: OutputFormat::Plain,
            include_references: true,
            include_risk_assessment: true,
            verbose: false,
        }
    }
}

/// Output format for impact analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    Plain,
    Json,
    GithubComment,
    Markdown,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "plain" => Some(OutputFormat::Plain),
            "json" => Some(OutputFormat::Json),
            "github-comment" | "github" => Some(OutputFormat::GithubComment),
            "markdown" | "md" => Some(OutputFormat::Markdown),
            _ => None,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Plain => "plain",
            OutputFormat::Json => "json",
            OutputFormat::GithubComment => "github-comment",
            OutputFormat::Markdown => "markdown",
        }
    }
}