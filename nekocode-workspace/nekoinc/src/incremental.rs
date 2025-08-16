//! Incremental analysis engine for detecting and processing file changes

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use nekocode_core::{Result, NekocodeError, SessionManager};

/// File metadata for change detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileMetadata {
    /// File path relative to session root
    pub path: PathBuf,
    /// Last modification time  
    pub modified_time: DateTime<Utc>,
    /// Hash of file content for accurate change detection
    pub content_hash: String,
    /// File size in bytes
    pub size: u64,
}

impl FileMetadata {
    /// Create FileMetadata from a file path
    pub fn from_path(path: &Path, base_path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let modified_time = metadata.modified()
            .map_err(|e| NekocodeError::Io(e))?;
        
        let modified_time = DateTime::<Utc>::from(modified_time);
        
        let content = fs::read(path)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let content_hash = Self::calculate_hash(&content);
        let relative_path = path.strip_prefix(base_path)
            .unwrap_or(path)
            .to_path_buf();
        
        Ok(FileMetadata {
            path: relative_path,
            modified_time,
            content_hash,
            size: metadata.len(),
        })
    }
    
    /// Calculate hash of content using DefaultHasher (fast, non-cryptographic)
    fn calculate_hash(content: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
    
    /// Check if this file has changed compared to another FileMetadata
    pub fn has_changed(&self, other: &FileMetadata) -> bool {
        self.content_hash != other.content_hash || 
        self.modified_time != other.modified_time ||
        self.size != other.size
    }
}

/// Change types for incremental analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    /// File was added
    Added,
    /// File was modified  
    Modified,
    /// File was deleted
    Deleted,
}

/// Represents a detected file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// Path of the changed file
    pub path: PathBuf,
    /// Type of change
    pub change_type: ChangeType,
    /// Previous metadata (None for Added files)
    pub previous_metadata: Option<FileMetadata>,
    /// Current metadata (None for Deleted files)
    pub current_metadata: Option<FileMetadata>,
}

/// Core change detection engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDetector {
    /// File hash cache for change detection
    file_cache: HashMap<PathBuf, FileMetadata>,
    /// Last scan time
    last_scan: DateTime<Utc>,
    /// Base path for relative path calculations
    base_path: PathBuf,
}

impl ChangeDetector {
    /// Create a new ChangeDetector
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            file_cache: HashMap::new(),
            last_scan: Utc::now(),
            base_path,
        }
    }
    
    /// Initialize the cache by scanning all files in the base path
    pub fn initialize(&mut self) -> Result<Vec<FileMetadata>> {
        let mut files = Vec::new();
        
        for entry in WalkDir::new(&self.base_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Skip directories and non-source files
            if !path.is_file() || !self.is_source_file(path) {
                continue;
            }
            
            match FileMetadata::from_path(path, &self.base_path) {
                Ok(metadata) => {
                    self.file_cache.insert(metadata.path.clone(), metadata.clone());
                    files.push(metadata);
                }
                Err(e) => {
                    log::warn!("Failed to process file {}: {}", path.display(), e);
                }
            }
        }
        
        self.last_scan = Utc::now();
        Ok(files)
    }
    
    /// Detect changes since last scan
    pub fn detect_changes(&mut self) -> Result<Vec<FileChange>> {
        let mut changes = Vec::new();
        let mut current_files = HashMap::new();
        
        // Scan current files
        for entry in WalkDir::new(&self.base_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            if !path.is_file() || !self.is_source_file(path) {
                continue;
            }
            
            match FileMetadata::from_path(path, &self.base_path) {
                Ok(metadata) => {
                    current_files.insert(metadata.path.clone(), metadata);
                }
                Err(e) => {
                    log::warn!("Failed to process file {}: {}", path.display(), e);
                }
            }
        }
        
        // Check for modified and added files
        for (path, current_metadata) in &current_files {
            if let Some(cached_metadata) = self.file_cache.get(path) {
                // File exists in cache - check if modified
                if current_metadata.has_changed(cached_metadata) {
                    changes.push(FileChange {
                        path: path.clone(),
                        change_type: ChangeType::Modified,
                        previous_metadata: Some(cached_metadata.clone()),
                        current_metadata: Some(current_metadata.clone()),
                    });
                }
            } else {
                // File not in cache - it's new
                changes.push(FileChange {
                    path: path.clone(),
                    change_type: ChangeType::Added,
                    previous_metadata: None,
                    current_metadata: Some(current_metadata.clone()),
                });
            }
        }
        
        // Check for deleted files
        for (path, cached_metadata) in &self.file_cache {
            if !current_files.contains_key(path) {
                changes.push(FileChange {
                    path: path.clone(),
                    change_type: ChangeType::Deleted,
                    previous_metadata: Some(cached_metadata.clone()),
                    current_metadata: None,
                });
            }
        }
        
        // Update cache with current state
        self.file_cache = current_files;
        self.last_scan = Utc::now();
        
        Ok(changes)
    }
    
    /// Check if a file should be included in analysis
    fn is_source_file(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            matches!(extension.to_lowercase().as_str(), 
                "js" | "jsx" | "mjs" | "cjs" |
                "ts" | "tsx" |
                "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" |
                "c" | "h" |
                "py" | "pyw" | "pyi" |
                "cs" |
                "go" |
                "rs"
            )
        } else {
            false
        }
    }
    
    /// Get the number of files in cache
    pub fn file_count(&self) -> usize {
        self.file_cache.len()
    }
    
    /// Get last scan time
    pub fn last_scan_time(&self) -> DateTime<Utc> {
        self.last_scan
    }
    
    /// Get base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

