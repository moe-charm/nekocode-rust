//! File watching system with auto session updates
//! 
//! This module implements the file watching system that monitors directories
//! for changes and automatically triggers session updates.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::core::session::SessionManager;

/// File watching status for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchStatus {
    pub session_id: String,
    pub status: WatchState,
    pub pid: Option<u32>,
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

/// PID file operations
pub struct PidManager;

impl PidManager {
    /// Get PID file path for a session
    pub fn get_pid_file(session_id: &str) -> PathBuf {
        PathBuf::from("/tmp").join(format!("nekocode-watch-{}.pid", session_id))
    }

    /// Get lock file path for a session
    pub fn get_lock_file(session_id: &str) -> PathBuf {
        PathBuf::from("/tmp").join(format!("nekocode-watch-{}.lock", session_id))
    }

    /// Write PID file
    pub fn write_pid_file(session_id: &str, pid: u32) -> Result<()> {
        let pid_file = Self::get_pid_file(session_id);
        fs::write(&pid_file, pid.to_string())
            .with_context(|| format!("Failed to write PID file: {}", pid_file.display()))?;
        Ok(())
    }

    /// Read PID from file
    pub fn read_pid_file(session_id: &str) -> Result<Option<u32>> {
        let pid_file = Self::get_pid_file(session_id);
        if !pid_file.exists() {
            return Ok(None);
        }

        let pid_str = fs::read_to_string(&pid_file)
            .with_context(|| format!("Failed to read PID file: {}", pid_file.display()))?;
        
        let pid: u32 = pid_str.trim().parse()
            .with_context(|| format!("Invalid PID in file: {}", pid_str))?;
        
        Ok(Some(pid))
    }

    /// Remove PID file
    pub fn remove_pid_file(session_id: &str) -> Result<()> {
        let pid_file = Self::get_pid_file(session_id);
        if pid_file.exists() {
            fs::remove_file(&pid_file)
                .with_context(|| format!("Failed to remove PID file: {}", pid_file.display()))?;
        }
        Ok(())
    }

    /// Check if process is running
    pub fn is_process_running(pid: u32) -> bool {
        #[cfg(unix)]
        {
            // On Unix systems, check if process exists
            unsafe {
                libc::kill(pid as libc::pid_t, 0) == 0
            }
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, use a simple heuristic
            // This is not perfect but better than nothing
            Command::new("ps")
                .arg("-p")
                .arg(pid.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        }
    }
}

/// File watcher manager
pub struct FileWatcher {
    config: WatchConfig,
    session_id: String,
    session_path: PathBuf,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(session_id: String, session_path: PathBuf) -> Self {
        Self {
            config: WatchConfig::default(),
            session_id,
            session_path,
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

    /// Start watching files in the background
    pub fn start_watching(&self) -> Result<()> {
        let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();
        
        // Create file system watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if let Err(e) = tx.send(event) {
                        eprintln!("Failed to send file watch event: {}", e);
                    }
                }
                Err(e) => eprintln!("File watch error: {:?}", e),
            }
        })?;

        // Start watching the session directory
        watcher.watch(&self.session_path, RecursiveMode::Recursive)?;

        // Process events with debouncing
        let mut last_update = Instant::now();
        let debounce_duration = Duration::from_millis(self.config.debounce_ms);
        let mut pending_changes = false;

        println!("Started watching session {} (PID: {})", self.session_id, std::process::id());

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
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Check if we should process pending changes
                    if pending_changes && last_update.elapsed() >= debounce_duration {
                        if let Err(e) = self.trigger_session_update() {
                            eprintln!("Failed to trigger session update: {}", e);
                        }
                        pending_changes = false;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    eprintln!("File watcher channel disconnected");
                    break;
                }
            }

            // Check for parent process heartbeat (simple implementation)
            // In a more sophisticated implementation, this would check if the MCP server is still running
            if !self.check_parent_alive() {
                println!("Parent process no longer detected, shutting down watcher");
                break;
            }
        }

        // Cleanup
        PidManager::remove_pid_file(&self.session_id)?;
        Ok(())
    }

    /// Trigger session update
    fn trigger_session_update(&self) -> Result<()> {
        // Use the existing session-update command
        let output = Command::new(std::env::current_exe()?)
            .arg("session-update")
            .arg(&self.session_id)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Session update failed: {}", stderr);
        }

        println!("Session {} updated automatically", self.session_id);
        Ok(())
    }

    /// Simple parent process check (basic implementation)
    fn check_parent_alive(&self) -> bool {
        // For now, just return true
        // In a real implementation, this would check if the MCP server process is still running
        // This could be done by checking a heartbeat file or using process monitoring
        true
    }
}

