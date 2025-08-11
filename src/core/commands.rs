//! Command system for NekoCode Rust
//! 
//! This module provides command structures and processing capabilities
//! for different analysis operations.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::core::types::{DirectoryAnalysis, AnalysisConfig};
use crate::core::session::AnalysisSession;

/// Available commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Analyze {
        path: PathBuf,
        output_format: String,
        include_tests: bool,
        config: AnalysisConfig,
    },
}

/// Command processor
pub struct CommandProcessor {
    session: AnalysisSession,
}

impl CommandProcessor {
    pub fn new() -> Self {
        Self {
            session: AnalysisSession::new(),
        }
    }
    
    pub fn with_session(session: AnalysisSession) -> Self {
        Self { session }
    }
    
    /// Execute a command and return the result
    pub async fn execute(&mut self, command: Command) -> Result<CommandResult> {
        match command {
            Command::Analyze { path, output_format, include_tests, config } => {
                self.session = AnalysisSession::with_config(config);
                let analysis = self.session.analyze_path(&path, include_tests).await?;
                
                Ok(CommandResult::Analysis {
                    analysis,
                    format: output_format,
                })
            }
        }
    }
}

/// Command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandResult {
    Analysis {
        analysis: DirectoryAnalysis,
        format: String,
    },
}

impl CommandResult {
    /// Format the result for output
    pub fn format_output(&self) -> Result<String> {
        match self {
            CommandResult::Analysis { analysis, format } => {
                match format.as_str() {
                    "json" => Ok(serde_json::to_string_pretty(analysis)?),
                    _ => anyhow::bail!("Unsupported output format: {}", format),
                }
            }
        }
    }
}

impl Default for CommandProcessor {
    fn default() -> Self {
        Self::new()
    }
}