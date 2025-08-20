//! Python-specific rules for smart refactoring

use nekocode_core::Result;
use crate::smart::{AstInfo, InsertPoint, Indent};
use super::{LanguageRules, find_function_by_name, find_class_by_name};

pub struct PythonRules;

impl PythonRules {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageRules for PythonRules {
    fn find_function_end(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint> {
        let func = find_function_by_name(ast_info, name)?;
        
        // For Python, we need to insert after the function body
        // with proper indentation (at the same level as 'def')
        
        // Find the next function or class at the same indentation level
        // or the end of file
        let mut insert_line = func.end_line + 1;
        
        // Check if there's another function/class after this one
        for other_func in &ast_info.functions {
            if other_func.start_line > func.end_line && other_func.start_line < insert_line {
                // Insert before the next function, leaving a blank line
                insert_line = other_func.start_line;
                break;
            }
        }
        
        for class in &ast_info.classes {
            if class.start_line > func.end_line && class.start_line < insert_line {
                // Insert before the next class, leaving a blank line
                insert_line = class.start_line;
                break;
            }
        }
        
        Ok(InsertPoint {
            line: insert_line,
            column: 0,  // Python functions start at column 0 (module level) or indented (class methods)
            indent_level: 0,  // Will be detected from context
        })
    }
    
    fn find_function_start(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint> {
        let func = find_function_by_name(ast_info, name)?;
        
        // Insert before the function definition
        // Check for decorators and docstrings
        let start_line = func.start_line;
        
        // TODO: Check for decorators by looking at lines before
        // For now, just insert at the function definition line
        
        Ok(InsertPoint {
            line: start_line,
            column: 0,
            indent_level: 0,
        })
    }
    
    fn find_class_insert_point(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint> {
        let class = find_class_by_name(ast_info, name)?;
        
        // For Python classes, find a good insertion point inside the class
        // Typically after __init__ or at the end of the class
        
        // Find the last method in the class
        let mut last_method_end = class.start_line;
        for func in &ast_info.functions {
            // Check if function is within the class boundaries
            if func.start_line > class.start_line && 
               func.end_line < class.end_line &&
               func.end_line > last_method_end {
                last_method_end = func.end_line;
            }
        }
        
        // Insert after the last method, or after the class definition if empty
        let insert_line = if last_method_end > class.start_line {
            last_method_end + 1
        } else {
            class.start_line + 1  // After class definition line
        };
        
        Ok(InsertPoint {
            line: insert_line,
            column: 4,  // Standard Python class method indentation
            indent_level: 1,
        })
    }
    
    fn find_imports_insert_point(&self, ast_info: &AstInfo) -> Result<InsertPoint> {
        // Find the last import statement, or insert at the beginning
        let last_import_line = 0u32;
        let mut found_imports = false;
        
        for import in &ast_info.imports {
            found_imports = true;
            // Parse the import string to get line number
            // For now, we'll use a simple approach
            // TODO: Enhance this with actual line numbers from AST
        }
        
        if found_imports && last_import_line > 0 {
            // Insert after the last import
            Ok(InsertPoint {
                line: last_import_line + 1,
                column: 0,
                indent_level: 0,
            })
        } else {
            // No imports found, insert at the beginning
            // But after module docstring if present
            Ok(InsertPoint {
                line: 1,  // At the beginning
                column: 0,
                indent_level: 0,
            })
        }
    }
    
    fn detect_indentation(&self, ast_info: &AstInfo, point: &InsertPoint) -> Indent {
        // Python standard is 4 spaces (PEP 8)
        // But we should detect from the file if possible
        
        // For now, use standard Python indentation
        if point.indent_level > 0 {
            Indent::Spaces(4 * point.indent_level)
        } else {
            Indent::Spaces(0)
        }
    }
    
    fn default_indentation(&self) -> Indent {
        Indent::Spaces(4)  // PEP 8 standard
    }
    
    fn is_comment_line(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with('#') || 
        trimmed.starts_with("'''") || 
        trimmed.starts_with("\"\"\"")
    }
}