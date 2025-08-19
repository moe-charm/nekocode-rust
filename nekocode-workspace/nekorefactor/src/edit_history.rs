//! Edit history tracking for all refactoring operations
//! 
//! Tracks all edits made by nekorefactor, including:
//! - File modifications
//! - Comment removals
//! - Code insertions
//! - Replacements
//! 
//! Allows for easy rollback and audit trail

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Maximum number of history entries to keep
const MAX_HISTORY_ENTRIES: usize = 1000;

/// History directory name
const HISTORY_DIR: &str = ".nekocode_history";

/// Edit operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditOperation {
    /// Text replacement
    Replace {
        pattern: String,
        replacement: String,
        occurrences: usize,
    },
    
    /// Code insertion
    Insert {
        position: InsertPosition,
        content: String,
    },
    
    /// Line movement
    MoveLines {
        start_line: usize,
        line_count: usize,
        target_line: usize,
    },
    
    /// Comment removal
    StripComments {
        removed_count: usize,
        kept_count: usize,
        size_reduction: f64,
    },
    
    /// Class/symbol movement
    MoveSymbol {
        symbol: String,
        from_file: PathBuf,
        to_file: PathBuf,
    },
    
    /// File creation
    CreateFile {
        template: Option<String>,
    },
    
    /// File deletion
    DeleteFile,
}

/// Position for insertions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsertPosition {
    Line(usize),
    AfterFunction(String),
    BeforeFunction(String),
    InClass(String),
    InImports,
    EndOfFile,
}

/// Single edit entry in history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditEntry {
    /// Unique ID for this edit
    pub id: String,
    
    /// Timestamp of the edit
    pub timestamp: DateTime<Utc>,
    
    /// File that was edited
    pub file_path: PathBuf,
    
    /// Operation performed
    pub operation: EditOperation,
    
    /// Original content before edit (for rollback)
    pub original_content: String,
    
    /// New content after edit
    pub new_content: String,
    
    /// Size difference
    pub size_diff: i64,
    
    /// Session ID if part of a session
    pub session_id: Option<String>,
    
    /// User-provided description
    pub description: Option<String>,
    
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl EditEntry {
    /// Create a new edit entry
    pub fn new(
        file_path: PathBuf,
        operation: EditOperation,
        original_content: String,
        new_content: String,
    ) -> Self {
        let size_diff = new_content.len() as i64 - original_content.len() as i64;
        
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            file_path,
            operation,
            original_content,
            new_content,
            size_diff,
            session_id: None,
            description: None,
            tags: Vec::new(),
        }
    }
    
    /// Create entry for comment stripping
    pub fn from_strip_comments(
        file_path: PathBuf,
        original: String,
        stripped: String,
        removed_count: usize,
        kept_count: usize,
    ) -> Self {
        let size_reduction = if original.is_empty() {
            0.0
        } else {
            ((original.len() - stripped.len()) as f64 / original.len() as f64) * 100.0
        };
        
        let operation = EditOperation::StripComments {
            removed_count,
            kept_count,
            size_reduction,
        };
        
        let mut entry = Self::new(file_path, operation, original, stripped);
        entry.tags.push("comment-removal".to_string());
        entry
    }
    
    /// Get a summary of this edit
    pub fn summary(&self) -> String {
        match &self.operation {
            EditOperation::Replace { pattern, replacement, occurrences } => {
                format!("Replaced '{}' with '{}' ({} occurrences)", pattern, replacement, occurrences)
            }
            EditOperation::Insert { position, .. } => {
                format!("Inserted code at {:?}", position)
            }
            EditOperation::MoveLines { start_line, line_count, target_line } => {
                format!("Moved {} lines from line {} to {}", line_count, start_line, target_line)
            }
            EditOperation::StripComments { removed_count, kept_count, size_reduction } => {
                format!("Removed {} comments, kept {}, reduced size by {:.1}%", 
                    removed_count, kept_count, size_reduction)
            }
            EditOperation::MoveSymbol { symbol, from_file, to_file } => {
                format!("Moved '{}' from {:?} to {:?}", symbol, from_file, to_file)
            }
            EditOperation::CreateFile { template } => {
                match template {
                    Some(t) => format!("Created file from template '{}'", t),
                    None => "Created new file".to_string(),
                }
            }
            EditOperation::DeleteFile => "Deleted file".to_string(),
        }
    }
}

/// Edit history manager
#[derive(Debug)]
pub struct EditHistory {
    /// History entries (newest first)
    entries: VecDeque<EditEntry>,
    
    /// History directory path
    history_dir: PathBuf,
    
    /// Maximum entries to keep in memory
    max_entries: usize,
}

impl EditHistory {
    /// Create or load edit history
    pub fn new() -> Result<Self> {
        let history_dir = Path::new(HISTORY_DIR);
        Self::with_dir(history_dir)
    }
    
    /// Create history with custom directory
    pub fn with_dir(history_dir: &Path) -> Result<Self> {
        // Ensure history directory exists
        fs::create_dir_all(history_dir)?;
        
        // Load existing history
        let entries = Self::load_entries(history_dir)?;
        
        Ok(Self {
            entries,
            history_dir: history_dir.to_path_buf(),
            max_entries: MAX_HISTORY_ENTRIES,
        })
    }
    
    /// Add a new edit entry
    pub fn add_entry(&mut self, entry: EditEntry) -> Result<()> {
        // Add to front (newest first)
        self.entries.push_front(entry.clone());
        
        // Trim if exceeds max
        while self.entries.len() > self.max_entries {
            self.entries.pop_back();
        }
        
        // Persist to disk
        self.save_entry(&entry)?;
        
        Ok(())
    }
    