/// Summary of incremental analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalSummary {
    /// Total files in session
    pub total_files: usize,
    /// Number of files changed
    pub changed_files: usize,
    /// Number of files added
    pub added_files: usize,
    /// Number of files deleted
    pub deleted_files: usize,
    /// Time taken for incremental analysis (milliseconds)
    pub analysis_time_ms: u64,
    /// Estimated speedup compared to full analysis
    pub estimated_speedup: f64,
}

impl IncrementalSummary {
    /// Create a new summary
    pub fn new(
        total_files: usize,
        changes: &[FileChange],
        analysis_time_ms: u64,
        full_analysis_time_ms: u64,
    ) -> Self {
        let changed_files = changes.iter()
            .filter(|c| c.change_type == ChangeType::Modified)
            .count();
        let added_files = changes.iter()
            .filter(|c| c.change_type == ChangeType::Added)
            .count();
        let deleted_files = changes.iter()
            .filter(|c| c.change_type == ChangeType::Deleted)
            .count();
        
        let estimated_speedup = if analysis_time_ms > 0 {
            full_analysis_time_ms as f64 / analysis_time_ms as f64
        } else {
            1.0
        };
        
        Self {
            total_files,
            changed_files,
            added_files,
            deleted_files,
            analysis_time_ms,
            estimated_speedup,
        }
    }
    
    /// Format summary for display
    pub fn format_summary(&self) -> String {
        format!(
            "⚡ Updated {} files in {}ms ({:.1}x speedup)\n\
             📊 Changes: {} modified, {} added, {} deleted\n\
             📁 Total files in session: {}",
            self.changed_files + self.added_files,
            self.analysis_time_ms,
            self.estimated_speedup,
            self.changed_files,
            self.added_files,
            self.deleted_files,
            self.total_files
        )
    }
}

/// Incremental analyzer that integrates with sessions
pub struct IncrementalAnalyzer {
    session_manager: SessionManager,
    detectors: HashMap<String, ChangeDetector>,
}

impl IncrementalAnalyzer {
    /// Create new incremental analyzer
    pub fn new() -> Result<Self> {
        Ok(Self {
            session_manager: SessionManager::new()?,
            detectors: HashMap::new(),
        })
    }
    
    /// Initialize change detection for a session
    pub fn initialize_session(&mut self, session_id: &str) -> Result<()> {
        let session = self.session_manager.get_session_mut(session_id)?;
        let base_path = session.info.path.clone();
        
        let mut detector = ChangeDetector::new(base_path);
        let files = detector.initialize()?;
        
        self.detectors.insert(session_id.to_string(), detector);
        
        println!("✅ Initialized incremental tracking for {} files", files.len());
        Ok(())
    }
    
    /// Detect and analyze changes for a session
    pub async fn analyze_changes(&mut self, session_id: &str) -> Result<IncrementalSummary> {
        let start_time = std::time::Instant::now();
        
        // Get or create detector
        if !self.detectors.contains_key(session_id) {
            self.initialize_session(session_id)?;
        }
        
        let detector = self.detectors.get_mut(session_id)
            .ok_or_else(|| NekocodeError::Session(format!("No detector for session {}", session_id)))?;
        
        // Detect changes
        let changes = detector.detect_changes()?;
        
        if changes.is_empty() {
            return Ok(IncrementalSummary {
                total_files: detector.file_count(),
                changed_files: 0,
                added_files: 0,
                deleted_files: 0,
                analysis_time_ms: start_time.elapsed().as_millis() as u64,
                estimated_speedup: 1.0,
            });
        }
        
        // Process changes
        println!("🔍 Detected {} changes", changes.len());
        for change in &changes {
            match change.change_type {
                ChangeType::Added => {
                    println!("  ➕ Added: {}", change.path.display());
                }
                ChangeType::Modified => {
                    println!("  📝 Modified: {}", change.path.display());
                }
                ChangeType::Deleted => {
                    println!("  ❌ Deleted: {}", change.path.display());
                }
            }
        }
        
        // TODO: Trigger actual re-analysis of changed files
        // This would call into the main nekocode analyzer
        // For now, we just report the changes
        
        let analysis_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Estimate full analysis time (rough estimate: 50ms per file)
        let estimated_full_time = detector.file_count() as u64 * 50;
        
        Ok(IncrementalSummary::new(
            detector.file_count(),
            &changes,
            analysis_time_ms,
            estimated_full_time,
        ))
    }
    
    /// Get change detector for a session
    pub fn get_detector(&self, session_id: &str) -> Option<&ChangeDetector> {
        self.detectors.get(session_id)
    }
}