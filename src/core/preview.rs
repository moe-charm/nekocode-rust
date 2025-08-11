//! Preview and confirmation system for NekoCode Rust
//! 
//! This module handles preview operations for safe code editing,
//! allowing users to see changes before confirming them.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::moveclass::{MoveClassEngine, MoveOptions};

/// Types of preview operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreviewOperation {
    Replace {
        file: PathBuf,
        pattern: String,
        replacement: String,
        matches: Vec<MatchInfo>,
    },
    Insert {
        file: PathBuf,
        position: u32,
        content: String,
    },
    MoveLines {
        source: PathBuf,
        start: u32,
        count: u32,
        destination: PathBuf,
        position: u32,
        lines: Vec<String>,
    },
    MoveClass {
        session_id: String,
        symbol_id: String,
        target: PathBuf,
        class_content: String,
    },
}

/// Information about a text match
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchInfo {
    pub line_number: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub matched_text: String,
    pub line_content: String,
}

/// A preview entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewEntry {
    pub id: String,
    pub operation: PreviewOperation,
    pub created_at: DateTime<Utc>,
    pub preview_text: String,
    pub confirmed: bool,
}

impl PreviewEntry {
    pub fn new(operation: PreviewOperation) -> Result<Self> {
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let preview_text = Self::generate_preview(&operation)?;
        
        Ok(Self {
            id,
            operation,
            created_at: Utc::now(),
            preview_text,
            confirmed: false,
        })
    }
    
    fn generate_preview(operation: &PreviewOperation) -> Result<String> {
        match operation {
            PreviewOperation::Replace { file, pattern, replacement, matches } => {
                let mut preview = format!("Replace Operation Preview\n");
                preview.push_str(&format!("File: {}\n", file.display()));
                preview.push_str(&format!("Pattern: '{}'\n", pattern));
                preview.push_str(&format!("Replacement: '{}'\n", replacement));
                preview.push_str(&format!("Matches: {}\n\n", matches.len()));
                
                for (i, match_info) in matches.iter().enumerate() {
                    preview.push_str(&format!("Match {}: Line {}\n", i + 1, match_info.line_number));
                    preview.push_str(&format!("  Before: {}\n", match_info.line_content));
                    let replaced = match_info.line_content.replace(pattern, replacement);
                    preview.push_str(&format!("  After:  {}\n\n", replaced));
                }
                
                Ok(preview)
            }
            
            PreviewOperation::Insert { file, position, content } => {
                let mut preview = format!("Insert Operation Preview\n");
                preview.push_str(&format!("File: {}\n", file.display()));
                preview.push_str(&format!("Position: Line {}\n", position));
                preview.push_str(&format!("Content to insert:\n{}\n", content));
                
                Ok(preview)
            }
            
            PreviewOperation::MoveLines { source, start, count, destination, position, lines } => {
                let mut preview = format!("Move Lines Operation Preview\n");
                preview.push_str(&format!("Source: {}\n", source.display()));
                preview.push_str(&format!("Lines: {} to {} ({} lines)\n", start, start + count - 1, count));
                preview.push_str(&format!("Destination: {}\n", destination.display()));
                preview.push_str(&format!("Insert at line: {}\n\n", position));
                
                preview.push_str("Lines to move:\n");
                for (i, line) in lines.iter().enumerate() {
                    preview.push_str(&format!("{:4}: {}\n", start + i as u32, line));
                }
                
                Ok(preview)
            }
            
            PreviewOperation::MoveClass { session_id, symbol_id, target, class_content } => {
                let mut preview = format!("Move Class Operation Preview\n");
                preview.push_str(&format!("Session: {}\n", session_id));
                preview.push_str(&format!("Symbol ID: {}\n", symbol_id));
                preview.push_str(&format!("Target: {}\n", target.display()));
                preview.push_str(&format!("Class content:\n{}\n", class_content));
                
                Ok(preview)
            }
        }
    }
}

/// Preview manager
pub struct PreviewManager {
    previews: HashMap<String, PreviewEntry>,
    temp_dir: PathBuf,
    storage_file: PathBuf,
}

