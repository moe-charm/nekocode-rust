//! Session management for NekoCode Rust
//! 
//! This module handles analysis sessions, file discovery, and orchestrates
//! the analysis process across different file types and analyzers.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::core::types::{
    AnalysisConfig, AnalysisResult, DirectoryAnalysis, FileInfo, Language,
};
use crate::core::ast::{ASTNode, ASTStatistics};
use crate::analyzers::javascript::{JavaScriptAnalyzer, TreeSitterJavaScriptAnalyzer};
use crate::analyzers::traits::LanguageAnalyzer;

/// Session storage for managing multiple analysis sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    
    // 🌳 AST data stored in session
    pub analysis_results: Vec<AnalysisResult>,
    pub combined_ast_stats: Option<ASTStatistics>,
}

/// Session directory management
const SESSION_DIR: &str = ".nekocode_sessions";

/// Global session manager with file-based persistence
pub struct SessionManager {
    sessions: HashMap<String, AnalysisSession>,
    session_info: HashMap<String, SessionInfo>,
    session_dir: PathBuf,
}

impl SessionManager {
    /// Create a new session manager with file-based persistence
    pub fn new() -> Result<Self> {
        let session_dir = std::env::current_dir()?.join(SESSION_DIR);
        
        // Create session directory if it doesn't exist
        if !session_dir.exists() {
            fs::create_dir_all(&session_dir)
                .with_context(|| format!("Failed to create session directory: {}", session_dir.display()))?;
        }
        
        let mut manager = Self {
            sessions: HashMap::new(),
            session_info: HashMap::new(),
            session_dir,
        };
        
        // Load existing sessions from disk
        manager.load_sessions_from_disk()?;
        
        Ok(manager)
    }
    
