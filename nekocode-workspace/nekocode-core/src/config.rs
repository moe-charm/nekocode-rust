//! Configuration management for NekoCode

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use crate::error::{NekocodeError, Result};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub analysis: AnalysisConfig,
    pub memory: MemoryConfig,
}

impl Config {
    /// Load configuration from file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| NekocodeError::Io(e))?;
        
        serde_json::from_str(&content)
            .map_err(|e| NekocodeError::Serde(e))
    }
    
    /// Save configuration to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| NekocodeError::Serde(e))?;
        
        fs::write(path, content)
            .map_err(|e| NekocodeError::Io(e))
    }
    
    /// Get default configuration
    pub fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            analysis: AnalysisConfig::default(),
            memory: MemoryConfig::default(),
        }
    }
}

/// General configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub session_dir: PathBuf,
    pub log_level: String,
    pub parallel_jobs: usize,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            session_dir: PathBuf::from(crate::SESSION_DIR),
            log_level: "info".to_string(),
            parallel_jobs: num_cpus::get(),
        }
    }
}

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub ignore_patterns: Vec<String>,
    pub include_patterns: Vec<String>,
    pub max_file_size_mb: usize,
    pub follow_symlinks: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            ignore_patterns: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
            include_patterns: vec![],
            max_file_size_mb: 10,
            follow_symlinks: false,
        }
    }
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_sessions_in_memory: usize,
    pub auto_save_interval_seconds: u64,
    pub cleanup_old_sessions_days: i64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_sessions_in_memory: 10,
            auto_save_interval_seconds: 300,
            cleanup_old_sessions_days: 30,
        }
    }
}