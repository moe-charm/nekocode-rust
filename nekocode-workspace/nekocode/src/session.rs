//! Session management and commands

use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;

use nekocode_core::{
    Result, NekocodeError,
    session::{SessionManager, SessionInfo},
    types::{AnalysisResult, Language}
};

use crate::analyzer::{
    Analyzer, JavaScriptAnalyzer, TypeScriptAnalyzer,
    PythonAnalyzer, RustAnalyzer, CppAnalyzer,
    GoAnalyzer, CSharpAnalyzer
};
use crate::ast::{ASTBuilder, ASTStatistics};

/// Session commands for AST operations
pub struct SessionCommands {
    session_manager: SessionManager,
}

impl SessionCommands {
    pub fn new() -> Result<Self> {
        Ok(Self {
            session_manager: SessionManager::new()?,
        })
    }
    
    /// Get AST statistics for a session
    pub async fn ast_stats(&mut self, session_id: &str) -> Result<String> {
        let session = self.session_manager.get_session_mut(session_id)?;
        
        let mut total_nodes = 0;
        let mut total_functions = 0;
        let mut total_classes = 0;
        let mut max_depth = 0;
        
        for result in &session.info.analysis_results {
            total_functions += result.functions.len();
            total_classes += result.classes.len();
        }
        
        Ok(format!(
            "📊 AST Statistics for session {}\n\
             Files analyzed: {}\n\
             Total functions: {}\n\
             Total classes: {}\n\
             Total nodes: {}\n\
             Max AST depth: {}",
            session_id,
            session.info.analysis_results.len(),
            total_functions,
            total_classes,
            total_nodes,
            max_depth
        ))
    }
    
    /// Query AST by path
    pub async fn ast_query(&mut self, session_id: &str, path: &str) -> Result<String> {
        let session = self.session_manager.get_session_mut(session_id)?;
        
        // This would require storing AST in session or rebuilding it
        // For now, return a placeholder
        Ok(format!("🔍 Querying AST path: {}\n(AST query functionality requires AST storage in session)", path))
    }
    
    /// Dump AST in specified format
    pub async fn ast_dump(&mut self, session_id: &str, format: &str) -> Result<String> {
        let session = self.session_manager.get_session_mut(session_id)?;
        
        match format {
            "json" => {
                // Return session analysis results as JSON
                let json = serde_json::to_string_pretty(&session.info.analysis_results)?;
                Ok(json)
            }
            "tree" => {
                let mut output = String::new();
                output.push_str(&format!("🌳 AST Tree for session {}\n", session_id));
                for result in &session.info.analysis_results {
                    output.push_str(&format!("\n📄 {}\n", result.file_info.path.display()));
                    output.push_str(&format!("  Functions: {}\n", result.functions.len()));
                    output.push_str(&format!("  Classes: {}\n", result.classes.len()));
                }
                Ok(output)
            }
            _ => {
                Ok(format!("Unknown format: {}. Supported: json, tree", format))
            }
        }
    }
}

/// Session updater for incremental updates
pub struct SessionUpdater {
    session_manager: SessionManager,
}

impl SessionUpdater {
    pub fn new() -> Result<Self> {
        Ok(Self {
            session_manager: SessionManager::new()?,
        })
    }
    
    /// Create a new session by analyzing a directory
    pub async fn create_session(&mut self, path: &Path) -> Result<String> {
        let session_id = self.session_manager.create_session(path.to_path_buf())?;
        
        // Analyze all files in the directory
        let mut analysis_results = Vec::new();
        
        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();
            
            if !entry_path.is_file() {
                continue;
            }
            
            // Detect language and create appropriate analyzer
            if let Some(mut analyzer) = Self::create_analyzer_for_file(entry_path) {
                let content = fs::read_to_string(entry_path)?;
                
                match analyzer.analyze(entry_path, &content).await {
                    Ok(result) => {
                        analysis_results.push(result);
                    }
                    Err(e) => {
                        eprintln!("Failed to analyze {}: {}", entry_path.display(), e);
                    }
                }
            }
        }
        
        // Update session with results
        let session = self.session_manager.get_session_mut(&session_id)?;
        session.info.analysis_results = analysis_results;
        session.save()?;
        
        println!("✅ Created session {} with {} files analyzed", 
            session_id, 
            session.info.analysis_results.len()
        );
        
        Ok(session_id.to_string())
    }
    
    /// Update an existing session
    pub async fn update_session(&mut self, session_id: &str) -> Result<()> {
        let session = self.session_manager.get_session_mut(session_id)?;
        let base_path = session.info.path.clone();
        
        // Re-analyze all files (in a real implementation, this would be incremental)
        let mut analysis_results = Vec::new();
        
        for entry in WalkDir::new(&base_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();
            
            if !entry_path.is_file() {
                continue;
            }
            
            if let Some(mut analyzer) = Self::create_analyzer_for_file(entry_path) {
                let content = fs::read_to_string(entry_path)?;
                
                match analyzer.analyze(entry_path, &content).await {
                    Ok(result) => {
                        analysis_results.push(result);
                    }
                    Err(e) => {
                        eprintln!("Failed to analyze {}: {}", entry_path.display(), e);
                    }
                }
            }
        }
        
        // Update session
        let session = self.session_manager.get_session_mut(session_id)?;
        session.info.analysis_results = analysis_results;
        session.save()?;
        
        println!("✅ Updated session {} with {} files", 
            session_id, 
            session.info.analysis_results.len()
        );
        
        Ok(())
    }
    
    /// Create appropriate analyzer for a file based on extension
    fn create_analyzer_for_file(path: &Path) -> Option<Box<dyn Analyzer>> {
        let ext = path.extension()?.to_str()?;
        
        match ext {
            "js" | "jsx" | "mjs" | "cjs" => {
                JavaScriptAnalyzer::new().ok().map(|a| Box::new(a) as Box<dyn Analyzer>)
            }
            "ts" | "tsx" => {
                TypeScriptAnalyzer::new().ok().map(|a| Box::new(a) as Box<dyn Analyzer>)
            }
            "py" | "pyw" | "pyi" => {
                PythonAnalyzer::new().ok().map(|a| Box::new(a) as Box<dyn Analyzer>)
            }
            "rs" => {
                RustAnalyzer::new().ok().map(|a| Box::new(a) as Box<dyn Analyzer>)
            }
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" | "c" | "h" => {
                CppAnalyzer::new().ok().map(|a| Box::new(a) as Box<dyn Analyzer>)
            }
            "go" => {
                GoAnalyzer::new().ok().map(|a| Box::new(a) as Box<dyn Analyzer>)
            }
            "cs" => {
                CSharpAnalyzer::new().ok().map(|a| Box::new(a) as Box<dyn Analyzer>)
            }
            _ => None
        }
    }
}