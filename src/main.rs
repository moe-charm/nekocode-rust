mod core;
mod analyzers;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::core::session::{AnalysisSession, SessionManager};
use crate::core::types::AnalysisConfig;
use crate::core::config::ConfigManager;
use crate::core::memory::{MemoryManager, MemoryType};
use crate::core::preview::PreviewManager;

#[derive(Parser)]
#[command(name = "nekocode-rust")]
#[command(about = "🦀 NekoCode Rust - High-performance code analysis tool")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze source code files (powered by ultra-fast Tree-sitter)
    Analyze {
        /// Path to analyze (file or directory)
        #[arg(value_name = "PATH")]
        path: PathBuf,
        
        /// Output format
        #[arg(short, long, default_value = "json")]
        format: String,
        
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
        
        /// Include test files
        #[arg(long)]
        include_tests: bool,
        
        /// Number of worker threads (default: 16)
        #[arg(short, long, default_value = "16")]
        threads: usize,
    },
    
    // SESSION MODE
    /// Create a new analysis session
    SessionCreate {
        /// Path to analyze
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
    
    /// Execute command in a session
    SessionCommand {
        /// Session ID
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
        
        /// Command to execute (stats, complexity, structure, find, include-cycles)
        #[arg(value_name = "COMMAND")]
        command: String,
        
        /// Additional arguments for the command
        #[arg(value_name = "ARGS")]
        args: Vec<String>,
    },
    
    // DIRECT EDIT
    /// Preview a replacement operation
    ReplacePreview {
        /// File to modify
        #[arg(value_name = "FILE")]
        file: PathBuf,
        
        /// Pattern to replace
        #[arg(value_name = "PATTERN")]
        pattern: String,
        
        /// Replacement text
        #[arg(value_name = "REPLACEMENT")]
        replacement: String,
    },
    
    /// Confirm a replacement operation
    ReplaceConfirm {
        /// Preview ID to confirm
        #[arg(value_name = "PREVIEW_ID")]
        preview_id: String,
    },
    
    /// Preview an insertion operation
    InsertPreview {
        /// File to modify
        #[arg(value_name = "FILE")]
        file: PathBuf,
        
        /// Line position to insert at
        #[arg(value_name = "POSITION")]
        position: u32,
        
        /// Content to insert
        #[arg(value_name = "CONTENT")]
        content: String,
    },
    
    /// Confirm an insertion operation
    InsertConfirm {
        /// Preview ID to confirm
        #[arg(value_name = "PREVIEW_ID")]
        preview_id: String,
    },
    
    /// Preview a line movement operation
    MovelinesPreview {
        /// Source file
        #[arg(value_name = "SOURCE")]
        source: PathBuf,
        
        /// Start line number
        #[arg(value_name = "START")]
        start: u32,
        
        /// Number of lines to move
        #[arg(value_name = "COUNT")]
        count: u32,
        
        /// Destination file
        #[arg(value_name = "DESTINATION")]
        destination: PathBuf,
        
        /// Target position in destination
        #[arg(value_name = "POSITION")]
        position: u32,
    },
    
    /// Confirm a line movement operation
    MovelinesConfirm {
        /// Preview ID to confirm
        #[arg(value_name = "PREVIEW_ID")]
        preview_id: String,
    },
    
    /// Preview a class movement operation
    MoveclassPreview {
        /// Session ID
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
        
        /// Symbol ID to move
        #[arg(value_name = "SYMBOL_ID")]
        symbol_id: String,
        
        /// Target file
        #[arg(value_name = "TARGET")]
        target: PathBuf,
    },
    
    /// Confirm a class movement operation
    MoveclassConfirm {
        /// Preview ID to confirm
        #[arg(value_name = "PREVIEW_ID")]
        preview_id: String,
    },
    
    // AST REVOLUTION
    /// Show AST statistics for a session
    AstStats {
        /// Session ID
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
    },
    
    /// Query AST structure
    AstQuery {
        /// Session ID
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
        
        /// Query path (e.g., "MyClass::myMethod")
        #[arg(value_name = "PATH")]
        path: String,
    },
    
    /// Analyze scope at a specific line
    ScopeAnalysis {
        /// Session ID
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
        
        /// Line number to analyze
        #[arg(value_name = "LINE")]
        line: u32,
    },
    
    /// Dump AST structure
    AstDump {
        /// Session ID
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
        
        /// Output format (tree, json)
        #[arg(value_name = "FORMAT", default_value = "tree")]
        format: String,
    },
    
    // MEMORY SYSTEM
    /// Memory system operations
    Memory {
        #[command(subcommand)]
        operation: MemoryOperation,
    },
    
    // SYSTEM
    /// Configuration management
    Config {
        #[command(subcommand)]
        operation: ConfigOperation,
    },
    
    /// List supported languages
    Languages,
}

