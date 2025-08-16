//! Session management - Core of NekoCode
//! 
//! This module handles session creation, storage, and retrieval.
//! Sessions are persisted to disk in JSON format for cross-tool sharing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::fs;

use crate::types::{AnalysisResult, Language};
use crate::error::{NekocodeError, Result};

/// Session information stored in .nekocode_sessions/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    
    // Analysis results (language-agnostic)
    pub analysis_results: Vec<AnalysisResult>,
    pub file_count: usize,
    pub total_lines: u32,
    pub languages: HashMap<Language, usize>,
    
    // Change tracking
    pub file_hashes: HashMap<PathBuf, String>,
    pub last_scan_time: Option<DateTime<Utc>>,
    
    // Session state
    pub version: String,
    pub is_dirty: bool,
}

impl SessionInfo {
    /// Create new session info
    pub fn new(id: String, path: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id,
            path,
            created_at: now,
            last_accessed: now,
            last_modified: now,
            metadata: HashMap::new(),
            analysis_results: Vec::new(),
            file_count: 0,
            total_lines: 0,
            languages: HashMap::new(),
            file_hashes: HashMap::new(),
            last_scan_time: None,
            version: crate::VERSION.to_string(),
            is_dirty: false,
        }
    }
    
    /// Update statistics from analysis results
    pub fn update_stats(&mut self) {
        self.file_count = self.analysis_results.len();
        self.total_lines = self.analysis_results
            .iter()
            .map(|r| r.metrics.lines_of_code)
            .sum();
        
        // Count languages
        self.languages.clear();
        for result in &self.analysis_results {
            *self.languages.entry(result.file_info.language).or_insert(0) += 1;
        }
        
        self.last_modified = Utc::now();
        self.is_dirty = true;
    }
}

/// Active session in memory
#[derive(Debug)]
pub struct Session {
    pub info: SessionInfo,
    session_dir: PathBuf,
}

impl Session {
    /// Create new session
    pub fn new(path: PathBuf) -> Result<Self> {
        let id = Uuid::new_v4().to_string()[..8].to_string();
        let info = SessionInfo::new(id, path);
        let session_dir = PathBuf::from(crate::SESSION_DIR);
        
        Ok(Session {
            info,
            session_dir,
        })
    }
    
    /// Load session from disk
    pub fn load(session_id: &str) -> Result<Self> {
        let session_dir = PathBuf::from(crate::SESSION_DIR);
        let file_path = session_dir.join(format!("{}.json", session_id));
        
        if !file_path.exists() {
            return Err(NekocodeError::SessionNotFound(session_id.to_string()));
        }
        
        let content = fs::read_to_string(&file_path)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let mut info: SessionInfo = serde_json::from_str(&content)
            .map_err(|e| NekocodeError::Serde(e))?;
        
        // Update last accessed time
        info.last_accessed = Utc::now();
        
        Ok(Session {
            info,
            session_dir,
        })
    }
    
    /// Save session to disk
    pub fn save(&mut self) -> Result<()> {
        // Create session directory if it doesn't exist
        if !self.session_dir.exists() {
            fs::create_dir_all(&self.session_dir)
                .map_err(|e| NekocodeError::Io(e))?;
        }
        
        let file_path = self.session_dir.join(format!("{}.json", self.info.id));
        let content = serde_json::to_string_pretty(&self.info)
            .map_err(|e| NekocodeError::Serde(e))?;
        
        fs::write(file_path, content)
            .map_err(|e| NekocodeError::Io(e))?;
        
        self.info.is_dirty = false;
        Ok(())
    }
    
    /// Get session ID
    pub fn id(&self) -> &str {
        &self.info.id
    }
    
    /// Get project path
    pub fn path(&self) -> &Path {
        &self.info.path
    }
    
    /// Update last accessed time
    pub fn touch(&mut self) {
        self.info.last_accessed = Utc::now();
        self.info.is_dirty = true;
    }
    
    /// Add analysis result
    pub fn add_analysis_result(&mut self, result: AnalysisResult) {
        // Update file hash if available
        if let Some(ref hash) = result.file_info.hash {
            self.info.file_hashes.insert(
                result.file_info.path.clone(),
                hash.clone()
            );
        }
        
        self.info.analysis_results.push(result);
        self.info.update_stats();
    }
    
    /// Get files of specific language
    pub fn get_files_by_language(&self, language: Language) -> Vec<&AnalysisResult> {
        self.info.analysis_results
            .iter()
            .filter(|r| r.file_info.language == language)
            .collect()
    }
    
