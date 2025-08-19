//! JavaScript-specific rules for smart refactoring

use nekocode_core::{Result};
use crate::smart::{AstInfo, InsertPoint, Indent};
use super::{LanguageRules, find_function_by_name, find_class_by_name};

pub struct JavaScriptRules;

impl JavaScriptRules {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageRules for JavaScriptRules {
    fn find_function_end(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint> {
        let func = find_function_by_name(ast_info, name)?;
        
        // JavaScript functions end with a closing brace
        // Insert after the function, maintaining module-level indentation
        Ok(InsertPoint {
            line: func.end_line + 1,
            column: 0,
            indent_level: 0,
        })
    }
    
    fn find_function_start(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint> {
        let func = find_function_by_name(ast_info, name)?;
        
        Ok(InsertPoint {
            line: func.start_line,
            column: 0,
            indent_level: 0,
        })
    }
    
    fn find_class_insert_point(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint> {
        let class = find_class_by_name(ast_info, name)?;
        
        // Find the last method in the class
        Ok(InsertPoint {
            line: class.end_line,  // Before the closing brace
            column: 2,  // Standard JS indentation
            indent_level: 1,
        })
    }
    
    fn find_imports_insert_point(&self, ast_info: &AstInfo) -> Result<InsertPoint> {
        // Find the last import/require statement
        Ok(InsertPoint {
            line: 1,
            column: 0,
            indent_level: 0,
        })
    }
    
    fn detect_indentation(&self, _ast_info: &AstInfo, point: &InsertPoint) -> Indent {
        // JavaScript commonly uses 2 spaces
        if point.indent_level > 0 {
            Indent::Spaces(2 * point.indent_level)
        } else {
            Indent::Spaces(0)
        }
    }
    
    fn default_indentation(&self) -> Indent {
        Indent::Spaces(2)
    }
    
    fn is_comment_line(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*")
    }
}