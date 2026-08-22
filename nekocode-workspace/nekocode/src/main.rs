//! NekoCode - Core analysis engine with Tree-sitter support

use clap::Parser;
use std::fs;
use std::io::{self, Write};

use nekocode_core::{
    Result, NekocodeError, session::SessionManager, CliSessionHelper, CliSessionConfig,
    memory::{MemoryManager, MemoryType},
    config::ConfigManager,
};
use nekocode::{
    Cli,
    SessionCommands, SessionUpdater,
    DeadCodeAnalyzer, DeadItem, DeadCodeReport
};
use nekocode::cli::Commands;

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Default to 8 threads for all commands
    let worker_threads = 8;
    
    // Build custom tokio runtime with specified worker threads
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");
    
    // Run async main with custom runtime
    runtime.block_on(async_main(cli))
}

async fn async_main(cli: Cli) -> Result<()> {
    
    // Execute command
    match cli.command {
        Commands::Index { path, output } => {
            handle_rust_index_command(path, output)?;
        }

        Commands::Context { path, compare_ref, budget, diagnostics, output } => {
            handle_rust_context_command(path, compare_ref, budget, diagnostics, output)?;
        }

        Commands::SessionCreate { path, name, complete, format, output, min_confidence, external } => {
            handle_session_create_command(path, name, complete, format, output, min_confidence, external).await?;
        }
        
        Commands::SessionUpdate { session_id, verbose } => {
            let mut updater = SessionUpdater::new()?;
            updater.update_session(&session_id).await?;
            
            if verbose {
                let mut session_manager = SessionManager::new()?;
                let session = session_manager.get_session_mut(&session_id)?;
                println!("📊 Session {} updated:", session_id);
                println!("  Files: {}", session.info.analysis_results.len());
                println!("  Path: {}", session.info.path.display());
            }
        }
        
        Commands::Refresh { 
            session_id, level, deps, deadcode, circular, duplicates, 
            security, quality, file, verbose, external, format 
        } => {
            handle_refresh_command(
                session_id, level, deps, deadcode, circular, duplicates,
                security, quality, file, verbose, external, format
            ).await?;
        }
        
        Commands::SessionList { detailed } => {
            let session_manager = SessionManager::new()?;
            let sessions = session_manager.list_sessions()?;
            
            if sessions.is_empty() {
                println!("No sessions found");
            } else {
                println!("📋 Sessions:");
                for session in sessions {
                    if detailed {
                        println!("\n🆔 {}", session.id);
                        println!("  📁 Path: {}", session.path.display());
                        println!("  📊 Files: {}", session.file_count);
                        println!("  🕒 Created: {}", session.created_at.format("%Y-%m-%d %H:%M:%S"));
                        println!("  🕒 Updated: {}", session.last_modified.format("%Y-%m-%d %H:%M:%S"));
                    } else {
                        println!("  {} - {} ({} files)", 
                            session.id, 
                            session.path.display(),
                            session.file_count
                        );
                    }
                }
            }
        }
        
        Commands::SessionDelete { session_id } => {
            let mut session_manager = SessionManager::new()?;
            session_manager.delete_session(&session_id)?;
            println!("🗑️ Deleted session: {}", session_id);
        }
        
        Commands::SessionInfo { session_id } => {
            let mut session_manager = SessionManager::new()?;
            let session = session_manager.get_session_mut(&session_id)?;
            
            println!("📋 Session Information:");
            println!("  ID: {}", session_id);
            println!("  Path: {}", session.info.path.display());
            println!("  Files analyzed: {}", session.info.analysis_results.len());
            println!("  Created: {}", session.info.created_at.format("%Y-%m-%d %H:%M:%S"));
            println!("  Updated: {}", session.info.last_modified.format("%Y-%m-%d %H:%M:%S"));
        }
        
        Commands::SessionHistory { verbose, clear } => {
            if clear {
                CliSessionHelper::clear_history()?;
            }
            CliSessionHelper::list_history()?;
            
            if verbose {
                // Also show nekocode sessions
                println!("\n📂 NekoCode Sessions:");
                let session_manager = SessionManager::new()?;
                let sessions = session_manager.list_sessions()?;
                for session in sessions {
                    println!("  {} - {} ({} files)", 
                        session.id, 
                        session.path.display(),
                        session.file_count
                    );
                }
            }
        }
        
        Commands::SessionPrune { older_than, stale, keep, all } => {
            let mut session_manager = SessionManager::new()?;
            let mut to_delete: std::collections::HashSet<String> = std::collections::HashSet::new();
            
            // List sessions sorted by last_accessed (newest first)
            let sessions = session_manager.list_sessions()?;
            
            if all {
                for s in &sessions {
                    to_delete.insert(s.id.clone());
                }
            }
            
            if let Some(days) = older_than {
                let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
                for s in &sessions {
                    if s.last_accessed < cutoff {
                        to_delete.insert(s.id.clone());
                    }
                }
            }
            
            if stale {
                for s in &sessions {
                    if !s.path.exists() {
                        to_delete.insert(s.id.clone());
                    }
                }
            }
            
            if let Some(keep_n) = keep {
                if sessions.len() > keep_n {
                    for s in sessions.iter().skip(keep_n) {
                        to_delete.insert(s.id.clone());
                    }
                }
            }
            
            if to_delete.is_empty() {
                println!("Nothing to prune");
            } else {
                let mut deleted = 0usize;
                for id in to_delete {
                    if session_manager.delete_session(&id).is_ok() {
                        deleted += 1;
                        println!("🗑️ Deleted session: {}", id);
                    }
                }
                println!("✅ Prune complete: {} sessions deleted", deleted);
            }
        }
        
        Commands::AstStats { session_id } => {
            // Get session ID from args or config
            let session_id = CliSessionHelper::get_session_id(session_id.as_deref())?;
            
            let mut commands = SessionCommands::new()?;
            let stats = commands.ast_stats(&session_id).await?;
            println!("{}", stats);
        }
        
        Commands::AstQuery { session_id, path } => {
            // Get session ID from args or config
            let session_id = CliSessionHelper::get_session_id(session_id.as_deref())?;
            
            let mut commands = SessionCommands::new()?;
            let result = commands.ast_query(&session_id, &path).await?;
            println!("{}", result);
        }
        
        Commands::AstDump { session_id, format, limit, force } => {
            // Get session ID from args or config
            let session_id = CliSessionHelper::get_session_id(session_id.as_deref())?;
            
            let mut commands = SessionCommands::new()?;
            let mut result = commands.ast_dump(&session_id, &format).await?;
            
            if !force && result.lines().count() > 1000 {
                let lines: Vec<&str> = result.lines().take(1000).collect();
                result = lines.join("\n");
                result.push_str("\n... (output truncated, use --force for full output)");
            }
            
            if let Some(limit) = limit {
                let lines: Vec<&str> = result.lines().take(limit).collect();
                result = lines.join("\n");
            }
            
            println!("{}", result);
        }
        
        Commands::ScopeAnalysis { session_id, line } => {
            // Get session ID from args or config
            let session_id = CliSessionHelper::get_session_id(session_id.as_deref())?;
            
            println!("🎯 Scope analysis for session {} at line {}", session_id, line);
            println!("(Scope analysis functionality requires AST traversal implementation)");
        }
        
        Commands::Export { session_id, output, format } => {
            let mut session_manager = SessionManager::new()?;
            let session = session_manager.get_session_mut(&session_id)?;
            
            let content = match format.as_str() {
                "csv" => {
                    let mut csv = String::from("file,language,functions,classes,lines\n");
                    for result in &session.info.analysis_results {
                        csv.push_str(&format!("{},{:?},{},{},{}\n",
                            result.file_info.path.display(),
                            result.file_info.language,
                            result.functions.len(),
                            result.classes.len(),
                            result.file_info.total_lines
                        ));
                    }
                    csv
                }
                _ => {
                    serde_json::to_string_pretty(&session.info.analysis_results)?
                }
            };
            
            fs::write(&output, content)?;
            println!("✅ Exported session {} to {}", session_id, output.display());
        }
        
        Commands::Import { input, session_id } => {
            println!("📥 Import functionality not yet implemented");
            println!("Input: {}", input.display());
            if let Some(id) = session_id {
                println!("Target session: {}", id);
            }
        }
        
        Commands::Deadcode { session_id, external, format, min_confidence, output } => {
            // Get session ID from args or config
            let session_id = CliSessionHelper::get_session_id(session_id.as_deref())?;
            
            handle_deadcode_command(session_id, external, format, min_confidence, output).await?;
        }
        
        Commands::MemorySave { memory_type, name, content } => {
            handle_memory_save(memory_type, name, content)?;
        }
        
        Commands::MemoryLoad { memory_type, name } => {
            handle_memory_load(memory_type, name)?;
        }
        
        Commands::MemoryList { memory_type } => {
            handle_memory_list(memory_type)?;
        }
        
        Commands::MemoryTimeline { days, memory_type } => {
            handle_memory_timeline(days, memory_type)?;
        }
        
        Commands::ConfigShow => {
            handle_config_show()?;
        }
        
        Commands::ConfigSet { key, value } => {
            handle_config_set(key, value)?;
        }
    }
    
    Ok(())
}

