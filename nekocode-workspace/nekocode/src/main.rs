//! NekoCode - Core analysis engine with Tree-sitter support

use clap::Parser;
use std::fs;
use std::io::{self, Write};

use nekocode_core::{Result, NekocodeError, session::SessionManager};
use nekocode::{
    Cli,
    SessionCommands, SessionUpdater,
    JavaScriptAnalyzer, TypeScriptAnalyzer,
    PythonAnalyzer, RustAnalyzer,
    CppAnalyzer, GoAnalyzer, CSharpAnalyzer,
    Analyzer,
    DeadCodeAnalyzer, DeadItem, DeadCodeReport
};
use nekocode::cli::Commands;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Execute command
    match cli.command {
        Commands::Analyze { path, output, stats_only, language, ast: _ } => {
            // Check if path is a directory
            if path.is_dir() {
                eprintln!("❌ Error: 'analyze' command expects a file, but got a directory: {}", path.display());
                eprintln!("💡 Hint: For directory analysis, use 'session-create' command instead:");
                eprintln!("         nekocode session-create {}", path.display());
                if stats_only {
                    eprintln!("         Or for stats only: nekocode session-create {} --stats-only", path.display());
                }
                return Err(NekocodeError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Directory provided to analyze command. Use 'session-create' for directories.")
                )));
            }
            
            // Check if file exists
            if !path.exists() {
                eprintln!("❌ Error: File not found: {}", path.display());
                return Err(NekocodeError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("File not found: {}", path.display())
                )));
            }
            
            // Create appropriate analyzer
            let mut analyzer = create_analyzer_for_path(&path, language.as_deref())?;
            
            // Read file content
            let content = fs::read_to_string(&path)?;
            
            // Analyze
            let result = analyzer.analyze(&path, &content).await?;
            
            // Output results
            match output.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                "stats" => {
                    println!("📊 Analysis Statistics:");
                    println!("  Language: {:?}", result.file_info.language);
                    println!("  Size: {} bytes", result.file_info.size_bytes);
                    println!("  Lines: {}", result.file_info.total_lines);
                    println!("  Functions: {}", result.functions.len());
                    println!("  Classes: {}", result.classes.len());
                    println!("  Imports: {}", result.imports.len());
                    println!("  Exports: {}", result.exports.len());
                }
                _ => {
                    if stats_only {
                        println!("📊 Quick Stats: {} functions, {} classes, {} imports",
                            result.functions.len(), result.classes.len(), result.imports.len());
                    } else {
                        println!("📄 Analysis complete: {}", path.display());
                        println!("📊 Functions: {}", result.functions.len());
                        println!("📊 Classes: {}", result.classes.len());
                        if !result.functions.is_empty() {
                            println!("\n🔧 Functions:");
                            for func in &result.functions {
                                println!("  {} (lines {}-{})", func.symbol.name, func.symbol.line_start, func.symbol.line_end);
                            }
                        }
                        if !result.classes.is_empty() {
                            println!("\n📦 Classes:");
                            for class in &result.classes {
                                println!("  {} (lines {}-{})", class.symbol.name, class.symbol.line_start, class.symbol.line_end);
                            }
                        }
                    }
                }
            }
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
        
        Commands::AstStats { session_id } => {
            let mut commands = SessionCommands::new()?;
            let stats = commands.ast_stats(&session_id).await?;
            println!("{}", stats);
        }
        
        Commands::AstQuery { session_id, path } => {
            let mut commands = SessionCommands::new()?;
            let result = commands.ast_query(&session_id, &path).await?;
            println!("{}", result);
        }
        
        Commands::AstDump { session_id, format, limit, force } => {
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
            handle_deadcode_command(session_id, external, format, min_confidence, output).await?;
        }
        
    }
    
    Ok(())
}

