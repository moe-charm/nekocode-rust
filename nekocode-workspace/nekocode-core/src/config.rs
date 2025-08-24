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

/// Configuration manager for easy access
pub struct ConfigManager {
    config: Config,
    config_path: PathBuf,
}

impl ConfigManager {
    /// Create new config manager
    pub fn new() -> Result<Self> {
        let config_path = Self::default_config_path()?;
        let config = if config_path.exists() {
            Config::load_from_file(&config_path)?
        } else {
            Config::default()
        };
        
        Ok(Self {
            config,
            config_path,
        })
    }
    
    /// Get default config path
    fn default_config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| NekocodeError::Config("HOME not set".to_string()))?;
        Ok(PathBuf::from(home).join(".nekocode").join("config.json"))
    }
    
    /// Get config value
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "session_dir" => Some(self.config.general.session_dir.display().to_string()),
            "log_level" => Some(self.config.general.log_level.clone()),
            "parallel_jobs" => Some(self.config.general.parallel_jobs.to_string()),
            "max_sessions_in_memory" => Some(self.config.memory.max_sessions_in_memory.to_string()),
            "auto_save_interval_seconds" => Some(self.config.memory.auto_save_interval_seconds.to_string()),
            "cleanup_old_sessions_days" => Some(self.config.memory.cleanup_old_sessions_days.to_string()),
            _ => None,
        }
    }
    
    /// Set config value
    pub fn set(&mut self, key: &str, value: String) -> Result<()> {
        match key {
            "log_level" => self.config.general.log_level = value,
            "parallel_jobs" => {
                self.config.general.parallel_jobs = value.parse()
                    .map_err(|_| NekocodeError::Config("Invalid parallel_jobs value".to_string()))?;
            }
            "max_sessions_in_memory" => {
                self.config.memory.max_sessions_in_memory = value.parse()
                    .map_err(|_| NekocodeError::Config("Invalid max_sessions_in_memory value".to_string()))?;
            }
            "auto_save_interval_seconds" => {
                self.config.memory.auto_save_interval_seconds = value.parse()
                    .map_err(|_| NekocodeError::Config("Invalid auto_save_interval_seconds value".to_string()))?;
            }
            "cleanup_old_sessions_days" => {
                self.config.memory.cleanup_old_sessions_days = value.parse()
                    .map_err(|_| NekocodeError::Config("Invalid cleanup_old_sessions_days value".to_string()))?;
            }
            _ => return Err(NekocodeError::Config(format!("Unknown config key: {}", key))),
        }
        
        // Save to file
        self.config.save_to_file(&self.config_path)?;
        Ok(())
    }
    
    /// Show all config values
    pub fn show_all(&self) -> String {
        format!(
            "📋 Configuration:\n\
             General:\n\
             - session_dir: {}\n\
             - log_level: {}\n\
             - parallel_jobs: {}\n\
             Memory:\n\
             - max_sessions_in_memory: {}\n\
             - auto_save_interval_seconds: {}\n\
             - cleanup_old_sessions_days: {}",
            self.config.general.session_dir.display(),
            self.config.general.log_level,
            self.config.general.parallel_jobs,
            self.config.memory.max_sessions_in_memory,
            self.config.memory.auto_save_interval_seconds,
            self.config.memory.cleanup_old_sessions_days,
        )
    }
}