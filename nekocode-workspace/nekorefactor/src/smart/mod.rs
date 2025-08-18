//! Smart refactoring with Tree-sitter AST integration

use std::path::{Path, PathBuf};
use nekocode_core::{Result, NekocodeError, Session, Language};
use serde::{Serialize, Deserialize};
use regex::Regex;

pub mod languages;

/// Smart refactoring engine using Tree-sitter AST
pub struct SmartRefactor {
    session: Option<Session>,
    session_id: String,
}

impl SmartRefactor {
    /// Create a new SmartRefactor instance with session
    pub async fn from_session_id(session_id: &str) -> Result<Self> {
        // Load session from nekocode
        let session = Session::load(session_id)?;
        
        Ok(Self {
            session: Some(session),
            session_id: session_id.to_string(),
        })
    }
    
    /// Smart insert with AST-based positioning
    pub async fn smart_insert(
        &self,
        file: &Path,
        content: &str,
        position: SmartPosition,
        preview: bool,
    ) -> Result<SmartResult> {
        // Get session
        let session = self.session.as_ref()
            .ok_or_else(|| NekocodeError::Session("No session loaded".to_string()))?;
        
        // Get AST from session
        let ast_info = self.get_ast_info(session, file).await?;
        
        // Find exact insertion point using Tree-sitter
        let insert_point = match position {
            SmartPosition::AfterFunction(ref name) => {
                self.find_function_end(&ast_info, name)?
            }
            SmartPosition::BeforeFunction(ref name) => {
                self.find_function_start(&ast_info, name)?
            }
            SmartPosition::InClass(ref name) => {
                self.find_class_insert_point(&ast_info, name)?
            }
            SmartPosition::InImports => {
                self.find_imports_insert_point(&ast_info)?
            }
            SmartPosition::Line(line) => {
                InsertPoint {
                    line,
                    column: 0,
                    indent_level: 0,
                }
            }
        };
        
        // Detect proper indentation
        let indent = self.detect_indentation(&ast_info, &insert_point);
        
        // Format content with proper indentation
        let formatted_content = self.format_content(content, indent, ast_info.language);
        
        // Create result
        let result = SmartResult {
            operation: "insert".to_string(),
            file: file.to_path_buf(),
            position: format!("Line {}, Column {}", insert_point.line, insert_point.column),
            content: formatted_content.clone(),
            preview_text: if preview {
                self.generate_preview(file, &insert_point, &formatted_content)?
            } else {
                String::new()
            },
            applied: false,
        };
        
        // Apply if not preview
        if !preview {
            self.apply_insertion(file, &insert_point, &formatted_content)?;
        }
        
        Ok(result)
    }
    
    /// Smart replace with scope awareness
    pub async fn smart_replace(
        &self,
        file: &Path,
        pattern: &str,
        replacement: &str,
        scope: Option<Scope>,
        use_regex: bool,
        preview: bool,
    ) -> Result<SmartResult> {
        // Get session
        let session = self.session.as_ref()
            .ok_or_else(|| NekocodeError::Session("No session loaded".to_string()))?;
        
        // Get AST from session
        let ast_info = self.get_ast_info(session, file).await?;
        
        // Find scope boundaries if specified
        let scope_range = if let Some(scope) = scope {
            Some(self.find_scope_range(&ast_info, &scope)?)
        } else {
            None
        };
        
        // Find all matches within scope
        let matches = self.find_matches(file, pattern, scope_range, use_regex)?;
        
        // Create result
        let result = SmartResult {
            operation: "replace".to_string(),
            file: file.to_path_buf(),
            position: format!("{} matches found", matches.len()),
            content: replacement.to_string(),
            preview_text: if preview {
                self.generate_replace_preview(&matches, pattern, replacement)?
            } else {
                String::new()
            },
            applied: false,
        };
        
        // Apply if not preview
        if !preview {
            self.apply_replacements(file, &matches, replacement)?;
        }
        
        Ok(result)
    }
    
