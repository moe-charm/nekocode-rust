//! CLI handling for nekorefactor

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nekorefactor")]
#[command(about = "Code refactoring tool", long_about = None)]
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
    /// Preview text replacement
    ReplacePreview {
        /// File to process
        file: PathBuf,
        
        /// Pattern to search for
        pattern: String,
        
        /// Replacement text
        replacement: String,
        
        /// Use regex pattern
        #[arg(long)]
        regex: bool,
        
        /// Case insensitive search
        #[arg(short = 'i', long)]
        ignore_case: bool,
        
        /// Match whole words only
        #[arg(short = 'w', long)]
        whole_word: bool,
    },
    
    /// Confirm and apply a preview
    ReplaceConfirm {
        /// Preview ID to confirm
        preview_id: String,
    },
    
    /// Preview inserting content
    InsertPreview {
        /// File to modify
        file: PathBuf,
        
        /// Position (start, end, or line number)
        position: String,
        
        /// Content to insert (or - for stdin)
        content: String,
    },
    
    /// Preview moving lines
    MoveLinesPreview {
        /// Source file
        source: PathBuf,
        
        /// Starting line number
        start_line: u32,
        
        /// Number of lines to move
        line_count: u32,
        
        /// Destination file
        destination: PathBuf,
        
        /// Line to insert at
        insert_line: u32,
    },
    
    /// Preview moving a class/function
    MoveClassPreview {
        /// Session ID
        session_id: String,
        
        /// Symbol ID to move
        symbol_id: String,
        
        /// Target file path
        target: PathBuf,
        
        /// Update imports automatically
        #[arg(long)]
        update_imports: bool,
    },
    
    /// Confirm and apply a move class preview
    MoveClassConfirm {
        /// Preview ID to confirm
        preview_id: String,
    },
    
    /// List all previews
    ListPreviews {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
        
        /// Show only pending previews
        #[arg(long)]
        pending: bool,
    },
    
    /// Extract function to new file
    ExtractFunction {
        /// Session ID
        session_id: String,
        
        /// Function name or ID
        function: String,
        
        /// Target file
        target: PathBuf,
        
        /// Dry run (don't actually move)
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Split file into multiple files
    SplitFile {
        /// File to split
        file: PathBuf,
        
        /// Split by (functions, classes, size)
        #[arg(long, default_value = "classes")]
        by: String,
        
        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}