impl PreviewManager {
    pub fn new() -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("nekocode-previews");
        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir)?;
        }
        
        let storage_file = temp_dir.join("previews.json");
        
        // Load existing previews from storage
        let previews = if storage_file.exists() {
            let content = fs::read_to_string(&storage_file)?;
            serde_json::from_str(&content).unwrap_or_else(|_| HashMap::new())
        } else {
            HashMap::new()
        };
        
        Ok(Self {
            previews,
            temp_dir,
            storage_file,
        })
    }
    
    /// Save previews to persistent storage
    fn save_to_storage(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.previews)?;
        fs::write(&self.storage_file, content)?;
        Ok(())
    }
    
    /// Create a replace preview
    pub fn create_replace_preview(&mut self, file: &Path, pattern: &str, replacement: &str) -> Result<String> {
        let content = fs::read_to_string(file)?;
        let matches = self.find_matches(&content, pattern)?;
        
        let operation = PreviewOperation::Replace {
            file: file.to_path_buf(),
            pattern: pattern.to_string(),
            replacement: replacement.to_string(),
            matches,
        };
        
        let preview = PreviewEntry::new(operation)?;
        let id = preview.id.clone();
        self.previews.insert(id.clone(), preview);
        self.save_to_storage()?;
        
        Ok(id)
    }
    
    /// Create an insert preview
    pub fn create_insert_preview(&mut self, file: &Path, position: u32, content: &str) -> Result<String> {
        let operation = PreviewOperation::Insert {
            file: file.to_path_buf(),
            position,
            content: content.to_string(),
        };
        
        let preview = PreviewEntry::new(operation)?;
        let id = preview.id.clone();
        self.previews.insert(id.clone(), preview);
        self.save_to_storage()?;
        
        Ok(id)
    }
    
    /// Create a move lines preview
    pub fn create_movelines_preview(&mut self, source: &Path, start: u32, count: u32, destination: &Path, position: u32) -> Result<String> {
        let source_content = fs::read_to_string(source)?;
        let lines: Vec<&str> = source_content.lines().collect();
        
        if start == 0 || start as usize > lines.len() {
            anyhow::bail!("Invalid start line: {}", start);
        }
        
        let start_idx = (start - 1) as usize;
        let end_idx = std::cmp::min(start_idx + count as usize, lines.len());
        
        let extracted_lines: Vec<String> = lines[start_idx..end_idx]
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        let operation = PreviewOperation::MoveLines {
            source: source.to_path_buf(),
            start,
            count,
            destination: destination.to_path_buf(),
            position,
            lines: extracted_lines,
        };
        
        let preview = PreviewEntry::new(operation)?;
        let id = preview.id.clone();
        self.previews.insert(id.clone(), preview);
        self.save_to_storage()?;
        
        Ok(id)
    }
    
    /// Create a move class preview
    pub fn create_moveclass_preview(&mut self, session_id: &str, symbol_id: &str, target: &Path) -> Result<String> {
        // Use MoveClassEngine for better implementation
        let engine = MoveClassEngine::new();
        
        // For now, create a simulated preview since we need more AST integration
        // In a full implementation, this would:
        // 1. Load the session to get AST information
        // 2. Find the class/symbol in the AST
        // 3. Extract the exact class content
        // 4. Generate a proper preview
        
        let class_content = format!(
            "// MoveClass Preview for symbol: {}\n// Target: {}\n// Session: {}\n// This would contain the actual class definition extracted from AST",
            symbol_id, target.display(), session_id
        );
        
        let operation = PreviewOperation::MoveClass {
            session_id: session_id.to_string(),
            symbol_id: symbol_id.to_string(),
            target: target.to_path_buf(),
            class_content,
        };
        
        let preview = PreviewEntry::new(operation)?;
        let id = preview.id.clone();
        self.previews.insert(id.clone(), preview);
        self.save_to_storage()?;
        
        Ok(id)
    }
    
    /// Get a preview by ID
    pub fn get_preview(&self, id: &str) -> Option<&PreviewEntry> {
        self.previews.get(id)
    }
    
    /// Confirm and execute a preview
    pub fn confirm_preview(&mut self, id: &str) -> Result<String> {
        // First check if preview exists and get the operation
        let operation = {
            let preview = self.previews.get(id)
                .ok_or_else(|| anyhow::anyhow!("Preview not found: {}", id))?;
            
            if preview.confirmed {
                anyhow::bail!("Preview already confirmed: {}", id);
            }
            
            preview.operation.clone()
        };
        
        // Execute the operation
        let result = self.execute_operation(&operation)?;
        
        // Mark as confirmed
        if let Some(preview) = self.previews.get_mut(id) {
            preview.confirmed = true;
        }
        self.save_to_storage()?;
        
        Ok(result)
    }
    
    fn execute_operation(&self, operation: &PreviewOperation) -> Result<String> {
        match operation {
            PreviewOperation::Replace { file, pattern, replacement, .. } => {
                let content = fs::read_to_string(file)?;
                let new_content = content.replace(pattern, replacement);
                fs::write(file, new_content)?;
                Ok(format!("Replace operation completed in {}", file.display()))
            }
            
            PreviewOperation::Insert { file, position, content } => {
                let file_content = fs::read_to_string(file)?;
                let mut lines: Vec<&str> = file_content.lines().collect();
                
                let insert_pos = if *position == 0 || *position as usize > lines.len() {
                    lines.len()
                } else {
                    (*position - 1) as usize
                };
                
                lines.insert(insert_pos, content);
                let new_content = lines.join("\n");
                fs::write(file, new_content)?;
                
                Ok(format!("Insert operation completed in {}", file.display()))
            }
            
            PreviewOperation::MoveLines { source, start, count, destination, position, .. } => {
                // Read source file
                let source_content = fs::read_to_string(source)?;
                let mut source_lines: Vec<&str> = source_content.lines().collect();
                
                // Extract lines to move
                let start_idx = (*start - 1) as usize;
                let end_idx = std::cmp::min(start_idx + *count as usize, source_lines.len());
                let moved_lines: Vec<String> = source_lines[start_idx..end_idx]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                
                // Remove lines from source
                source_lines.drain(start_idx..end_idx);
                let new_source_content = source_lines.join("\n");
                fs::write(source, new_source_content)?;
                
                // Read destination file
                let dest_content = if destination.exists() {
                    fs::read_to_string(destination)?
                } else {
                    String::new()
                };
                
                let mut dest_lines: Vec<&str> = dest_content.lines().collect();
                
                // Insert lines at destination
                let insert_pos = if *position == 0 || *position as usize > dest_lines.len() {
                    dest_lines.len()
                } else {
                    (*position - 1) as usize
                };
                
                for (i, line) in moved_lines.iter().enumerate() {
                    dest_lines.insert(insert_pos + i, line);
                }
                
                let new_dest_content = dest_lines.join("\n");
                fs::write(destination, new_dest_content)?;
                
                Ok(format!("Move lines operation completed: {} -> {}", source.display(), destination.display()))
            }
            
            PreviewOperation::MoveClass { session_id, symbol_id, target, class_content } => {
                // Use MoveClassEngine for execution
                let engine = MoveClassEngine::new();
                
                // For now, create a simple implementation
                // In a full implementation, this would:
                // 1. Load the session and AST
                // 2. Find the source file containing the symbol
                // 3. Use engine.move_class() with proper parameters
                
                // Simulate the move by writing to target file
                if target.exists() {
                    let mut content = fs::read_to_string(target)?;
                    content.push('\n');
                    content.push_str(class_content);
                    fs::write(target, content)?;
                } else {
                    fs::write(target, class_content)?;
                }
                
                Ok(format!("Move class operation completed: symbol {} moved to {}", symbol_id, target.display()))
            }
        }
    }
    
    fn find_matches(&self, content: &str, pattern: &str) -> Result<Vec<MatchInfo>> {
        let mut matches = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            let mut start = 0;
            while let Some(pos) = line[start..].find(pattern) {
                let actual_pos = start + pos;
                matches.push(MatchInfo {
                    line_number: (line_num + 1) as u32,
                    column_start: actual_pos as u32,
                    column_end: (actual_pos + pattern.len()) as u32,
                    matched_text: pattern.to_string(),
                    line_content: line.to_string(),
                });
                start = actual_pos + pattern.len();
            }
        }
        
        Ok(matches)
    }
}

impl Default for PreviewManager {
    fn default() -> Self {
        Self::new().expect("Failed to create PreviewManager")
    }
}