//! Preview and confirmation system for refactoring operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use nekocode_core::{Result, NekocodeError};

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
        position: InsertPosition,
        content: String,
    },
    MoveLines {
        source: PathBuf,
        start_line: u32,
        line_count: u32,
        destination: PathBuf,
        insert_line: u32,
        lines: Vec<String>,
    },
    MoveClass {
        session_id: String,
        symbol_id: String,
        source_file: PathBuf,
        target_file: PathBuf,
        class_content: String,
    },
    Delete {
        file: PathBuf,
        start_line: u32,
        end_line: u32,
        content: Vec<String>,
    },
}

/// Position for insert operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsertPosition {
    Start,
    End,
    Line(u32),
    AfterLine(u32),
    BeforeLine(u32),
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
    pub applied: bool,
}

impl PreviewEntry {
    /// Create new preview entry
    pub fn new(operation: PreviewOperation) -> Result<Self> {
        let id = Uuid::new_v4().to_string()[..8].to_string();
        let preview_text = Self::generate_preview(&operation)?;
        
        Ok(Self {
            id,
            operation,
            created_at: Utc::now(),
            preview_text,
            confirmed: false,
            applied: false,
        })
    }
    
    /// Generate preview text for operation
    fn generate_preview(operation: &PreviewOperation) -> Result<String> {
        match operation {
            PreviewOperation::Replace { file, pattern, replacement, matches } => {
                let mut preview = String::new();
                preview.push_str("🔄 Replace Operation Preview\n");
                preview.push_str(&format!("📁 File: {}\n", file.display()));
                preview.push_str(&format!("🔍 Pattern: '{}'\n", pattern));
                preview.push_str(&format!("✏️  Replacement: '{}'\n", replacement));
                preview.push_str(&format!("📊 Matches: {}\n\n", matches.len()));
                
                for (i, match_info) in matches.iter().enumerate().take(5) {
                    preview.push_str(&format!("Match #{}: Line {}\n", i + 1, match_info.line_number));
                    preview.push_str(&format!("  Before: {}\n", match_info.line_content.trim()));
                    let replaced = match_info.line_content.replace(pattern, replacement);
                    preview.push_str(&format!("  After:  {}\n\n", replaced.trim()));
                }
                
                if matches.len() > 5 {
                    preview.push_str(&format!("... and {} more matches\n", matches.len() - 5));
                }
                
                Ok(preview)
            }
            
            PreviewOperation::Insert { file, position, content } => {
                let mut preview = String::new();
                preview.push_str("➕ Insert Operation Preview\n");
                preview.push_str(&format!("📁 File: {}\n", file.display()));
                preview.push_str(&format!("📍 Position: {:?}\n", position));
                preview.push_str("📝 Content to insert:\n");
                
                let lines: Vec<&str> = content.lines().collect();
                for (i, line) in lines.iter().enumerate().take(10) {
                    preview.push_str(&format!("  {}: {}\n", i + 1, line));
                }
                
                if lines.len() > 10 {
                    preview.push_str(&format!("  ... and {} more lines\n", lines.len() - 10));
                }
                
                Ok(preview)
            }
            
            PreviewOperation::MoveLines { source, start_line, line_count, destination, insert_line, lines } => {
                let mut preview = String::new();
                preview.push_str("🚚 Move Lines Operation Preview\n");
                preview.push_str(&format!("📁 Source: {}\n", source.display()));
                preview.push_str(&format!("📍 Lines: {} to {}\n", start_line, start_line + line_count - 1));
                preview.push_str(&format!("📁 Destination: {}\n", destination.display()));
                preview.push_str(&format!("📍 Insert at line: {}\n", insert_line));
                preview.push_str(&format!("📊 Lines to move: {}\n\n", lines.len()));
                
                for (i, line) in lines.iter().enumerate().take(5) {
                    preview.push_str(&format!("  {}: {}\n", start_line + i as u32, line));
                }
                
                if lines.len() > 5 {
                    preview.push_str(&format!("  ... and {} more lines\n", lines.len() - 5));
                }
                
                Ok(preview)
            }
            
            PreviewOperation::MoveClass { session_id, symbol_id, source_file, target_file, class_content } => {
                let mut preview = String::new();
                preview.push_str("🏗️ Move Class Operation Preview\n");
                preview.push_str(&format!("🆔 Session: {}\n", session_id));
                preview.push_str(&format!("🏷️ Symbol: {}\n", symbol_id));
                preview.push_str(&format!("📁 Source: {}\n", source_file.display()));
                preview.push_str(&format!("📁 Target: {}\n", target_file.display()));
                
                let lines: Vec<&str> = class_content.lines().collect();
                preview.push_str(&format!("📊 Lines: {}\n\n", lines.len()));
                
                for (i, line) in lines.iter().enumerate().take(10) {
                    preview.push_str(&format!("  {}: {}\n", i + 1, line));
                }
                
                if lines.len() > 10 {
                    preview.push_str(&format!("  ... and {} more lines\n", lines.len() - 10));
                }
                
                Ok(preview)
            }
            
            PreviewOperation::Delete { file, start_line, end_line, content } => {
                let mut preview = String::new();
                preview.push_str("🗑️ Delete Operation Preview\n");
                preview.push_str(&format!("📁 File: {}\n", file.display()));
                preview.push_str(&format!("📍 Lines: {} to {}\n", start_line, end_line));
                preview.push_str(&format!("📊 Lines to delete: {}\n\n", content.len()));
                
                for (i, line) in content.iter().enumerate().take(5) {
                    preview.push_str(&format!("  {}: {}\n", start_line + i as u32, line));
                }
                
                if content.len() > 5 {
                    preview.push_str(&format!("  ... and {} more lines\n", content.len() - 5));
                }
                
                Ok(preview)
            }
        }
    }
}