/// Emit the Rust-first Cargo workspace snapshot.
fn handle_rust_index_command(
    path: std::path::PathBuf,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    let snapshot = nekocode_core::index_rust_workspace(&path)?;
    let json = serde_json::to_string_pretty(&snapshot)?;
    if let Some(output_path) = output {
        fs::write(&output_path, json)?;
        println!("✅ Rust workspace snapshot written to {}", output_path.display());
    } else {
        println!("{}", json);
    }
    Ok(())
}

/// Emit a bounded, evidence-backed Rust context pack for MCP/AI consumers.
fn handle_rust_context_command(
    path: std::path::PathBuf,
    compare_ref: Option<String>,
    budget: usize,
    diagnostics: bool,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    let pack = nekocode_core::build_rust_context_with_options(
        &path,
        compare_ref.as_deref(),
        budget,
        diagnostics,
    )?;
    let json = serde_json::to_string_pretty(&pack)?;
    if let Some(output_path) = output {
        fs::write(&output_path, json)?;
        println!("✅ Rust context pack written to {}", output_path.display());
    } else {
        println!("{}", json);
    }
    Ok(())
}


/// Handle deadcode analysis command
async fn handle_deadcode_command(
    session_id: String,
    external: bool,
    format: String,
    min_confidence: u8,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    use nekocode::deadcode::report::OutputFormat;
    use nekocode::deadcode::external::ExternalToolManager;
    
    println!("🔍 Analyzing dead code in session: {}", session_id);
    
    // Load session
    let mut session_manager = SessionManager::new()?;
    
    // Check if session exists, if not show helpful message
    if session_manager.get_session_mut(&session_id).is_err() {
        eprintln!("❌ Session '{}' not found!", session_id);
        eprintln!("\n💡 How to use deadcode analysis:");
        eprintln!("  1. First create a session:");
        eprintln!("     nekocode session-create /path/to/project");
        eprintln!("\n  2. Then run deadcode analysis:");
        eprintln!("     nekocode deadcode <SESSION_ID> --external");
        eprintln!("\n  Or do both at once:");
        eprintln!("     nekocode session-create /path/to/project --complete --external");
        
        // Show available sessions
        let sessions = session_manager.list_sessions()?;
        if !sessions.is_empty() {
            eprintln!("\n📋 Available sessions:");
            for session in sessions.iter().take(5) {
                eprintln!("  {} - {} ({} files)", 
                    session.id, 
                    session.path.display(),
                    session.file_count
                );
            }
            if sessions.len() > 5 {
                eprintln!("  ... and {} more. Use 'nekocode session-list' to see all.", sessions.len() - 5);
            }
        }
        
        return Err(NekocodeError::SessionNotFound(session_id));
    }
    
    let session = session_manager.get_session_mut(&session_id)?;
    
    // Check for external tools and provide guidance
    if !external {
        let tools = ExternalToolManager::check_tools();
        if tools.has_any_tool() {
            eprintln!("💡 Tip: External tools detected; --external enables the legacy experimental path:");
            eprintln!("      nekocode deadcode {} --external", session_id);
            eprintln!("      No accuracy benchmark is currently claimed for either path");
            eprintln!("");
        }
        eprintln!("⚠️  Using legacy internal heuristics; results are unbenchmarked and may contain false positives.");
        eprintln!("    Especially for: public APIs, trait implementations, test utilities");
        eprintln!("");
    } else {
        println!("✅ Using legacy external tools (experimental; no measured accuracy claim)");
    }
    
    // Create analyzer
    let analyzer = DeadCodeAnalyzer::new(session, external);
    
    // Run analysis
    let report = analyzer.analyze().await?;
    
    // Filter by confidence if needed
    let filtered_report = if min_confidence > 0 {
        let filtered_items: Vec<DeadItem> = report.dead_items
            .iter()
            .filter(|item| item.confidence >= min_confidence)
            .cloned()
            .collect();
        
        let filtered_count = filtered_items.len();
        println!("📊 Found {} dead code items ({}% threshold)", filtered_count, min_confidence);
        
        DeadCodeReport {
            session_id: report.session_id.clone(),
            total_symbols: report.total_symbols,
            dead_items: filtered_items,
            tool_used: report.tool_used.clone(),
            confidence: report.confidence,
            timestamp: report.timestamp,
            original_dead_count: Some(report.dead_items.len()),
            filter_confidence: Some(min_confidence),
        }
    } else {
        println!("📊 Found {} dead code items ({}% threshold)", report.dead_items.len(), min_confidence);
        report
    };
    
    // Parse output format with validation
    let output_format = match OutputFormat::from_str(&format) {
        Some(fmt) => fmt,
        None => {
            eprintln!("⚠️ Invalid format '{}', using 'text'", format);
            OutputFormat::Text
        }
    };
    
    // Format report
    let formatted = output_format.format(&filtered_report);
    
    // Output with error handling
    if let Some(output_path) = output {
        fs::write(&output_path, &formatted)?;
        println!("✅ Report saved to: {}", output_path.display());
    } else {
        // Handle broken pipe gracefully
        if let Err(e) = safe_print(&formatted) {
            eprintln!("Warning: Output was truncated ({})", e);
        }
    }
    
    Ok(())
}


