//! Python-specific dead code detection using vulture

use nekocode_core::{Result, NekocodeError, Language};
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::deadcode::{DeadItem, SymbolType};

/// Python dead code analyzer using vulture
pub struct PythonDeadCodeAnalyzer;

impl PythonDeadCodeAnalyzer {
    /// Analyze with vulture
    pub async fn analyze_with_vulture(files: &[PathBuf]) -> Result<Vec<DeadItem>> {
        // Filter Python files
        let python_files: Vec<&PathBuf> = files
            .iter()
            .filter(|f| Self::is_python_file(f))
            .collect();

        if python_files.is_empty() {
            return Ok(Vec::new());
        }

        // Run vulture on Python files
        let mut cmd = Command::new("vulture");
        
        // Add confidence threshold
        cmd.args(&["--min-confidence", "60"]);
        
        // Add files
        for file in &python_files {
            cmd.arg(file);
        }

        let output = cmd
            .output()
            .map_err(|e| NekocodeError::External(format!("Failed to run vulture: {}", e)))?;

        // Vulture outputs to stdout
        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_vulture_output(&stdout)
    }

    /// Check if file is a Python file
    fn is_python_file(file: &Path) -> bool {
        file.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "py" || ext == "pyw")
            .unwrap_or(false)
    }

    /// Parse vulture output
    fn parse_vulture_output(output: &str) -> Result<Vec<DeadItem>> {
        let mut dead_items = Vec::new();

        for line in output.lines() {
            if let Some(item) = Self::parse_vulture_line(line) {
                dead_items.push(item);
            }
        }

        Ok(dead_items)
    }

    /// Parse single vulture output line
    fn parse_vulture_line(line: &str) -> Option<DeadItem> {
        // Vulture output format examples:
        // "file.py:10: unused function 'unused_func' (60% confidence)"
        // "file.py:5: unused class 'UnusedClass' (80% confidence)"
        // "file.py:15: unused variable 'unused_var' (90% confidence)"
        // "file.py:20: unused import 'unused_module' (100% confidence)"

        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 3 {
            return None;
        }

        // Extract file path and line number
        let file_path = PathBuf::from(parts[0]);
        let line_num: u32 = parts[1].parse().ok()?;

        // Extract the rest of the message
        let message = parts[2..].join(":");
        
        // Parse the message to extract symbol info
        Self::parse_vulture_message(&message, file_path, line_num)
    }

    /// Parse vulture message to extract symbol information
    fn parse_vulture_message(message: &str, file_path: PathBuf, line_num: u32) -> Option<DeadItem> {
        // Look for patterns like "unused function 'name' (confidence%)"
        
        // Extract symbol type
        let symbol_type = if message.contains("function") {
            SymbolType::Function
        } else if message.contains("class") {
            SymbolType::Class
        } else if message.contains("variable") {
            SymbolType::Variable
        } else if message.contains("import") {
            SymbolType::Module
        } else if message.contains("property") {
            SymbolType::Variable
        } else if message.contains("attribute") {
            SymbolType::Variable
        } else {
            // Default to function if we can't determine
            SymbolType::Function
        };

        // Extract symbol name (between single quotes)
        let symbol_name = Self::extract_quoted_name(message)?;

        // Extract confidence percentage
        let confidence = Self::extract_confidence(message);

        // Generate reason string
        let reason = format!("vulture: {}", Self::extract_reason_type(message));

        Some(DeadItem {
            name: symbol_name,
            symbol_type,
            file_path,
            line_start: line_num,
            line_end: line_num, // Vulture doesn't provide end line
            language: Language::Python,
            confidence,
            reason,
        })
    }

    /// Extract symbol name from between quotes
    fn extract_quoted_name(message: &str) -> Option<String> {
        if let Some(start) = message.find('\'') {
            if let Some(end) = message[start + 1..].find('\'') {
                return Some(message[start + 1..start + 1 + end].to_string());
            }
        }
        None
    }

    /// Extract confidence percentage
    fn extract_confidence(message: &str) -> u8 {
        // Look for pattern "(XX% confidence)"
        if let Some(start) = message.find('(') {
            if let Some(percent_pos) = message[start..].find('%') {
                let conf_str = &message[start + 1..start + percent_pos];
                if let Ok(confidence) = conf_str.parse::<u8>() {
                    return confidence;
                }
            }
        }
        70 // Default confidence
    }

    /// Extract the type of unused item for reason
    fn extract_reason_type(message: &str) -> String {
        if message.contains("function") {
            "unused function".to_string()
        } else if message.contains("class") {
            "unused class".to_string()
        } else if message.contains("variable") {
            "unused variable".to_string()
        } else if message.contains("import") {
            "unused import".to_string()
        } else if message.contains("property") {
            "unused property".to_string()
        } else if message.contains("attribute") {
            "unused attribute".to_string()
        } else {
            "unused code".to_string()
        }
    }

    /// Check if vulture is available
    pub fn check_vulture_available() -> bool {
        Command::new("vulture")
            .args(&["--version"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get installation instructions for vulture
    pub fn get_install_instructions() -> Vec<String> {
        vec![
            "# Install vulture for Python dead code detection".to_string(),
            "pip install vulture".to_string(),
            "".to_string(),
            "# Or with conda:".to_string(),
            "conda install -c conda-forge vulture".to_string(),
        ]
    }

    /// Run vulture with custom configuration
    pub async fn analyze_with_config(
        files: &[PathBuf],
        min_confidence: u8,
        ignore_decorators: &[&str],
        ignore_names: &[&str],
    ) -> Result<Vec<DeadItem>> {
        let python_files: Vec<&PathBuf> = files
            .iter()
            .filter(|f| Self::is_python_file(f))
            .collect();

        if python_files.is_empty() {
            return Ok(Vec::new());
        }

        let mut cmd = Command::new("vulture");
        
        // Set confidence threshold
        cmd.args(&["--min-confidence", &min_confidence.to_string()]);
        
        // Add ignore patterns if any
        for decorator in ignore_decorators {
            cmd.args(&["--ignore-decorators", decorator]);
        }
        
        for name in ignore_names {
            cmd.args(&["--ignore-names", name]);
        }

        // Add files
        for file in &python_files {
            cmd.arg(file);
        }

        let output = cmd
            .output()
            .map_err(|e| NekocodeError::External(format!("Failed to run vulture: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_vulture_output(&stdout)
    }

    /// Analyze Python project with smart defaults
    pub async fn analyze_smart(project_dir: &Path) -> Result<Vec<DeadItem>> {
        // Find all Python files in project
        let python_files = Self::find_python_files(project_dir)?;
        
        if python_files.is_empty() {
            return Ok(Vec::new());
        }

        // Use smart configuration for common Python projects
        let ignore_decorators = vec![
            "@app.route",       // Flask routes
            "@api.route",       // API routes
            "@click.command",   // Click commands
            "@pytest.fixture",  // Pytest fixtures
            "@staticmethod",    // Static methods
            "@classmethod",     // Class methods
            "@property",        // Properties
        ];

        let ignore_names = vec![
            "__*__",           // Magic methods
            "test_*",          // Test functions
            "*_test",          // Test functions
            "setUp",           // Test setup
            "tearDown",        // Test teardown
            "main",            // Main function
        ];

        Self::analyze_with_config(&python_files, 60, &ignore_decorators, &ignore_names).await
    }

    /// Find all Python files recursively
    fn find_python_files(dir: &Path) -> Result<Vec<PathBuf>> {
        let mut python_files = Vec::new();
        
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)
                .map_err(|e| NekocodeError::Io(e))?
            {
                let entry = entry.map_err(|e| NekocodeError::Io(e))?;
                let path = entry.path();
                
                if path.is_dir() {
                    // Skip common non-source directories
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if matches!(dir_name, "__pycache__" | ".git" | ".venv" | "venv" | ".tox" | "build" | "dist") {
                            continue;
                        }
                    }
                    
                    // Recurse into subdirectory
                    let mut sub_files = Self::find_python_files(&path)?;
                    python_files.append(&mut sub_files);
                } else if Self::is_python_file(&path) {
                    python_files.push(path);
                }
            }
        }
        
        Ok(python_files)
    }
}