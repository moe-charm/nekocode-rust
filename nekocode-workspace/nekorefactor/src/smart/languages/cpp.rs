//! C++-specific rules for smart refactoring

use nekocode_core::{Result};
use crate::smart::{AstInfo, InsertPoint, Indent};
use super::{LanguageRules, find_function_by_name, find_class_by_name};

pub struct CppRules;

impl CppRules {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageRules for CppRules {
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
        let class = find_class_by_name(ast_info, name)?;
        
        // Insert at the end of public section or end of class
        Ok(InsertPoint {
            line: class.end_line,
            column: 4,
            indent_level: 1,
        })
    }
    
    fn find_imports_insert_point(&self, _ast_info: &AstInfo) -> Result<InsertPoint> {
        // C++ uses #include
        Ok(InsertPoint {
            line: 1,
            column: 0,
            indent_level: 0,
        })
    }
    
    fn detect_indentation(&self, _ast_info: &AstInfo, point: &InsertPoint) -> Indent {
        if point.indent_level > 0 {
            Indent::Spaces(4 * point.indent_level)
        } else {
            Indent::Spaces(0)
        }
    }
    
    fn default_indentation(&self) -> Indent {
        Indent::Spaces(4)
    }
    
    fn is_comment_line(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("//") || trimmed.starts_with("/*")
    }
}