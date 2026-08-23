//! Session management and commands

use std::path::Path;
use std::fs;
use walkdir::WalkDir;

use nekocode_core::{
    Result,
    session::SessionManager
};

use crate::analyzer::{
    Analyzer, JavaScriptAnalyzer, TypeScriptAnalyzer,
    PythonAnalyzer, RustAnalyzer, CppAnalyzer,
    GoAnalyzer, CSharpAnalyzer
};

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
        
        let mut total_functions = 0;
        let mut total_classes = 0;
        
        for result in &session.info.analysis_results {
            total_functions += result.functions.len();
            total_classes += result.classes.len();
        }
        
        Ok(format!(
            "📊 AST Statistics for session {}\n\
             Files analyzed: {}\n\
             Total functions: {}\n\
             Total classes: {}",
            session_id,
            session.info.analysis_results.len(),
            total_functions,
            total_classes
        ))
    }
    
    /// Query AST by path (searches for symbols, functions, classes)
    pub async fn ast_query(&mut self, session_id: &str, query: &str) -> Result<String> {
        let session = self.session_manager.get_session_mut(session_id)?;
        
        let mut output = String::new();
        output.push_str(&format!("🔍 Searching for: {}\n", query));
        output.push_str(&"=".repeat(50));
        output.push('\n');
        
        let mut found_count = 0;
        
        // Search in classes
        for result in &session.info.analysis_results {
            for class in &result.classes {
                if class.symbol.name.contains(query) || class.symbol.name == query {
                    found_count += 1;
                    output.push_str(&format!("\n📦 Class: {}\n", class.symbol.name));
                    output.push_str(&format!("   📄 File: {}\n", result.file_info.path.display()));
                    output.push_str(&format!("   📍 Lines: {}-{}\n", class.symbol.line_start, class.symbol.line_end));
                    output.push_str(&format!("   🔧 Methods: {}\n", class.methods.len()));
                    if !class.methods.is_empty() {
                        output.push_str("   📝 Methods:\n");
                        for method_id in &class.methods {
                            // Methods are stored as Symbol IDs (strings), just display them
                            output.push_str(&format!("      - {}\n", method_id));
                        }
                    }
                }
            }
            
            // Search in functions
            for func in &result.functions {
                if func.symbol.name.contains(query) || func.symbol.name == query {
                    found_count += 1;
                    output.push_str(&format!("\n⚡ Function: {}\n", func.symbol.name));
                    output.push_str(&format!("   📄 File: {}\n", result.file_info.path.display()));
                    output.push_str(&format!("   📍 Lines: {}-{}\n", func.symbol.line_start, func.symbol.line_end));
                    if let Some(ref ret_type) = func.return_type {
                        output.push_str(&format!("   📝 Returns: {}\n", ret_type));
                    }
                    if func.is_async {
                        output.push_str("   ⚡ Async function\n");
                    }
                }
            }
        }
        
        if found_count == 0 {
            output.push_str(&format!("\n❌ No matches found for '{}'\n", query));
            output.push_str("\n💡 Tips:\n");
            output.push_str("  - Try a partial name (e.g., 'String' instead of 'StringBox')\n");
            output.push_str("  - Check spelling and case\n");
            output.push_str("  - Use ast-dump to see all available symbols\n");
        } else {
            output.push_str(&format!("\n✅ Found {} matches\n", found_count));
        }
        
        Ok(output)
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
            "flat" => {
                // Flat list of symbols: one per line
                let mut lines = Vec::new();
                for result in &session.info.analysis_results {
                    for func in &result.functions {
                        lines.push(format!(
                            "FUNC\t{}\t{}:{}-{}",
                            result.file_info.path.display(),
                            func.symbol.name,
                            func.symbol.line_start,
                            func.symbol.line_end
                        ));
                    }
                    for class in &result.classes {
                        lines.push(format!(
                            "CLASS\t{}\t{}:{}-{}",
                            result.file_info.path.display(),
                            class.symbol.name,
                            class.symbol.line_start,
                            class.symbol.line_end
                        ));
                    }
                }
                Ok(lines.join("\n"))
            }
            "summary" => {
                // Summary totals across the session
                let file_count = session.info.analysis_results.len();
                let func_total: usize = session.info.analysis_results.iter().map(|r| r.functions.len()).sum();
                let class_total: usize = session.info.analysis_results.iter().map(|r| r.classes.len()).sum();
                Ok(format!(
                    "AST Summary for session {}\nFiles: {}\nFunctions: {}\nClasses: {}",
                    session_id, file_count, func_total, class_total
                ))
            }
            _ => {
                Ok(format!("Unknown format: {}. Supported: json, tree, flat, summary", format))
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
        session.info.update_stats();  // Update file_count and other statistics
        session.save()?;
        
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
        session.info.update_stats();  // Update file_count and other statistics
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
