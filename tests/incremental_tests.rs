//! Comprehensive test suite for incremental analysis functionality
//! 
//! Tests the core incremental analysis features including change detection,
//! session updates, and performance improvements.

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use std::fs;
    use std::path::PathBuf;
    
    use nekocode_rust::core::incremental::{ChangeDetector, ChangeType, IncrementalSummary};
    use nekocode_rust::core::session::SessionManager;
    use nekocode_rust::commands::session_update::handle_session_update;
    
    /// Test basic change detection functionality
    #[tokio::test]
    async fn test_change_detection_basic() {
        let temp_dir = TempDir::new().unwrap();
        let mut detector = ChangeDetector::new(temp_dir.path().to_path_buf());
        
        // Initialize with empty directory
        let files = detector.initialize().unwrap();
        assert_eq!(files.len(), 0);
        
        // Add a JavaScript file
        let js_file = temp_dir.path().join("test.js");
        fs::write(&js_file, "console.log('hello world');").unwrap();
        
        // Detect the addition
        let changes = detector.detect_changes().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Added);
        assert_eq!(changes[0].path, PathBuf::from("test.js"));
        
        // No changes on second scan
        let changes = detector.detect_changes().unwrap();
        assert_eq!(changes.len(), 0);
        
        // Modify the file
        fs::write(&js_file, "console.log('hello world updated');").unwrap();
        
        let changes = detector.detect_changes().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Modified);
        
        // Delete the file
        fs::remove_file(&js_file).unwrap();
        
        let changes = detector.detect_changes().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Deleted);
    }
    
    /// Test change detection with multiple file types
    #[tokio::test]
    async fn test_change_detection_multiple_languages() {
        let temp_dir = TempDir::new().unwrap();
        let mut detector = ChangeDetector::new(temp_dir.path().to_path_buf());
        
        // Create files of different languages
        let files = vec![
            ("test.js", "console.log('JavaScript');"),
            ("test.py", "print('Python')"),
            ("test.cpp", "#include <iostream>\nint main() { return 0; }"),
            ("test.rs", "fn main() { println!(\"Rust\"); }"),
            ("test.go", "package main\nfunc main() {}"),
        ];
        
        for (filename, content) in &files {
            let file_path = temp_dir.path().join(filename);
            fs::write(&file_path, content).unwrap();
        }
        
        let changes = detector.detect_changes().unwrap();
        assert_eq!(changes.len(), files.len());
        
        // Verify all files are detected as added
        for change in &changes {
            assert_eq!(change.change_type, ChangeType::Added);
        }
    }
    
    /// Test incremental session update with no changes
    #[tokio::test]
    async fn test_session_update_no_changes() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create initial file
        let test_file = temp_dir.path().join("test.js");
        fs::write(&test_file, "console.log('initial');").unwrap();
        
        // Create session
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();
        
        // Update session without any changes
        let summary = session_manager.update_session_incremental(&session_id).await.unwrap();
        
        assert_eq!(summary.changed_files, 0);
        assert_eq!(summary.added_files, 0);
        assert_eq!(summary.deleted_files, 0);
        assert!(summary.analysis_time_ms < 1000); // Should be very fast
    }
    
    /// Test incremental session update with file modifications
    #[tokio::test]
    async fn test_session_update_with_modifications() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create initial files
        let file1 = temp_dir.path().join("file1.js");
        let file2 = temp_dir.path().join("file2.py");
        fs::write(&file1, "console.log('file1');").unwrap();
        fs::write(&file2, "print('file2')").unwrap();
        
        // Create session
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();
        
        // Modify one file
        fs::write(&file1, "console.log('file1 modified');").unwrap();
        
        // Update session
        let summary = session_manager.update_session_incremental(&session_id).await.unwrap();
        
        assert_eq!(summary.changed_files, 1);
        assert_eq!(summary.added_files, 0);
        assert_eq!(summary.deleted_files, 0);
        assert_eq!(summary.total_files, 2);
    }
    
    /// Test incremental session update with file additions
    #[tokio::test]
    async fn test_session_update_with_additions() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create initial file
        let file1 = temp_dir.path().join("file1.js");
        fs::write(&file1, "console.log('file1');").unwrap();
        
        // Create session
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();
        
        // Add new files
        let file2 = temp_dir.path().join("file2.py");
        let file3 = temp_dir.path().join("file3.rs");
        fs::write(&file2, "print('file2')").unwrap();
        fs::write(&file3, "fn main() {}").unwrap();
        
        // Update session
        let summary = session_manager.update_session_incremental(&session_id).await.unwrap();
        
        assert_eq!(summary.changed_files, 0);
        assert_eq!(summary.added_files, 2);
        assert_eq!(summary.deleted_files, 0);
        assert_eq!(summary.total_files, 3);
    }
    
    /// Test incremental session update with file deletions
    #[tokio::test]
    async fn test_session_update_with_deletions() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create initial files
        let file1 = temp_dir.path().join("file1.js");
        let file2 = temp_dir.path().join("file2.py");
        let file3 = temp_dir.path().join("file3.rs");
        fs::write(&file1, "console.log('file1');").unwrap();
        fs::write(&file2, "print('file2')").unwrap();
        fs::write(&file3, "fn main() {}").unwrap();
        
        // Create session
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();
        
        // Delete files
        fs::remove_file(&file2).unwrap();
        fs::remove_file(&file3).unwrap();
        
        // Update session
        let summary = session_manager.update_session_incremental(&session_id).await.unwrap();
        
        assert_eq!(summary.changed_files, 0);
        assert_eq!(summary.added_files, 0);
        assert_eq!(summary.deleted_files, 2);
        assert_eq!(summary.total_files, 1);
    }
    
    /// Test CLI session-update command
    #[tokio::test]
    async fn test_cli_session_update_command() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create initial file
        let test_file = temp_dir.path().join("test.js");
        fs::write(&test_file, "console.log('initial');").unwrap();
        
        // Create session
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();
        
        // Test basic update
        let result = handle_session_update(&session_id, false, false).await.unwrap();
        assert!(result.contains("0 modified"));
        assert!(result.contains("0 added"));
        
        // Modify file and test again
        fs::write(&test_file, "console.log('modified');").unwrap();
        
        let result = handle_session_update(&session_id, false, false).await.unwrap();
        assert!(result.contains("1 modified") || result.contains("Updated 1 files"));
    }
    
    /// Test dry run functionality
    #[tokio::test]
    async fn test_session_update_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create initial file
        let test_file = temp_dir.path().join("test.js");
        fs::write(&test_file, "console.log('initial');").unwrap();
        
        // Create session
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();
        
        // Add new file
        let new_file = temp_dir.path().join("new.js");
        fs::write(&new_file, "console.log('new');").unwrap();
        
        // Test dry run
        let result = handle_session_update(&session_id, false, true).await.unwrap();
        assert!(result.contains("pending changes"));
        assert!(result.contains("new.js"));
        assert!(result.contains("Run without --dry-run"));
        
        // Verify session wasn't actually updated
        let session_info = session_manager.get_session_info(&session_id).unwrap();
        assert_eq!(session_info.analysis_results.len(), 1); // Should still be 1
    }
    
    /// Test verbose output
    #[tokio::test]
    async fn test_session_update_verbose() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create initial file
        let test_file = temp_dir.path().join("test.js");
        fs::write(&test_file, "console.log('initial');").unwrap();
        
        // Create session
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();
        
        // Test verbose output
        let result = handle_session_update(&session_id, true, false).await.unwrap();
        
        // Should be valid JSON
        let json_result: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(json_result["session_id"].is_string());
        assert!(json_result["summary"].is_object());
        assert!(json_result["performance"].is_object());
    }
    
    /// Test performance improvement (speedup calculation)
    #[tokio::test]
    async fn test_performance_improvement() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create multiple files to simulate a larger project
        for i in 0..10 {
            let file_path = temp_dir.path().join(format!("file{}.js", i));
            fs::write(&file_path, format!("console.log('file{}');", i)).unwrap();
        }
        
        // Create session
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();
        
        // Modify only one file
        let file_path = temp_dir.path().join("file0.js");
        fs::write(&file_path, "console.log('file0 modified');").unwrap();
        
        // Update session and measure performance
        let start = std::time::Instant::now();
        let summary = session_manager.update_session_incremental(&session_id).await.unwrap();
        let duration = start.elapsed();
        
        // Verify only one file was updated
        assert_eq!(summary.changed_files, 1);
        assert_eq!(summary.total_files, 10);
        
        // Performance should be good (under 3 seconds as per requirements)
        assert!(duration.as_secs() < 3);
        
        // Estimated speedup should be significant
        assert!(summary.estimated_speedup > 10.0);
    }
    
    /// Test error handling for invalid session ID
    #[tokio::test]
    async fn test_invalid_session_id() {
        let result = handle_session_update("invalid-session-id", false, false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Session not found"));
    }
    
    /// Test incremental summary formatting
    #[test]
    fn test_incremental_summary_formatting() {
        let summary = IncrementalSummary::new(100, &[], 1500, 45000);
        let formatted = summary.format_summary();
        
        assert!(formatted.contains("Updated 0 files"));
        assert!(formatted.contains("1500ms"));
        assert!(formatted.contains("30.0x speedup"));
        assert!(formatted.contains("Total files in session: 100"));
    }
    
    /// Test backward compatibility with existing sessions
    #[tokio::test]
    async fn test_backward_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create initial file
        let test_file = temp_dir.path().join("test.js");
        fs::write(&test_file, "console.log('initial');").unwrap();
        
        // Create session
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();
        
        // Update should work (change detector is initialized during session creation)
        let summary = session_manager.update_session_incremental(&session_id).await.unwrap();
        
        // Should succeed without errors
        assert_eq!(summary.total_files, 1);
    }
}