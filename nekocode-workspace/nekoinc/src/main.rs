//! NekoInc - Incremental analysis and file watching tool

use clap::Parser;
use std::fs;

use nekocode_core::{Result, NekocodeError};
use nekoinc::{
    Cli, 
    IncrementalAnalyzer,
    WatchManager
};
use nekoinc::cli::Commands;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Execute command
    match cli.command {
        Commands::Init { session_id } => {
            let mut analyzer = IncrementalAnalyzer::new()?;
            analyzer.initialize_session(&session_id)?;
            println!("✅ Initialized incremental tracking for session {}", session_id);
        }
        
        Commands::Update { session_id, verbose } => {
            let mut analyzer = IncrementalAnalyzer::new()?;
            let summary = analyzer.analyze_changes(&session_id).await?;
            
            println!("{}", summary.format_summary());
            
            if verbose && (summary.changed_files > 0 || summary.added_files > 0 || summary.deleted_files > 0) {
                println!("\n📋 Detailed changes:");
                
                // Get detector to show detailed changes
                if let Some(detector) = analyzer.get_detector(&session_id) {
                    println!("  Last scan: {}", detector.last_scan_time().format("%Y-%m-%d %H:%M:%S"));
                    println!("  Base path: {}", detector.base_path().display());
                    println!("  Tracked files: {}", detector.file_count());
                }
            }
        }
        
        Commands::Watch { session_id, debounce: _ } => {
            let mut manager = WatchManager::new()?;
            manager.start_watch(&session_id).await?;
            
            println!("Press Ctrl+C to stop watching...");
            
            // Keep the main thread alive
            tokio::signal::ctrl_c().await
                .expect("Failed to listen for Ctrl+C");
            
            manager.stop_watch(&session_id).await?;
        }
        
        Commands::StopWatch { session_id } => {
            let mut manager = WatchManager::new()?;
            manager.stop_watch(&session_id).await?;
        }
        
        Commands::Status { session_id } => {
            let manager = WatchManager::new()?;
            
            if let Some(id) = session_id {
                // Show status for specific session
                if let Some(status) = manager.get_status(&id).await {
                    println!("📊 Watch status for session {}:", id);
                    println!("  Status: {:?}", status.status);
                    println!("  Started at: {}", status.started_at.format("%Y-%m-%d %H:%M:%S"));
                    if let Some(last_update) = status.last_update {
                        println!("  Last update: {}", last_update.format("%Y-%m-%d %H:%M:%S"));
                    }
                } else {
                    println!("Session {} is not being watched", id);
                }
            } else {
                // Show all active watches
                let active = manager.list_active_watches().await;
                
                if active.is_empty() {
                    println!("No active watch sessions");
                } else {
                    println!("📊 Active watch sessions:");
                    for status in active {
                        println!("  {} - {:?} (started {})", 
                            status.session_id,
                            status.status,
                            status.started_at.format("%H:%M:%S")
                        );
                    }
                }
            }
        }
        
        Commands::StopAll => {
            let mut manager = WatchManager::new()?;
            manager.stop_all_watches().await?;
        }
        
        Commands::Diff { session1, session2, format } => {
            let mut analyzer1 = IncrementalAnalyzer::new()?;
            let mut analyzer2 = IncrementalAnalyzer::new()?;
            
            analyzer1.initialize_session(&session1)?;
            analyzer2.initialize_session(&session2)?;
            
            let detector1 = analyzer1.get_detector(&session1)
                .ok_or_else(|| NekocodeError::Session(format!("No detector for session {}", session1)))?;
            let detector2 = analyzer2.get_detector(&session2)
                .ok_or_else(|| NekocodeError::Session(format!("No detector for session {}", session2)))?;
            
            match format.as_str() {
                "json" => {
                    let diff = serde_json::json!({
                        "session1": {
                            "id": session1,
                            "files": detector1.file_count(),
                            "last_scan": detector1.last_scan_time(),
                        },
                        "session2": {
                            "id": session2,
                            "files": detector2.file_count(),
                            "last_scan": detector2.last_scan_time(),
                        }
                    });
                    println!("{}", serde_json::to_string_pretty(&diff)?);
                }
                _ => {
                    println!("📊 Session comparison:");
                    println!("  Session 1: {} ({} files)", session1, detector1.file_count());
                    println!("  Session 2: {} ({} files)", session2, detector2.file_count());
                    
                    let diff = (detector1.file_count() as i32 - detector2.file_count() as i32).abs();
                    println!("  Difference: {} files", diff);
                }
            }
        }
        
        Commands::Stats { session_id } => {
            let analyzer = IncrementalAnalyzer::new()?;
            
            if let Some(detector) = analyzer.get_detector(&session_id) {
                println!("📊 Incremental analysis statistics for session {}:", session_id);
                println!("  Tracked files: {}", detector.file_count());
                println!("  Last scan: {}", detector.last_scan_time().format("%Y-%m-%d %H:%M:%S"));
                println!("  Base path: {}", detector.base_path().display());
            } else {
                println!("No incremental tracking data for session {}", session_id);
                println!("Run 'nekoinc init {}' to initialize tracking", session_id);
            }
        }
        
        Commands::Reset { session_id } => {
            let mut analyzer = IncrementalAnalyzer::new()?;
            analyzer.initialize_session(&session_id)?;
            println!("✅ Reset incremental tracking for session {}", session_id);
        }
        
        Commands::Export { session_id, output, format } => {
            let mut analyzer = IncrementalAnalyzer::new()?;
            
            // Get changes
            let summary = analyzer.analyze_changes(&session_id).await?;
            
            match format.as_str() {
                "csv" => {
                    let csv_content = format!(
                        "metric,value\n\
                        total_files,{}\n\
                        changed_files,{}\n\
                        added_files,{}\n\
                        deleted_files,{}\n\
                        analysis_time_ms,{}\n\
                        estimated_speedup,{}",
                        summary.total_files,
                        summary.changed_files,
                        summary.added_files,
                        summary.deleted_files,
                        summary.analysis_time_ms,
                        summary.estimated_speedup
                    );
                    
                    fs::write(&output, csv_content)
                        .map_err(|e| NekocodeError::Io(e))?;
                }
                _ => {
                    // Default to JSON
                    let json_content = serde_json::to_string_pretty(&summary)?;
                    fs::write(&output, json_content)
                        .map_err(|e| NekocodeError::Io(e))?;
                }
            }
            
            println!("✅ Exported change history to {}", output.display());
        }
    }
    
    Ok(())
}