/// Preview manager for handling multiple previews
pub struct PreviewManager {
    previews: HashMap<String, PreviewEntry>,
    max_previews: usize,
}

impl PreviewManager {
    /// Create new preview manager
    pub fn new() -> Self {
        Self {
            previews: HashMap::new(),
            max_previews: 100,
        }
    }
    
    /// Add a preview
    pub fn add_preview(&mut self, operation: PreviewOperation) -> Result<String> {
        // Clean up old previews if we hit the limit
        if self.previews.len() >= self.max_previews {
            self.cleanup_old_previews();
        }
        
        let entry = PreviewEntry::new(operation)?;
        let id = entry.id.clone();
        self.previews.insert(id.clone(), entry);
        Ok(id)
    }
    
    /// Get a preview by ID
    pub fn get_preview(&self, id: &str) -> Option<&PreviewEntry> {
        self.previews.get(id)
    }
    
    /// Get mutable preview by ID
    pub fn get_preview_mut(&mut self, id: &str) -> Option<&mut PreviewEntry> {
        self.previews.get_mut(id)
    }
    
    /// Confirm a preview
    pub fn confirm_preview(&mut self, id: &str) -> Result<()> {
        let preview = self.previews.get_mut(id)
            .ok_or_else(|| NekocodeError::Preview(format!("Preview not found: {}", id)))?;
        
        if preview.applied {
            return Err(NekocodeError::Preview("Preview already applied".to_string()));
        }
        
        preview.confirmed = true;
        Ok(())
    }
    
    /// Apply a confirmed preview
    pub fn apply_preview(&mut self, id: &str) -> Result<()> {
        // First check the preview status
        {
            let preview = self.previews.get(id)
                .ok_or_else(|| NekocodeError::Preview(format!("Preview not found: {}", id)))?;
            
            if !preview.confirmed {
                return Err(NekocodeError::Preview("Preview not confirmed".to_string()));
            }
            
            if preview.applied {
                return Err(NekocodeError::Preview("Preview already applied".to_string()));
            }
        }
        
        // Get operation to apply
        let operation = {
            let preview = self.previews.get(id).unwrap();
            preview.operation.clone()
        };
        
        // Apply the operation based on type
        match &operation {
            PreviewOperation::Replace { file, pattern, replacement, .. } => {
                self.apply_replace(file, pattern, replacement)?;
            }
            PreviewOperation::Insert { file, position, content } => {
                self.apply_insert(file, position, content)?;
            }
            PreviewOperation::MoveLines { source, start_line, line_count, destination, insert_line, .. } => {
                self.apply_move_lines(source, *start_line, *line_count, destination, *insert_line)?;
            }
            PreviewOperation::Delete { file, start_line, end_line, .. } => {
                self.apply_delete(file, *start_line, *end_line)?;
            }
            PreviewOperation::MoveClass { .. } => {
                // MoveClass requires more complex handling
                return Err(NekocodeError::Preview("MoveClass requires session context".to_string()));
            }
        }
        
        // Mark as applied
        if let Some(preview) = self.previews.get_mut(id) {
            preview.applied = true;
        }
        
        Ok(())
    }
    
