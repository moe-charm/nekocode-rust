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
    
    pub fn execute_session_command(&mut self, session_id: &str, command: &str, args: &[String]) -> Result<String> {
        match command {
            "stats" => {
                let session = self.get_session(session_id)
                    .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
                Ok(format!("Session {} statistics:\n{:?}", session_id, session.get_stats()))
            }
            "complexity" => {
                let session = self.get_session(session_id)
                    .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
                Ok(format!("Session {} complexity analysis:\n{:?}", session_id, session.get_complexity()))
            }
            "structure" => {
                let session = self.get_session(session_id)
                    .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
                Ok(format!("Session {} structure:\n{:?}", session_id, session.get_structure()))
            }
            "find" => {
                let term = args.get(0).unwrap_or(&String::new()).clone();
                let session = self.get_session(session_id)
                    .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
                Ok(format!("Session {} find results for '{}':\n{:?}", session_id, term, session.find_symbols(&term)))
            }
            "include-cycles" => {
                let session = self.get_session(session_id)
                    .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
                Ok(format!("Session {} include cycles:\n{:?}", session_id, session.find_include_cycles()))
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