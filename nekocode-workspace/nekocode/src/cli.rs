//! CLI interface for NekoCode

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// NekoCode - Core analysis engine with Tree-sitter support
#[derive(Parser, Debug)]
#[command(name = "nekocode")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Analyze a file or directory without creating a session
    Analyze {
        /// Path to analyze
        path: PathBuf,
        
        /// Output format (text, json, stats)
        #[arg(short, long, default_value = "text")]
        output: String,
        
        /// Show only statistics
        #[arg(long)]
        stats_only: bool,
        
        /// Language to use (auto-detect if not specified)
        #[arg(short, long)]
        language: Option<String>,
        
        /// Build AST tree
        #[arg(long)]
        ast: bool,
    },
    
    /// Create a new analysis session
    SessionCreate {
        /// Path to the project directory
        path: PathBuf,
        
        /// Session name (optional)
        #[arg(short, long)]
        name: Option<String>,
    },
    
    /// Update an existing session
    SessionUpdate {
        /// Session ID to update
        session_id: String,
        
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// List all sessions
    SessionList {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },
    
    /// Delete a session
    SessionDelete {
        /// Session ID to delete
        session_id: String,
    },
    
    /// Show session information
    SessionInfo {
        /// Session ID
        session_id: String,
    },
    
    /// AST operations on a session
    AstStats {
        /// Session ID
        session_id: String,
    },
    
    /// Query AST by path (e.g., "MyClass::myMethod")
    AstQuery {
        /// Session ID
        session_id: String,
        
        /// Query path
        path: String,
    },
    
    /// Dump AST tree
    AstDump {
        /// Session ID
        session_id: String,
        
        /// Output format (tree, json, flat)
        #[arg(short, long, default_value = "tree")]
        format: String,
        
        /// Limit output lines
        #[arg(short, long)]
        limit: Option<usize>,
        
        /// Force full output (ignore token limit)
        #[arg(long)]
        force: bool,
    },
    
    /// Scope analysis for a specific line
    ScopeAnalysis {
        /// Session ID
        session_id: String,
        
        /// Line number to analyze
        line: u32,
    },
    
    /// Export session data
    Export {
        /// Session ID
        session_id: String,
        
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
        
        /// Export format (json, csv)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    
    /// Import session data
    Import {
        /// Input file path
        input: PathBuf,
        
        /// Session ID (create new if not specified)
        #[arg(short, long)]
        session_id: Option<String>,
    },
}