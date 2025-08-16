//! CLI handling for nekoimpact

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nekoimpact")]
#[command(about = "Impact analysis tool for code changes", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Analyze impact of changes in a session
    Analyze {
        /// Session ID to analyze
        session_id: String,
        
        /// Output format (plain, json, github-comment, markdown)
        #[arg(short = 'f', long, default_value = "plain")]
        format: String,
        
        /// Include detailed references
        #[arg(long)]
        references: bool,
        
        /// Files to specifically analyze
        #[arg(long)]
        files: Vec<PathBuf>,
    },
    
    /// Compare two sessions for impact
    Compare {
        /// Base session ID
        #[arg(long)]
        base: String,
        
        /// Head session ID
        #[arg(long)]
        head: String,
        
        /// Output format
        #[arg(short = 'f', long, default_value = "plain")]
        format: String,
    },
    
    /// Analyze impact against Git reference
    Diff {
        /// Session ID
        session_id: String,
        
        /// Git reference to compare against (e.g., main, HEAD~1)
        #[arg(long, default_value = "main")]
        compare_ref: String,
        
        /// Output format
        #[arg(short = 'f', long, default_value = "github-comment")]
        format: String,
    },
    
    /// Generate dependency graph
    Graph {
        /// Session ID
        session_id: String,
        
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Graph format (dot, mermaid, json)
        #[arg(long, default_value = "dot")]
        graph_format: String,
    },
    
    /// List sessions with impact data
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },
}