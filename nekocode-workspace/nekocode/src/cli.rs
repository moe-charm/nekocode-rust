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
    /// Create a new analysis session
    /// 
    /// Examples:
    /// 
    ///   nekocode session-create /my/project
    /// 
    ///   nekocode session-create /my/project --complete --external
    /// 
    ///   nekocode session-create /my/project --complete --format github-comment
    SessionCreate {
        /// Path to the project directory
        path: PathBuf,
        
        /// Session name (optional)
        #[arg(short, long)]
        name: Option<String>,
        
        /// Run complete analysis with dead code detection
        #[arg(long, help = "Run full analysis including dead code detection")]
        complete: bool,
        
        /// Output format for complete analysis (text, json, github-comment, csv, summary)
        #[arg(long, default_value = "text")]
        format: String,
        
        /// Save complete analysis report to file
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Minimum confidence threshold for dead code detection (0-100)
        #[arg(long, default_value = "60")]
        min_confidence: u8,
        
        /// Use external tools for 90%+ accuracy (strongly recommended with --complete)
        /// Without this: 60% accuracy with many false positives
        /// With this: 90%+ accuracy using cargo clippy, vulture, etc.
        #[arg(long, help = "Use external tools for accurate dead code detection (90%+ vs 60%)")]
        external: bool,
    },
    
    /// Update an existing session
    SessionUpdate {
        /// Session ID to update
        session_id: String,
        
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Refresh session analysis with smart level detection
    /// 
    /// Examples:
    /// 
    ///   nekocode refresh abc123                    # Smart auto-detection
    /// 
    ///   nekocode refresh abc123 --level project   # Force L2 project analysis
    /// 
    ///   nekocode refresh abc123 --deadcode        # Force L3 deadcode analysis
    /// 
    ///   nekocode refresh abc123 --security        # Force L4 security analysis
    Refresh {
        /// Session ID to refresh
        session_id: String,
        
        /// Analysis level (file, project, cross, advanced, smart)
        #[arg(short, long)]
        level: Option<String>,
        
        /// Force L2: Refresh dependencies and project structure
        #[arg(long)]
        deps: bool,
        
        /// Force L3: Refresh dead code detection
        #[arg(long)]
        deadcode: bool,
        
        /// Force L3: Check for circular dependencies
        #[arg(long)]
        circular: bool,
        
        /// Force L3: Detect code duplications
        #[arg(long)]
        duplicates: bool,
        
        /// Force L4: Run security analysis
        #[arg(long)]
        security: bool,
        
        /// Force L4: Calculate quality metrics
        #[arg(long)]
        quality: bool,
        
        /// Refresh specific file only (L1 level)
        #[arg(short, long)]
        file: Option<String>,
        
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
        
        /// Use external tools where applicable
        #[arg(long)]
        external: bool,
        
        /// Output format (text, json, github-comment)
        #[arg(long, default_value = "text")]
        format: String,
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
    
    /// Show session history
    SessionHistory {
        /// Show all details
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// AST operations on a session
    AstStats {
        /// Session ID (optional - uses last session if not provided)
        #[arg(short, long)]
        session_id: Option<String>,
    },
    
    /// Query AST by path (e.g., "MyClass::myMethod")
    AstQuery {
        /// Query path
        path: String,
        
        /// Session ID (optional - uses last session if not provided)
        #[arg(short, long)]
        session_id: Option<String>,
    },
    
    /// Dump AST tree
    AstDump {
        /// Session ID (optional - uses last session if not provided)
        #[arg(short, long)]
        session_id: Option<String>,
        
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
        /// Line number to analyze
        line: u32,
        
        /// Session ID (optional - uses last session if not provided)
        #[arg(short, long)]
        session_id: Option<String>,
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
    
    /// Detect dead code in a session
    /// 
    /// First create a session, then analyze it:
    /// 
    ///   1. Create session: nekocode session-create /path/to/project
    ///      → Returns session ID (e.g., abc123)
    /// 
    ///   2. Analyze dead code: nekocode deadcode abc123 --external
    /// 
    /// Or do both at once:
    /// 
    ///   nekocode session-create /path/to/project --complete --external
    /// 
    /// Examples:
    /// 
    ///   nekocode deadcode abc123 --external
    ///   nekocode deadcode abc123 --format github-comment --min-confidence 90
    Deadcode {
        /// Session ID to analyze (optional - uses last session if not provided)
        #[arg(short, long)]
        session_id: Option<String>,
        
        /// Use external tools for higher accuracy (90%+ vs 60% internal)
        /// Recommended: cargo clippy (Rust), vulture (Python), staticcheck (Go)
        #[arg(long, help = "Use external tools for 90%+ accuracy (recommended)")]
        external: bool,
        
        /// Output format (text, json, github-comment, csv, summary)
        #[arg(short, long, default_value = "text")]
        format: String,
        
        /// Minimum confidence threshold (0-100)
        #[arg(long, default_value = "60")]
        min_confidence: u8,
        
        /// Save report to file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
}