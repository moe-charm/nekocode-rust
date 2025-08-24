//! CLI Session Configuration - Auto-memory for session IDs
//! 
//! This module provides automatic session memory for CLI commands,
//! making the CLI experience match the MCP server behavior.
//! Sessions are stored in ~/.nekocode/cli_session.json

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use std::fs;
use std::io::Write;

use crate::error::{NekocodeError, Result};

/// CLI session configuration stored in ~/.nekocode/cli_session.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliSessionConfig {
    /// Current session ID (automatically used when not specified)
    pub current_session_id: Option<String>,
    
    /// Path that was analyzed for current session
    pub current_path: Option<PathBuf>,
    
    /// Timestamp of last update
    pub last_updated: DateTime<Utc>,
    
    /// Session history for quick switching
    pub session_history: Vec<SessionHistoryEntry>,
    
    /// Settings for CLI behavior
    pub settings: CliSettings,
}

/// Entry in session history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHistoryEntry {
    pub session_id: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub description: Option<String>,
}

/// CLI-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliSettings {
    /// Auto-save session after session-create command
    pub auto_save_session: bool,
    
    /// Maximum number of sessions to keep in history
    pub max_history_size: usize,
    
    /// Show hints about session usage
    pub show_session_hints: bool,

    /// Automatically prune old sessions on session-create
    pub auto_prune_enabled: bool,

    /// Keep this many most recent sessions (auto-prune)
    pub auto_prune_keep_recent: usize,

    /// Delete sessions not accessed for N days (None to disable)
    pub auto_prune_max_age_days: Option<i64>,

    /// Delete sessions whose project path no longer exists
    pub auto_prune_delete_stale: bool,
}

impl Default for CliSettings {
    fn default() -> Self {
        Self {
            auto_save_session: true,
            max_history_size: 10,
            show_session_hints: true,
            auto_prune_enabled: true,
            auto_prune_keep_recent: 5,
            auto_prune_max_age_days: Some(30),
            auto_prune_delete_stale: true,
        }
    }
}

impl Default for CliSessionConfig {
    fn default() -> Self {
        Self {
            current_session_id: None,
            current_path: None,
            last_updated: Utc::now(),
            session_history: Vec::new(),
            settings: CliSettings::default(),
        }
    }
}

impl CliSessionConfig {
    /// Get the configuration file path
    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| NekocodeError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not find home directory"
            )))?;
        
        let config_dir = home.join(".nekocode");
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }
        
        Ok(config_dir.join("cli_session.json"))
    }
    
    /// Load configuration from disk
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        
        if !path.exists() {
            // Return default if file doesn't exist
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(&path)?;
        let config: Self = serde_json::from_str(&content)
            .map_err(|e| NekocodeError::Parse(format!("Failed to parse CLI session config: {}", e)))?;
        
        Ok(config)
    }
    
    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| NekocodeError::Parse(format!("Failed to serialize CLI session config: {}", e)))?;
        
        let mut file = fs::File::create(&path)?;
        file.write_all(json.as_bytes())?;
        
        Ok(())
    }
    
    /// Set current session (used after session-create command)
    pub fn set_current_session(&mut self, session_id: String, path: PathBuf) -> Result<()> {
        self.current_session_id = Some(session_id.clone());
        self.current_path = Some(path.clone());
        self.last_updated = Utc::now();
        
        // Add to history
        self.add_to_history(session_id, path)?;
        
        // Save if auto-save is enabled
        if self.settings.auto_save_session {
            self.save()?;
        }
        
        Ok(())
    }
    
    /// Add session to history
    fn add_to_history(&mut self, session_id: String, path: PathBuf) -> Result<()> {
        let now = Utc::now();
        
        // Check if session already exists in history
        if let Some(entry) = self.session_history.iter_mut()
            .find(|e| e.session_id == session_id) {
            // Update last used time
            entry.last_used = now;
        } else {
            // Add new entry
            let entry = SessionHistoryEntry {
                session_id,
                path,
                created_at: now,
                last_used: now,
                description: None,
            };
            
            self.session_history.insert(0, entry);
            
            // Trim history if too large
            if self.session_history.len() > self.settings.max_history_size {
                self.session_history.truncate(self.settings.max_history_size);
            }
        }
        
        Ok(())
    }
    
    /// Get current session ID (if available)
    pub fn get_current_session_id(&self) -> Option<&str> {
        self.current_session_id.as_deref()
    }
    
    /// Clear current session
    pub fn clear_current_session(&mut self) -> Result<()> {
        self.current_session_id = None;
        self.current_path = None;
        self.last_updated = Utc::now();
        
        if self.settings.auto_save_session {
            self.save()?;
        }
        
        Ok(())
    }
    
    /// Get session from history by ID
    pub fn get_from_history(&self, session_id: &str) -> Option<&SessionHistoryEntry> {
        self.session_history.iter()
            .find(|e| e.session_id == session_id)
    }
    
    /// Show hint about using session
    pub fn show_hint(&self) -> Option<String> {
        if !self.settings.show_session_hints {
            return None;
        }
        
        if let Some(ref session_id) = self.current_session_id {
            Some(format!(
                "💡 Using session: {} (from {})\n   To use a different session, specify --session-id",
                &session_id[..8.min(session_id.len())],
                self.current_path.as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
            ))
        } else {
            Some("💡 No active session. Run 'nekocode session-create <path>' to create one.".to_string())
        }
    }
}