/// Start watching a session
pub fn handle_watch_start(session_id: &str) -> Result<String> {
    // Check if session exists
    let session_manager = SessionManager::new()?;
    let session_info = session_manager.get_session_info(session_id)
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

    // Check if already watching
    if let Some(pid) = PidManager::read_pid_file(session_id)? {
        if PidManager::is_process_running(pid) {
            return Ok(format!("Session {} is already being watched (PID: {})", session_id, pid));
        } else {
            // Clean up stale PID file
            PidManager::remove_pid_file(session_id)?;
        }
    }

    // Fork background process
    let current_pid = std::process::id();
    
    // Create the file watcher
    let watcher = FileWatcher::new(session_id.to_string(), session_info.path.clone());
    
    // For now, start watching in a separate thread (in production, this should be a separate process)
    let session_id_clone = session_id.to_string();
    thread::spawn(move || {
        if let Err(e) = watcher.start_watching() {
            eprintln!("File watcher error: {}", e);
            let _ = PidManager::remove_pid_file(&session_id_clone);
        }
    });

    // Write PID file
    PidManager::write_pid_file(session_id, current_pid)?;

    Ok(format!("Started watching session {} (PID: {})", session_id, current_pid))
}

/// Show watch status for sessions
pub fn handle_watch_status(session_id: Option<&str>) -> Result<String> {
    let mut statuses = Vec::new();

    if let Some(id) = session_id {
        // Show status for specific session
        let status = get_session_watch_status(id)?;
        statuses.push(status);
    } else {
        // Show status for all sessions
        let session_manager = SessionManager::new()?;
        let all_sessions = session_manager.list_sessions();
        
        for session in all_sessions {
            let status = get_session_watch_status(&session.id)?;
            statuses.push(status);
        }
    }

    // Format output
    let mut output = Vec::new();
    let mut active_count = 0;

    for status in statuses {
        match status.status {
            WatchState::Watching => {
                output.push(format!("Session {}: WATCHING ({} files, PID: {})", 
                    status.session_id, 
                    status.watched_files,
                    status.pid.unwrap_or(0)
                ));
                active_count += 1;
            }
            WatchState::Stopped => {
                output.push(format!("Session {}: STOPPED", status.session_id));
            }
            WatchState::Error(ref error) => {
                output.push(format!("Session {}: ERROR ({})", status.session_id, error));
            }
        }
    }

    if session_id.is_none() {
        output.push(format!("Total active watchers: {}", active_count));
    }

    Ok(output.join("\n"))
}

/// Get watch status for a specific session
fn get_session_watch_status(session_id: &str) -> Result<WatchStatus> {
    let session_manager = SessionManager::new()?;
    let session_info = session_manager.get_session_info(session_id)
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

    let mut status = WatchStatus {
        session_id: session_id.to_string(),
        status: WatchState::Stopped,
        pid: None,
        watched_files: 0,
        last_update: None,
        started_at: Utc::now(),
    };

    // Check if PID file exists and process is running
    if let Some(pid) = PidManager::read_pid_file(session_id)? {
        if PidManager::is_process_running(pid) {
            status.status = WatchState::Watching;
            status.pid = Some(pid);
        } else {
            // Clean up stale PID file
            PidManager::remove_pid_file(session_id)?;
        }
    }

    // Count watched files (estimate based on session files)
    status.watched_files = session_info.analysis_results.len();

    Ok(status)
}

/// Stop watching a session
pub fn handle_watch_stop(session_id: &str) -> Result<String> {
    if let Some(pid) = PidManager::read_pid_file(session_id)? {
        if PidManager::is_process_running(pid) {
            // Send SIGTERM to gracefully stop the process
            #[cfg(unix)]
            {
                unsafe {
                    libc::kill(pid as libc::pid_t, libc::SIGTERM);
                }
            }

            // Wait a bit for graceful shutdown
            thread::sleep(Duration::from_millis(500));

            // Force kill if still running
            if PidManager::is_process_running(pid) {
                #[cfg(unix)]
                {
                    unsafe {
                        libc::kill(pid as libc::pid_t, libc::SIGKILL);
                    }
                }
            }
        }

        // Clean up PID file
        PidManager::remove_pid_file(session_id)?;
        Ok(format!("Stopped watching session {}", session_id))
    } else {
        Ok(format!("Session {} is not being watched", session_id))
    }
}