    /// Apply replace operation
    fn apply_replace(&self, file: &Path, pattern: &str, replacement: &str) -> Result<()> {
        let content = fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let new_content = content.replace(pattern, replacement);
        
        fs::write(file, new_content)
            .map_err(|e| NekocodeError::Io(e))?;
        
        Ok(())
    }
    
    /// Apply insert operation
    fn apply_insert(&self, file: &Path, position: &InsertPosition, content: &str) -> Result<()> {
        let file_content = fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let lines: Vec<&str> = file_content.lines().collect();
        let mut new_lines = Vec::new();
        
        match position {
            InsertPosition::Start => {
                new_lines.push(content);
                for line in lines {
                    new_lines.push(line);
                }
            }
            InsertPosition::End => {
                for line in lines {
                    new_lines.push(line);
                }
                new_lines.push(content);
            }
            InsertPosition::Line(n) | InsertPosition::AfterLine(n) => {
                let insert_at = *n as usize;
                for (i, line) in lines.iter().enumerate() {
                    if i == insert_at {
                        new_lines.push(content);
                    }
                    new_lines.push(line);
                }
            }
            InsertPosition::BeforeLine(n) => {
                let insert_at = (*n as usize).saturating_sub(1);
                for (i, line) in lines.iter().enumerate() {
                    if i == insert_at {
                        new_lines.push(content);
                    }
                    new_lines.push(line);
                }
            }
        }
        
        let new_content = new_lines.join("\n");
        fs::write(file, new_content)
            .map_err(|e| NekocodeError::Io(e))?;
        
        Ok(())
    }
    
    /// Apply move lines operation
    fn apply_move_lines(
        &self,
        source: &Path,
        start_line: u32,
        line_count: u32,
        destination: &Path,
        insert_line: u32
    ) -> Result<()> {
        // Read source file
        let source_content = fs::read_to_string(source)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let source_lines: Vec<&str> = source_content.lines().collect();
        
        // Extract lines to move
        let start_idx = (start_line - 1) as usize;
        let end_idx = start_idx + line_count as usize;
        
        if end_idx > source_lines.len() {
            return Err(NekocodeError::Preview("Line range out of bounds".to_string()));
        }
        
        let lines_to_move: Vec<String> = source_lines[start_idx..end_idx]
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        // Remove lines from source
        let mut new_source_lines: Vec<&str> = Vec::new();
        for (i, line) in source_lines.iter().enumerate() {
            if i < start_idx || i >= end_idx {
                new_source_lines.push(line);
            }
        }
        
        // Write updated source
        let new_source_content = new_source_lines.join("\n");
        fs::write(source, new_source_content)
            .map_err(|e| NekocodeError::Io(e))?;
        
        // Insert into destination
        if source == destination {
            // Moving within same file - need to adjust insert position
            // TODO: Implement same-file move logic
            return Ok(());
        }
        
        let dest_content = fs::read_to_string(destination)
            .unwrap_or_default();
        
        let mut dest_lines: Vec<String> = dest_content.lines().map(|s| s.to_string()).collect();
        let insert_idx = (insert_line - 1) as usize;
        
        for (i, line) in lines_to_move.iter().enumerate() {
            dest_lines.insert(insert_idx + i, line.clone());
        }
        
        let new_dest_content = dest_lines.join("\n");
        fs::write(destination, new_dest_content)
            .map_err(|e| NekocodeError::Io(e))?;
        
        Ok(())
    }
    
    /// Apply delete operation
    fn apply_delete(&self, file: &Path, start_line: u32, end_line: u32) -> Result<()> {
        let content = fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();
        
        for (i, line) in lines.iter().enumerate() {
            let line_num = (i + 1) as u32;
            if line_num < start_line || line_num > end_line {
                new_lines.push(*line);
            }
        }
        
        let new_content = new_lines.join("\n");
        fs::write(file, new_content)
            .map_err(|e| NekocodeError::Io(e))?;
        
        Ok(())
    }
    
    /// List all previews
    pub fn list_previews(&self) -> Vec<&PreviewEntry> {
        let mut previews: Vec<_> = self.previews.values().collect();
        previews.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        previews
    }
    
    /// Clean up old confirmed/applied previews
    fn cleanup_old_previews(&mut self) {
        let cutoff = Utc::now() - chrono::Duration::hours(1);
        
        self.previews.retain(|_, preview| {
            preview.created_at > cutoff || (!preview.confirmed && !preview.applied)
        });
    }
}

impl Default for PreviewManager {
    fn default() -> Self {
        Self::new()
    }
}