use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use std::collections::HashSet;
use nekocode_core::{Result, NekocodeError, SessionManager, AnalysisResult};
use crate::language_detection::LanguageDetector;

/// File splitting engine
pub struct FileSplitter {
    session_manager: SessionManager,
    language_detector: LanguageDetector,
}

#[derive(Debug, Clone)]
pub struct SplitResult {
    pub original_file: PathBuf,
    pub split_files: Vec<SplitFileInfo>,
    pub total_functions: usize,
    pub total_lines: usize,
}

#[derive(Debug, Clone)]
pub struct SplitFileInfo {
    pub file_path: PathBuf,
    pub function_name: String,
    pub lines: u32,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone)]
pub enum SplitBy {
    Functions,
    Classes,
    Size(usize),
}

impl std::str::FromStr for SplitBy {
    type Err = String;
    
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "functions" => Ok(SplitBy::Functions),
            "classes" => Ok(SplitBy::Classes),
            s if s.starts_with("size:") => {
                let size = s[5..].parse::<usize>()
                    .map_err(|_| format!("Invalid size: {}", s))?;
                Ok(SplitBy::Size(size))
            }
            _ => Err(format!("Invalid split type: {}. Use 'functions', 'classes', or 'size:N'", s))
        }
    }
}

impl FileSplitter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            session_manager: SessionManager::new()?,
            language_detector: LanguageDetector::new(),
        })
    }
    
    /// Split file by functions
    pub async fn split_file(
        &mut self,
        file_path: &Path,
        split_by: SplitBy,
        output_dir: Option<&Path>,
        verbose: bool,
    ) -> Result<SplitResult> {
        if !file_path.exists() {
            return Err(NekocodeError::FileNotFound(file_path.to_string_lossy().to_string()));
        }
        
        if verbose {
            println!("🔍 Analyzing file: {}", file_path.display());
        }
        
        // Create temporary session for analysis
        let session_id = self.session_manager.create_session(file_path.to_path_buf())?;
        
        // Run analysis using nekocode binary
        if verbose {
            println!("📊 Running analysis with nekocode...");
        }
        
        // Call nekocode analyze command
        let output = Command::new("./target/debug/nekocode")
            .arg("analyze")
            .arg(file_path)
            .arg("--output")
            .arg("json")
            .output()
            .map_err(|e| NekocodeError::AnalysisError(format!("Failed to run nekocode: {}", e)))?;
        
        if !output.status.success() {
            return Err(NekocodeError::AnalysisError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        // Parse the JSON output
        let json_str = String::from_utf8_lossy(&output.stdout);
        let analysis_result: AnalysisResult = serde_json::from_str(&json_str)
            .map_err(|e| NekocodeError::AnalysisError(format!("Failed to parse analysis result: {}", e)))?;
        
        // Store analysis result in session
        {
            let session = self.session_manager.get_session_mut(&session_id)?;
            session.info.analysis_results.push(analysis_result.clone());
            session.save()?;
        }
        
        let output_directory = output_dir.unwrap_or_else(|| file_path.parent().unwrap()).to_path_buf();
        
        match split_by {
            SplitBy::Functions => self.split_by_functions(file_path, &analysis_result, &output_directory, verbose).await,
            SplitBy::Classes => self.split_by_classes(file_path, &analysis_result, &output_directory, verbose).await,
            SplitBy::Size(size) => self.split_by_size(file_path, size, &output_directory, verbose).await,
        }
    }
    
    /// Split file by functions (each function gets its own file)
    async fn split_by_functions(
        &self,
        file_path: &Path,
        analysis_result: &nekocode_core::AnalysisResult,
        output_dir: &Path,
        verbose: bool,
    ) -> Result<SplitResult> {
        let file_content = fs::read_to_string(file_path)?;
        let lines: Vec<&str> = file_content.lines().collect();
        
        // Create output directory
        fs::create_dir_all(output_dir)?;
        
        let mut split_files = Vec::new();
        let file_stem = file_path.file_stem().unwrap().to_string_lossy();
        let extension = file_path.extension().unwrap_or_default().to_string_lossy();
        
        if verbose {
            println!("📊 Found {} functions to split", analysis_result.functions.len());
        }
        
        // Process each function
        for (i, func) in analysis_result.functions.iter().enumerate() {
            let start_line = func.symbol.line_start as usize;
            let end_line = func.symbol.line_end as usize;
            
            if start_line == 0 || end_line == 0 || start_line > lines.len() || end_line > lines.len() {
                if verbose {
                    println!("⚠️ Skipping function {} (invalid line range: {}-{})", 
                            func.symbol.name, start_line, end_line);
                }
                continue;
            }
            
            // Extract function content
            let function_lines = &lines[start_line-1..end_line];
            let function_content = function_lines.join("\n");
            
            // Generate output file name
            let output_file = output_dir.join(format!("{}_{:02}_{}.{}", 
                file_stem, i + 1, func.symbol.name, extension));
            
            // Add necessary imports/headers for the language
            let full_content = self.generate_file_header(file_path, &function_content)?;
            
            // Write the split file
            fs::write(&output_file, full_content)?;
            
            split_files.push(SplitFileInfo {
                file_path: output_file.clone(),
                function_name: func.symbol.name.clone(),
                lines: (end_line - start_line + 1) as u32,
                start_line: start_line as u32,
                end_line: end_line as u32,
            });
            
            if verbose {
                println!("✅ Created: {} (lines {}-{})", 
                        output_file.display(), start_line, end_line);
            }
        }
        
        Ok(SplitResult {
            original_file: file_path.to_path_buf(),
            split_files,
            total_functions: analysis_result.functions.len(),
            total_lines: lines.len(),
        })
    }
    
    /// Split file by classes (each class gets its own file)
    async fn split_by_classes(
        &self,
        file_path: &Path,
        analysis_result: &nekocode_core::AnalysisResult,
        output_dir: &Path,
        verbose: bool,
    ) -> Result<SplitResult> {
        let file_content = fs::read_to_string(file_path)?;
        let lines: Vec<&str> = file_content.lines().collect();
        
        // Create output directory
        fs::create_dir_all(output_dir)?;
        
        let mut split_files = Vec::new();
        let file_stem = file_path.file_stem().unwrap().to_string_lossy();
        let extension = file_path.extension().unwrap_or_default().to_string_lossy();
        
        if verbose {
            println!("📊 Found {} classes to split", analysis_result.classes.len());
        }
        
        // Track which functions have been included in class files
        let mut included_functions = HashSet::new();
        
        // Process each class
        for (i, class) in analysis_result.classes.iter().enumerate() {
            let start_line = class.symbol.line_start as usize;
            let end_line = class.symbol.line_end as usize;
            
            if start_line == 0 || end_line == 0 || start_line > lines.len() || end_line > lines.len() {
                if verbose {
                    println!("⚠️ Skipping class {} (invalid line range: {}-{})", 
                            class.symbol.name, start_line, end_line);
                }
                continue;
            }
            
            // Extract class content and its impl blocks
            let mut class_content = Vec::new();
            let mut processed_impl_blocks = std::collections::HashSet::new();
            
            // Add the class/struct definition
            class_content.extend_from_slice(&lines[start_line-1..end_line]);
            
            // Find all impl blocks for this class
            for func in &analysis_result.functions {
                // Check if function is within an impl block for this class
                if let Some(impl_line) = find_impl_block_for_class(&lines, &class.symbol.name, func.symbol.line_start as usize) {
                    // Mark this function as included
                    included_functions.insert(func.symbol.line_start);
                    
                    // Skip if we've already processed this impl block
                    if processed_impl_blocks.contains(&impl_line) {
                        continue;
                    }
                    
                    let impl_start = impl_line;
                    let impl_end = find_impl_block_end(&lines, impl_start);
                    
                    // Only add if not already included in the struct definition
                    if impl_start > end_line {
                        processed_impl_blocks.insert(impl_line);
                        class_content.push("");  // Add empty line separator
                        class_content.extend_from_slice(&lines[impl_start-1..impl_end]);
                    }
                }
            }
            
            let class_text = class_content.join("\n");
            
            // Generate output file name
            let output_file = output_dir.join(format!("{}_{:02}_{}.{}", 
                file_stem, i + 1, class.symbol.name, extension));
            
            // Add necessary imports/headers for the language
            let full_content = self.generate_class_file_header(file_path, &class_text)?;
            
            // Write the split file
            fs::write(&output_file, full_content)?;
            
            split_files.push(SplitFileInfo {
                file_path: output_file.clone(),
                function_name: class.symbol.name.clone(),
                lines: (end_line - start_line + 1) as u32,
                start_line: start_line as u32,
                end_line: end_line as u32,
            });
            
            if verbose {
                println!("✅ Created: {} (lines {}-{})", 
                        output_file.display(), start_line, end_line);
            }
        }
        
        // Add standalone functions to a separate file if any exist
        let standalone_functions: Vec<_> = analysis_result.functions.iter()
            .filter(|f| !included_functions.contains(&f.symbol.line_start))
            .collect();
            
        if !standalone_functions.is_empty() {
            let output_file = output_dir.join(format!("{}_standalone.{}", file_stem, extension));
            let mut standalone_content = Vec::new();
            
            for func in &standalone_functions {
                let start = func.symbol.line_start as usize;
                let end = func.symbol.line_end as usize;
                
                if start > 0 && end <= lines.len() {
                    standalone_content.extend_from_slice(&lines[start-1..end]);
                    standalone_content.push("");  // Add separator
                }
            }
            
            let standalone_text = standalone_content.join("\n");
            let full_content = self.generate_file_header(file_path, &standalone_text)?;
            
            fs::write(&output_file, full_content)?;
            
            split_files.push(SplitFileInfo {
                file_path: output_file.clone(),
                function_name: "standalone_functions".to_string(),
                lines: standalone_functions.len() as u32,
                start_line: 0,
                end_line: 0,
            });
            
            if verbose {
                println!("✅ Created: {} ({} standalone functions)", 
                        output_file.display(), standalone_functions.len());
            }
        }
        
        Ok(SplitResult {
            original_file: file_path.to_path_buf(),
            split_files,
            total_functions: analysis_result.functions.len(),
            total_lines: lines.len(),
        })
    }
    
    /// Split file by size (split when reaching specified line count)
    async fn split_by_size(
        &self,
        _file_path: &Path,
        _max_lines: usize,
        _output_dir: &Path,
        _verbose: bool,
    ) -> Result<SplitResult> {
        // TODO: Implement size-based splitting
        Err(NekocodeError::NotImplemented("Size-based splitting not yet implemented".to_string()))
    }
    
    /// Generate appropriate file header for classes
    fn generate_class_file_header(&self, original_file: &Path, class_content: &str) -> Result<String> {
        let language = self.language_detector.detect_language(original_file)?;
        
        match language.as_str() {
            "rust" => {
                // Add common Rust imports for classes
                Ok(format!(
                    "// Extracted from: {}\n// Auto-generated by nekorefactor split-file\n\nuse std::collections::HashMap;\n\n{}\n",
                    original_file.display(),
                    class_content
                ))
            }
            "python" => {
                // Add common Python imports for classes
                Ok(format!(
                    "# Extracted from: {}\n# Auto-generated by nekorefactor split-file\n\n{}\n",
                    original_file.display(),
                    class_content
                ))
            }
            "javascript" | "typescript" => {
                // Add common JS/TS imports for classes
                Ok(format!(
                    "// Extracted from: {}\n// Auto-generated by nekorefactor split-file\n\n{}\n",
                    original_file.display(),
                    class_content
                ))
            }
            _ => {
                // Generic header
                Ok(format!(
                    "// Extracted from: {}\n// Auto-generated by nekorefactor split-file\n\n{}\n",
                    original_file.display(),
                    class_content
                ))
            }
        }
    }
    
    /// Generate appropriate file header based on language
    fn generate_file_header(&self, original_file: &Path, function_content: &str) -> Result<String> {
        let language = self.language_detector.detect_language(original_file)?;
        
        match language.as_str() {
            "rust" => {
                // Add common Rust imports
                Ok(format!(
                    "// Extracted from: {}\n// Auto-generated by nekorefactor split-file\n\n{}\n",
                    original_file.display(),
                    function_content
                ))
            }
            "python" => {
                // Add common Python imports
                Ok(format!(
                    "# Extracted from: {}\n# Auto-generated by nekorefactor split-file\n\n{}\n",
                    original_file.display(),
                    function_content
                ))
            }
            "javascript" | "typescript" => {
                // Add common JS/TS imports
                Ok(format!(
                    "// Extracted from: {}\n// Auto-generated by nekorefactor split-file\n\n{}\n",
                    original_file.display(),
                    function_content
                ))
            }
            _ => {
                // Generic header
                Ok(format!(
                    "// Extracted from: {}\n// Auto-generated by nekorefactor split-file\n\n{}\n",
                    original_file.display(),
                    function_content
                ))
            }
        }
    }
}

/// Helper function to find impl block for a class
fn find_impl_block_for_class(lines: &[&str], class_name: &str, func_line: usize) -> Option<usize> {
    if func_line == 0 || func_line > lines.len() {
        return None;
    }
    
    // First check if this function line is indented (likely a method)
    let func_line_text = lines[func_line - 1];
    if !func_line_text.starts_with("    ") && !func_line_text.starts_with("\t") {
        // Top-level function, not a method
        return None;
    }
    
    // Search backwards from function line for impl block
    let mut brace_depth = 0;
    for i in (0..func_line.min(lines.len())).rev() {
        let line = lines[i];
        let trimmed = line.trim();
        
        // Track brace depth to ensure we're in the right scope
        for ch in line.chars().rev() {
            if ch == '}' {
                brace_depth += 1;
            } else if ch == '{' {
                brace_depth = if brace_depth > 0 { brace_depth - 1 } else { 0 };
            }
        }
        
        // Only check for impl if we're at the right brace level
        if brace_depth == 0 {
            if trimmed.starts_with(&format!("impl {}", class_name)) || 
               (trimmed.starts_with("impl ") && trimmed.contains(class_name)) {
                // Verify this impl block contains our function
                let impl_end = find_impl_block_end(lines, i + 1);
                if func_line <= impl_end {
                    return Some(i + 1);  // Return 1-based line number
                }
            }
        }
        
        // Stop if we hit another struct/class definition
        if trimmed.starts_with("struct ") || trimmed.starts_with("class ") || 
           trimmed.starts_with("pub struct ") || trimmed.starts_with("pub class ") {
            break;
        }
    }
    None
}

/// Helper function to find end of impl block
fn find_impl_block_end(lines: &[&str], start_line: usize) -> usize {
    let mut brace_count = 0;
    let mut found_opening = false;
    
    for i in (start_line-1)..lines.len() {
        for ch in lines[i].chars() {
            if ch == '{' {
                brace_count += 1;
                found_opening = true;
            } else if ch == '}' {
                brace_count -= 1;
                if found_opening && brace_count == 0 {
                    return i + 1;  // Return 1-based line number
                }
            }
        }
    }
    
    lines.len()
}

/// Helper function to check if a function is a method (inside impl block)
fn is_method_function(lines: &[&str], func_line: usize) -> bool {
    if func_line == 0 || func_line > lines.len() {
        return false;
    }
    
    // Check indentation - methods are usually indented
    let func_line_str = lines[func_line - 1];
    if !func_line_str.starts_with("    ") && !func_line_str.starts_with("\t") {
        // Top-level function (not indented) - likely standalone
        if func_line_str.trim_start().starts_with("fn ") || 
           func_line_str.trim_start().starts_with("pub fn ") {
            return false;
        }
    }
    
    // Search backwards from function line for impl block
    let mut brace_depth = 0;
    for i in (0..func_line.min(lines.len())).rev() {
        let line = lines[i];
        
        // Count braces to track nesting
        for ch in line.chars().rev() {
            if ch == '}' {
                brace_depth += 1;
            } else if ch == '{' {
                if brace_depth > 0 {
                    brace_depth -= 1;
                } else {
                    // Found an opening brace at the right level
                    let trimmed = line.trim();
                    if trimmed.starts_with("impl ") || trimmed.starts_with("impl<") {
                        return true;
                    }
                }
            }
        }
        
        // If we've exited all blocks, this is not a method
        if brace_depth == 0 && i < func_line - 1 {
            let trimmed = lines[i].trim();
            if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("#") {
                // Hit non-empty, non-comment line outside any block
                return false;
            }
        }
    }
    
    false
}