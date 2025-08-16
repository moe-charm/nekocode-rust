//! CLI interface for NekoInc

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// NekoInc - Incremental analysis and file watching tool
#[derive(Parser, Debug)]
#[command(name = "nekoinc")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize incremental tracking for a session
    Init {
        /// Session ID to initialize
        session_id: String,
    },
    
    /// Detect and analyze changes in a session
    Update {
        /// Session ID to update
        session_id: String,
        
        /// Show detailed change information
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Start watching a session for changes
    Watch {
        /// Session ID to watch
        session_id: String,
        
        /// Debounce time in milliseconds
        #[arg(short, long, default_value = "500")]
        debounce: u64,
    },
    
    /// Stop watching a session
    StopWatch {
        /// Session ID to stop watching
        session_id: String,
    },
    
    /// Show watch status for sessions
    Status {
        /// Session ID (show all if not specified)
        session_id: Option<String>,
    },
    
    /// Stop all active watchers
    StopAll,
    
    /// Compare two sessions to find differences
    Diff {
        /// First session ID
        session1: String,
        
        /// Second session ID
        session2: String,
        
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    
    /// Show incremental analysis statistics
    Stats {
        /// Session ID
        session_id: String,
    },
    
    /// Reset incremental tracking for a session
    Reset {
        /// Session ID to reset
        session_id: String,
    },
    
    /// Export change history for a session
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
}