#[derive(Subcommand)]
enum MemoryOperation {
    /// Save content to memory
    Save {
        /// Memory type (auto, memo, api, cache)
        #[arg(value_name = "TYPE")]
        memory_type: String,
        
        /// Memory name/key
        #[arg(value_name = "NAME")]
        name: String,
        
        /// Content to save (optional, can be read from stdin)
        #[arg(value_name = "CONTENT")]
        content: Option<String>,
    },
    
    /// Load content from memory
    Load {
        /// Memory type
        #[arg(value_name = "TYPE")]
        memory_type: String,
        
        /// Memory name/key
        #[arg(value_name = "NAME")]
        name: String,
    },
    
    /// List memories
    List {
        /// Optional memory type filter
        #[arg(value_name = "TYPE")]
        memory_type: Option<String>,
    },
    
    /// Show memory timeline
    Timeline {
        /// Optional memory type filter
        #[arg(value_name = "TYPE")]
        memory_type: Option<String>,
        
        /// Number of days to show
        #[arg(long, default_value = "7")]
        days: u32,
    },
}

#[derive(Subcommand)]
enum ConfigOperation {
    /// Show current configuration
    Show,
    
    /// Set configuration value
    Set {
        /// Configuration key
        #[arg(value_name = "KEY")]
        key: String,
        
        /// Configuration value
        #[arg(value_name = "VALUE")]
        value: String,
    },
}

fn main() -> Result<()> {
    // Parse CLI to get thread count first
    let cli: Cli = clap::Parser::parse();
    let threads = match &cli.command {
        Commands::Analyze { threads, .. } => *threads,
        _ => 16, // Default for other commands
    };
    
    // 🚀 Build custom tokio runtime with configurable worker threads
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(threads)
        .enable_all()
        .build()?;
    
    rt.block_on(async_main())
}

