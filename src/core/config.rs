//! Configuration management for NekoCode Rust

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// General settings
    pub general: GeneralConfig,
    
    /// Analysis settings
    pub analysis: AnalysisConfig,
    
    /// Memory system settings
    pub memory: MemoryConfig,
    
    /// Custom key-value pairs
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub io_threads: u32,
    pub cpu_threads: u32,
    pub verbose: bool,
    pub progress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub include_tests: bool,
    pub include_comments: bool,
    pub max_file_size_mb: u32,
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub storage_path: PathBuf,
    pub max_memories: u32,
    pub cleanup_days: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                io_threads: 4,
                cpu_threads: 0, // 0 = auto-detect
                verbose: false,
                progress: false,
            },
            analysis: AnalysisConfig {
                include_tests: false,
                include_comments: true,
                max_file_size_mb: 100,
                exclude_patterns: vec![
                    "node_modules".to_string(),
                    "target".to_string(),
                    ".git".to_string(),
                    "dist".to_string(),
                    "build".to_string(),
                ],
            },
            memory: MemoryConfig {
                storage_path: PathBuf::from(".nekocode_memories"),
                max_memories: 1000,
                cleanup_days: 30,
            },
            custom: HashMap::new(),
        }
    }
}

/// Configuration manager
pub struct ConfigManager {
    config: Config,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config_path = PathBuf::from(".nekocode_config.json");
        let config = Self::load_config(&config_path).unwrap_or_default();
        
        Self {
            config,
            config_path,
        }
    }
    
    pub fn get(&self) -> &Config {
        &self.config
    }
    
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        // Parse key path (e.g., "general.verbose", "analysis.include_tests")
        let parts: Vec<&str> = key.split('.').collect();
        
        match parts.as_slice() {
            ["general", "io_threads"] => {
                self.config.general.io_threads = value.parse()?;
            }
            ["general", "cpu_threads"] => {
                self.config.general.cpu_threads = value.parse()?;
            }
            ["general", "verbose"] => {
                self.config.general.verbose = value.parse()?;
            }
            ["general", "progress"] => {
                self.config.general.progress = value.parse()?;
            }
            ["analysis", "include_tests"] => {
                self.config.analysis.include_tests = value.parse()?;
            }
            ["analysis", "include_comments"] => {
                self.config.analysis.include_comments = value.parse()?;
            }
            ["analysis", "max_file_size_mb"] => {
                self.config.analysis.max_file_size_mb = value.parse()?;
            }
            ["memory", "max_memories"] => {
                self.config.memory.max_memories = value.parse()?;
            }
            ["memory", "cleanup_days"] => {
                self.config.memory.cleanup_days = value.parse()?;
            }
            _ => {
                // Store as custom key-value
                self.config.custom.insert(key.to_string(), value.to_string());
            }
        }
        
        self.save()?;
        Ok(())
    }
    
    pub fn show(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.config)?)
    }
    
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(&self.config_path, json)?;
        Ok(())
    }
    
    fn load_config(path: &PathBuf) -> Result<Config> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}