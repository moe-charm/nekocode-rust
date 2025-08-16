//! File I/O utilities for NekoCode

use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;
use sha2::{Sha256, Digest};
use std::io::Read;
use crate::error::{NekocodeError, Result};
use crate::types::Language;

/// File processor for handling file operations
#[derive(Debug)]
pub struct FileProcessor {
    root_path: PathBuf,
    ignore_patterns: Vec<String>,
}

impl FileProcessor {
    /// Create new file processor
    pub fn new(root_path: &Path) -> Result<Self> {
        if !root_path.exists() {
            return Err(NekocodeError::FileNotFound(
                root_path.display().to_string()
            ));
        }
        
        Ok(Self {
            root_path: root_path.to_path_buf(),
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
        })
    }
    
    /// Set ignore patterns
    pub fn set_ignore_patterns(&mut self, patterns: Vec<String>) {
        self.ignore_patterns = patterns;
    }
    
    /// Check if path should be ignored
    pub fn should_ignore(&self, path: &Path) -> bool {
        for pattern in &self.ignore_patterns {
            if path.to_string_lossy().contains(pattern) {
                return true;
            }
        }
        false
    }
    
    /// Discover files in the root path
    pub fn discover_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for entry in WalkDir::new(&self.root_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok()) {
            
            let path = entry.path();
            
            // Skip directories and ignored paths
            if path.is_dir() || self.should_ignore(path) {
                continue;
            }
            
            // Only include supported language files
            if Language::from_path(path) != Language::Unknown {
                files.push(path.to_path_buf());
            }
        }
        
        Ok(files)
    }
    
    /// Read file content
    pub fn read_file(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path)
            .map_err(|e| NekocodeError::Io(e))
    }
    
    /// Calculate file hash
    pub fn calculate_hash(&self, path: &Path) -> Result<String> {
        let mut file = fs::File::open(path)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer)
                .map_err(|e| NekocodeError::Io(e))?;
            
            if bytes_read == 0 {
                break;
            }
            
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }
    
    /// Get file size in bytes
    pub fn get_file_size(&self, path: &Path) -> Result<u64> {
        let metadata = fs::metadata(path)
            .map_err(|e| NekocodeError::Io(e))?;
        Ok(metadata.len())
    }
}

/// Path utilities
pub struct PathUtils;

impl PathUtils {
    /// Make path relative to base
    pub fn make_relative(path: &Path, base: &Path) -> PathBuf {
        path.strip_prefix(base)
            .unwrap_or(path)
            .to_path_buf()
    }
    
    /// Normalize path (remove ./ and ../)
    pub fn normalize(path: &Path) -> PathBuf {
        let mut components = Vec::new();
        
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::CurDir => {
                    // Skip
                }
                c => {
                    components.push(c);
                }
            }
        }
        
        components.iter().collect()
    }
    
    /// Check if path is hidden (starts with .)
    pub fn is_hidden(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with('.'))
            .unwrap_or(false)
    }
}