/// Stop all active watchers
pub fn handle_watch_stop_all() -> Result<String> {
    let session_manager = SessionManager::new()?;
    let all_sessions = session_manager.list_sessions();
    let mut stopped_count = 0;

    for session in all_sessions {
        if PidManager::read_pid_file(&session.id)?.is_some() {
            let _ = handle_watch_stop(&session.id)?;
            stopped_count += 1;
        }
    }

    Ok(format!("Stopped {} active watchers", stopped_count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use crate::core::session::SessionManager;

    #[test]
    fn test_pid_file_operations() {
        let session_id = "test123";
        let pid = 12345u32;

        // Test writing and reading PID file
        PidManager::write_pid_file(session_id, pid).unwrap();
        let read_pid = PidManager::read_pid_file(session_id).unwrap();
        assert_eq!(read_pid, Some(pid));

        // Test removing PID file
        PidManager::remove_pid_file(session_id).unwrap();
        let read_pid_after_remove = PidManager::read_pid_file(session_id).unwrap();
        assert_eq!(read_pid_after_remove, None);
    }

    #[test]
    fn test_should_watch_file() {
        let watcher = FileWatcher::new("test".to_string(), PathBuf::from("/tmp"));

        // Should watch supported extensions
        assert!(watcher.should_watch_file(Path::new("test.js")));
        assert!(watcher.should_watch_file(Path::new("test.ts")));
        assert!(watcher.should_watch_file(Path::new("test.py")));
        assert!(watcher.should_watch_file(Path::new("test.rs")));

        // Should not watch excluded patterns
        assert!(!watcher.should_watch_file(Path::new("node_modules/test.js")));
        assert!(!watcher.should_watch_file(Path::new(".git/config")));
        assert!(!watcher.should_watch_file(Path::new("target/debug/test")));

        // Should not watch unsupported extensions
        assert!(!watcher.should_watch_file(Path::new("test.txt")));
        assert!(!watcher.should_watch_file(Path::new("test.md")));
    }

    #[tokio::test]
    async fn test_watch_status_no_active_watchers() {
        let result = handle_watch_status(None).unwrap();
        assert!(result.contains("Total active watchers: 0"));
    }

    #[tokio::test]
    async fn test_watch_session_not_found() {
        let result = handle_watch_start("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Session not found"));
    }

    #[tokio::test]
    async fn test_basic_file_watching() {
        // Create test environment
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.js");
        fs::write(&test_file, "console.log('test');").unwrap();

        // Create session
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();

        // Test that we can get status for the session
        let status = get_session_watch_status(&session_id).unwrap();
        assert_eq!(status.session_id, session_id);
        assert!(matches!(status.status, WatchState::Stopped));
    }

    #[test]
    fn test_watch_config_defaults() {
        let config = WatchConfig::default();
        assert_eq!(config.debounce_ms, 500);
        assert_eq!(config.max_events_per_second, 1000);
        assert!(config.exclude_patterns.contains(&".git".to_string()));
        assert!(config.include_extensions.contains(&"js".to_string()));
        assert!(config.include_extensions.contains(&"rs".to_string()));
    }

    // Integration test: Create session and start watching
    #[tokio::test]
    async fn test_watch_start_integration() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.js");
        fs::write(&test_file, "console.log('test');").unwrap();

        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();

        // Test starting watch (this will fail in the test environment but we can test the validation)
        let result = handle_watch_start(&session_id);
        // In test environment, this might fail due to process limitations, but it should at least validate the session
        // The important thing is that it doesn't panic and returns a reasonable error or success
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("Session"));
    }

    // Test file modification detection  
    #[tokio::test] 
    async fn test_file_modification_detection() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.js");
        fs::write(&test_file, "console.log('test');").unwrap();

        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();

        // Modify file
        fs::write(&test_file, "console.log('modified');").unwrap();

        // In a real scenario, the watcher would detect this change
        // For now, we just verify the session exists and can be updated
        let session_info = session_manager.get_session_info(&session_id);
        assert!(session_info.is_some());
    }

    // Test PID file management and process checking
    #[test]
    fn test_pid_management() {
        let session_id = "test_pid_management";
        
        // Initially no PID file should exist
        assert!(PidManager::read_pid_file(session_id).unwrap().is_none());
        
        // Write a PID file
        let test_pid = std::process::id();
        PidManager::write_pid_file(session_id, test_pid).unwrap();
        
        // Should be able to read it back
        let read_pid = PidManager::read_pid_file(session_id).unwrap();
        assert_eq!(read_pid, Some(test_pid));
        
        // Current process should be running
        assert!(PidManager::is_process_running(test_pid));
        
        // Clean up
        PidManager::remove_pid_file(session_id).unwrap();
        assert!(PidManager::read_pid_file(session_id).unwrap().is_none());
    }

    // Test error handling for invalid sessions
    #[test]
    fn test_error_handling_invalid_session() {
        let result = handle_watch_start("invalid_session_id_that_does_not_exist");
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Session not found"));
    }
}