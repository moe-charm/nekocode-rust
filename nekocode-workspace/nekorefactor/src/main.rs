//! NekoRefactor - Code refactoring tool

mod preview;
mod replace;
mod moveclass;
mod cli;

use clap::Parser;
use std::io::{self, Read};

use nekocode_core::{Result, NekocodeError};
use crate::cli::{Cli, Commands};
use crate::preview::{PreviewManager, InsertPosition};
use crate::replace::{ReplaceEngine, ReplaceOptions};
use crate::moveclass::{MoveClassEngine, MoveOptions};

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
        Commands::ReplacePreview { file, pattern, replacement, regex, ignore_case, whole_word } => {
            let options = ReplaceOptions {
                use_regex: regex,
                case_sensitive: !ignore_case,
                whole_word,
                multiline: false,
            };
            
            let engine = ReplaceEngine::new(options);
            let preview_op = engine.create_preview(file, pattern, replacement)?;
            let preview_id = preview_manager.add_preview(preview_op)?;
            
            let preview = preview_manager.get_preview(&preview_id).unwrap();
            println!("{}", preview.preview_text);
            println!("\n✨ Preview ID: {}", preview_id);
            println!("Use 'nekorefactor replace-confirm {}' to apply changes", preview_id);
        }
        
        Commands::ReplaceConfirm { preview_id } => {
            preview_manager.confirm_preview(&preview_id)?;
            preview_manager.apply_preview(&preview_id)?;
            println!("✅ Preview {} applied successfully", preview_id);
        }
        
        Commands::InsertPreview { file, position, content } => {
            let insert_content = if content == "-" {
                // Read from stdin
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)
                    .map_err(|e| NekocodeError::Io(e))?;
                buffer
            } else {
                content
            };
            
            let insert_pos = parse_insert_position(&position)?;
            
            let preview_op = preview::PreviewOperation::Insert {
                file,
                position: insert_pos,
                content: insert_content,
            };
            
            let preview_id = preview_manager.add_preview(preview_op)?;
            let preview = preview_manager.get_preview(&preview_id).unwrap();
            
            println!("{}", preview.preview_text);
            println!("\n✨ Preview ID: {}", preview_id);
        }
        
        Commands::MoveLinesPreview { source, start_line, line_count, destination, insert_line } => {
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
            let preview = preview_manager.get_preview(&preview_id).unwrap();
            
            println!("{}", preview.preview_text);
            println!("\n✨ Preview ID: {}", preview_id);
        }
        
        Commands::MoveClassPreview { session_id, symbol_id, target, update_imports } => {
            let options = MoveOptions {
                update_imports,
                ..Default::default()
            };
            
            let mut engine = MoveClassEngine::new(options)?;
            
            // Create preview (dry run)
            let mut dry_run_engine = MoveClassEngine::new(MoveOptions {
                dry_run: true,
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
            
            // Create preview operation for confirmation
            let preview_op = preview::PreviewOperation::MoveClass {
                session_id: session_id.clone(),
                symbol_id: symbol_id.clone(),
                source_file: result.source_file,
                target_file: result.target_file,
                class_content: String::new(), // Will be filled during actual move
            };
            
            let preview_id = preview_manager.add_preview(preview_op)?;
            println!("\n✨ Preview ID: {}", preview_id);
            println!("Use 'nekorefactor moveclass-confirm {}' to apply", preview_id);
        }
        
        Commands::MoveClassConfirm { preview_id } => {
            let preview = preview_manager.get_preview(&preview_id)
                .ok_or_else(|| NekocodeError::Preview(format!("Preview not found: {}", preview_id)))?;
            
            if let preview::PreviewOperation::MoveClass { session_id, symbol_id, target_file, .. } = &preview.operation {
                let mut engine = MoveClassEngine::default()?;
                let result = engine.move_symbol(session_id, symbol_id, target_file).await?;
                
                if result.success {
                    println!("✅ Successfully moved {}", result.symbol_name);
                    println!("   {} lines moved", result.lines_moved);
                    if !result.imports_updated.is_empty() {
                        println!("   {} imports updated", result.imports_updated.len());
                    }
                } else {
                    println!("❌ Move operation failed");
                }
            } else {
                return Err(NekocodeError::Preview("Invalid preview type for moveclass-confirm".to_string()));
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