/// Helper functions for CLI commands
pub struct CliSessionHelper;

impl CliSessionHelper {
    /// Get session ID from args or config
    pub fn get_session_id(explicit_id: Option<&str>) -> Result<String> {
        // If explicitly provided, use it
        if let Some(id) = explicit_id {
            return Ok(id.to_string());
        }
        
        // Try to load from config
        let config = CliSessionConfig::load()?;
        
        config.get_current_session_id()
            .map(|s| s.to_string())
            .ok_or_else(|| NekocodeError::SessionNotFound(
                "No session ID provided and no active session found. Run 'nekocode session-create <path>' first.".to_string()
            ))
    }
    
    /// Save session after session-create command
    pub fn save_session(session_id: String, path: PathBuf) -> Result<()> {
        let mut config = CliSessionConfig::load()?;
        config.set_current_session(session_id, path)?;
        
        if config.settings.show_session_hints {
            println!("✅ Session saved. Future commands will use this session automatically.");
            println!("   Session ID: {}", &config.current_session_id.as_ref().unwrap()[..8.min(8)]);
        }
        
        Ok(())
    }
    
    /// Show current session info
    pub fn show_current_session() -> Result<()> {
        let config = CliSessionConfig::load()?;
        
        if let Some(hint) = config.show_hint() {
            println!("{}", hint);
        }
        
        Ok(())
    }
    
    /// List session history
    pub fn list_history() -> Result<()> {
        let config = CliSessionConfig::load()?;
        
        if config.session_history.is_empty() {
            println!("No session history found.");
            return Ok(());
        }
        
        println!("📜 Session History:");
        for (i, entry) in config.session_history.iter().enumerate() {
            let current = config.current_session_id.as_ref()
                .map(|id| id == &entry.session_id)
                .unwrap_or(false);
            
            let marker = if current { "→" } else { " " };
            
            println!("{} {}. {} | {} | Last used: {}",
                marker,
                i + 1,
                &entry.session_id[..8.min(entry.session_id.len())],
                entry.path.display(),
                entry.last_used.format("%Y-%m-%d %H:%M")
            );
        }
        
        Ok(())
    }

    /// Clear CLI session history (does not remove on-disk analysis sessions)
    pub fn clear_history() -> Result<()> {
        let mut config = CliSessionConfig::load()?;
        config.session_history.clear();
        config.last_updated = Utc::now();
        config.save()?;
        println!("🧹 Cleared CLI session history");
        Ok(())
    }
}
