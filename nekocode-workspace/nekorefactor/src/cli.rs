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
    /// Replace text in file
    Replace {
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
        
        /// Preview only, don't apply changes
        #[arg(long, alias = "dry-run")]
        preview: bool,
    },
    
    
    /// Create a new file with template
    CreateFile {
        /// File path to create
        file: PathBuf,
        
        /// Template to use (python-cli, rust-lib, js-module, etc.)
        #[arg(long)]
        template: Option<String>,
        
        /// Force overwrite if file exists
        #[arg(long)]
        force: bool,
    },
    
    /// Insert content into file
    Insert {
        /// File to modify
        file: PathBuf,
        
        /// Content to insert (or - for stdin)
        content: String,
        
        /// Position (start, end, or line number) - optional if using semantic options
        #[arg(required_unless_present_any = ["after_function", "before_function", "in_imports", "after_class"])]
        position: Option<String>,
        
        /// Insert after a specific function
        #[arg(long)]
        after_function: Option<String>,
        
        /// Insert before a specific function
        #[arg(long)]
        before_function: Option<String>,
        
        /// Insert in imports section
        #[arg(long)]
        in_imports: bool,
        
        /// Insert after a specific class
        #[arg(long)]
        after_class: Option<String>,
        
        /// Preview only, don't apply changes
        #[arg(long, alias = "dry-run")]
        preview: bool,
    },
    
    
    /// Move lines between files
    MoveLines {
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
        
        /// Preview only, don't apply changes
        #[arg(long, alias = "dry-run")]
        preview: bool,
    },
    
    /// Move a class or function to another file
    MoveClass {
        /// Session ID
        session_id: String,
        
        /// Symbol ID to move
        symbol_id: String,
        
        /// Target file path
        target: PathBuf,
        
        /// Update imports automatically
        #[arg(long)]
        update_imports: bool,
        
        /// Preview only, don't apply changes
        #[arg(long, alias = "dry-run")]
        preview: bool,
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
    
    /// Smart refactoring with Tree-sitter AST
    Smart {
        #[command(subcommand)]
        command: SmartCommands,
    },
}

#[derive(Subcommand)]
pub enum SmartCommands {
    /// Smart insert using AST for precise positioning
    Insert {
        /// Session ID from nekocode
        session_id: String,
        
        /// File to modify
        file: PathBuf,
        
        /// Content to insert
        content: String,
        
        /// Insert after a specific function
        #[arg(long, conflicts_with_all = &["before_function", "in_class", "in_imports", "line"])]
        after_function: Option<String>,
        
        /// Insert before a specific function
        #[arg(long, conflicts_with_all = &["after_function", "in_class", "in_imports", "line"])]
        before_function: Option<String>,
        
        /// Insert inside a specific class
        #[arg(long, conflicts_with_all = &["after_function", "before_function", "in_imports", "line"])]
        in_class: Option<String>,
        
        /// Insert in imports section
        #[arg(long, conflicts_with_all = &["after_function", "before_function", "in_class", "line"])]
        in_imports: bool,
        
        /// Line number (fallback when no semantic position)
        #[arg(long, conflicts_with_all = &["after_function", "before_function", "in_class", "in_imports"])]
        line: Option<u32>,
        
        /// Preview mode
        #[arg(long)]
        preview: bool,
    },
    
    /// Smart replace with scope awareness
    Replace {
        /// Session ID from nekocode
        session_id: String,
        
        /// File to process
        file: PathBuf,
        
        /// Pattern to search for
        pattern: String,
        
        /// Replacement text
        replacement: String,
        
        /// Limit to specific class
        #[arg(long)]
        in_class: Option<String>,
        
        /// Limit to specific function
        #[arg(long)]
        in_function: Option<String>,
        
        /// Use regex
        #[arg(long)]
        regex: bool,
        
        /// Preview mode
        #[arg(long)]
        preview: bool,
    },
    
    /// Move symbol to another file using AST
    Move {
        /// Session ID from nekocode
        session_id: String,
        
        /// Symbol path (e.g., "MyClass::method" or "function_name")
        symbol: String,
        
        /// Target file
        target: PathBuf,
        
        /// Update imports automatically
        #[arg(long)]
        update_imports: bool,
        
        /// Preview mode
        #[arg(long)]
        preview: bool,
    },
}