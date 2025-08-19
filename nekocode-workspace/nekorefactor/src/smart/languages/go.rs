//! Go-specific rules for smart refactoring

use nekocode_core::{Result};
use crate::smart::{AstInfo, InsertPoint, Indent};
use super::{LanguageRules, find_function_by_name};

pub struct GoRules;

impl GoRules {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageRules for GoRules {
    fn find_function_end(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint> {
        let func = find_function_by_name(ast_info, name)?;
        
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
        // Go uses structs and methods, not classes
        Ok(InsertPoint {
            line: 1,
            column: 0,
            indent_level: 0,
        })
    }
    
    fn find_imports_insert_point(&self, _ast_info: &AstInfo) -> Result<InsertPoint> {
        // Go uses import statements
        Ok(InsertPoint {
            line: 1,
            column: 0,
            indent_level: 0,
        })
    }
    
    fn detect_indentation(&self, _ast_info: &AstInfo, point: &InsertPoint) -> Indent {
        // Go uses tabs
        if point.indent_level > 0 {
            Indent::Tabs(point.indent_level)
        } else {
            Indent::Tabs(0)
        }
    }
    
    fn default_indentation(&self) -> Indent {
        Indent::Tabs(1)  // Go standard
    }
    
    fn is_comment_line(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("//") || trimmed.starts_with("/*")
    }
}