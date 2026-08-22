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
    /// Index a Rust workspace using Cargo metadata.
    ///
    /// This is the Rust-first MVP entry point. It records workspace/package
    /// structure without pretending to replace rustc or rust-analyzer.
    Index {
        /// Path to a Rust workspace or Cargo.toml
        path: PathBuf,

        /// Write the JSON snapshot to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Build a bounded, Git-diff-focused Rust context pack.
    Context {
        /// Path to a Rust workspace or Cargo.toml
        path: PathBuf,

        /// Git ref to compare with HEAD (for example, origin/main)
        #[arg(long)]
        compare_ref: Option<String>,

        /// Approximate token budget for the JSON context pack
        #[arg(long, default_value = "8000")]
        budget: usize,

        /// Include cargo check diagnostics from all workspace targets
        #[arg(long)]
        diagnostics: bool,

        /// Write the JSON context pack to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

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
        
        /// Legacy heuristic filter threshold (0-100; not a measured accuracy)
        #[arg(long, default_value = "60")]
        min_confidence: u8,
        
        /// Use legacy external dead-code tools; results are experimental and unbenchmarked
        #[arg(long, help = "Use legacy external dead-code tools (experimental)")]
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

        /// Clear CLI session memory (history only)
        #[arg(long, help = "Clear CLI session history (keeps current session unless cleared separately)")]
        clear: bool,
    },

    /// Prune stored sessions from disk
    ///
    /// Examples:
    ///   nekocode session-prune --older-than 14     # Delete sessions not used in 14+ days
    ///   nekocode session-prune --stale             # Delete sessions whose path no longer exists
    ///   nekocode session-prune --keep 5            # Keep 5 most recent, delete others
    ///   nekocode session-prune --all               # Delete all sessions
    SessionPrune {
        /// Delete sessions not accessed for N days
        #[arg(long, value_name = "DAYS")]
        older_than: Option<i64>,

        /// Delete sessions whose project path no longer exists
        #[arg(long)]
        stale: bool,

        /// Keep only the N most recent sessions, delete others
        #[arg(long, value_name = "N")]
        keep: Option<usize>,

        /// Delete all sessions
        #[arg(long)]
        all: bool,
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
        
        /// Use legacy external dead-code tools; results are experimental and unbenchmarked
        #[arg(long, help = "Use legacy external dead-code tools (experimental)")]
        external: bool,
        
        /// Output format (text, json, github-comment, csv, summary)
        #[arg(short, long, default_value = "text")]
        format: String,
        
        /// Legacy heuristic filter threshold (0-100; not a measured accuracy)
        #[arg(long, default_value = "60")]
        min_confidence: u8,
        
        /// Save report to file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Save memory content
    MemorySave {
        /// Memory type (auto, memo, api, cache)
        memory_type: String,
        
        /// Memory name
        name: String,
        
        /// Content to save
        content: String,
    },
    
    /// Load memory content
    MemoryLoad {
        /// Memory type (auto, memo, api, cache)
        memory_type: String,
        
        /// Memory name
        name: String,
    },
    
    /// List memories
    MemoryList {
        /// Filter by memory type (optional)
        #[arg(short = 't', long)]
        memory_type: Option<String>,
    },
    
    /// Show memory timeline
    MemoryTimeline {
        /// Number of days to show
        #[arg(short, long, default_value = "7")]
        days: i64,
        
        /// Filter by memory type (optional)
        #[arg(short = 't', long)]
        memory_type: Option<String>,
    },
    
    /// Show configuration
    ConfigShow,
    
    /// Set configuration value
    ConfigSet {
        /// Configuration key
        key: String,
        
        /// Configuration value
        value: String,
    },
}
