mod core;
mod analyzers;
mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::core::session::{AnalysisSession, SessionManager};
use crate::core::types::{AnalysisConfig, DirectoryAnalysis};
use crate::core::config::ConfigManager;
use crate::core::memory::{MemoryManager, MemoryType};
use crate::core::preview::PreviewManager;
use crate::core::impact::{ImpactAnalyzer, ImpactConfig, OutputFormatter, RiskLevel};

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
        
        /// Show only statistics summary (compact output)
        #[arg(long)]
        stats_only: bool,
        
        /// Number of worker threads (default: 16)
        #[arg(short, long, default_value = "16")]
        threads: usize,
    },
    
    /// Analyze code changes and show their impact across the codebase
    AnalyzeImpact {
        /// Path to analyze (file or directory)
        #[arg(value_name = "PATH")]
        path: PathBuf,
        
        /// Output format (plain, json, github-comment)
        #[arg(short, long, default_value = "plain")]
        format: String,
        
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
        
        /// Include test files in analysis
        #[arg(long)]
        include_tests: bool,
        
        /// Compare against specific git reference (branch, commit, tag)
        #[arg(long)]
        compare_ref: Option<String>,
        
        /// Skip circular dependency analysis
        #[arg(long)]
        skip_circular: bool,
        
        /// Risk threshold for reporting (low, medium, high)
        #[arg(long, default_value = "low")]
        risk_threshold: String,
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
    
    /// Update a session with incremental analysis
    SessionUpdate {
        /// Session ID to update
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
        
        /// Show detailed output
        #[arg(short, long)]
        verbose: bool,
        
        /// Show what would be updated without making changes
        #[arg(long)]
        dry_run: bool,
    },

    // FILE WATCHING SYSTEM
    /// Start file watching for a session
    WatchStart {
        /// Session ID to watch
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
    },

    /// Show file watching status
    WatchStatus {
        /// Optional session ID (if not provided, shows all)
        #[arg(value_name = "SESSION_ID")]
        session_id: Option<String>,
    },

    /// Stop file watching for a session
    WatchStop {
        /// Session ID to stop watching
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
    },

    /// Stop all active file watchers
    WatchStopAll,

    // HIDDEN COMMANDS (not shown in help)
    /// Internal daemon command for file watching
    #[command(hide = true)]
    WatchDaemon {
        /// Session ID to watch
        session_id: String,
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

/// Extract summary statistics from analysis result
fn extract_summary(result: &DirectoryAnalysis) -> String {
    let mut summary = Vec::new();
    summary.push("📊 **解析結果サマリー**\n".to_string());
    
    // 基本情報
    summary.push(format!("📁 パス: {}", result.directory_path.display()));
    
    // ファイル統計
    let total_files = result.files.len();
    summary.push(format!("📄 総ファイル数: {}", total_files));
    
    // 言語別統計と総計
    let mut lang_counts = std::collections::HashMap::new();
    let mut total_functions = 0;
    let mut total_classes = 0;
    let mut total_lines = 0;
    let mut total_code_lines = 0;
    
    for file in &result.files {
        // 言語カウント
        let lang = &file.language;
        *lang_counts.entry(format!("{:?}", lang)).or_insert(0) += 1;
        
        // 機能カウント
        total_functions += file.functions.len();
        total_classes += file.classes.len();
        
        // 行数カウント
        total_lines += file.file_info.total_lines;
        total_code_lines += file.file_info.code_lines;
    }
    
    summary.push(format!("\n📈 **統計情報:**"));
    summary.push(format!("  • 総行数: {}", total_lines));
    summary.push(format!("  • コード行数: {}", total_code_lines));
    summary.push(format!("  • 関数数: {}", total_functions));
    summary.push(format!("  • クラス数: {}", total_classes));
    
    if !lang_counts.is_empty() {
        summary.push(format!("\n🗂️ **言語別:**"));
        let mut sorted_langs: Vec<_> = lang_counts.into_iter().collect();
        sorted_langs.sort_by(|a, b| a.0.cmp(&b.0));
        for (lang, count) in sorted_langs {
            summary.push(format!("  • {}: {} files", lang, count));
        }
    }
    
    summary.join("\n")
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
        Commands::Analyze { path, format, verbose, include_tests, stats_only, threads } => {
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
            
            // Check if stats_only mode is enabled
            if stats_only {
                let summary = extract_summary(&result);
                println!("{}", summary);
            } else {
                match format.as_str() {
                    "json" => {
                        let json = serde_json::to_string_pretty(&result)?;
                        println!("{}", json);
                    }
                    _ => {
                        anyhow::bail!("Unsupported output format: {}", format);
                    }
                }
            }
            
            if verbose {
                println!("✅ Analysis completed!");
            }
        }
        
        Commands::AnalyzeImpact { path, format, verbose, include_tests, compare_ref, skip_circular, risk_threshold } => {
            if verbose {
                println!("🔍 NekoCode Impact Analysis Starting...");
                println!("📂 Target: {}", path.display());
                println!("📊 Format: {}", format);
            }
            
            // Parse risk threshold
            let risk_level = match risk_threshold.as_str() {
                "low" => RiskLevel::Low,
                "medium" => RiskLevel::Medium,
                "high" => RiskLevel::High,
                _ => {
                    anyhow::bail!("Invalid risk threshold: {}. Use 'low', 'medium', or 'high'", risk_threshold);
                }
            };
            
            // Create impact configuration
            let config = ImpactConfig {
                include_tests,
                compare_ref,
                skip_circular,
                risk_threshold: risk_level,
                verbose,
            };
            
            // Create analyzer and run analysis
            let analyzer = ImpactAnalyzer::new(config);
            let result = analyzer.analyze_impact(&path).await?;
            
            // Format and output results
            match format.as_str() {
                "plain" => {
                    println!("{}", OutputFormatter::format_plain(&result));
                }
                "json" => {
                    let json = OutputFormatter::format_json(&result)?;
                    println!("{}", json);
                }
                "github-comment" => {
                    println!("{}", OutputFormatter::format_github_comment(&result));
                }
                _ => {
                    anyhow::bail!("Unsupported output format: {}. Use 'plain', 'json', or 'github-comment'", format);
                }
            }
            
            if verbose {
                println!("✅ Impact analysis completed!");
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
        
        Commands::SessionUpdate { session_id, verbose, dry_run } => {
            use nekocode_rust::commands::session_update::handle_session_update;
            let result = handle_session_update(&session_id, verbose, dry_run).await?;
            println!("{}", result);
        }

        // FILE WATCHING SYSTEM
        Commands::WatchStart { session_id } => {
            use crate::commands::watch::handle_watch_start;
            let result = handle_watch_start(&session_id)?;
            println!("{}", result);
        }

        Commands::WatchStatus { session_id } => {
            use crate::commands::watch::handle_watch_status;
            let result = handle_watch_status(session_id.as_deref())?;
            println!("{}", result);
        }

        Commands::WatchStop { session_id } => {
            use crate::commands::watch::handle_watch_stop;
            let result = handle_watch_stop(&session_id)?;
            println!("{}", result);
        }

        Commands::WatchStopAll => {
            use crate::commands::watch::handle_watch_stop_all;
            let result = handle_watch_stop_all()?;
            println!("{}", result);
        }

        Commands::WatchDaemon { session_id } => {
            use crate::commands::watch::handle_watch_daemon;
            // This is a background daemon process - don't print output to avoid noise
            if let Err(e) = handle_watch_daemon(&session_id).await {
                eprintln!("Watch daemon error: {}", e);
                std::process::exit(1);
            }
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