async fn async_main() -> Result<()> {
    env_logger::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Analyze { path, format, verbose, include_tests, threads } => {
            let mut config = AnalysisConfig::default();
            config.verbose_output = verbose;
            config.include_test_files = include_tests;
            
            // Create session for Tree-sitter analysis
            let mut session = AnalysisSession::with_config(config);
            
            if verbose {
                println!("🦀 NekoCode Rust Analysis Starting...");
                println!("📂 Target: {}", path.display());
                println!("⚡ Parser: TREE-SITTER 🚀");
                println!("🧵 Worker Threads: {}", threads);
            }
            
            let result = session.analyze_path(&path, include_tests).await?;
            
            match format.as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(&result)?;
                    println!("{}", json);
                }
                _ => {
                    anyhow::bail!("Unsupported output format: {}", format);
                }
            }
            
            if verbose {
                println!("✅ Analysis completed!");
            }
        }
        
        // SESSION MODE
        Commands::SessionCreate { path } => {
            let mut session_manager = SessionManager::new()?;
            let session_id = session_manager.create_session(&path).await?;
            println!("Session created: {}", session_id);
        }
        
        Commands::SessionCommand { session_id, command, args } => {
            let mut session_manager = SessionManager::new()?;
            let result = session_manager.execute_session_command(&session_id, &command, &args)?;
            println!("{}", result);
        }
        
        // DIRECT EDIT
        Commands::ReplacePreview { file, pattern, replacement } => {
            let mut preview_manager = PreviewManager::new()?;
            let preview_id = preview_manager.create_replace_preview(&file, &pattern, &replacement)?;
            let preview = preview_manager.get_preview(&preview_id).unwrap();
            println!("Preview ID: {}", preview_id);
            println!("{}", preview.preview_text);
        }
        
        Commands::ReplaceConfirm { preview_id } => {
            let mut preview_manager = PreviewManager::new()?;
            let result = preview_manager.confirm_preview(&preview_id)?;
            println!("{}", result);
        }
        
        Commands::InsertPreview { file, position, content } => {
            let mut preview_manager = PreviewManager::new()?;
            let preview_id = preview_manager.create_insert_preview(&file, position, &content)?;
            let preview = preview_manager.get_preview(&preview_id).unwrap();
            println!("Preview ID: {}", preview_id);
            println!("{}", preview.preview_text);
        }
        
        Commands::InsertConfirm { preview_id } => {
            let mut preview_manager = PreviewManager::new()?;
            let result = preview_manager.confirm_preview(&preview_id)?;
            println!("{}", result);
        }
        
        Commands::MovelinesPreview { source, start, count, destination, position } => {
            let mut preview_manager = PreviewManager::new()?;
            let preview_id = preview_manager.create_movelines_preview(&source, start, count, &destination, position)?;
            let preview = preview_manager.get_preview(&preview_id).unwrap();
            println!("Preview ID: {}", preview_id);
            println!("{}", preview.preview_text);
        }
        
        Commands::MovelinesConfirm { preview_id } => {
            let mut preview_manager = PreviewManager::new()?;
            let result = preview_manager.confirm_preview(&preview_id)?;
            println!("{}", result);
        }
        
        Commands::MoveclassPreview { session_id, symbol_id, target } => {
            let mut preview_manager = PreviewManager::new()?;
            let preview_id = preview_manager.create_moveclass_preview(&session_id, &symbol_id, &target)?;
            let preview = preview_manager.get_preview(&preview_id).unwrap();
            println!("Preview ID: {}", preview_id);
            println!("{}", preview.preview_text);
        }
        
        Commands::MoveclassConfirm { preview_id } => {
            let mut preview_manager = PreviewManager::new()?;
            let result = preview_manager.confirm_preview(&preview_id)?;
            println!("{}", result);
        }
        
        // AST REVOLUTION - Real implementations
        Commands::AstStats { session_id } => {
            let mut session_manager = SessionManager::new()?;
            let result = session_manager.handle_ast_stats(&session_id)?;
            println!("{}", result);
        }
        
        Commands::AstQuery { session_id, path } => {
            let session_manager = SessionManager::new()?;
            let result = session_manager.handle_ast_query(&session_id, &path)?;
            println!("{}", result);
        }
        
        Commands::ScopeAnalysis { session_id, line } => {
            let session_manager = SessionManager::new()?;
            let result = session_manager.handle_scope_analysis(&session_id, line)?;
            println!("{}", result);
        }
        
        Commands::AstDump { session_id, format } => {
            let session_manager = SessionManager::new()?;
            let result = session_manager.handle_ast_dump(&session_id, &format)?;
            println!("{}", result);
        }
        
        // MEMORY SYSTEM
        Commands::Memory { operation } => {
            let config = ConfigManager::new();
            let memory_manager = MemoryManager::new(config.get().memory.storage_path.clone())?;
            
            match operation {
                MemoryOperation::Save { memory_type, name, content } => {
                    let mem_type: MemoryType = memory_type.parse()?;
                    let content = content.unwrap_or_else(|| {
                        // Read from stdin if no content provided
                        use std::io::Read;
                        let mut buffer = String::new();
                        std::io::stdin().read_to_string(&mut buffer).unwrap_or_default();
                        buffer
                    });
                    let id = memory_manager.save(&name, mem_type, &content)?;
                    println!("Memory saved with ID: {}", id);
                }
                
                MemoryOperation::Load { memory_type, name } => {
                    let mem_type: MemoryType = memory_type.parse()?;
                    let entry = memory_manager.load(&name, mem_type)?;
                    println!("{}", entry.content);
                }
                
                MemoryOperation::List { memory_type } => {
                    let mem_type = if let Some(t) = memory_type {
                        Some(t.parse()?)
                    } else {
                        None
                    };
                    let entries = memory_manager.list(mem_type)?;
                    
                    println!("Memory Entries:");
                    for entry in entries {
                        println!("  {} [{}] {} - {}", 
                                entry.id, 
                                entry.memory_type, 
                                entry.name,
                                entry.created_at.format("%Y-%m-%d %H:%M:%S"));
                    }
                }
                
                MemoryOperation::Timeline { memory_type, days } => {
                    let mem_type = if let Some(t) = memory_type {
                        Some(t.parse()?)
                    } else {
                        None
                    };
                    let entries = memory_manager.timeline(mem_type, days)?;
                    
                    println!("Memory Timeline (last {} days):", days);
                    for entry in entries {
                        println!("  {} [{}] {} - {}", 
                                entry.id, 
                                entry.memory_type, 
                                entry.name,
                                entry.created_at.format("%Y-%m-%d %H:%M:%S"));
                    }
                }
            }
        }
        
        // SYSTEM
        Commands::Config { operation } => {
            let mut config_manager = ConfigManager::new();
            
            match operation {
                ConfigOperation::Show => {
                    let config_json = config_manager.show()?;
                    println!("{}", config_json);
                }
                
                ConfigOperation::Set { key, value } => {
                    config_manager.set(&key, &value)?;
                    println!("Configuration updated: {} = {}", key, value);
                }
            }
        }
        
        Commands::Languages => {
            println!("Supported Languages:");
            println!("  🟨 JavaScript (.js, .mjs, .jsx, .cjs)");
            println!("  🔷 TypeScript (.ts, .tsx)");
            println!("  🔵 C++ (.cpp, .cxx, .cc, .hpp, .hxx, .hh)");
            println!("  🔵 C (.c, .h)");
            println!("  🐍 Python (.py, .pyw, .pyi)");
            println!("  🟦 C# (.cs)");
            println!("  🐹 Go (.go)");
            println!("  🦀 Rust (.rs)");
        }
    }
    
    Ok(())
}