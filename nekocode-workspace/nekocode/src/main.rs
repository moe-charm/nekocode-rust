//! NekoCode - Core analysis engine with Tree-sitter support

use clap::Parser;
use std::fs;

use nekocode_core::{Result, NekocodeError, session::SessionManager};
use nekocode::{
    Cli,
    SessionCommands, SessionUpdater,
    JavaScriptAnalyzer, TypeScriptAnalyzer,
    PythonAnalyzer, RustAnalyzer,
    CppAnalyzer, GoAnalyzer, CSharpAnalyzer,
    Analyzer
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
        Commands::Analyze { path, output, stats_only, language, ast } => {
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
        
        Commands::SessionCreate { path, name } => {
            let mut updater = SessionUpdater::new()?;
            let session_id = updater.create_session(&path).await?;
            
            println!("✅ Created session: {}", session_id);
            if let Some(name) = name {
                println!("📝 Name: {}", name);
            }
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
            _ => Err(NekocodeError::LanguageNotSupported(lang.to_string()))
        }
    } else {
        // Auto-detect from file extension
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| NekocodeError::LanguageNotSupported("Unknown file extension".to_string()))?;
        
        match ext {
            "js" | "jsx" | "mjs" | "cjs" => Ok(Box::new(JavaScriptAnalyzer::new()?)),
            "ts" | "tsx" => Ok(Box::new(TypeScriptAnalyzer::new()?)),
            "py" | "pyw" | "pyi" => Ok(Box::new(PythonAnalyzer::new()?)),
            "rs" => Ok(Box::new(RustAnalyzer::new()?)),
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" | "c" | "h" => Ok(Box::new(CppAnalyzer::new()?)),
            "go" => Ok(Box::new(GoAnalyzer::new()?)),
            "cs" => Ok(Box::new(CSharpAnalyzer::new()?)),
            _ => Err(NekocodeError::LanguageNotSupported(ext.to_string()))
        }
    }
}