//! Memory management for NekoCode

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::error::{NekocodeError, Result};

/// Memory type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum MemoryType {
    Auto,
    Memo,
    Api,
    Cache,
}

impl MemoryType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Some(MemoryType::Auto),
            "memo" => Some(MemoryType::Memo),
            "api" => Some(MemoryType::Api),
            "cache" => Some(MemoryType::Cache),
            _ => None,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Auto => "auto",
            MemoryType::Memo => "memo",
            MemoryType::Api => "api",
            MemoryType::Cache => "cache",
        }
    }
}

/// Memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
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
        Self {
            name,
            memory_type,
            content,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }
}

/// Memory manager for storing and retrieving memory entries
#[derive(Debug)]
pub struct MemoryManager {
    memories: HashMap<(MemoryType, String), MemoryEntry>,
}

impl MemoryManager {
    /// Create new memory manager
    pub fn new() -> Self {
        Self {
            memories: HashMap::new(),
        }
    }
    
    /// Save memory entry
    pub fn save(&mut self, memory_type: MemoryType, name: String, content: String) -> Result<()> {
        let entry = MemoryEntry::new(name.clone(), memory_type, content);
        self.memories.insert((memory_type, name), entry);
        Ok(())
    }
    
    /// Load memory entry
    pub fn load(&self, memory_type: MemoryType, name: &str) -> Result<&MemoryEntry> {
        self.memories
            .get(&(memory_type, name.to_string()))
            .ok_or_else(|| NekocodeError::Memory(
                format!("Memory not found: {}:{}", memory_type.as_str(), name)
            ))
    }
    
    /// List memories by type
    pub fn list_by_type(&self, memory_type: Option<MemoryType>) -> Vec<&MemoryEntry> {
        self.memories
            .values()
            .filter(|entry| {
                memory_type.map(|t| entry.memory_type == t).unwrap_or(true)
            })
            .collect()
    }
    
    /// Delete memory entry
    pub fn delete(&mut self, memory_type: MemoryType, name: &str) -> Result<()> {
        self.memories
            .remove(&(memory_type, name.to_string()))
            .ok_or_else(|| NekocodeError::Memory(
                format!("Memory not found: {}:{}", memory_type.as_str(), name)
            ))?;
        Ok(())
    }
    
    /// Clear all memories
    pub fn clear(&mut self) {
        self.memories.clear();
    }
    
    /// Get timeline of memories (sorted by creation date)
    pub fn get_timeline(&self, days: i64) -> Vec<&MemoryEntry> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let mut entries: Vec<_> = self.memories
            .values()
            .filter(|entry| entry.created_at >= cutoff)
            .collect();
        
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries
    }
    
    /// Get memory count by type
    pub fn get_stats(&self) -> HashMap<MemoryType, usize> {
        let mut stats = HashMap::new();
        for entry in self.memories.values() {
            *stats.entry(entry.memory_type).or_insert(0) += 1;
        }
        stats
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}