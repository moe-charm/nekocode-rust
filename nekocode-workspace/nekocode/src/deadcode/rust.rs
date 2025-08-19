//! Rust-specific dead code detection using cargo clippy

use nekocode_core::{Result, NekocodeError, Language};
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::deadcode::{DeadItem, SymbolType};

/// Rust dead code analyzer using cargo clippy
pub struct RustDeadCodeAnalyzer;

impl RustDeadCodeAnalyzer {
    /// Analyze with cargo clippy
    pub async fn analyze_with_clippy(files: &[PathBuf]) -> Result<Vec<DeadItem>> {
        // Find the project root (directory containing Cargo.toml)
        let project_root = Self::find_project_root(files)?;
        
        // Run cargo clippy with dead code warnings
        let output = Command::new("cargo")
            .current_dir(&project_root)
            .args(&[
                "clippy",
                "--message-format=json",
                "--",
                "-W", "dead_code",
                "-W", "unused_imports",
                "-W", "unused_variables",
                "-A", "clippy::all", // Disable other clippy warnings
            ])
            .output()
            .map_err(|e| NekocodeError::External(format!("Failed to run cargo clippy: {}", e)))?;

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_clippy_json(&stdout, &project_root)
    }

    /// Analyze with cargo-machete for unused dependencies
    pub async fn analyze_unused_deps(project_root: &Path) -> Result<Vec<DeadItem>> {
        let output = Command::new("cargo-machete")
            .current_dir(project_root)
            .output()
            .map_err(|e| NekocodeError::External(format!("Failed to run cargo-machete: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_machete_output(&stdout, project_root)
    }

    /// Find Cargo.toml in project hierarchy
    fn find_project_root(files: &[PathBuf]) -> Result<PathBuf> {
        if files.is_empty() {
            return Err(NekocodeError::External("No files provided".to_string()));
        }

        // Start from first file's directory
        let mut current = files[0].parent()
            .ok_or_else(|| NekocodeError::External("Invalid file path".to_string()))?;

        // Walk up until we find Cargo.toml
        loop {
            let cargo_toml = current.join("Cargo.toml");
            if cargo_toml.exists() {
                return Ok(current.to_path_buf());
            }

            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        }

        Err(NekocodeError::External("No Cargo.toml found in project hierarchy".to_string()))
    }

    /// Parse cargo clippy JSON output
    fn parse_clippy_json(json_output: &str, project_root: &Path) -> Result<Vec<DeadItem>> {
        let mut dead_items = Vec::new();

        for line in json_output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Parse each JSON line
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(message) = value.get("message") {
                    if let Some(item) = Self::parse_clippy_message(message, project_root) {
                        dead_items.push(item);
                    }
                }
            }
        }

        Ok(dead_items)
    }

    /// Parse single clippy message
    fn parse_clippy_message(message: &serde_json::Value, project_root: &Path) -> Option<DeadItem> {
        // Extract message text
        let msg_text = message.get("message")?.as_str()?;
        
        // Only process dead code warnings
        if !Self::is_deadcode_warning(msg_text) {
            return None;
        }

        // Extract span information
        let spans = message.get("spans")?.as_array()?;
        if spans.is_empty() {
            return None;
        }

        let primary_span = &spans[0];
        let file_name = primary_span.get("file_name")?.as_str()?;
        let line_start = primary_span.get("line_start")?.as_u64()? as u32;
        let line_end = primary_span.get("line_end")?.as_u64()? as u32;

        // Extract symbol name and type
        let (symbol_name, symbol_type) = Self::extract_symbol_info(msg_text)?;

        // Build file path relative to project root
        let file_path = project_root.join(file_name);

        Some(DeadItem {
            name: symbol_name,
            symbol_type,
            file_path,
            line_start,
            line_end,
            language: Language::Rust,
            confidence: 95, // Clippy is very accurate
            reason: Self::extract_warning_code(message),
        })
    }

    /// Check if message is about dead code
    fn is_deadcode_warning(msg: &str) -> bool {
        msg.contains("never used") ||
        msg.contains("never constructed") ||
        msg.contains("never read") ||
        msg.contains("unused") && (
            msg.contains("function") ||
            msg.contains("struct") ||
            msg.contains("enum") ||
            msg.contains("variable") ||
            msg.contains("import")
        )
    }

    /// Extract symbol name and type from warning message
    fn extract_symbol_info(msg: &str) -> Option<(String, SymbolType)> {
        // Examples:
        // "function `unused_func` is never used"
        // "struct `UnusedStruct` is never constructed"
        // "variable `unused_var` is never read"

        if let Some(start) = msg.find('`') {
            if let Some(end) = msg[start + 1..].find('`') {
                let symbol_name = msg[start + 1..start + 1 + end].to_string();
                
                let symbol_type = if msg.contains("function") {
                    SymbolType::Function
                } else if msg.contains("struct") || msg.contains("enum") {
                    SymbolType::Class
                } else if msg.contains("variable") || msg.contains("field") {
                    SymbolType::Variable
                } else if msg.contains("const") {
                    SymbolType::Constant
                } else if msg.contains("mod") || msg.contains("use") {
                    SymbolType::Module
                } else {
                    SymbolType::Function // Default
                };

                return Some((symbol_name, symbol_type));
            }
        }

        None
    }

    /// Extract warning code (like "dead_code" or "unused_imports")
    fn extract_warning_code(message: &serde_json::Value) -> String {
        if let Some(code) = message.get("code") {
            if let Some(code_str) = code.get("code") {
                if let Some(code_name) = code_str.as_str() {
                    return format!("clippy: {}", code_name);
                }
            }
        }
        "clippy: dead_code".to_string()
    }

    /// Parse cargo-machete output for unused dependencies
    fn parse_machete_output(output: &str, project_root: &Path) -> Result<Vec<DeadItem>> {
        let mut dead_items = Vec::new();
        let mut in_unused_section = false;

        for line in output.lines() {
            let trimmed = line.trim();
            
            // Look for start of unused dependencies section
            if trimmed.contains("found the following unused dependencies") {
                in_unused_section = true;
                continue;
            }
            
            // Skip until we're in unused section
            if !in_unused_section {
                continue;
            }
            
            // End of unused section
            if trimmed.starts_with("If you believe") || trimmed == "Done!" {
                break;
            }
            
            // Skip project headers like "deadcode-test -- ./Cargo.toml:"
            if trimmed.contains(" -- ") && trimmed.contains("Cargo.toml:") {
                continue;
            }
            
            // Parse crate names (indented lines with just the crate name)
            if line.starts_with('\t') || line.starts_with("    ") {
                let crate_name = trimmed.to_string();
                if !crate_name.is_empty() && !crate_name.contains(' ') {
                    dead_items.push(DeadItem {
                        name: crate_name,
                        symbol_type: SymbolType::Module,
                        file_path: project_root.join("Cargo.toml"),
                        line_start: 0, // Line number not available
                        line_end: 0,
                        language: Language::Rust,
                        confidence: 85, // Cargo-machete is quite reliable
                        reason: "cargo-machete: unused dependency".to_string(),
                    });
                }
            }
        }

        Ok(dead_items)
    }

    /// Extract crate name from cargo-machete line
    fn extract_crate_name(line: &str) -> Option<String> {
        // Try to extract from quotes first
        if let Some(start) = line.find('\'') {
            if let Some(end) = line[start + 1..].find('\'') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }

        // Try to extract from "dependency: name" format
        if let Some(colon_pos) = line.find(':') {
            let rest = line[colon_pos + 1..].trim();
            if let Some(space_pos) = rest.find(' ') {
                return Some(rest[..space_pos].to_string());
            } else {
                return Some(rest.to_string());
            }
        }

        None
    }

    /// Check if cargo clippy is available
    pub fn check_clippy_available() -> bool {
        Command::new("cargo")
            .args(&["clippy", "--version"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Check if cargo-machete is available
    pub fn check_machete_available() -> bool {
        Command::new("cargo-machete")
            .args(&["--version"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get installation instructions for missing tools
    pub fn get_install_instructions() -> Vec<String> {
        vec![
            "# Install cargo clippy (usually included with Rust)".to_string(),
            "rustup component add clippy".to_string(),
            "".to_string(),
            "# Install cargo-machete for unused dependency detection".to_string(),
            "cargo install cargo-machete".to_string(),
        ]
    }

    /// Run comprehensive Rust analysis
    pub async fn analyze_comprehensive(files: &[PathBuf]) -> Result<Vec<DeadItem>> {
        let project_root = Self::find_project_root(files)?;
        let mut all_items = Vec::new();

        // Run cargo clippy for code analysis
        if Self::check_clippy_available() {
            let mut clippy_items = Self::analyze_with_clippy(files).await?;
            all_items.append(&mut clippy_items);
        }

        // Run cargo-machete for dependency analysis
        if Self::check_machete_available() {
            let mut machete_items = Self::analyze_unused_deps(&project_root).await?;
            all_items.append(&mut machete_items);
        }

        Ok(all_items)
    }
}