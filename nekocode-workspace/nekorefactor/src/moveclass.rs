//! Move class/function functionality

use std::path::{Path, PathBuf};
use std::fs;

use nekocode_core::{
    Result, NekocodeError, SessionManager, Language,
    SymbolInfo, FunctionInfo, ClassInfo
};

/// Options for move operations
#[derive(Debug, Clone)]
pub struct MoveOptions {
    pub update_imports: bool,
    pub create_target_if_missing: bool,
    pub preserve_comments: bool,
    pub dry_run: bool,
}

impl Default for MoveOptions {
    fn default() -> Self {
        Self {
            update_imports: true,
            create_target_if_missing: true,
            preserve_comments: true,
            dry_run: false,
        }
    }
}

/// Move class engine for refactoring operations
pub struct MoveClassEngine {
    session_manager: SessionManager,
    options: MoveOptions,
}

impl MoveClassEngine {
    /// Create new move class engine
    pub fn new(options: MoveOptions) -> Result<Self> {
        Ok(Self {
            session_manager: SessionManager::new()?,
            options,
        })
    }
    
    /// Create with default options
    pub fn default() -> Result<Self> {
        Self::new(MoveOptions::default())
    }
    
    /// Move a symbol (class/function) to another file
    pub async fn move_symbol(
        &mut self,
        session_id: &str,
        symbol_id: &str,
        target_file: &Path
    ) -> Result<MoveResult> {
        // Get session and find symbol
        let symbol_info = {
            let session = self.session_manager.get_session_mut(session_id)?;
            Self::find_symbol_in_session(session, symbol_id)?
        };
        
        // Extract the symbol content from source file
        let source_file = &symbol_info.file_path;
        let symbol_content = self.extract_symbol_content(source_file, &symbol_info)?;
        
        // Check if target file exists
        if !target_file.exists() && !self.options.create_target_if_missing {
            return Err(NekocodeError::Refactoring(
                format!("Target file does not exist: {}", target_file.display())
            ));
        }
        
        // Create move result
        let mut result = MoveResult {
            symbol_name: symbol_info.name.clone(),
            symbol_type: format!("{:?}", symbol_info.symbol_type),
            source_file: source_file.clone(),
            target_file: target_file.to_path_buf(),
            lines_moved: symbol_content.lines().count(),
            imports_updated: Vec::new(),
            success: false,
        };
        
        if self.options.dry_run {
            result.success = true;
            return Ok(result);
        }
        
        // Perform the move
        self.perform_move(source_file, target_file, &symbol_info, &symbol_content)?;
        
        // Update imports if requested
        if self.options.update_imports {
            let updated = self.update_imports_for_move(&symbol_info, source_file, target_file)?;
            result.imports_updated = updated;
        }
        
        result.success = true;
        Ok(result)
    }
    
    /// Find symbol in session
    fn find_symbol_in_session(
        session: &nekocode_core::Session,
        symbol_id: &str
    ) -> Result<SymbolInfo> {
        for analysis_result in &session.info.analysis_results {
            for symbol in &analysis_result.symbols {
                if symbol.id == symbol_id {
                    return Ok(symbol.clone());
                }
            }
        }
        
        Err(NekocodeError::Refactoring(
            format!("Symbol not found: {}", symbol_id)
        ))
    }
    
    /// Extract symbol content from file
    fn extract_symbol_content(
        &self,
        file: &Path,
        symbol: &SymbolInfo
    ) -> Result<String> {
        let content = fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let lines: Vec<&str> = content.lines().collect();
        let start_idx = (symbol.line_start as usize).saturating_sub(1);
        let end_idx = symbol.line_end as usize;
        
        if end_idx > lines.len() {
            return Err(NekocodeError::Refactoring(
                "Symbol line range out of bounds".to_string()
            ));
        }
        
        let symbol_lines = &lines[start_idx..end_idx];
        
        // Include leading comments if preserving
        let mut final_lines = Vec::new();
        
        if self.options.preserve_comments && start_idx > 0 {
            // Look for comments before the symbol
            for i in (0..start_idx).rev() {
                let line = lines[i].trim();
                if line.starts_with("//") || line.starts_with("/*") || line.starts_with("*") || line.starts_with("///") {
                    final_lines.insert(0, lines[i]);
                } else if !line.is_empty() {
                    break;
                }
            }
        }
        
        final_lines.extend(symbol_lines);
        
        Ok(final_lines.join("\n"))
    }
    
    /// Perform the actual move operation
    fn perform_move(
        &self,
        source_file: &Path,
        target_file: &Path,
        symbol: &SymbolInfo,
        symbol_content: &str
    ) -> Result<()> {
        // Remove from source file
        self.remove_symbol_from_file(source_file, symbol)?;
        
        // Add to target file
        self.add_symbol_to_file(target_file, symbol_content, symbol.language)?;
        
        Ok(())
    }
    