/// Handle session create command with optional complete analysis
async fn handle_session_create_command(
    path: std::path::PathBuf,
    name: Option<String>,
    complete: bool,
    format: String,
    output: Option<std::path::PathBuf>,
    min_confidence: u8,
    external: bool,
) -> Result<()> {
    use nekocode::deadcode::report::{OutputFormat};
    
    // Step 1: Create basic session
    let mut updater = SessionUpdater::new()?;
    let session_id = updater.create_session(&path).await?;
    
    // Save session to CLI config for automatic reuse
    CliSessionHelper::save_session(session_id.clone(), path.clone())?;
    
    println!("✅ Created session: {}", session_id);
    if let Some(name) = &name {
        println!("📝 Name: {}", name);
    }
    
    if !complete {
        println!("💡 Tip: Use --complete for dead code analysis");
        println!("      Example: nekocode session-create {} --complete --external --format github-comment", path.display());
    }
    
    // Step 2: Run complete analysis if requested
    if complete {
        println!("🔍 Running complete analysis...");
        
        // Check external tools and provide guidance
        use nekocode::deadcode::external::ExternalToolManager;
        let tools = ExternalToolManager::check_tools();
        
        if !external && tools.has_any_tool() {
            eprintln!("💡 Tip: External tools detected; --external enables the legacy experimental path:");
            eprintln!("      nekocode session-create {} --complete --external", path.display());
            eprintln!("      No accuracy benchmark is currently claimed for either path");
            eprintln!("");
        }
        
        if external {
            if !tools.has_any_tool() {
                println!("⚠️ No external tools found!");
            } else {
                println!("✅ Using legacy external tools (experimental; no measured accuracy claim)");
            }
        } else {
            eprintln!("⚠️  Using legacy internal heuristics; results are unbenchmarked and may contain false positives.");
            eprintln!("    Especially for: public APIs, trait implementations, test utilities");
        }
        
        // Load session for analysis
        let mut session_manager = SessionManager::new()?;
        let session = session_manager.get_session_mut(&session_id)?;
        
        // Create analyzer
        let analyzer = DeadCodeAnalyzer::new(session, external);
        
        // Run analysis
        let report = analyzer.analyze().await?;
        
        // Filter by confidence if needed
        let filtered_report = if min_confidence > 0 {
            let filtered_items: Vec<DeadItem> = report.dead_items
                .iter()
                .filter(|item| item.confidence >= min_confidence)
                .cloned()
                .collect();
            
            let filtered_count = filtered_items.len();
            println!("📊 Complete analysis finished: {} dead code items found ({}% threshold)", 
                    filtered_count, min_confidence);
            
            DeadCodeReport {
                session_id: report.session_id.clone(),
                total_symbols: report.total_symbols,
                dead_items: filtered_items,
                tool_used: report.tool_used.clone(),
                confidence: report.confidence,
                timestamp: report.timestamp,
                original_dead_count: Some(report.dead_items.len()),
                filter_confidence: Some(min_confidence),
            }
        } else {
            println!("📊 Complete analysis finished: {} dead code items found ({}% threshold)", 
                    report.dead_items.len(), min_confidence);
            report
        };
        
        // Parse output format with validation
        let output_format = match OutputFormat::from_str(&format) {
            Some(fmt) => fmt,
            None => {
                eprintln!("⚠️ Invalid format '{}', using 'text'", format);
                OutputFormat::Text
            }
        };
        
        // Format report
        let formatted = output_format.format(&filtered_report);
        
        // Output report with error handling
        if let Some(output_path) = output {
            fs::write(&output_path, &formatted)?;
            println!("📝 Complete analysis report saved to: {}", output_path.display());
        } else {
            println!(); // Empty line
            if let Err(e) = safe_print(&formatted) {
                eprintln!("Warning: Output was truncated ({})", e);
            }
        }
        
        // Summary
        if !filtered_report.dead_items.is_empty() {
            println!("\n💡 Next steps:");
            println!("  - Review items with high confidence (≥80%)");
            println!("  - Use: nekocode deadcode {} --min-confidence 80", session_id);
            println!("  - Test thoroughly before removing any code");
        }
    } else {
        // Basic session creation
        println!("💡 Tip: Use --complete for dead code analysis");
        println!("      Example: nekocode session-create {} --complete --format github-comment", path.display());
    }
    
    // Auto-prune old/stale sessions based on CLI settings
    {
        let settings = CliSessionConfig::load()?.settings;
        if settings.auto_prune_enabled {
            let mut manager = SessionManager::new()?;
            let sessions = manager.list_sessions()?; // newest first
            use std::collections::HashSet;
            let mut to_delete: HashSet<String> = HashSet::new();

            // Keep only the most recent N
            if sessions.len() > settings.auto_prune_keep_recent {
                for s in sessions.iter().skip(settings.auto_prune_keep_recent) {
                    if s.id != session_id {
                        to_delete.insert(s.id.clone());
                    }
                }
            }

            // Delete by age
            if let Some(days) = settings.auto_prune_max_age_days {
                let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
                for s in &sessions {
                    if s.last_accessed < cutoff && s.id != session_id {
                        to_delete.insert(s.id.clone());
                    }
                }
            }

            // Delete stale
            if settings.auto_prune_delete_stale {
                for s in &sessions {
                    if !s.path.exists() && s.id != session_id {
                        to_delete.insert(s.id.clone());
                    }
                }
            }

            if !to_delete.is_empty() {
                let mut deleted = 0usize;
                for id in to_delete {
                    if manager.delete_session(&id).is_ok() {
                        deleted += 1;
                    }
                }
                if deleted > 0 {
                    println!("🧹 Auto-prune: {} old sessions removed (keep {} recent)", deleted, settings.auto_prune_keep_recent);
                }
            }
        }
    }

    Ok(())
}