    /// Load all sessions from disk
    fn load_sessions_from_disk(&mut self) -> Result<()> {
        if !self.session_dir.exists() {
            return Ok(());
        }
        
        for entry in fs::read_dir(&self.session_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(session_id) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(session_info) = self.load_session_info(session_id) {
                        // Create analysis session from stored data
                        let mut session = AnalysisSession::new();
                        
                        // Store session info
                        self.session_info.insert(session_id.to_string(), session_info);
                        self.sessions.insert(session_id.to_string(), session);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Load session info from disk
    fn load_session_info(&self, session_id: &str) -> Result<SessionInfo> {
        let session_file = self.session_dir.join(format!("{}.json", session_id));
        let content = fs::read_to_string(&session_file)
            .with_context(|| format!("Failed to read session file: {}", session_file.display()))?;
        
        let session_info: SessionInfo = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse session file: {}", session_file.display()))?;
        
        Ok(session_info)
    }
    
    /// Save session info to disk
    fn save_session_info(&self, session_info: &SessionInfo) -> Result<()> {
        let session_file = self.session_dir.join(format!("{}.json", session_info.id));
        let content = serde_json::to_string_pretty(session_info)
            .with_context(|| "Failed to serialize session info")?;
        
        fs::write(&session_file, content)
            .with_context(|| format!("Failed to write session file: {}", session_file.display()))?;
        
        Ok(())
    }
    
    pub async fn create_session(&mut self, path: &Path) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let mut session = AnalysisSession::new();
        
        // Initialize session with path analysis  
        let analysis_results = session.analyze_path(path, false).await?;
        
        // Extract analysis results from DirectoryAnalysis
        let files = analysis_results.files;
        
        // Calculate combined AST statistics
        let combined_ast_stats = Self::calculate_combined_ast_stats(&files);
        
        let session_info = SessionInfo {
            id: session_id.clone(),
            path: path.to_path_buf(),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            metadata: HashMap::new(),
            analysis_results: files,
            combined_ast_stats,
        };
        
        // Save to disk
        self.save_session_info(&session_info)?;
        
        self.sessions.insert(session_id.clone(), session);
        self.session_info.insert(session_id.clone(), session_info);
        
        Ok(session_id)
    }
    
    /// Calculate combined AST statistics from multiple files
    fn calculate_combined_ast_stats(analysis_results: &[AnalysisResult]) -> Option<ASTStatistics> {
        let mut combined = ASTStatistics::default();
        let mut has_ast_data = false;
        
        for result in analysis_results {
            if let Some(ref stats) = result.ast_statistics {
                has_ast_data = true;
                combined.total_nodes += stats.total_nodes;
                combined.max_depth = combined.max_depth.max(stats.max_depth);
                combined.classes += stats.classes;
                combined.functions += stats.functions;
                combined.methods += stats.methods;
                combined.variables += stats.variables;
                combined.control_structures += stats.control_structures;
                
                // Merge node type counts
                for (node_type, count) in &stats.node_type_counts {
                    *combined.node_type_counts.entry(node_type.clone()).or_insert(0) += count;
                }
            }
        }
        
        if has_ast_data {
            Some(combined)
        } else {
            None
        }
    }
    
    pub fn get_session(&mut self, session_id: &str) -> Option<&mut AnalysisSession> {
        // Update last accessed time first
        if self.session_info.contains_key(session_id) {
            if let Some(info) = self.session_info.get_mut(session_id) {
                info.last_accessed = Utc::now();
                // Clone the info to avoid borrowing issues
                let info_clone = info.clone();
                let _ = self.save_session_info(&info_clone); // Ignore save errors for now
            }
        }
        self.sessions.get_mut(session_id)
    }
    
    pub fn get_session_info(&self, session_id: &str) -> Option<&SessionInfo> {
        self.session_info.get(session_id)
    }
    
    pub fn list_sessions(&self) -> Vec<&SessionInfo> {
        self.session_info.values().collect()
    }
    
    // 🌳 AST Revolution Command Implementations
    
    /// Get AST statistics for a session
    pub fn handle_ast_stats(&mut self, session_id: &str) -> Result<String> {
        let session_info = self.get_session_info(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            
        if let Some(ref stats) = session_info.combined_ast_stats {
            let result = serde_json::json!({
                "ast_statistics": {
                    "classes": stats.classes,
                    "functions": stats.functions,
                    "methods": stats.methods,
                    "variables": stats.variables,
                    "control_structures": stats.control_structures,
                    "total_nodes": stats.total_nodes,
                    "max_depth": stats.max_depth,
                    "node_type_counts": stats.node_type_counts
                }
            });
            Ok(serde_json::to_string_pretty(&result)?)
        } else {
            Ok(serde_json::json!({
                "ast_statistics": {
                    "classes": 0,
                    "functions": 0,
                    "methods": 0,
                    "variables": 0,
                    "control_structures": 0,
                    "total_nodes": 0,
                    "max_depth": 0,
                    "node_type_counts": {}
                }
            }).to_string())
        }
    }
    
    /// Query AST by path
    pub fn handle_ast_query(&self, session_id: &str, path: &str) -> Result<String> {
        let session_info = self.get_session_info(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            
        let mut results = Vec::new();
        
        // Search through all analysis results
        for analysis_result in &session_info.analysis_results {
            if let Some(ref ast_root) = analysis_result.ast_root {
                let matches = ast_root.query_by_path(path);
                for node in matches {
                    results.push(serde_json::json!({
                        "file": analysis_result.file_info.path,
                        "node_type": node.type_string(),
                        "name": node.name,
                        "scope_path": node.scope_path,
                        "start_line": node.start_line,
                        "end_line": node.end_line,
                        "attributes": node.attributes
                    }));
                }
            }
        }
        
        let result = serde_json::json!({
            "query_path": path,
            "matches": results
        });
        
        Ok(serde_json::to_string_pretty(&result)?)
    }
    
    /// Analyze scope at specific line
    pub fn handle_scope_analysis(&self, session_id: &str, line: u32) -> Result<String> {
        let session_info = self.get_session_info(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            
        let mut results = Vec::new();
        
        // Search through all analysis results for nodes at the specified line
        for analysis_result in &session_info.analysis_results {
            if let Some(ref ast_root) = analysis_result.ast_root {
                if let Some(node) = ast_root.find_node_at_line(line) {
                    results.push(serde_json::json!({
                        "file": analysis_result.file_info.path,
                        "node_type": node.type_string(),
                        "name": node.name,
                        "scope_path": node.scope_path,
                        "start_line": node.start_line,
                        "end_line": node.end_line,
                        "depth": node.depth,
                        "attributes": node.attributes
                    }));
                }
            }
        }
        
        let result = serde_json::json!({
            "line_number": line,
            "scope_analysis": results
        });
        
        Ok(serde_json::to_string_pretty(&result)?)
    }
    
    /// Dump AST structure
    pub fn handle_ast_dump(&self, session_id: &str, format: &str) -> Result<String> {
        let session_info = self.get_session_info(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            
        let mut output = String::new();
        
        for analysis_result in &session_info.analysis_results {
            if let Some(ref ast_root) = analysis_result.ast_root {
                output.push_str(&format!("=== {} ===\n", analysis_result.file_info.path.display()));
                
                match format {
                    "tree" => {
                        output.push_str(&ast_root.dump_as_tree(0));
                    }
                    "json" => {
                        let json = serde_json::to_string_pretty(ast_root)?;
                        output.push_str(&json);
                        output.push('\n');
                    }
                    "flat" => {
                        output.push_str(&ast_root.dump_as_flat());
                        output.push('\n');
                    }
                    _ => {
                        anyhow::bail!("Unsupported format: {}. Use 'tree', 'json', or 'flat'", format);
                    }
                }
                output.push('\n');
            }
        }
        
        Ok(output)
    }
    
    /// Calculate real session statistics from stored analysis data
    fn calculate_session_stats(&self, session_info: &SessionInfo) -> Result<serde_json::Value> {
        let mut total_files = 0;
        let mut total_lines = 0;
        let mut total_code_lines = 0;
        let mut total_functions = 0;
        let mut total_classes = 0;
        let mut language_counts = std::collections::HashMap::new();
        
        for result in &session_info.analysis_results {
            total_files += 1;
            total_lines += result.file_info.total_lines;
            total_code_lines += result.file_info.code_lines;
            total_functions += result.functions.len();
            total_classes += result.classes.len();
            
            let lang_str = format!("{:?}", result.language);
            *language_counts.entry(lang_str).or_insert(0) += 1;
        }
        
        let mut language_breakdown = Vec::new();
        for (lang, count) in language_counts {
            language_breakdown.push(serde_json::json!({
                "language": lang,
                "file_count": count
            }));
        }
        
        Ok(serde_json::json!({
            "session_id": session_info.id,
            "created_at": session_info.created_at,
            "last_accessed": session_info.last_accessed,
            "project_path": session_info.path,
            "file_statistics": {
                "total_files": total_files,
                "total_lines": total_lines,
                "total_code_lines": total_code_lines,
                "code_ratio": if total_lines > 0 { total_code_lines as f64 / total_lines as f64 } else { 0.0 }
            },
            "code_statistics": {
                "total_functions": total_functions,
                "total_classes": total_classes
            },
            "language_breakdown": language_breakdown,
            "ast_statistics": session_info.combined_ast_stats
        }))
    }
    
    /// Calculate complexity analysis from AST data
    fn calculate_session_complexity(&self, session_info: &SessionInfo) -> Result<serde_json::Value> {
        let mut complexity_by_file = Vec::new();
        let mut total_complexity = 0;
        let mut complexity_distribution = std::collections::HashMap::new();
        
        for result in &session_info.analysis_results {
            let mut file_complexity = 0;
            let mut function_complexities = Vec::new();
            
            // Calculate complexity for each function
            for function in &result.functions {
                // Simple complexity based on control structures + 1
                let mut complexity = 1;
                
                if let Some(ref ast_stats) = result.ast_statistics {
                    // Rough estimate: control structures contribute to complexity
                    complexity += ast_stats.control_structures / result.functions.len().max(1) as u32;
                }
                
                function_complexities.push(serde_json::json!({
                    "name": function.name,
                    "complexity": complexity,
                    "line_start": function.start_line,
                    "line_end": function.end_line
                }));
                
                file_complexity += complexity;
                total_complexity += complexity;
                
                // Track complexity distribution
                let complexity_range = match complexity {
                    1..=5 => "low",
                    6..=10 => "medium", 
                    11..=20 => "high",
                    _ => "very_high"
                };
                *complexity_distribution.entry(complexity_range).or_insert(0) += 1;
            }
            
            if !function_complexities.is_empty() {
                complexity_by_file.push(serde_json::json!({
                    "file": result.file_info.path,
                    "language": format!("{:?}", result.language),
                    "total_complexity": file_complexity,
                    "function_count": result.functions.len(),
                    "average_complexity": if !result.functions.is_empty() { 
                        file_complexity as f64 / result.functions.len() as f64 
                    } else { 0.0 },
                    "functions": function_complexities
                }));
            }
        }
        
        Ok(serde_json::json!({
            "session_id": session_info.id,
            "total_complexity": total_complexity,
            "total_functions": session_info.analysis_results.iter().map(|r| r.functions.len()).sum::<usize>(),
            "average_complexity": if !session_info.analysis_results.is_empty() {
                total_complexity as f64 / session_info.analysis_results.iter().map(|r| r.functions.len()).sum::<usize>() as f64
            } else { 0.0 },
            "complexity_distribution": complexity_distribution,
            "files": complexity_by_file
        }))
    }
    
    /// Calculate project structure analysis
    fn calculate_session_structure(&self, session_info: &SessionInfo) -> Result<serde_json::Value> {
        let mut structure_by_language = std::collections::HashMap::new();
        let mut directory_structure = std::collections::HashMap::new();
        
        for result in &session_info.analysis_results {
            let lang_str = format!("{:?}", result.language);
            let lang_entry = structure_by_language.entry(lang_str).or_insert_with(|| serde_json::json!({
                "files": [],
                "total_classes": 0,
                "total_functions": 0,
                "total_lines": 0
            }));
            
            // Update language totals
            lang_entry["total_classes"] = serde_json::Value::from(lang_entry["total_classes"].as_u64().unwrap_or(0) + result.classes.len() as u64);
            lang_entry["total_functions"] = serde_json::Value::from(lang_entry["total_functions"].as_u64().unwrap_or(0) + result.functions.len() as u64);
            lang_entry["total_lines"] = serde_json::Value::from(lang_entry["total_lines"].as_u64().unwrap_or(0) + result.file_info.total_lines as u64);
            
            // Add file info
            lang_entry["files"].as_array_mut().unwrap().push(serde_json::json!({
                "path": result.file_info.path,
                "classes": result.classes.len(),
                "functions": result.functions.len(),
                "lines": result.file_info.total_lines
            }));
            
            // Track directory structure
            if let Some(parent) = result.file_info.path.parent() {
                let dir_str = parent.to_string_lossy().to_string();
                let dir_entry = directory_structure.entry(dir_str).or_insert_with(|| serde_json::json!({
                    "files": 0,
                    "total_lines": 0,
                    "languages": std::collections::HashMap::<String, u32>::new()
                }));
                
                dir_entry["files"] = serde_json::Value::from(dir_entry["files"].as_u64().unwrap_or(0) + 1);
                dir_entry["total_lines"] = serde_json::Value::from(dir_entry["total_lines"].as_u64().unwrap_or(0) + result.file_info.total_lines as u64);
            }
        }
        
        Ok(serde_json::json!({
            "session_id": session_info.id,
            "project_path": session_info.path,
            "languages": structure_by_language,
            "directories": directory_structure,
            "summary": {
                "total_files": session_info.analysis_results.len(),
                "total_languages": structure_by_language.len(),
                "total_directories": directory_structure.len()
            }
        }))
    }
    
    /// Find symbols matching the search term
    fn find_session_symbols(&self, session_info: &SessionInfo, term: &str) -> Result<serde_json::Value> {
        let mut matches = Vec::new();
        let term_lower = term.to_lowercase();
        
        for result in &session_info.analysis_results {
            // Search in classes
            for class in &result.classes {
                if class.name.to_lowercase().contains(&term_lower) {
                    matches.push(serde_json::json!({
                        "type": "class",
                        "name": class.name,
                        "file": result.file_info.path,
                        "line_start": class.start_line,
                        "line_end": class.end_line,
                        "scope": "global"
                    }));
                }
            }
            
            // Search in functions
            for function in &result.functions {
                if function.name.to_lowercase().contains(&term_lower) {
                    matches.push(serde_json::json!({
                        "type": "function",
                        "name": function.name,
                        "file": result.file_info.path,
                        "line_start": function.start_line,
                        "line_end": function.end_line,
                        "scope": "global",
                        "parameters": function.parameters
                    }));
                }
            }
            
            // Search in AST nodes if available
            if let Some(ref ast_root) = result.ast_root {
                let ast_matches = self.search_ast_nodes(ast_root, &term_lower);
                for ast_match in ast_matches {
                    let name = if ast_match.name.is_empty() { "anonymous" } else { &ast_match.name };
                    matches.push(serde_json::json!({
                        "type": "ast_node",
                        "name": name,
                        "node_type": ast_match.type_string(),
                        "file": result.file_info.path,
                        "line_start": ast_match.start_line,
                        "line_end": ast_match.end_line,
                        "scope_path": ast_match.scope_path
                    }));
                }
            }
        }
        
        Ok(serde_json::json!({
            "session_id": session_info.id,
            "search_term": term,
            "total_matches": matches.len(),
            "matches": matches
        }))
    }
    
    /// Helper method to search AST nodes recursively
    fn search_ast_nodes<'a>(&self, node: &'a ASTNode, term: &str) -> Vec<&'a ASTNode> {
        let mut results = Vec::new();
        
        // Check current node
        if !node.name.is_empty() && node.name.to_lowercase().contains(term) {
            results.push(node);
        }
        
        // Search children recursively
        for child in &node.children {
            results.extend(self.search_ast_nodes(child, term));
        }
        
        results
    }
    
    /// Find circular dependencies for all supported languages
    fn find_session_include_cycles(&self, session_info: &SessionInfo) -> Result<serde_json::Value> {
        let mut dependencies = std::collections::HashMap::new();
        let mut cycles = Vec::new();
        
        // Build dependency graph for all languages
        for result in &session_info.analysis_results {
            let file_path = result.file_info.path.to_string_lossy().to_string();
            let mut resolved_deps = Vec::new();
            
            // Extract imports for all languages
            for import in &result.imports {
                if let Some(resolved_path) = self.resolve_import_path(&import.module_path, &result.file_info.path, &result.language) {
                    resolved_deps.push(resolved_path);
                }
            }
            
            dependencies.insert(file_path, resolved_deps);
        }
        
        // Simple cycle detection using DFS
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();
        
        for file in dependencies.keys() {
            if !visited.contains(file) {
                if let Some(cycle) = self.detect_cycle_dfs(file, &dependencies, &mut visited, &mut rec_stack) {
                    cycles.push(cycle);
                }
            }
        }
        
        Ok(serde_json::json!({
            "session_id": session_info.id,
            "total_files_analyzed": dependencies.len(),
            "cycles_found": cycles.len(),
            "dependency_graph": dependencies,
            "cycles": cycles
        }))
    }
    
    /// Helper method for cycle detection using DFS
    fn detect_cycle_dfs(
        &self,
        file: &str,
        dependencies: &std::collections::HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>
    ) -> Option<Vec<String>> {
        visited.insert(file.to_string());
        rec_stack.insert(file.to_string());
        
        if let Some(deps) = dependencies.get(file) {
            for dep in deps {
                if !visited.contains(dep) {
                    if let Some(cycle) = self.detect_cycle_dfs(dep, dependencies, visited, rec_stack) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    // Found a cycle
                    return Some(vec![file.to_string(), dep.clone()]);
                }
            }
        }
        
        rec_stack.remove(file);
        None
    }
    
    /// Resolve import path based on language-specific rules
    fn resolve_import_path(&self, import_path: &str, current_file: &std::path::Path, language: &crate::core::types::Language) -> Option<String> {
        use crate::core::types::Language;
        
        // Clean the import path first (remove extra whitespace and newlines)
        let clean_import_path = import_path.trim();
        
        match language {
            // JavaScript/TypeScript - handle relative and module paths
            Language::JavaScript | Language::TypeScript => {
                if clean_import_path.starts_with("./") || clean_import_path.starts_with("../") {
                    // Relative import - resolve relative to current file
                    if let Some(parent) = current_file.parent() {
                        let resolved = parent.join(clean_import_path);
                        // Canonicalize the path to handle ./ and ../
                        if let Ok(canonical) = resolved.canonicalize() {
                            return Some(canonical.to_string_lossy().to_string());
                        } else {
                            // Try different extensions
                            for ext in &[".js", ".ts", ".jsx", ".tsx", ".mjs"] {
                                let with_ext = resolved.with_extension(&ext[1..]);
                                if let Ok(canonical) = with_ext.canonicalize() {
                                    return Some(canonical.to_string_lossy().to_string());
                                }
                            }
                            // Try as directory with index file
                            for ext in &[".js", ".ts"] {
                                let index_file = resolved.join(format!("index{}", ext));
                                if let Ok(canonical) = index_file.canonicalize() {
                                    return Some(canonical.to_string_lossy().to_string());
                                }
                            }
                            // Return the resolved path even if file doesn't exist (for analysis)
                            return Some(resolved.to_string_lossy().to_string());
                        }
                    }
                } else if !clean_import_path.starts_with("@") && !clean_import_path.contains("/") {
                    // Might be a local module, skip node_modules for now
                    return None;
                }
                None
            }
            
            // Python - handle relative and absolute imports
            Language::Python => {
                if clean_import_path.starts_with('.') {
                    // Relative import
                    if let Some(parent) = current_file.parent() {
                        let module_path = clean_import_path.trim_start_matches('.');
                        let py_path = parent.join(module_path.replace('.', "/") + ".py");
                        return Some(py_path.to_string_lossy().to_string());
                    }
                } else {
                    // Absolute import - convert module.submodule to path
                    if let Some(parent) = current_file.parent() {
                        // Simple heuristic: look for module in same directory structure
                        let py_path = parent.join(clean_import_path.replace('.', "/") + ".py");
                        if py_path.exists() {
                            return Some(py_path.to_string_lossy().to_string());
                        }
                    }
                }
                None
            }
            
            // C/C++ - handle include paths
            Language::C | Language::Cpp => {
                if clean_import_path.starts_with('"') {
                    // Local include with quotes
                    let clean_path = clean_import_path.trim_matches('"');
                    if let Some(parent) = current_file.parent() {
                        let include_path = parent.join(clean_path);
                        return Some(include_path.to_string_lossy().to_string());
                    }
                } else if clean_import_path.starts_with('<') {
                    // System include, skip for cycle detection
                    return None;
                } else {
                    // Direct path
                    if let Some(parent) = current_file.parent() {
                        let include_path = parent.join(clean_import_path);
                        return Some(include_path.to_string_lossy().to_string());
                    }
                }
                None
            }
            
            // C# - handle using statements
            Language::CSharp => {
                // For now, only handle relative file imports (not namespace imports)
                if clean_import_path.contains('.') && !clean_import_path.starts_with("System") {
                    if let Some(parent) = current_file.parent() {
                        let cs_path = parent.join(clean_import_path.replace('.', "/") + ".cs");
                        if cs_path.exists() {
                            return Some(cs_path.to_string_lossy().to_string());
                        }
                    }
                }
                None
            }
            
            // Go - handle import paths
            Language::Go => {
                // Handle relative imports starting with ./
                if clean_import_path.starts_with("./") || clean_import_path.starts_with("../") {
                    if let Some(parent) = current_file.parent() {
                        let go_path = parent.join(clean_import_path);
                        return Some(go_path.to_string_lossy().to_string());
                    }
                }
                None
            }
            
            // Rust - handle use statements
            Language::Rust => {
                // Handle relative crate imports
                if clean_import_path.starts_with("crate::") || clean_import_path.starts_with("super::") {
                    if let Some(parent) = current_file.parent() {
                        let module_path = clean_import_path.replace("::", "/").replace("crate/", "").replace("super/", "../");
                        let rs_path = parent.join(module_path + ".rs");
                        if rs_path.exists() {
                            return Some(rs_path.to_string_lossy().to_string());
                        }
                    }
                }
                None
            }
            
            _ => None,
        }
    }
    
    pub fn execute_session_command(&mut self, session_id: &str, command: &str, args: &[String]) -> Result<String> {
        // Get session info which contains the actual analysis data
        let session_info = self.get_session_info(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        
        match command {
            "stats" => {
                let stats = self.calculate_session_stats(session_info)?;
                Ok(serde_json::to_string_pretty(&stats)?)
            }
            "complexity" => {
                let complexity = self.calculate_session_complexity(session_info)?;
                Ok(serde_json::to_string_pretty(&complexity)?)
            }
            "structure" => {
                let structure = self.calculate_session_structure(session_info)?;
                Ok(serde_json::to_string_pretty(&structure)?)
            }
            "find" => {
                let term = args.get(0).unwrap_or(&String::new()).clone();
                let results = self.find_session_symbols(session_info, &term)?;
                Ok(serde_json::to_string_pretty(&results)?)
            }
            "include-cycles" => {
                let cycles = self.find_session_include_cycles(session_info)?;
                Ok(serde_json::to_string_pretty(&cycles)?)
            }
            _ => anyhow::bail!("Unknown session command: {}", command),
        }
    }
}

/// Main analysis session coordinator
pub struct AnalysisSession {
    config: AnalysisConfig,
}

impl AnalysisSession {
    pub fn new() -> Self {
        Self {
            config: AnalysisConfig::default(),
        }
    }
    
    pub fn with_config(config: AnalysisConfig) -> Self {
        Self { config }
    }
    
    /// Analyze a single file or directory
    pub async fn analyze_path(&mut self, path: &Path, include_tests: bool) -> Result<DirectoryAnalysis> {
        self.config.include_test_files = include_tests;
        
        if path.is_file() {
            self.analyze_single_file(path).await
        } else if path.is_dir() {
            self.analyze_directory(path).await
        } else {
            anyhow::bail!("Path does not exist or is not accessible: {}", path.display());
        }
    }
    
    /// Analyze a single file
    async fn analyze_single_file(&self, file_path: &Path) -> Result<DirectoryAnalysis> {
        let mut directory_analysis = DirectoryAnalysis::new(
            file_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
        );
        
        let result = self.analyze_file(file_path).await
            .with_context(|| format!("Failed to analyze file: {}", file_path.display()))?;
        
        directory_analysis.files.push(result);
        directory_analysis.update_summary();
        
        Ok(directory_analysis)
    }
    
    /// Analyze a directory
    async fn analyze_directory(&self, dir_path: &Path) -> Result<DirectoryAnalysis> {
        println!("🔍 [RUST] Starting directory analysis: {}", dir_path.display());
        let start_total = std::time::Instant::now();
        
        let mut directory_analysis = DirectoryAnalysis::new(dir_path.to_path_buf());
        
        // Discover files
        let start_scan = std::time::Instant::now();
        let files = self.discover_files(dir_path)?;
        let scan_duration = start_scan.elapsed();
        println!("📁 [RUST] File discovery took: {:.3}s, found {} files", scan_duration.as_secs_f64(), files.len());
        
        if self.config.verbose_output {
            println!("📁 Found {} files to analyze", files.len());
            for file in &files {
                println!("  - {}", file.display());
            }
        }
        
        // Analyze files in parallel
        let start_analysis = std::time::Instant::now();
        println!("⚡ [RUST] Starting {} analysis (parallel={})", 
                 if self.config.enable_parallel_processing { "PARALLEL" } else { "SEQUENTIAL" },
                 self.config.enable_parallel_processing);
        
        let results: Result<Vec<_>> = if self.config.enable_parallel_processing {
            // 🚀 Use spawn_blocking with chunk processing for better parallelization
            let total_files = files.len();
            println!("🔧 [RUST] Creating {} spawn_blocking tasks for parallel processing", total_files);
            
            let futures: Vec<_> = files.into_iter().enumerate().map(|(i, file_path)| {
                let config = self.config.clone();
                tokio::task::spawn_blocking(move || {
                    if i % 100 == 0 || i == total_files - 1 {
                        println!("🔄 [RUST] Processing file {}/{} on thread {:?}: {}", 
                                i + 1, total_files, 
                                std::thread::current().id(), 
                                file_path.display());
                    }
                    // Create a temporary session for this task
                    let temp_session = AnalysisSession::with_config(config);
                    // Use the sync version of the runtime
                    tokio::runtime::Handle::current().block_on(async {
                        temp_session.analyze_file(&file_path).await
                    })
                })
            }).collect();
            
            println!("🚀 [RUST] Spawned {} blocking tasks, waiting for completion...", futures.len());
            
            // Process futures concurrently
            let results = futures::future::join_all(futures).await;
            results.into_iter().collect::<Result<Vec<_>, _>>().map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
                .into_iter().collect()
        } else {
            let mut results = Vec::new();
            for file_path in &files {
                results.push(self.analyze_file(file_path).await);
            }
            results.into_iter().collect()
        };
        
        directory_analysis.files = results?;
        let analysis_duration = start_analysis.elapsed();
        println!("🔄 [RUST] File analysis took: {:.3}s ({} files)", analysis_duration.as_secs_f64(), directory_analysis.files.len());
        
        let start_summary = std::time::Instant::now();
        directory_analysis.update_summary();
        let summary_duration = start_summary.elapsed();
        println!("📊 [RUST] Summary generation took: {:.3}s", summary_duration.as_secs_f64());
        
        let total_duration = start_total.elapsed();
        println!("🏁 [RUST] Total directory analysis took: {:.3}s", total_duration.as_secs_f64());
        
        if self.config.verbose_output {
            println!("✅ Analyzed {} files successfully", directory_analysis.files.len());
        }
        
        Ok(directory_analysis)
    }
    
    /// Discover files in a directory based on configuration
    fn discover_files(&self, dir_path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for entry in WalkDir::new(dir_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Skip directories
            if !path.is_file() {
                continue;
            }
            
            // Check if path should be excluded
            if self.should_exclude_path(path) {
                continue;
            }
            
            // Check if file extension is supported
            if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                let ext_with_dot = format!(".{}", extension);
                
                if self.config.included_extensions.contains(&ext_with_dot) {
                    // Skip test files if not requested
                    if !self.config.include_test_files && self.is_test_file(path) {
                        continue;
                    }
                    
                    files.push(path.to_path_buf());
                }
            }
        }
        
        Ok(files)
    }
    
    /// Check if a path should be excluded based on patterns
    fn should_exclude_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        for pattern in &self.config.excluded_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }
        
        false
    }
    
    /// Check if a file is a test file
    fn is_test_file(&self, path: &Path) -> bool {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        let path_str = path.to_string_lossy().to_lowercase();
        
        // Check for specific test file patterns
        file_name.contains("test") ||
        file_name.contains("spec") ||
        path_str.contains("__tests__") ||
        path_str.ends_with(".test.js") ||
        path_str.ends_with(".test.ts") ||
        path_str.ends_with(".spec.js") ||
        path_str.ends_with(".spec.ts") ||
        path_str.contains("/test/") ||
        path_str.contains("/tests/") ||
        path_str.contains("/spec/") ||
        path_str.contains("/specs/")
    }
    
    /// Analyze a specific file
    async fn analyze_file(&self, file_path: &Path) -> Result<AnalysisResult> {
        // Read file content
        let content = tokio::fs::read_to_string(file_path).await
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
        
        // Create file info
        let metadata = tokio::fs::metadata(file_path).await
            .with_context(|| format!("Failed to get metadata for: {}", file_path.display()))?;
        
        let mut file_info = FileInfo::new(file_path.to_path_buf());
        file_info.size_bytes = metadata.len();
        file_info.total_lines = content.lines().count() as u32;
        
        // Calculate basic line statistics
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                file_info.empty_lines += 1;
            } else if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                file_info.comment_lines += 1;
            } else {
                file_info.code_lines += 1;
            }
        }
        