    /// Check if file has changed
    pub fn has_file_changed(&self, path: &Path, current_hash: &str) -> bool {
        self.info.file_hashes
            .get(path)
            .map(|stored_hash| stored_hash != current_hash)
            .unwrap_or(true)
    }
    
    /// Remove analysis result for a file
    pub fn remove_file(&mut self, path: &Path) {
        self.info.analysis_results.retain(|r| r.file_info.path != path);
        self.info.file_hashes.remove(path);
        self.info.update_stats();
    }
    
    /// Clear all analysis results
    pub fn clear(&mut self) {
        self.info.analysis_results.clear();
        self.info.file_hashes.clear();
        self.info.update_stats();
    }
}

/// Session manager with file-based persistence
#[derive(Debug)]
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    session_dir: PathBuf,
}

impl SessionManager {
    /// Create new session manager
    pub fn new() -> Result<Self> {
        let session_dir = PathBuf::from(crate::SESSION_DIR);
        Ok(SessionManager {
            sessions: HashMap::new(),
            session_dir,
        })
    }
    
    /// Create new session
    pub fn create_session(&mut self, path: PathBuf) -> Result<String> {
        let mut session = Session::new(path)?;
        let id = session.id().to_string();
        
        // Save to disk immediately
        session.save()?;
        
        // Store in memory
        self.sessions.insert(id.clone(), session);
        
        Ok(id)
    }
    
    /// Get session (load from disk if needed)
    pub fn get_session(&mut self, session_id: &str) -> Result<&Session> {
        // Load from disk if not in memory
        if !self.sessions.contains_key(session_id) {
            let session = Session::load(session_id)?;
            self.sessions.insert(session_id.to_string(), session);
        }
        
        self.sessions.get(session_id)
            .ok_or_else(|| NekocodeError::SessionNotFound(session_id.to_string()))
    }
    
    /// Get mutable session (load from disk if needed)
    pub fn get_session_mut(&mut self, session_id: &str) -> Result<&mut Session> {
        // Load from disk if not in memory
        if !self.sessions.contains_key(session_id) {
            let session = Session::load(session_id)?;
            self.sessions.insert(session_id.to_string(), session);
        }
        
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| NekocodeError::SessionNotFound(session_id.to_string()))?;
        
        session.touch();
        Ok(session)
    }
    
    /// Save session to disk
    pub fn save_session(&mut self, session_id: &str) -> Result<()> {
        let session = self.get_session_mut(session_id)?;
        session.save()
    }
    
    /// List all sessions
    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let mut sessions = Vec::new();
        
        if !self.session_dir.exists() {
            return Ok(sessions);
        }
        
        for entry in fs::read_dir(&self.session_dir)
            .map_err(|e| NekocodeError::Io(e))? {
            
            let entry = entry.map_err(|e| NekocodeError::Io(e))?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(info) = serde_json::from_str::<SessionInfo>(&content) {
                        sessions.push(info);
                    }
                }
            }
        }
        
        // Sort by last accessed time (newest first)
        sessions.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        
        Ok(sessions)
    }
    
    /// Delete session
    pub fn delete_session(&mut self, session_id: &str) -> Result<()> {
        // Remove from memory
        self.sessions.remove(session_id);
        
        // Remove from disk
        let file_path = self.session_dir.join(format!("{}.json", session_id));
        if file_path.exists() {
            fs::remove_file(file_path)
                .map_err(|e| NekocodeError::Io(e))?;
        }
        
        Ok(())
    }
    
    /// Clean up old sessions (older than days)
    pub fn cleanup_old_sessions(&mut self, days: i64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let sessions = self.list_sessions()?;
        let mut deleted = 0;
        
        for session in sessions {
            if session.last_accessed < cutoff {
                if self.delete_session(&session.id).is_ok() {
                    deleted += 1;
                }
            }
        }
        
        Ok(deleted)
    }
}

/// Trait for providing session functionality to tools
#[async_trait::async_trait]
pub trait SessionProvider {
    /// Get current session
    async fn get_session(&self, session_id: &str) -> Result<&Session>;
    
    /// Get mutable session
    async fn get_session_mut(&mut self, session_id: &str) -> Result<&mut Session>;
    
    /// Create new session
    async fn create_session(&mut self, path: PathBuf) -> Result<String>;
    
    /// List all sessions
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>>;
    
    /// Save session to disk
    async fn save_session(&mut self, session_id: &str) -> Result<()>;
}