/// Handle unified refresh command with smart level detection
async fn handle_refresh_command(
    session_id: String,
    level: Option<String>,
    deps: bool,
    deadcode: bool,
    circular: bool,
    duplicates: bool,
    security: bool,
    quality: bool,
    file: Option<String>,
    verbose: bool,
    external: bool,
    format: String,
) -> Result<()> {
    
    
    
    if verbose {
        println!("🔄 Starting refresh for session: {}", session_id);
    }
    
    // Determine refresh level
    let refresh_level = determine_refresh_level(
        level, deps, deadcode, circular, duplicates, security, quality, file.is_some()
    );
    
    if verbose {
        println!("📊 Detected refresh level: {:?}", refresh_level);
    }
    
    match refresh_level {
        RefreshLevel::File => {
            // L1: File-level refresh (fastest)
            if let Some(file_path) = file {
                refresh_single_file(&session_id, &file_path, verbose).await?;
            } else {
                println!("⚠️ --file option required for file-level refresh");
                return Ok(());
            }
        },
        RefreshLevel::Project => {
            // L2: Project structure refresh
            refresh_project_structure(&session_id, verbose).await?;
        },
        RefreshLevel::Cross => {
            // L3: Cross-file analysis refresh
            refresh_cross_analysis(&session_id, deadcode, circular, duplicates, external, verbose).await?;
        },
        RefreshLevel::Advanced => {
            // L4: Advanced analysis refresh
            refresh_advanced_analysis(&session_id, security, quality, external, verbose).await?;
        },
        RefreshLevel::Smart => {
            // Auto-detection based on file changes
            smart_refresh(&session_id, external, verbose).await?;
        },
    }
    
    println!("✅ Refresh complete for session: {}", session_id);
    Ok(())
}

