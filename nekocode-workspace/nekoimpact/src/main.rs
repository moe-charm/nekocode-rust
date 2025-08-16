//! NekoImpact - Impact analysis tool for code changes

mod impact;
mod analyzer;
mod cli;

use clap::Parser;
use nekocode_core::{SessionManager, Result, NekocodeError};
use crate::cli::{Cli, Commands};
use crate::impact::{ImpactAnalyzer, ImpactResult};
use crate::analyzer::{AnalysisOptions, OutputFormat};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Create impact analyzer
    let mut analyzer = ImpactAnalyzer::new()?;
    
    // Execute command
    match cli.command {
        Commands::Analyze { session_id, format, references, files } => {
            let output_format = OutputFormat::from_str(&format)
                .ok_or_else(|| NekocodeError::Config(format!("Invalid format: {}", format)))?;
            
            let result = analyzer.analyze_session(&session_id).await?;
            print_result(&result, output_format, cli.verbose);
        }
        
        Commands::Compare { base, head, format } => {
            let output_format = OutputFormat::from_str(&format)
                .ok_or_else(|| NekocodeError::Config(format!("Invalid format: {}", format)))?;
            
            let result = analyzer.compare_sessions(&base, &head).await?;
            print_result(&result, output_format, cli.verbose);
        }
        
        Commands::Diff { session_id, compare_ref, format } => {
            let output_format = OutputFormat::from_str(&format)
                .ok_or_else(|| NekocodeError::Config(format!("Invalid format: {}", format)))?;
            
            // TODO: Implement Git integration
            println!("Git diff analysis not yet implemented");
            println!("Session: {}, Compare ref: {}", session_id, compare_ref);
        }
        
        Commands::Graph { session_id, output, graph_format } => {
            // TODO: Implement graph generation
            println!("Graph generation not yet implemented");
            println!("Session: {}, Format: {}", session_id, graph_format);
            if let Some(output) = output {
                println!("Output: {}", output.display());
            }
        }
        
        Commands::List { detailed } => {
            let session_manager = SessionManager::new()?;
            let sessions = session_manager.list_sessions()?;
            
            if sessions.is_empty() {
                println!("No sessions found");
            } else {
                println!("Found {} sessions:", sessions.len());
                for session in sessions {
                    if detailed {
                        println!("\n📊 Session: {}", session.id);
                        println!("   Path: {}", session.path.display());
                        println!("   Files: {}", session.file_count);
                        println!("   Lines: {}", session.total_lines);
                        println!("   Created: {}", session.created_at.format("%Y-%m-%d %H:%M"));
                        println!("   Modified: {}", session.last_modified.format("%Y-%m-%d %H:%M"));
                        
                        if !session.languages.is_empty() {
                            println!("   Languages:");
                            for (lang, count) in &session.languages {
                                println!("     - {}: {} files", lang, count);
                            }
                        }
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
    }
    
    Ok(())
}

/// Print impact result in specified format
fn print_result(result: &ImpactResult, format: OutputFormat, verbose: bool) {
    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(result).unwrap();
            println!("{}", json);
        }
        
        OutputFormat::Plain => {
            println!("🔍 Impact Analysis Results");
            println!("{}", "=".repeat(50));
            
            if result.changed_symbols.is_empty() {
                println!("✅ No changes detected");
            } else {
                println!("📝 Changed symbols: {}", result.changed_symbols.len());
                println!("📁 Affected files: {}", result.affected_files.len());
                println!("🔗 Total references: {}", result.total_references);
                
                if !result.breaking_changes.is_empty() {
                    println!("\n⚠️  Breaking Changes:");
                    for change in &result.breaking_changes {
                        println!("  - {} in {}", change.symbol, change.file_path.display());
                        println!("    {}", change.description);
                    }
                }
                
                println!("\n📊 Risk Assessment:");
                let assessment = &result.risk_assessment;
                println!("  Overall Risk: {} {}", 
                    assessment.overall_risk.emoji(),
                    assessment.overall_risk.as_str()
                );
                println!("  High Risk: {}", assessment.high_risk_count);
                println!("  Medium Risk: {}", assessment.medium_risk_count);
                println!("  Low Risk: {}", assessment.low_risk_count);
                println!("  Breaking Changes: {}", assessment.breaking_change_count);
                println!("\n{}", assessment.recommendation);
                
                if verbose {
                    println!("\n📋 Changed Symbols:");
                    for symbol in &result.changed_symbols {
                        println!("  {} {} - {} ({}:{})",
                            symbol.risk_level.emoji(),
                            symbol.name,
                            symbol.change_type.as_str(),
                            symbol.file_path.display(),
                            symbol.line_number
                        );
                    }
                }
            }
        }
        
        OutputFormat::GithubComment => {
            println!("## 🔍 Impact Analysis Report\n");
            
            if result.changed_symbols.is_empty() {
                println!("✅ **No changes detected**");
            } else {
                println!("### 📊 Summary");
                println!("- **Changed symbols:** {}", result.changed_symbols.len());
                println!("- **Affected files:** {}", result.affected_files.len());
                println!("- **Total references:** {}", result.total_references);
                
                let assessment = &result.risk_assessment;
                println!("\n### {} Risk Assessment", assessment.overall_risk.emoji());
                println!("| Risk Level | Count |");
                println!("|------------|-------|");
                println!("| 🔴 High    | {} |", assessment.high_risk_count);
                println!("| 🟡 Medium  | {} |", assessment.medium_risk_count);
                println!("| 🟢 Low     | {} |", assessment.low_risk_count);
                
                if !result.breaking_changes.is_empty() {
                    println!("\n### ⚠️ Breaking Changes");
                    for change in &result.breaking_changes {
                        println!("- **{}** - {}", change.symbol, change.description);
                        println!("  - File: `{}`", change.file_path.display());
                        println!("  - Line: {}", change.line_number);
                    }
                }
                
                println!("\n### 💡 Recommendation");
                println!("{}", assessment.recommendation);
            }
            
            println!("\n---");
            println!("*Generated by NekoImpact v{}*", env!("CARGO_PKG_VERSION"));
        }
        
        OutputFormat::Markdown => {
            println!("# Impact Analysis Report\n");
            
            if result.changed_symbols.is_empty() {
                println!("No changes detected.");
            } else {
                println!("## Summary\n");
                println!("- Changed symbols: {}", result.changed_symbols.len());
                println!("- Affected files: {}", result.affected_files.len());
                println!("- Total references: {}", result.total_references);
                
                if verbose {
                    println!("\n## Changed Symbols\n");
                    for symbol in &result.changed_symbols {
                        println!("### {}\n", symbol.name);
                        println!("- Type: {}", symbol.symbol_type);
                        println!("- Change: {}", symbol.change_type.as_str());
                        println!("- File: `{}`", symbol.file_path.display());
                        println!("- Line: {}", symbol.line_number);
                        println!("- Risk: {}", symbol.risk_level.as_str());
                        println!("- Breaking: {}", symbol.breaking_change);
                        
                        if let Some(ref before) = symbol.signature_before {
                            println!("- Before: `{}`", before);
                        }
                        if let Some(ref after) = symbol.signature_after {
                            println!("- After: `{}`", after);
                        }
                    }
                }
            }
        }
    }
}