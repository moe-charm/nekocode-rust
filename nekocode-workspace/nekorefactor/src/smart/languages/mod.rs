//! Language-specific rules for smart refactoring

use nekocode_core::{Language, Result, NekocodeError};
use crate::smart::{AstInfo, InsertPoint, Indent};

mod python;
mod javascript;
mod typescript;
mod rust;
mod go;
mod cpp;
mod csharp;

/// Language-specific rules trait
pub trait LanguageRules: Send + Sync {
    /// Find the end of a function for insertion
    fn find_function_end(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint>;
    
    /// Find the start of a function for insertion
    fn find_function_start(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint>;
    
    /// Find insertion point within a class
    fn find_class_insert_point(&self, ast_info: &AstInfo, name: &str) -> Result<InsertPoint>;
    
    /// Find insertion point in imports section
    fn find_imports_insert_point(&self, ast_info: &AstInfo) -> Result<InsertPoint>;
    
    /// Detect indentation at a given point
    fn detect_indentation(&self, ast_info: &AstInfo, point: &InsertPoint) -> Indent;
    
    /// Get default indentation for this language
    fn default_indentation(&self) -> Indent;
    
    /// Check if a line is a comment
    fn is_comment_line(&self, line: &str) -> bool;
    
    /// Get the preferred line ending
    fn line_ending(&self) -> &'static str {
        "\n"
    }
}

/// Get language-specific rules
pub fn get_rules(language: Language) -> Result<Box<dyn LanguageRules>> {
    match language {
        Language::Python => Ok(Box::new(python::PythonRules::new())),
        Language::JavaScript => Ok(Box::new(javascript::JavaScriptRules::new())),
        Language::TypeScript => Ok(Box::new(typescript::TypeScriptRules::new())),
        Language::Rust => Ok(Box::new(rust::RustRules::new())),
        Language::Go => Ok(Box::new(go::GoRules::new())),
        Language::Cpp => Ok(Box::new(cpp::CppRules::new())),
        Language::CSharp => Ok(Box::new(csharp::CSharpRules::new())),
        _ => Err(NekocodeError::Refactoring(
            format!("Language {:?} not yet supported for smart refactoring", language)
        )),
    }
}

/// Common helper for finding function in AST
pub(crate) fn find_function_by_name<'a>(
    ast_info: &'a AstInfo,
    name: &str
) -> Result<&'a crate::smart::FunctionInfo> {
    ast_info.functions
        .iter()
        .find(|f| f.name == name)
        .ok_or_else(|| NekocodeError::Refactoring(
            format!("Function '{}' not found", name)
        ))
}

/// Common helper for finding class in AST
pub(crate) fn find_class_by_name<'a>(
    ast_info: &'a AstInfo,
    name: &str
) -> Result<&'a crate::smart::ClassInfo> {
    ast_info.classes
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| NekocodeError::Refactoring(
            format!("Class '{}' not found", name)
        ))
}