        file_info.code_ratio = if file_info.total_lines > 0 {
            file_info.code_lines as f64 / file_info.total_lines as f64
        } else {
            0.0
        };
        
        // Determine language
        let language = if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
            Language::from_extension(&format!(".{}", extension))
        } else {
            Language::Unknown
        };
        
        // Create base analysis result
        let mut result = AnalysisResult::new(file_info, language);
        
        // Perform language-specific analysis
        match language {
            Language::JavaScript | Language::TypeScript => {
                // 🚀 Always use Tree-sitter (fastest parser)
                let mut analyzer = TreeSitterJavaScriptAnalyzer::new()
                    .map_err(|e| anyhow::anyhow!("Failed to create tree-sitter analyzer: {}", e))?;
                result = analyzer.analyze(&content, file_path.to_string_lossy().as_ref()).await?;
                result.language = language; // Ensure correct language is set
            }
            Language::Python => {
                use crate::analyzers::python::PythonAnalyzer;
                let mut analyzer = PythonAnalyzer::new();
                result = analyzer.analyze(&content, file_path.to_string_lossy().as_ref()).await?;
                result.language = language; // Ensure correct language is set
            }
            Language::Cpp => {
                use crate::analyzers::cpp::CppAnalyzer;
                let mut analyzer = CppAnalyzer::new();
                result = analyzer.analyze(&content, file_path.to_string_lossy().as_ref()).await?;
                result.language = language; // Ensure correct language is set
            }
            Language::CSharp => {
                use crate::analyzers::csharp::CSharpAnalyzer;
                let mut analyzer = CSharpAnalyzer::new();
                result = analyzer.analyze(&content, file_path.to_string_lossy().as_ref()).await?;
                result.language = language; // Ensure correct language is set
            }
            Language::Go => {
                use crate::analyzers::go::GoAnalyzer;
                let mut analyzer = GoAnalyzer::new();
                result = analyzer.analyze(&content, file_path.to_string_lossy().as_ref()).await?;
                result.language = language; // Ensure correct language is set
            }
            Language::Rust => {
                use crate::analyzers::rust::RustAnalyzer;
                let mut analyzer = RustAnalyzer::new();
                result = analyzer.analyze(&content, file_path.to_string_lossy().as_ref()).await?;
                result.language = language; // Ensure correct language is set
            }
            Language::Unknown => {
                if self.config.verbose_output {
                    println!("⚠️  Skipping unknown file type: {}", file_path.display());
                }
            }
            _ => {
                if self.config.verbose_output {
                    println!("⚠️  Language not yet implemented: {:?} for {}", language, file_path.display());
                }
            }
        }
        
        // Update statistics
        result.update_statistics();
        
        Ok(result)
    }
    
    // Session-specific methods for new functionality
    pub fn get_stats(&self) -> String {
        "Session statistics placeholder".to_string()
    }
    
    pub fn get_complexity(&self) -> String {
        "Session complexity analysis placeholder".to_string()
    }
    
    pub fn get_structure(&self) -> String {
        "Session structure analysis placeholder".to_string()
    }
    
    pub fn find_symbols(&self, term: &str) -> String {
        format!("Symbol search results for '{}' placeholder", term)
    }
    
    pub fn find_include_cycles(&self) -> String {
        "Include cycle analysis placeholder".to_string()
    }
}

impl Default for AnalysisSession {
    fn default() -> Self {
        Self::new()
    }
}