    /// Get recent entries
    pub fn get_recent(&self, count: usize) -> Vec<&EditEntry> {
        self.entries.iter().take(count).collect()
    }
    
    /// Get entry by ID
    pub fn get_by_id(&self, id: &str) -> Option<&EditEntry> {
        self.entries.iter().find(|e| e.id == id)
    }
    
    /// Get entries for a specific file
    pub fn get_by_file(&self, file_path: &Path) -> Vec<&EditEntry> {
        self.entries
            .iter()
            .filter(|e| e.file_path == file_path)
            .collect()
    }
    
    /// Get entries by session ID
    pub fn get_by_session(&self, session_id: &str) -> Vec<&EditEntry> {
        self.entries
            .iter()
            .filter(|e| e.session_id.as_deref() == Some(session_id))
            .collect()
    }
    
    /// Rollback an edit
    pub fn rollback(&self, entry_id: &str) -> Result<()> {
        let entry = self.get_by_id(entry_id)
            .ok_or_else(|| anyhow::anyhow!("Entry not found: {}", entry_id))?;
        
        // Write original content back to file
        fs::write(&entry.file_path, &entry.original_content)?;
        
        Ok(())
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> HistoryStats {
        let mut stats = HistoryStats::default();
        
        for entry in &self.entries {
            stats.total_edits += 1;
            stats.total_size_change += entry.size_diff;
            
            match entry.operation {
                EditOperation::Replace { .. } => stats.replacements += 1,
                EditOperation::Insert { .. } => stats.insertions += 1,
                EditOperation::MoveLines { .. } => stats.line_moves += 1,
                EditOperation::StripComments { .. } => stats.comment_removals += 1,
                EditOperation::MoveSymbol { .. } => stats.symbol_moves += 1,
                EditOperation::CreateFile { .. } => stats.files_created += 1,
                EditOperation::DeleteFile => stats.files_deleted += 1,
            }
        }
        
        if !self.entries.is_empty() {
            stats.oldest_entry = Some(self.entries.back().unwrap().timestamp);
            stats.newest_entry = Some(self.entries.front().unwrap().timestamp);
        }
        
        stats
    }
    
    /// Clear history
    pub fn clear(&mut self) -> Result<()> {
        self.entries.clear();
        
        // Remove all history files
        for entry in fs::read_dir(&self.history_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                fs::remove_file(entry.path())?;
            }
        }
        
        Ok(())
    }
    
    /// Save entry to disk
    fn save_entry(&self, entry: &EditEntry) -> Result<()> {
        let file_name = format!("{}.json", entry.id);
        let file_path = self.history_dir.join(file_name);
        
        let json = serde_json::to_string_pretty(entry)?;
        fs::write(file_path, json)?;
        
        Ok(())
    }
    
    /// Load entries from disk
    fn load_entries(history_dir: &Path) -> Result<VecDeque<EditEntry>> {
        let mut entries = VecDeque::new();
        
        if !history_dir.exists() {
            return Ok(entries);
        }
        
        // Read all JSON files
        let mut files: Vec<_> = fs::read_dir(history_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().extension().and_then(|s| s.to_str()) == Some("json")
            })
            .collect();
        
        // Sort by modification time (newest first)
        files.sort_by_key(|entry| {
            entry.metadata()
                .and_then(|m| m.modified())
                .unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH)
        });
        files.reverse();
        
        // Load entries (up to max)
        for file in files.iter().take(MAX_HISTORY_ENTRIES) {
            let content = fs::read_to_string(file.path())?;
            if let Ok(entry) = serde_json::from_str::<EditEntry>(&content) {
                entries.push_back(entry);
            }
        }
        
        Ok(entries)
    }
}

/// History statistics
#[derive(Debug, Default)]
pub struct HistoryStats {
    pub total_edits: usize,
    pub replacements: usize,
    pub insertions: usize,
    pub line_moves: usize,
    pub comment_removals: usize,
    pub symbol_moves: usize,
    pub files_created: usize,
    pub files_deleted: usize,
    pub total_size_change: i64,
    pub oldest_entry: Option<DateTime<Utc>>,
    pub newest_entry: Option<DateTime<Utc>>,
}

impl HistoryStats {
    pub fn display(&self) -> String {
        format!(
            r#"📊 Edit History Statistics
================================
Total edits: {}
  - Replacements: {}
  - Insertions: {}
  - Line moves: {}
  - Comment removals: {}
  - Symbol moves: {}
  - Files created: {}
  - Files deleted: {}

Total size change: {} bytes
Period: {} to {}
"#,
            self.total_edits,
            self.replacements,
            self.insertions,
            self.line_moves,
            self.comment_removals,
            self.symbol_moves,
            self.files_created,
            self.files_deleted,
            self.total_size_change,
            self.oldest_entry
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            self.newest_entry
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "N/A".to_string()),
        )
    }
}

/// Global history instance
static mut GLOBAL_HISTORY: Option<EditHistory> = None;
static HISTORY_INIT: std::sync::Once = std::sync::Once::new();

/// Get the global history instance
pub fn get_history() -> &'static mut EditHistory {
    unsafe {
        HISTORY_INIT.call_once(|| {
            GLOBAL_HISTORY = Some(EditHistory::new().expect("Failed to initialize history"));
        });
        GLOBAL_HISTORY.as_mut().expect("History not initialized")
    }
}

/// Record an edit in global history
pub fn record_edit(entry: EditEntry) -> Result<()> {
    get_history().add_entry(entry)
}