    /// Move symbol to another file
    pub async fn smart_move(
        &self,
        symbol_path: &str,
        target: &Path,
        update_imports: bool,
        preview: bool,
    ) -> Result<SmartResult> {
        // Parse symbol path (e.g., "MyClass::method" or "function_name")
        let (symbol_type, symbol_name) = self.parse_symbol_path(symbol_path)?;
        
        // Get session
        let session = self.session.as_ref()
            .ok_or_else(|| NekocodeError::Session("No session loaded".to_string()))?;
        
        // Find symbol in session
        let symbol_info = session.find_symbol(&symbol_name)?;
        
        // Extract symbol code
        let symbol_code = self.extract_symbol_code(&symbol_info)?;
        
        // Create result
        let result = SmartResult {
            operation: "move".to_string(),
            file: target.to_path_buf(),
            position: format!("Moving {} to {}", symbol_name, target.display()),
            content: symbol_code.clone(),
            preview_text: if preview {
                format!("Will move:\n{}\n\nTo: {}", symbol_code, target.display())
            } else {
                String::new()
            },
            applied: false,
        };
        
        // Apply if not preview
        if !preview {
            self.apply_move(&symbol_info, target, &symbol_code, update_imports)?;
        }
        
        Ok(result)
    }
    
    // Helper methods
    
    async fn get_ast_info(&self, session: &Session, file: &Path) -> Result<AstInfo> {
        // Find analysis result for this file in session
        let analysis_result = session.info.analysis_results
            .iter()
            .find(|result| result.file_info.path == file)
            .ok_or_else(|| NekocodeError::Session(
                format!("No analysis result found for file: {}", file.display())
            ))?;

        // Convert from core types to Smart refactoring types
        let functions = analysis_result.functions
            .iter()
            .map(|func| FunctionInfo {
                name: func.symbol.name.clone(),
                start_line: func.symbol.line_start,
                end_line: func.symbol.line_end,
            })
            .collect();

        let classes = analysis_result.classes
            .iter()
            .map(|class| ClassInfo {
                name: class.symbol.name.clone(),
                start_line: class.symbol.line_start,
                end_line: class.symbol.line_end,
            })
            .collect();

        let imports = analysis_result.imports
            .iter()
            .map(|import| import.module.clone())
            .collect();

        Ok(AstInfo {
            language: analysis_result.file_info.language,
            functions,
            classes,
            imports,
        })
    }
    