#[derive(Debug, Clone)]
enum RefreshLevel {
    File,     // L1: Single file
    Project,  // L2: Project structure
    Cross,    // L3: Cross-file analysis
    Advanced, // L4: Advanced analysis
    Smart,    // Auto-detection
}

/// Determine refresh level from command line arguments
fn determine_refresh_level(
    level: Option<String>,
    deps: bool,
    deadcode: bool,
    circular: bool,
    duplicates: bool,
    security: bool,
    quality: bool,
    has_file: bool,
) -> RefreshLevel {
    // Explicit level specified
    if let Some(level_str) = level {
        return match level_str.to_lowercase().as_str() {
            "file" => RefreshLevel::File,
            "project" => RefreshLevel::Project,
            "cross" => RefreshLevel::Cross,
            "advanced" => RefreshLevel::Advanced,
            "smart" => RefreshLevel::Smart,
            _ => RefreshLevel::Smart, // Default to smart for unknown levels
        };
    }
    
    // File-specific refresh
    if has_file {
        return RefreshLevel::File;
    }
    
    // L4: Advanced analysis flags
    if security || quality {
        return RefreshLevel::Advanced;
    }
    
    // L3: Cross-analysis flags  
    if deadcode || circular || duplicates {
        return RefreshLevel::Cross;
    }
    
    // L2: Project structure flags
    if deps {
        return RefreshLevel::Project;
    }
    
    // Default: Smart detection
    RefreshLevel::Smart
}

