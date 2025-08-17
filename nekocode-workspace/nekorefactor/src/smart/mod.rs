//! Smart refactoring with Tree-sitter AST integration

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use nekocode_core::{Result, NekocodeError, Session, Language};
use serde::{Serialize, Deserialize};

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
    
    async fn get_ast_info(&self, _session: &Session, _file: &Path) -> Result<AstInfo> {
        // TODO: This would call nekocode to get AST
        // For now, return mock data for testing
        Ok(AstInfo {
            language: Language::Python,
            functions: vec![
                FunctionInfo {
                    name: "main".to_string(),
                    start_line: 7,
                    end_line: 9,
                },
            ],
            classes: vec![
                ClassInfo {
                    name: "TestClass".to_string(),
                    start_line: 11,
                    end_line: 16,
                },
            ],
            imports: vec!["os".to_string(), "sys".to_string()],
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
        // Find all matches in file within optional range
        // Placeholder implementation
        Ok(vec![])
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
        // Apply all replacements
        // Placeholder implementation
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
        // Extract code for the symbol
        // Placeholder implementation
        Ok("// Symbol code here".to_string())
    }
    
    fn apply_move(&self, symbol_info: &nekocode_core::SymbolInfo, target: &Path, code: &str, update_imports: bool) -> Result<()> {
        // Apply the move operation
        // Placeholder implementation
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
#[derive(Debug)]
struct Match {
    line: u32,
    column: u32,
    text: String,
}