    fn find_function_end(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint> {
        // Use language-specific rules to find function end
        let language_rules = languages::get_rules(ast_info.language)?;
        language_rules.find_function_end(ast_info, name)
    }
    
    fn find_function_start(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint> {
        // Use language-specific rules to find function start
        let language_rules = languages::get_rules(ast_info.language)?;
        language_rules.find_function_start(ast_info, name)
    }
    
    fn find_class_insert_point(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint> {
        // Use language-specific rules to find class insertion point
        let language_rules = languages::get_rules(ast_info.language)?;
        language_rules.find_class_insert_point(ast_info, name)
    }
    
    fn find_imports_insert_point(&self, ast_info: &AstInfo) -> Result<InsertPoint> {
        // Use language-specific rules to find imports section
        let language_rules = languages::get_rules(ast_info.language)?;
        language_rules.find_imports_insert_point(ast_info)
    }
    
    fn detect_indentation(&self, ast_info: &AstInfo, point: &InsertPoint) -> Indent {
        // Detect indentation style from context
        let language_rules = languages::get_rules(ast_info.language).unwrap();
        language_rules.detect_indentation(ast_info, point)
    }
    
    fn format_content(&self, content: &str, indent: Indent, language: Language) -> String {
        // Format content with proper indentation
        let indent_str = match indent {
            Indent::Spaces(n) => " ".repeat(n as usize),
            Indent::Tabs(n) => "\t".repeat(n as usize),
        };
        
        // Add indentation to each line
        content.lines()
            .map(|line| {
                if line.is_empty() {
                    line.to_string()
                } else {
                    format!("{}{}", indent_str, line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    fn generate_preview(&self, file: &Path, point: &InsertPoint, content: &str) -> Result<String> {
        // Generate preview text showing where content will be inserted
        let file_content = std::fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let lines: Vec<&str> = file_content.lines().collect();
        let line_idx = (point.line - 1) as usize;
        
        let mut preview = String::new();
        
        // Show context before
        if line_idx > 0 {
            preview.push_str(&format!("  {}: {}\n", point.line - 1, lines[line_idx - 1]));
        }
        
        // Show insertion
        preview.push_str(&format!("+ {}: {}\n", point.line, content));
        
        // Show context after
        if line_idx < lines.len() {
            preview.push_str(&format!("  {}: {}\n", point.line + 1, lines[line_idx]));
        }
        
        Ok(preview)
    }
    
    fn apply_insertion(&self, file: &Path, point: &InsertPoint, content: &str) -> Result<()> {
        // Read file
        let file_content = std::fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let mut lines: Vec<String> = file_content.lines().map(|s| s.to_string()).collect();
        
        // Insert at the specified line
        let line_idx = (point.line - 1) as usize;
        lines.insert(line_idx, content.to_string());
        
        // Write back
        let new_content = lines.join("\n");
        std::fs::write(file, new_content)
            .map_err(|e| NekocodeError::Io(e))?;
        
        Ok(())
    }
    
    fn find_scope_range(&self, ast_info: &AstInfo, scope: &Scope) -> Result<Range> {
        // Find the range of the specified scope
        match scope {
            Scope::InClass(name) => {
                // Find class boundaries
                for class in &ast_info.classes {
                    if class.name == *name {
                        return Ok(Range {
                            start_line: class.start_line,
                            end_line: class.end_line,
                        });
                    }
                }
                Err(NekocodeError::Refactoring(format!("Class '{}' not found", name)))
            }
            Scope::InFunction(name) => {
                // Find function boundaries
                for func in &ast_info.functions {
                    if func.name == *name {
                        return Ok(Range {
                            start_line: func.start_line,
                            end_line: func.end_line,
                        });
                    }
                }
                Err(NekocodeError::Refactoring(format!("Function '{}' not found", name)))
            }
        }
    }
    
    fn find_matches(&self, file: &Path, pattern: &str, range: Option<Range>, use_regex: bool) -> Result<Vec<Match>> {
        // Read file content
        let content = std::fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let mut matches = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        for (idx, line) in lines.iter().enumerate() {
            let line_num = (idx + 1) as u32;
            
            // Check if line is within range
            if let Some(ref r) = range {
                if line_num < r.start_line || line_num > r.end_line {
                    continue;
                }
            }
            
            // Find matches in this line
            if use_regex {
                // Regex matching
                if let Ok(re) = Regex::new(pattern) {
                    for mat in re.find_iter(line) {
                        matches.push(Match {
                            line: line_num,
                            column: mat.start() as u32,
                            text: mat.as_str().to_string(),
                        });
                    }
                }
            } else {
                // Literal string matching
                let mut start = 0;
                while let Some(pos) = line[start..].find(pattern) {
                    let col = start + pos;
                    matches.push(Match {
                        line: line_num,
                        column: col as u32,
                        text: pattern.to_string(),
                    });
                    start = col + pattern.len();
                }
            }
        }
        
        Ok(matches)
    }
    
    fn generate_replace_preview(&self, matches: &[Match], pattern: &str, replacement: &str) -> Result<String> {
        // Generate preview showing all replacements
        let mut preview = String::new();
        for m in matches {
            preview.push_str(&format!("Line {}: {} -> {}\n", m.line, pattern, replacement));
        }
        Ok(preview)
    }
    
    fn apply_replacements(&self, file: &Path, matches: &[Match], replacement: &str) -> Result<()> {
        // Read file content
        let content = std::fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        // Apply replacements from last to first to maintain positions
        let mut sorted_matches = matches.to_vec();
        sorted_matches.sort_by(|a, b| {
            b.line.cmp(&a.line)
                .then(b.column.cmp(&a.column))
        });
        
        for mat in sorted_matches {
            let line_idx = (mat.line - 1) as usize;
            if line_idx < lines.len() {
                let line = &lines[line_idx];
                let start = mat.column as usize;
                let end = start + mat.text.len();
                
                if end <= line.len() {
                    let new_line = format!(
                        "{}{}{}",
                        &line[..start],
                        replacement,
                        &line[end..]
                    );
                    lines[line_idx] = new_line;
                }
            }
        }
        
        // Write back to file
        let new_content = lines.join("\n");
        std::fs::write(file, new_content)
            .map_err(|e| NekocodeError::Io(e))?;
        
        Ok(())
    }
    
    fn parse_symbol_path(&self, path: &str) -> Result<(String, String)> {
        // Parse "Class::method" or "function"
        if let Some(pos) = path.find("::") {
            let class = path[..pos].to_string();
            let method = path[pos+2..].to_string();
            Ok((class, method))
        } else {
            Ok(("function".to_string(), path.to_string()))
        }
    }
    
    fn extract_symbol_code(&self, symbol_info: &nekocode_core::SymbolInfo) -> Result<String> {
        // Read file containing the symbol
        let content = std::fs::read_to_string(&symbol_info.file_path)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let lines: Vec<&str> = content.lines().collect();
        
        // Extract lines from start to end of symbol
        let start_idx = (symbol_info.line_start - 1) as usize;
        let end_idx = symbol_info.line_end as usize;
        
        if start_idx >= lines.len() || end_idx > lines.len() {
            return Err(NekocodeError::Refactoring(
                format!("Line range {}-{} out of bounds", symbol_info.line_start, symbol_info.line_end)
            ));
        }
        
        // Extract the symbol code
        let symbol_lines = &lines[start_idx..end_idx];
        let code = symbol_lines.join("\n");
        
        Ok(code)
    }
    
    fn apply_move(&self, symbol_info: &nekocode_core::SymbolInfo, target: &Path, code: &str, update_imports: bool) -> Result<()> {
        // Step 1: Remove the symbol from source file
        let source_content = std::fs::read_to_string(&symbol_info.file_path)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let mut source_lines: Vec<String> = source_content.lines().map(|s| s.to_string()).collect();
        
        // Remove the symbol lines (in reverse to maintain indices)
        let start_idx = (symbol_info.line_start - 1) as usize;
        let end_idx = symbol_info.line_end as usize;
        
        if start_idx < source_lines.len() && end_idx <= source_lines.len() {
            // Remove lines from end to start
            for _ in start_idx..end_idx {
                source_lines.remove(start_idx);
            }
        }
        
        // Write back source file
        let new_source = source_lines.join("\n");
        std::fs::write(&symbol_info.file_path, new_source)
            .map_err(|e| NekocodeError::Io(e))?;
        
        // Step 2: Add the symbol to target file
        if target.exists() {
            // Append to existing file
            let target_content = std::fs::read_to_string(target)
                .map_err(|e| NekocodeError::Io(e))?;
            
            let new_target = format!("{}\n\n{}", target_content, code);
            std::fs::write(target, new_target)
                .map_err(|e| NekocodeError::Io(e))?;
        } else {
            // Create new file with the symbol
            std::fs::write(target, code)
                .map_err(|e| NekocodeError::Io(e))?;
        }
        
        // Step 3: Update imports if requested
        if update_imports {
            // TODO: Implement import updates based on language
            // This would require language-specific logic to:
            // 1. Add import in target file if needed
            // 2. Update imports in source file if needed
            // 3. Update other files that import this symbol
        }
        
        Ok(())
    }
}

/// Position for smart operations
#[derive(Debug)]
pub enum SmartPosition {
    AfterFunction(String),
    BeforeFunction(String),
    InClass(String),
    InImports,
    Line(u32),
}

/// Scope for smart replace
#[derive(Debug)]
pub enum Scope {
    InClass(String),
    InFunction(String),
}

/// Result of smart operation
#[derive(Debug, Serialize, Deserialize)]
pub struct SmartResult {
    pub operation: String,
    pub file: PathBuf,
    pub position: String,
    pub content: String,
    pub preview_text: String,
    pub applied: bool,
}

/// Insertion point with context
#[derive(Debug)]
pub struct InsertPoint {
    pub line: u32,
    pub column: u32,
    pub indent_level: u32,
}

/// Indentation style
#[derive(Debug, Clone, Copy)]
pub enum Indent {
    Spaces(u32),
    Tabs(u32),
}

/// AST information from nekocode
#[derive(Debug)]
pub struct AstInfo {
    pub language: Language,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<String>,
}

#[derive(Debug)]
pub struct FunctionInfo {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug)]
pub struct ClassInfo {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
}

/// Range in file
#[derive(Debug)]
struct Range {
    start_line: u32,
    end_line: u32,
}

/// Match found in file
#[derive(Debug, Clone)]
struct Match {
    line: u32,
    column: u32,
    text: String,
}