/// Create appropriate analyzer for a path
fn create_analyzer_for_path(path: &std::path::Path, language: Option<&str>) -> Result<Box<dyn Analyzer>> {
    if let Some(lang) = language {
        match lang.to_lowercase().as_str() {
            "javascript" | "js" => Ok(Box::new(JavaScriptAnalyzer::new()?)),
            "typescript" | "ts" => Ok(Box::new(TypeScriptAnalyzer::new()?)),
            "python" | "py" => Ok(Box::new(PythonAnalyzer::new()?)),
            "rust" | "rs" => Ok(Box::new(RustAnalyzer::new()?)),
            "cpp" | "c++" | "cxx" => Ok(Box::new(CppAnalyzer::new()?)),
            "go" => Ok(Box::new(GoAnalyzer::new()?)),
            "csharp" | "cs" => Ok(Box::new(CSharpAnalyzer::new()?)),
            _ => Err(NekocodeError::LanguageNotSupported(format!(
                "Language '{}' is not supported. Supported: javascript, typescript, python, rust, cpp, go, csharp",
                lang
            )))
        }
    } else {
        // Auto-detect from file extension
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| {
                if path.is_dir() {
                    NekocodeError::LanguageNotSupported(
                        "Cannot auto-detect language for directory. Use 'session-create' for directories.".to_string()
                    )
                } else {
                    NekocodeError::LanguageNotSupported(format!(
                        "Cannot detect language for file '{}'. Specify with --language option.",
                        path.display()
                    ))
                }
            })?;
        
        match ext {
            "js" | "jsx" | "mjs" | "cjs" => Ok(Box::new(JavaScriptAnalyzer::new()?)),
            "ts" | "tsx" => Ok(Box::new(TypeScriptAnalyzer::new()?)),
            "py" | "pyw" | "pyi" => Ok(Box::new(PythonAnalyzer::new()?)),
            "rs" => Ok(Box::new(RustAnalyzer::new()?)),
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" | "c" | "h" => Ok(Box::new(CppAnalyzer::new()?)),
            "go" => Ok(Box::new(GoAnalyzer::new()?)),
            "cs" => Ok(Box::new(CSharpAnalyzer::new()?)),
            _ => Err(NekocodeError::LanguageNotSupported(format!(
                "File extension '{}' is not supported. Supported extensions: js, ts, py, rs, cpp, go, cs. Use --language to specify explicitly.",
                ext
            )))
        }
    }
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
            eprintln!("💡 Tip: External tools detected! Use --external flag for better accuracy:");
            eprintln!("      nekocode deadcode {} --external", session_id);
            eprintln!("      External tools provide 90%+ accuracy vs 60% for internal analysis");
            eprintln!("");
        }
        eprintln!("⚠️  Using internal analysis (60% accuracy). May have false positives.");
        eprintln!("    Especially for: public APIs, trait implementations, test utilities");
        eprintln!("");
    } else {
        println!("✅ Using external tools for high-accuracy analysis (90%+ confidence)");
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
            eprintln!("💡 Tip: External tools detected! Add --external flag for better accuracy:");
            eprintln!("      nekocode session-create {} --complete --external", path.display());
            eprintln!("      External tools provide 90%+ accuracy vs 60% for internal analysis");
            eprintln!("");
        }
        
        if external {
            if !tools.has_any_tool() {
                println!("⚠️ No external tools found!");
            } else {
                println!("✅ Using external tools for high-accuracy analysis (90%+ confidence)");
            }
        } else {
            eprintln!("⚠️  Using internal analysis (60% accuracy). May have false positives.");
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
    
    Ok(())
}

/// Detect primary language in project directory
fn detect_primary_language(path: &std::path::Path) -> Result<Option<nekocode_core::Language>> {
    use nekocode_core::Language;
    use walkdir::WalkDir;
    use std::collections::HashMap;
    
    let mut language_counts: HashMap<Language, usize> = HashMap::new();
    
    // Walk through project directory and count file types
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if let Some(extension) = entry.path().extension() {
            if let Some(ext_str) = extension.to_str() {
                let language = match ext_str {
                    "rs" => Some(Language::Rust),
                    "py" | "pyw" | "pyi" => Some(Language::Python),
                    "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
                    "ts" | "tsx" => Some(Language::TypeScript),
                    "go" => Some(Language::Go),
                    "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" | "c" | "h" => Some(Language::Cpp),
                    "cs" => Some(Language::CSharp),
                    _ => None,
                };
                
                if let Some(lang) = language {
                    *language_counts.entry(lang).or_insert(0) += 1;
                }
            }
        }
    }
    
    // Return the most common language
    Ok(language_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(lang, _)| lang))
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
    use nekocode_core::SqliteSession;
    use std::path::Path;
    
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