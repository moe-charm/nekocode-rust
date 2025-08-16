//! File watching system with auto session updates

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use nekocode_core::{Result, NekocodeError, SessionManager};
use crate::incremental::IncrementalAnalyzer;

/// File watching status for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchStatus {
    pub session_id: String,
    pub status: WatchState,
    pub watched_files: usize,
    pub last_update: Option<DateTime<Utc>>,
    pub started_at: DateTime<Utc>,
}

/// Watch state enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatchState {
    Watching,
    Stopped,
    Error(String),
}

/// File watcher configuration
#[derive(Debug, Clone)]
pub struct WatchConfig {
    pub debounce_ms: u64,
    pub max_events_per_second: usize,
    pub exclude_patterns: Vec<String>,
    pub include_extensions: Vec<String>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500,
            max_events_per_second: 1000,
            exclude_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".DS_Store".to_string(),
                "*.tmp".to_string(),
                "*.log".to_string(),
                ".nekocode_sessions".to_string(),
            ],
            include_extensions: vec![
                "js".to_string(),
                "mjs".to_string(),
                "jsx".to_string(),
                "cjs".to_string(),
                "ts".to_string(),
                "tsx".to_string(),
                "cpp".to_string(),
                "cxx".to_string(),
                "cc".to_string(),
                "hpp".to_string(),
                "hxx".to_string(),
                "hh".to_string(),
                "c".to_string(),
                "h".to_string(),
                "py".to_string(),
                "pyw".to_string(),
                "pyi".to_string(),
                "cs".to_string(),
                "go".to_string(),
                "rs".to_string(),
            ],
        }
    }
}

/// File watcher for a session
pub struct FileWatcher {
    config: WatchConfig,
    session_id: String,
    session_path: PathBuf,
    incremental_analyzer: Arc<Mutex<IncrementalAnalyzer>>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(
        session_id: String,
        session_path: PathBuf,
        incremental_analyzer: Arc<Mutex<IncrementalAnalyzer>>
    ) -> Self {
        Self {
            config: WatchConfig::default(),
            session_id,
            session_path,
            incremental_analyzer,
        }
    }
    
    /// Check if a file should be watched based on configuration
    fn should_watch_file(&self, path: &Path) -> bool {
        // Check if path contains any excluded patterns
        let path_str = path.to_string_lossy();
        for pattern in &self.config.exclude_patterns {
            if path_str.contains(pattern) {
                return false;
            }
        }
        
        // Check if file has a supported extension
        if let Some(extension) = path.extension() {
            let ext_str = extension.to_string_lossy().to_lowercase();
            return self.config.include_extensions.contains(&ext_str);
        }
        
        false
    }
    
    /// Start watching files
    pub async fn start_watching(self) -> Result<()> {
        let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();
        
        // Create file system watcher
        let mut watcher = notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if let Err(e) = tx.send(event) {
                        eprintln!("Failed to send file watch event: {}", e);
                    }
                }
                Err(e) => eprintln!("File watch error: {:?}", e),
            }
        }).map_err(|e| NekocodeError::Watch(format!("Failed to create watcher: {}", e)))?;
        
        // Start watching the session directory
        watcher.watch(&self.session_path, RecursiveMode::Recursive)
            .map_err(|e| NekocodeError::Watch(format!("Failed to watch path: {}", e)))?;
        
        // Process events with debouncing
        let mut last_update = Instant::now();
        let debounce_duration = Duration::from_millis(self.config.debounce_ms);
        let mut pending_changes = false;
        
        println!("👀 Started watching session {} at path: {}", self.session_id, self.session_path.display());
        println!("📁 Monitoring {} supported file types", self.config.include_extensions.len());
        
        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    // Check if any of the changed files should be watched
                    let should_process = match &event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                            event.paths.iter().any(|path| self.should_watch_file(path))
                        }
                        _ => false,
                    };
                    
                    if should_process {
                        pending_changes = true;
                        last_update = Instant::now();
                        println!("📝 File change detected: {:?}", event.paths);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Check if we should process pending changes
                    if pending_changes && last_update.elapsed() >= debounce_duration {
                        println!("⚡ Triggering incremental analysis after {}ms debounce", self.config.debounce_ms);
                        
                        // Trigger incremental analysis
                        let session_id = self.session_id.clone();
                        let analyzer = Arc::clone(&self.incremental_analyzer);
                        
                        tokio::spawn(async move {
                            let mut analyzer = analyzer.lock().await;
                            match analyzer.analyze_changes(&session_id).await {
                                Ok(summary) => {
                                    println!("{}", summary.format_summary());
                                }
                                Err(e) => {
                                    eprintln!("❌ Failed to analyze changes: {}", e);
                                }
                            }
                        });
                        
                        pending_changes = false;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    println!("📡 File watcher channel disconnected");
                    break;
                }
            }
        }
        
        println!("🛑 Stopped watching session {}", self.session_id);
        Ok(())
    }
}

