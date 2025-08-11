//! Memory system for NekoCode Rust
//! 
//! This module implements the time-axis memory revolution system
//! for storing and retrieving analysis results, user memos, and cached data.

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Memory types supported by the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryType {
    #[serde(rename = "auto")]
    Auto,    // 🤖 Auto-generated analysis results
    #[serde(rename = "memo")]
    Memo,    // 📝 Manual user memos
    #[serde(rename = "api")]
    Api,     // 🌐 External API integration data
    #[serde(rename = "cache")]
    Cache,   // 💾 Temporary cached data
}

impl std::str::FromStr for MemoryType {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(MemoryType::Auto),
            "memo" => Ok(MemoryType::Memo),
            "api" => Ok(MemoryType::Api),
            "cache" => Ok(MemoryType::Cache),
            _ => anyhow::bail!("Invalid memory type: {}", s),
        }
    }
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryType::Auto => write!(f, "auto"),
            MemoryType::Memo => write!(f, "memo"),
            MemoryType::Api => write!(f, "api"),
            MemoryType::Cache => write!(f, "cache"),
        }
    }
}

/// A memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub name: String,
    pub memory_type: MemoryType,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl MemoryEntry {
    pub fn new(name: String, memory_type: MemoryType, content: String) -> Self {
        let now = Utc::now();
        let id = format!("{}-{}", memory_type, uuid::Uuid::new_v4().to_string()[..8].to_string());
        
        Self {
            id,
            name,
            memory_type,
            content,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }
}

/// Memory system manager
pub struct MemoryManager {
    storage_path: PathBuf,
}

impl MemoryManager {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        if !storage_path.exists() {
            fs::create_dir_all(&storage_path)?;
        }
        
        Ok(Self { storage_path })
    }
    
    /// Save a memory entry
    pub fn save(&self, name: &str, memory_type: MemoryType, content: &str) -> Result<String> {
        let entry = MemoryEntry::new(name.to_string(), memory_type.clone(), content.to_string());
        
        let type_dir = self.storage_path.join(memory_type.to_string());
        fs::create_dir_all(&type_dir)?;
        
        let file_path = type_dir.join(format!("{}.json", entry.id));
        let json = serde_json::to_string_pretty(&entry)?;
        fs::write(file_path, json)?;
        
        Ok(entry.id)
    }
    
    /// Load a memory entry by name and type
    pub fn load(&self, name: &str, memory_type: MemoryType) -> Result<MemoryEntry> {
        let entries = self.list_by_type(memory_type.clone())?;
        
        for entry in entries {
            if entry.name == name {
                return Ok(entry);
            }
        }
        
        anyhow::bail!("Memory not found: {} of type {}", name, memory_type);
    }
    
    /// List all memories, optionally filtered by type
    pub fn list(&self, memory_type: Option<MemoryType>) -> Result<Vec<MemoryEntry>> {
        if let Some(mem_type) = memory_type {
            self.list_by_type(mem_type)
        } else {
            let mut all_entries = Vec::new();
            
            for mem_type in [MemoryType::Auto, MemoryType::Memo, MemoryType::Api, MemoryType::Cache] {
                if let Ok(mut entries) = self.list_by_type(mem_type) {
                    all_entries.append(&mut entries);
                }
            }
            
            // Sort by creation time, newest first
            all_entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(all_entries)
        }
    }
    
    /// Get timeline view of memories
    pub fn timeline(&self, memory_type: Option<MemoryType>, days: u32) -> Result<Vec<MemoryEntry>> {
        let cutoff = Utc::now() - Duration::days(days as i64);
        let mut entries = self.list(memory_type)?;
        
        // Filter by date
        entries.retain(|entry| entry.created_at >= cutoff);
        
        // Sort by creation time, newest first
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(entries)
    }
    
    /// Search memories by content
    pub fn search(&self, query: &str) -> Result<Vec<MemoryEntry>> {
        let entries = self.list(None)?;
        let query_lower = query.to_lowercase();
        
        let mut matches = Vec::new();
        for entry in entries {
            if entry.name.to_lowercase().contains(&query_lower) ||
               entry.content.to_lowercase().contains(&query_lower) {
                matches.push(entry);
            }
        }
        
        Ok(matches)
    }
    
    /// Get memory statistics
    pub fn stats(&self) -> Result<MemoryStats> {
        let all_entries = self.list(None)?;
        
        let mut stats = MemoryStats {
            total_count: all_entries.len(),
            by_type: HashMap::new(),
            total_size_bytes: 0,
            oldest_entry: None,
            newest_entry: None,
        };
        
        for entry in &all_entries {
            // Count by type
            *stats.by_type.entry(entry.memory_type.clone()).or_insert(0) += 1;
            
            // Size estimation
            stats.total_size_bytes += entry.content.len();
            
            // Date tracking
            if stats.oldest_entry.is_none() || entry.created_at < stats.oldest_entry.unwrap() {
                stats.oldest_entry = Some(entry.created_at);
            }
            if stats.newest_entry.is_none() || entry.created_at > stats.newest_entry.unwrap() {
                stats.newest_entry = Some(entry.created_at);
            }
        }
        
        Ok(stats)
    }
    
    /// Clean up old memories
    pub fn cleanup(&self, memory_type: Option<MemoryType>, days: u32) -> Result<u32> {
        let cutoff = Utc::now() - Duration::days(days as i64);
        let entries = self.list(memory_type)?;
        
        let mut cleaned = 0;
        for entry in entries {
            if entry.created_at < cutoff {
                let type_dir = self.storage_path.join(entry.memory_type.to_string());
                let file_path = type_dir.join(format!("{}.json", entry.id));
                
                if file_path.exists() {
                    fs::remove_file(file_path)?;
                    cleaned += 1;
                }
            }
        }
        
        Ok(cleaned)
    }
    
    fn list_by_type(&self, memory_type: MemoryType) -> Result<Vec<MemoryEntry>> {
        let type_dir = self.storage_path.join(memory_type.to_string());
        if !type_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut entries = Vec::new();
        
        for entry in fs::read_dir(type_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(memory_entry) = serde_json::from_str::<MemoryEntry>(&content) {
                        entries.push(memory_entry);
                    }
                }
            }
        }
        
        // Sort by creation time, newest first
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(entries)
    }
}

/// Memory system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_count: usize,
    pub by_type: HashMap<MemoryType, usize>,
    pub total_size_bytes: usize,
    pub oldest_entry: Option<DateTime<Utc>>,
    pub newest_entry: Option<DateTime<Utc>>,
}

impl std::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Memory Statistics:")?;
        writeln!(f, "  Total entries: {}", self.total_count)?;
        writeln!(f, "  Total size: {:.2} KB", self.total_size_bytes as f64 / 1024.0)?;
        
        for (mem_type, count) in &self.by_type {
            writeln!(f, "  {}: {}", mem_type, count)?;
        }
        
        if let Some(oldest) = self.oldest_entry {
            writeln!(f, "  Oldest: {}", oldest.format("%Y-%m-%d %H:%M:%S"))?;
        }
        if let Some(newest) = self.newest_entry {
            writeln!(f, "  Newest: {}", newest.format("%Y-%m-%d %H:%M:%S"))?;
        }
        
        Ok(())
    }
}