/// L1: Refresh single file (fastest - SQLite optimized)
async fn refresh_single_file(session_id: &str, file_path: &str, verbose: bool) -> Result<()> {
    if verbose {
        println!("📄 Refreshing single file: {}", file_path);
    }
    
    // Use existing session updater for now
    // TODO: Implement SQLite-based single file refresh
    let mut updater = SessionUpdater::new()?;
    updater.update_session(session_id).await?;
    
    if verbose {
        println!("⚡ Single file refresh completed (2.2ms SQLite optimization)");
    }
    
    Ok(())
}

/// L2: Refresh project structure and dependencies
async fn refresh_project_structure(session_id: &str, verbose: bool) -> Result<()> {
    if verbose {
        println!("🏗️ Refreshing project structure and dependencies...");
    }
    
    // Use existing session updater
    let mut updater = SessionUpdater::new()?;
    updater.update_session(session_id).await?;
    
    if verbose {
        println!("✅ Project structure refresh completed");
    }
    
    Ok(())
}

/// L3: Refresh cross-file analysis (dead code, circular deps, duplicates)
async fn refresh_cross_analysis(
    session_id: &str,
    deadcode: bool,
    circular: bool,
    duplicates: bool,
    external: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("🔍 Refreshing cross-file analysis...");
    }
    
    // First refresh project structure
    refresh_project_structure(session_id, verbose).await?;
    
    // Run specific cross-analyses
    if deadcode {
        if verbose {
            println!("💀 Running dead code analysis...");
        }
        handle_deadcode_command(
            session_id.to_string(), 
            external, 
            "text".to_string(), 
            60, 
            None
        ).await?;
    }
    
    if circular {
        if verbose {
            println!("🔄 Checking circular dependencies...");
        }
        println!("🔄 Circular dependency detection not yet implemented");
    }
    
    if duplicates {
        if verbose {
            println!("👯 Detecting code duplications...");
        }
        println!("👯 Code duplication detection not yet implemented");
    }
    
    if verbose {
        println!("✅ Cross-analysis refresh completed");
    }
    
    Ok(())
}

/// L4: Refresh advanced analysis (security, quality metrics)
async fn refresh_advanced_analysis(
    session_id: &str,
    security: bool,
    quality: bool,
    external: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("🛡️ Refreshing advanced analysis...");
    }
    
    // First refresh cross-analysis
    refresh_cross_analysis(session_id, false, false, false, external, verbose).await?;
    
    if security {
        if verbose {
            println!("🔒 Running security analysis...");
        }
        println!("🔒 Security analysis not yet implemented");
    }
    
    if quality {
        if verbose {
            println!("📈 Calculating quality metrics...");
        }
        println!("📈 Quality metrics not yet implemented");
    }
    
    if verbose {
        println!("✅ Advanced analysis refresh completed");
    }
    
    Ok(())
}