/// Watch manager for handling multiple watch sessions
pub struct WatchManager {
    session_manager: SessionManager,
    incremental_analyzer: Arc<Mutex<IncrementalAnalyzer>>,
    active_watchers: Arc<Mutex<HashMap<String, WatchStatus>>>,
}

impl WatchManager {
    /// Create new watch manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            session_manager: SessionManager::new()?,
            incremental_analyzer: Arc::new(Mutex::new(IncrementalAnalyzer::new()?)),
            active_watchers: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    /// Start watching a session
    pub async fn start_watch(&mut self, session_id: &str) -> Result<()> {
        // Check if session exists
        let session = self.session_manager.get_session_mut(session_id)?;
        let session_path = session.info.path.clone();
        
        // Check if already watching
        {
            let watchers = self.active_watchers.lock().await;
            if watchers.contains_key(session_id) {
                return Err(NekocodeError::Watch(
                    format!("Session {} is already being watched", session_id)
                ));
            }
        }
        
        // Initialize incremental analyzer for this session
        {
            let mut analyzer = self.incremental_analyzer.lock().await;
            analyzer.initialize_session(session_id)?;
        }
        
        // Create watcher
        let watcher = FileWatcher::new(
            session_id.to_string(),
            session_path.clone(),
            Arc::clone(&self.incremental_analyzer)
        );
        
        // Add to active watchers
        {
            let mut watchers = self.active_watchers.lock().await;
            watchers.insert(session_id.to_string(), WatchStatus {
                session_id: session_id.to_string(),
                status: WatchState::Watching,
                watched_files: 0, // Will be updated
                last_update: None,
                started_at: Utc::now(),
            });
        }
        
        // Start watching in background
        let session_id_clone = session_id.to_string();
        let active_watchers = Arc::clone(&self.active_watchers);
        
        tokio::spawn(async move {
            if let Err(e) = watcher.start_watching().await {
                eprintln!("Watch error for session {}: {}", session_id_clone, e);
                
                // Update status to error
                let mut watchers = active_watchers.lock().await;
                if let Some(status) = watchers.get_mut(&session_id_clone) {
                    status.status = WatchState::Error(e.to_string());
                }
            } else {
                // Update status to stopped
                let mut watchers = active_watchers.lock().await;
                if let Some(status) = watchers.get_mut(&session_id_clone) {
                    status.status = WatchState::Stopped;
                }
            }
        });
        
        println!("✅ Started watching session {}", session_id);
        Ok(())
    }
    
    /// Stop watching a session
    pub async fn stop_watch(&mut self, session_id: &str) -> Result<()> {
        let mut watchers = self.active_watchers.lock().await;
        if let Some(mut status) = watchers.remove(session_id) {
            status.status = WatchState::Stopped;
            println!("🛑 Stopped watching session {}", session_id);
            Ok(())
        } else {
            Err(NekocodeError::Watch(
                format!("Session {} is not being watched", session_id)
            ))
        }
    }
    
    /// Get watch status for a session
    pub async fn get_status(&self, session_id: &str) -> Option<WatchStatus> {
        let watchers = self.active_watchers.lock().await;
        watchers.get(session_id).cloned()
    }
    
    /// List all active watch sessions
    pub async fn list_active_watches(&self) -> Vec<WatchStatus> {
        let watchers = self.active_watchers.lock().await;
        watchers.values().cloned().collect()
    }
    
    /// Stop all active watches
    pub async fn stop_all_watches(&mut self) -> Result<()> {
        let mut watchers = self.active_watchers.lock().await;
        let count = watchers.len();
        watchers.clear();
        println!("🛑 Stopped {} active watch sessions", count);
        Ok(())
    }
}

use std::collections::HashMap;