    /// Remove symbol from file
    fn remove_symbol_from_file(
        &self,
        file: &Path,
        symbol: &SymbolInfo
    ) -> Result<()> {
        let content = fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();
        
        let start_idx = (symbol.line_start as usize).saturating_sub(1);
        let end_idx = symbol.line_end as usize;
        
        for (i, line) in lines.iter().enumerate() {
            if i < start_idx || i >= end_idx {
                new_lines.push(*line);
            }
        }
        
        // Remove extra blank lines
        let new_content = new_lines.join("\n");
        let cleaned = self.remove_extra_blank_lines(&new_content);
        
        fs::write(file, cleaned)
            .map_err(|e| NekocodeError::Io(e))?;
        
        Ok(())
    }
    
    /// Add symbol to file
    fn add_symbol_to_file(
        &self,
        file: &Path,
        symbol_content: &str,
        language: Language
    ) -> Result<()> {
        // Create file if it doesn't exist
        if !file.exists() {
            if self.options.create_target_if_missing {
                // Create with appropriate header
                let header = self.generate_file_header(file, language);
                let content = format!("{}\n\n{}", header, symbol_content);
                fs::write(file, content)
                    .map_err(|e| NekocodeError::Io(e))?;
                return Ok(());
            } else {
                return Err(NekocodeError::Refactoring(
                    format!("Target file does not exist: {}", file.display())
                ));
            }
        }
        
        // Read existing content
        let existing = fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        // Find appropriate insertion point
        let insertion_point = self.find_insertion_point(&existing, language);
        
        // Insert the symbol
        let new_content = if insertion_point == existing.len() {
            // Append at end
            format!("{}\n\n{}", existing, symbol_content)
        } else {
            // Insert at specific position
            let mut lines: Vec<&str> = existing.lines().collect();
            lines.insert(insertion_point, symbol_content);
            lines.join("\n")
        };
        
        fs::write(file, new_content)
            .map_err(|e| NekocodeError::Io(e))?;
        
        Ok(())
    }
    
    /// Generate file header for new files
    fn generate_file_header(&self, file: &Path, language: Language) -> String {
        let filename = file.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        match language {
            Language::Rust => {
                format!("//! {}\n\n", filename)
            }
            Language::JavaScript | Language::TypeScript => {
                format!("/**\n * {}\n */\n", filename)
            }
            Language::Python => {
                format!("\"\"\"\n{}\n\"\"\"\n", filename)
            }
            Language::Go => {
                format!("// Package main - {}\npackage main\n", filename)
            }
            Language::Cpp | Language::C => {
                format!("/**\n * {}\n */\n\n#pragma once\n", filename)
            }
            Language::CSharp => {
                format!("// {}\n\nusing System;\n", filename)
            }
            _ => String::new()
        }
    }
    
    /// Find appropriate insertion point in file
    fn find_insertion_point(&self, content: &str, language: Language) -> usize {
        let lines: Vec<&str> = content.lines().collect();
        
        // Skip past file headers, imports, etc.
        let mut idx = 0;
        let mut in_header = true;
        
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            
            match language {
                Language::Rust => {
                    if in_header && (trimmed.starts_with("//!") || trimmed.starts_with("use ") || trimmed.starts_with("mod ")) {
                        idx = i + 1;
                    } else {
                        in_header = false;
                    }
                }
                Language::JavaScript | Language::TypeScript => {
                    if in_header && (trimmed.starts_with("import ") || trimmed.starts_with("const ") || trimmed.starts_with("//")) {
                        idx = i + 1;
                    } else {
                        in_header = false;
                    }
                }
                Language::Python => {
                    if in_header && (trimmed.starts_with("import ") || trimmed.starts_with("from ") || trimmed.starts_with("#")) {
                        idx = i + 1;
                    } else {
                        in_header = false;
                    }
                }
                _ => {
                    // Default: insert after any initial comments
                    if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.is_empty() {
                        idx = i + 1;
                    } else {
                        break;
                    }
                }
            }
            
            if !in_header {
                break;
            }
        }
        
        idx
    }
    
    /// Update imports after moving a symbol
    fn update_imports_for_move(
        &self,
        symbol: &SymbolInfo,
        source_file: &Path,
        target_file: &Path
    ) -> Result<Vec<PathBuf>> {
        // This is a simplified version
        // Real implementation would need to:
        // 1. Find all files that import the symbol
        // 2. Update their import statements
        // 3. Handle different import styles for each language
        
        let mut updated_files = Vec::new();
        
        // TODO: Implement actual import updating logic
        // This would require parsing imports in all project files
        // and updating them to point to the new location
        
        Ok(updated_files)
    }
    
    /// Remove extra blank lines from content
    fn remove_extra_blank_lines(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut prev_blank = false;
        
        for line in lines {
            let is_blank = line.trim().is_empty();
            
            if is_blank && prev_blank {
                // Skip extra blank lines
                continue;
            }
            
            result.push(line);
            prev_blank = is_blank;
        }
        
        result.join("\n")
    }
}

/// Result of a move operation
#[derive(Debug, Clone)]
pub struct MoveResult {
    pub symbol_name: String,
    pub symbol_type: String,
    pub source_file: PathBuf,
    pub target_file: PathBuf,
    pub lines_moved: usize,
    pub imports_updated: Vec<PathBuf>,
    pub success: bool,
}