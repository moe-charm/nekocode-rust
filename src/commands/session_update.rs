//! Session update command implementation
//! 
//! This module handles the session-update CLI command for incremental analysis.

use anyhow::Result;
use crate::core::session::SessionManager;
use crate::core::incremental::IncrementalSummary;
use serde_json;

/// Handle the session-update command
pub async fn handle_session_update(
    session_id: &str, 
    verbose: bool, 
    dry_run: bool
) -> Result<String> {
    let mut session_manager = SessionManager::new()?;
    
    if dry_run {
        // For dry run, just show what would be updated without making changes
        return handle_dry_run(&mut session_manager, session_id).await;
    }
    
    // Perform the actual incremental update
    let summary = session_manager.update_session_incremental(session_id).await?;
    
    if verbose {
        // Return detailed JSON output
        let detailed_output = serde_json::json!({
            "session_id": session_id,
            "summary": {
                "total_files": summary.total_files,
                "changed_files": summary.changed_files,
                "added_files": summary.added_files,
                "deleted_files": summary.deleted_files,
                "analysis_time_ms": summary.analysis_time_ms,
                "estimated_speedup": summary.estimated_speedup
            },
            "performance": {
                "analysis_time": format!("{}ms", summary.analysis_time_ms),
                "speedup": format!("{:.1}x", summary.estimated_speedup),
                "status": if summary.changed_files + summary.added_files == 0 {
                    "no_changes"
                } else {
                    "updated"
                }
            }
        });
        
        Ok(serde_json::to_string_pretty(&detailed_output)?)
    } else {
        // Return simple summary
        Ok(summary.format_summary())
    }
}

/// Handle dry run mode - show what would be updated
async fn handle_dry_run(
    session_manager: &mut SessionManager,
    session_id: &str,
) -> Result<String> {
    // Get the session info
    let session_info = session_manager.get_session_info(session_id)
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
    
    // Check if change detector exists
    if session_info.change_detector.is_none() {
        return Ok(format!(
            "Session {} would be initialized with incremental analysis capabilities\n\
             Current files: {}\n\
             Note: First run will initialize change detection",
            session_id,
            session_info.analysis_results.len()
        ));
    }
    
    // Clone the change detector to avoid borrowing issues
    let mut change_detector = session_info.change_detector.as_ref().unwrap().clone();
    
    // Detect changes without updating the session
    let changes = change_detector.detect_changes()?;
    
    if changes.is_empty() {
        Ok(format!(
            "Session {} is up to date\n\
             No files have changed since last analysis\n\
             Total files: {}",
            session_id,
            session_info.analysis_results.len()
        ))
    } else {
        let mut output = Vec::new();
        output.push(format!("Session {} has pending changes:", session_id));
        output.push(format!("Total files in session: {}", session_info.analysis_results.len()));
        output.push("".to_string());
        
        let mut added_count = 0;
        let mut modified_count = 0;
        let mut deleted_count = 0;
        
        for change in &changes {
            match change.change_type {
                crate::core::incremental::ChangeType::Added => {
                    output.push(format!("  + {}", change.path.display()));
                    added_count += 1;
                }
                crate::core::incremental::ChangeType::Modified => {
                    output.push(format!("  M {}", change.path.display()));
                    modified_count += 1;
                }
                crate::core::incremental::ChangeType::Deleted => {
                    output.push(format!("  - {}", change.path.display()));
                    deleted_count += 1;
                }
            }
        }
        
        output.push("".to_string());
        output.push(format!(
            "Summary: {} added, {} modified, {} deleted",
            added_count, modified_count, deleted_count
        ));
        output.push("Run without --dry-run to apply these changes".to_string());
        
        Ok(output.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use crate::core::session::SessionManager;
    
    #[tokio::test]
    async fn test_session_update_no_changes() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.js");
        fs::write(&test_file, "console.log('test');").unwrap();
        
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();
        
        // Update without any changes
        let result = handle_session_update(&session_id, false, false).await.unwrap();
        
        assert!(result.contains("0 modified"));
        assert!(result.contains("0 added"));
        assert!(result.contains("0 deleted"));
    }
    
    #[tokio::test]
    async fn test_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.js");
        fs::write(&test_file, "console.log('test');").unwrap();
        
        let mut session_manager = SessionManager::new().unwrap();
        let session_id = session_manager.create_session(temp_dir.path()).await.unwrap();
        
        // Add a new file
        let new_file = temp_dir.path().join("new.js");
        fs::write(&new_file, "console.log('new');").unwrap();
        
        let result = handle_session_update(&session_id, false, true).await.unwrap();
        
        assert!(result.contains("pending changes"));
        assert!(result.contains("new.js"));
    }
}