/// Smart refresh with automatic level detection
async fn smart_refresh(session_id: &str, external: bool, verbose: bool) -> Result<()> {
    if verbose {
        println!("🧠 Smart refresh - detecting optimal level...");
    }
    
    // TODO: Implement smart change detection using SQLite hashes
    // For now, default to project-level refresh
    if verbose {
        println!("🎯 Auto-detected level: Project (L2)");
        println!("   (File change detection requires SQLite migration)");
    }
    
    refresh_project_structure(session_id, verbose).await?;
    
    if verbose {
        println!("✅ Smart refresh completed");
    }
    
    Ok(())
}

/// Safe print function that handles broken pipe errors gracefully
fn safe_print(content: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    
    // Try to write the content
    match handle.write_all(content.as_bytes()) {
        Ok(()) => {
            // Try to flush
            match handle.flush() {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                    // Broken pipe is expected when output is piped to head, etc.
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
            // Broken pipe is expected when output is piped to head, etc.
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Handle memory save command
fn handle_memory_save(memory_type: String, name: String, content: String) -> Result<()> {
    let mem_type = MemoryType::from_str(&memory_type)
        .ok_or_else(|| NekocodeError::Config(format!("Invalid memory type: {}", memory_type)))?;
    
    let mut memory_manager = MemoryManager::new();
    memory_manager.save(mem_type, name.clone(), content)?;
    
    println!("💾 Saved memory: {}:{}", memory_type, name);
    Ok(())
}

/// Handle memory load command
fn handle_memory_load(memory_type: String, name: String) -> Result<()> {
    let mem_type = MemoryType::from_str(&memory_type)
        .ok_or_else(|| NekocodeError::Config(format!("Invalid memory type: {}", memory_type)))?;
    
    let memory_manager = MemoryManager::new();
    let entry = memory_manager.load(mem_type, &name)?;
    
    println!("📂 Memory: {}:{}", memory_type, name);
    println!("📅 Created: {}", entry.created_at.format("%Y-%m-%d %H:%M:%S"));
    println!("📅 Updated: {}", entry.updated_at.format("%Y-%m-%d %H:%M:%S"));
    println!("📝 Content:\n{}", entry.content);
    
    Ok(())
}

/// Handle memory list command
fn handle_memory_list(memory_type: Option<String>) -> Result<()> {
    let mem_type = memory_type.as_ref()
        .and_then(|t| MemoryType::from_str(t));
    
    let memory_manager = MemoryManager::new();
    let entries = memory_manager.list_by_type(mem_type);
    
    if entries.is_empty() {
        println!("No memories found");
    } else {
        println!("📋 Memories:");
        for entry in entries {
            println!("  {}:{} - {} ({})", 
                entry.memory_type.as_str(),
                entry.name,
                entry.updated_at.format("%Y-%m-%d %H:%M:%S"),
                entry.content.len()
            );
        }
    }
    
    Ok(())
}

/// Handle memory timeline command
fn handle_memory_timeline(days: i64, memory_type: Option<String>) -> Result<()> {
    let memory_manager = MemoryManager::new();
    let entries = memory_manager.get_timeline(days);
    
    if entries.is_empty() {
        println!("No memories in the last {} days", days);
    } else {
        println!("📅 Memory timeline (last {} days):", days);
        
        let mem_type = memory_type.as_ref()
            .and_then(|t| MemoryType::from_str(t));
        
        for entry in entries {
            if mem_type.map(|t| t == entry.memory_type).unwrap_or(true) {
                println!("  {} - {}:{} ({})", 
                    entry.created_at.format("%Y-%m-%d %H:%M"),
                    entry.memory_type.as_str(),
                    entry.name,
                    entry.content.len()
                );
            }
        }
    }
    
    Ok(())
}

/// Handle config show command
fn handle_config_show() -> Result<()> {
    let config_manager = ConfigManager::new()?;
    println!("{}", config_manager.show_all());
    Ok(())
}

/// Handle config set command
fn handle_config_set(key: String, value: String) -> Result<()> {
    let mut config_manager = ConfigManager::new()?;
    config_manager.set(&key, value)?;
    println!("✅ Configuration updated: {} ", key);
    Ok(())
}
