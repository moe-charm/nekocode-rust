//! NekoRefactor - Code refactoring tool

mod preview;
mod replace;
mod moveclass;
mod cli;
mod smart;

use clap::Parser;
use std::io::{self, Read};

use nekocode_core::{Result, NekocodeError};
use crate::cli::{Cli, Commands, SmartCommands};
use crate::preview::{PreviewManager, InsertPosition};
use crate::replace::{ReplaceEngine, ReplaceOptions};
use crate::moveclass::{MoveClassEngine, MoveOptions};
use crate::smart::{SmartRefactor, SmartPosition, Scope};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Create preview manager
    let mut preview_manager = PreviewManager::new();
    
    // Execute command
    match cli.command {
        Commands::Replace { file, pattern, replacement, regex, ignore_case, whole_word, preview } => {
            let options = ReplaceOptions {
                use_regex: regex,
                case_sensitive: !ignore_case,
                whole_word,
                multiline: false,
            };
            
            let engine = ReplaceEngine::new(options);
            let preview_op = engine.create_preview(file, pattern, replacement)?;
            let preview_id = preview_manager.add_preview(preview_op)?;
            
            if preview {
                // Preview mode
                let preview = preview_manager.get_preview(&preview_id).unwrap();
                println!("{}", preview.preview_text);
                println!("\n✨ Preview ID: {}", preview_id);
            } else {
                // Apply immediately
                preview_manager.confirm_preview(&preview_id)?;
                preview_manager.apply_preview(&preview_id)?;
                println!("✅ Replacement applied successfully");
            }
        }
        
        Commands::CreateFile { file, template, force } => {
            // Check if file exists
            if file.exists() && !force {
                eprintln!("❌ File already exists: {:?}", file);
                eprintln!("Use --force to overwrite");
                std::process::exit(1);
            }
            
            // Get template content
            let content = get_template_content(template.as_deref(), &file)?;
            
            // Write file
            std::fs::write(&file, content)
                .map_err(|e| NekocodeError::Io(e))?;
            
            println!("✅ Created file: {:?}", file);
            
            if let Some(tpl) = template {
                println!("📝 Using template: {}", tpl);
            }
        }
        
        Commands::Insert { file, content, position, after_function, before_function, in_imports, after_class, preview } => {
            let insert_content = if content == "-" {
                // Read from stdin
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)
                    .map_err(|e| NekocodeError::Io(e))?;
                buffer
            } else {
                content
            };
            
            // Determine position based on semantic options or position string
            let insert_pos = if let Some(func_name) = after_function {
                find_semantic_position(&file, SemanticPosition::AfterFunction(func_name))?
            } else if let Some(func_name) = before_function {
                find_semantic_position(&file, SemanticPosition::BeforeFunction(func_name))?
            } else if in_imports {
                find_semantic_position(&file, SemanticPosition::InImports)?
            } else if let Some(class_name) = after_class {
                find_semantic_position(&file, SemanticPosition::AfterClass(class_name))?
            } else if let Some(pos) = position {
                parse_insert_position(&pos)?
            } else {
                return Err(NekocodeError::Config("No position specified".to_string()));
            };
            
            let preview_op = preview::PreviewOperation::Insert {
                file,
                position: insert_pos,
                content: insert_content,
            };
            
            let preview_id = preview_manager.add_preview(preview_op)?;
            
            if preview {
                // Preview mode
                let preview = preview_manager.get_preview(&preview_id).unwrap();
                println!("{}", preview.preview_text);
                println!("\n✨ Preview ID: {}", preview_id);
            } else {
                // Apply immediately
                preview_manager.confirm_preview(&preview_id)?;
                preview_manager.apply_preview(&preview_id)?;
                println!("✅ Content inserted successfully");
            }
        }
        
        Commands::MoveLines { source, start_line, line_count, destination, insert_line, preview } => {
            // Read lines to move
            let source_content = std::fs::read_to_string(&source)
                .map_err(|e| NekocodeError::Io(e))?;
            
            let lines: Vec<&str> = source_content.lines().collect();
            let start_idx = (start_line - 1) as usize;
            let end_idx = start_idx + line_count as usize;
            
            if end_idx > lines.len() {
                return Err(NekocodeError::Refactoring(
                    "Line range out of bounds".to_string()
                ));
            }
            
            let lines_to_move: Vec<String> = lines[start_idx..end_idx]
                .iter()
                .map(|s| s.to_string())
                .collect();
            
            let preview_op = preview::PreviewOperation::MoveLines {
                source,
                start_line,
                line_count,
                destination,
                insert_line,
                lines: lines_to_move,
            };
            
            let preview_id = preview_manager.add_preview(preview_op)?;
            
            if preview {
                // Preview mode
                let preview = preview_manager.get_preview(&preview_id).unwrap();
                println!("{}", preview.preview_text);
                println!("\n✨ Preview ID: {}", preview_id);
            } else {
                // Apply immediately
                preview_manager.confirm_preview(&preview_id)?;
                preview_manager.apply_preview(&preview_id)?;
                println!("✅ Lines moved successfully");
            }
        }
        
        Commands::MoveClass { session_id, symbol_id, target, update_imports, preview } => {
            if preview {
                // Preview mode - dry run only
                let mut dry_run_engine = MoveClassEngine::new(MoveOptions {
                    dry_run: true,
                    update_imports,
                    ..Default::default()
                })?;
                
                let result = dry_run_engine.move_symbol(&session_id, &symbol_id, &target).await?;
                
                println!("🏗️ Move Class Preview");
                println!("Symbol: {}", result.symbol_name);
                println!("Type: {}", result.symbol_type);
                println!("From: {}", result.source_file.display());
                println!("To: {}", result.target_file.display());
                println!("Lines: {}", result.lines_moved);
                
                if update_imports {
                    println!("📦 Will update imports automatically");
                }
                
                // Create preview operation for potential later confirmation
                let preview_op = preview::PreviewOperation::MoveClass {
                    session_id: session_id.clone(),
                    symbol_id: symbol_id.clone(),
                    source_file: result.source_file,
                    target_file: result.target_file,
                    class_content: String::new(),
                };
                
                let preview_id = preview_manager.add_preview(preview_op)?;
                println!("\n✨ Preview ID: {}", preview_id);
            } else {
                // Apply immediately
                let mut engine = MoveClassEngine::new(MoveOptions {
                    update_imports,
                    ..Default::default()
                })?;
                
                let result = engine.move_symbol(&session_id, &symbol_id, &target).await?;
                
                if result.success {
                    println!("✅ Successfully moved {}", result.symbol_name);
                    println!("   {} lines moved", result.lines_moved);
                    if !result.imports_updated.is_empty() {
                        println!("   {} imports updated", result.imports_updated.len());
                    }
                } else {
                    println!("❌ Move operation failed");
                }
            }
        }
        
        Commands::ListPreviews { detailed, pending } => {
            let previews = preview_manager.list_previews();
            
            if previews.is_empty() {
                println!("No previews found");
            } else {
                println!("📋 Previews:");
                for preview in previews {
                    if pending && (preview.confirmed || preview.applied) {
                        continue;
                    }
                    
                    if detailed {
                        println!("\n🆔 {}", preview.id);
                        println!("   Created: {}", preview.created_at.format("%Y-%m-%d %H:%M"));
                        println!("   Status: {}", if preview.applied {
                            "Applied"
                        } else if preview.confirmed {
                            "Confirmed"
                        } else {
                            "Pending"
                        });
                        println!("   Operation: {:?}", match &preview.operation {
                            preview::PreviewOperation::Replace { .. } => "Replace",
                            preview::PreviewOperation::Insert { .. } => "Insert",
                            preview::PreviewOperation::MoveLines { .. } => "MoveLines",
                            preview::PreviewOperation::MoveClass { .. } => "MoveClass",
                            preview::PreviewOperation::Delete { .. } => "Delete",
                        });
                    } else {
                        let status = if preview.applied {
                            "✅"
                        } else if preview.confirmed {
                            "⏳"
                        } else {
                            "📝"
                        };
                        println!("  {} {} - {}", status, preview.id, preview.created_at.format("%H:%M"));
                    }
                }
            }
        }
        
        Commands::ExtractFunction { session_id, function, target, dry_run } => {
            let options = MoveOptions {
                dry_run,
                ..Default::default()
            };
            
            let mut engine = MoveClassEngine::new(options)?;
            let result = engine.move_symbol(&session_id, &function, &target).await?;
            
            if dry_run {
                println!("🔍 Dry run - no changes made");
            }
            
            if result.success {
                println!("✅ Successfully extracted {}", result.symbol_name);
                println!("   Moved to: {}", result.target_file.display());
                println!("   Lines: {}", result.lines_moved);
            } else {
                println!("❌ Extraction failed");
            }
        }
        
        Commands::SplitFile { file, by, output } => {
            // TODO: Implement file splitting
            println!("File splitting not yet implemented");
            println!("File: {}", file.display());
            println!("Split by: {}", by);
            if let Some(output) = output {
                println!("Output: {}", output.display());
            }
        }
        
        Commands::Smart { command } => {
            match command {
                SmartCommands::Insert { session_id, file, content, after_function, before_function, in_class, in_imports, line, preview } => {
                    // Create smart refactor instance
                    let smart = SmartRefactor::from_session_id(&session_id).await?;
                    
                    // Determine position
                    let position = if let Some(func) = after_function {
                        SmartPosition::AfterFunction(func)
                    } else if let Some(func) = before_function {
                        SmartPosition::BeforeFunction(func)
                    } else if let Some(class) = in_class {
                        SmartPosition::InClass(class)
                    } else if in_imports {
                        SmartPosition::InImports
                    } else if let Some(line_num) = line {
                        SmartPosition::Line(line_num)
                    } else {
                        return Err(NekocodeError::Config("No position specified for smart insert".to_string()));
                    };
                    
                    // Execute smart insert
                    let result = smart.smart_insert(&file, &content, position, preview).await?;
                    
                    if preview {
                        println!("🔍 Smart Insert Preview");
                        println!("{}", result.preview_text);
                    } else {
                        println!("✅ Smart insert completed at {}", result.position);
                    }
                }
                
                SmartCommands::Replace { session_id, file, pattern, replacement, in_class, in_function, regex, preview } => {
                    // Create smart refactor instance
                    let smart = SmartRefactor::from_session_id(&session_id).await?;
                    
                    // Determine scope
                    let scope = if let Some(class) = in_class {
                        Some(Scope::InClass(class))
                    } else if let Some(func) = in_function {
                        Some(Scope::InFunction(func))
                    } else {
                        None
                    };
                    
                    // Execute smart replace
                    let result = smart.smart_replace(&file, &pattern, &replacement, scope, regex, preview).await?;
                    
                    if preview {
                        println!("🔍 Smart Replace Preview");
                        println!("{}", result.preview_text);
                    } else {
                        println!("✅ Smart replace completed: {}", result.position);
                    }
                }
                
                SmartCommands::Move { session_id, symbol, target, update_imports, preview } => {
                    // Create smart refactor instance
                    let smart = SmartRefactor::from_session_id(&session_id).await?;
                    
                    // Execute smart move
                    let result = smart.smart_move(&symbol, &target, update_imports, preview).await?;
                    
                    if preview {
                        println!("🔍 Smart Move Preview");
                        println!("{}", result.preview_text);
                    } else {
                        println!("✅ Smart move completed: {}", result.position);
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Parse insert position from string
fn parse_insert_position(pos: &str) -> Result<InsertPosition> {
    match pos.to_lowercase().as_str() {
        "start" | "begin" => Ok(InsertPosition::Start),
        "end" => Ok(InsertPosition::End),
        s => {
            if let Ok(line) = s.parse::<u32>() {
                Ok(InsertPosition::Line(line))
            } else if s.starts_with("after:") {
                let line = s[6..].parse::<u32>()
                    .map_err(|_| NekocodeError::Config(format!("Invalid line number: {}", s)))?;
                Ok(InsertPosition::AfterLine(line))
            } else if s.starts_with("before:") {
                let line = s[7..].parse::<u32>()
                    .map_err(|_| NekocodeError::Config(format!("Invalid line number: {}", s)))?;
                Ok(InsertPosition::BeforeLine(line))
            } else {
                Err(NekocodeError::Config(format!("Invalid position: {}", pos)))
            }
        }
    }
}

/// Semantic position types
enum SemanticPosition {
    AfterFunction(String),
    BeforeFunction(String),
    InImports,
    AfterClass(String),
}

/// Find semantic position in file
fn find_semantic_position(file: &std::path::PathBuf, position: SemanticPosition) -> Result<InsertPosition> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| NekocodeError::Io(e))?;
    
    let lines: Vec<&str> = content.lines().collect();
    
    match position {
        SemanticPosition::AfterFunction(func_name) => {
            // Find function definition
            for (i, line) in lines.iter().enumerate() {
                if line.contains(&format!("def {}", func_name)) || 
                   line.contains(&format!("fn {}", func_name)) ||
                   line.contains(&format!("function {}", func_name)) {
                    // Find end of function (next function or end of file)
                    for j in i+1..lines.len() {
                        if lines[j].starts_with("def ") || 
                           lines[j].starts_with("fn ") ||
                           lines[j].starts_with("function ") ||
                           lines[j].starts_with("class ") ||
                           (!lines[j].starts_with(" ") && !lines[j].starts_with("\t") && !lines[j].is_empty()) {
                            return Ok(InsertPosition::Line(j as u32 + 1));
                        }
                    }
                    return Ok(InsertPosition::End);
                }
            }
            Err(NekocodeError::Refactoring(format!("Function '{}' not found", func_name)))
        }
        SemanticPosition::BeforeFunction(func_name) => {
            for (i, line) in lines.iter().enumerate() {
                if line.contains(&format!("def {}", func_name)) || 
                   line.contains(&format!("fn {}", func_name)) ||
                   line.contains(&format!("function {}", func_name)) {
                    return Ok(InsertPosition::Line(i as u32 + 1));
                }
            }
            Err(NekocodeError::Refactoring(format!("Function '{}' not found", func_name)))
        }
        SemanticPosition::InImports => {
            // Find last import statement
            let mut last_import = 0;
            for (i, line) in lines.iter().enumerate() {
                if line.starts_with("import ") || 
                   line.starts_with("from ") ||
                   line.starts_with("use ") ||
                   line.starts_with("#include") {
                    last_import = i;
                }
            }
            if last_import > 0 {
                Ok(InsertPosition::AfterLine(last_import as u32 + 1))
            } else {
                Ok(InsertPosition::Start)
            }
        }
        SemanticPosition::AfterClass(class_name) => {
            for (i, line) in lines.iter().enumerate() {
                if line.contains(&format!("class {}", class_name)) || 
                   line.contains(&format!("struct {}", class_name)) {
                    // Find end of class
                    let mut brace_count = 0;
                    for j in i..lines.len() {
                        brace_count += lines[j].chars().filter(|&c| c == '{').count() as i32;
                        brace_count -= lines[j].chars().filter(|&c| c == '}').count() as i32;
                        if brace_count == 0 && j > i {
                            return Ok(InsertPosition::AfterLine(j as u32 + 1));
                        }
                    }
                }
            }
            Err(NekocodeError::Refactoring(format!("Class '{}' not found", class_name)))
        }
    }
}

/// Get template content based on template name
fn get_template_content(template: Option<&str>, file: &std::path::PathBuf) -> Result<String> {
    match template {
        Some("python-cli") => Ok(r#"#!/usr/bin/env python3
"""
CLI application template
"""

import argparse
import sys

def main():
    parser = argparse.ArgumentParser(description='TODO: Add description')
    parser.add_argument('--verbose', '-v', action='store_true', help='Verbose output')
    args = parser.parse_args()
    
    print("Hello from Python CLI!")
    return 0

if __name__ == "__main__":
    sys.exit(main())
"#.to_string()),
        
        Some("rust-lib") => Ok(r#"//! Library module

use std::error::Error;

/// Main library function
pub fn process() -> Result<(), Box<dyn Error>> {
    println!("Hello from Rust library!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process() {
        assert!(process().is_ok());
    }
}
"#.to_string()),
        
        Some("js-module") => Ok(r#"/**
 * JavaScript module template
 */

export function hello() {
    console.log("Hello from JS module!");
}

export default {
    hello
};
"#.to_string()),
        
        Some(t) => Err(NekocodeError::Config(format!("Unknown template: {}", t))),
        
        None => {
            // Detect by file extension
            let ext = file.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
                
            match ext {
                "py" => Ok("#!/usr/bin/env python3\n\n".to_string()),
                "rs" => Ok("//! Module\n\n".to_string()),
                "js" | "mjs" => Ok("// JavaScript file\n\n".to_string()),
                "ts" => Ok("// TypeScript file\n\n".to_string()),
                _ => Ok("".to_string()),
            }
        }
    }
}