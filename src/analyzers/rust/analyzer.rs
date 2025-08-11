//! Rust analyzer implementation
use anyhow::Result;
use async_trait::async_trait;

use crate::core::types::{
    AnalysisResult, ClassInfo, ComplexityInfo, FileInfo, FunctionInfo, ImportInfo, 
    ImportType, Language
};
use crate::analyzers::traits::LanguageAnalyzer;

pub struct RustAnalyzer;

impl RustAnalyzer {
    pub fn new() -> Self {
        Self
    }
    
    fn calculate_complexity(&self, content: &str) -> ComplexityInfo {
        let mut complexity = ComplexityInfo::new();
        
        let complexity_keywords = [
            "if ", "else ", "for ", "while ", "loop ", "match ", 
            "if let", "while let", "&&", "||", "?", "async ", "await"
        ];
        
        for keyword in &complexity_keywords {
            complexity.cyclomatic_complexity += content.matches(keyword).count() as u32;
        }
        
        // Count match arms which add complexity
        complexity.cyclomatic_complexity += content.matches(" => ").count() as u32;
        
        // Calculate nesting depth
        let mut current_depth = 0;
        let mut max_depth = 0;
        
        for ch in content.chars() {
            match ch {
                '{' => {
                    current_depth += 1;
                    max_depth = max_depth.max(current_depth);
                }
                '}' => {
                    if current_depth > 0 {
                        current_depth -= 1;
                    }
                }
                _ => {}
            }
        }
        
        complexity.max_nesting_depth = max_depth;
        complexity.update_rating();
        complexity
    }
    
    fn extract_functions(&self, content: &str) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        
        if let Ok(re) = regex::Regex::new(r"(?:pub\s+)?(?:async\s+)?fn\s+(\w+)") {
            for caps in re.captures_iter(content) {
                if let Some(name) = caps.get(1) {
                    let mut func = FunctionInfo::new(name.as_str().to_string());
                    let full_match = caps.get(0).unwrap().as_str();
                    
                    if full_match.contains("async") {
                        func.is_async = true;
                    }
                    if full_match.contains("pub") {
                        func.metadata.insert("is_public".to_string(), "true".to_string());
                    }
                    
                    functions.push(func);
                }
            }
        }
        
        functions
    }
    
    fn extract_structs(&self, content: &str) -> Vec<ClassInfo> {
        let mut items = Vec::new();
        
        // Extract structs
        if let Ok(re) = regex::Regex::new(r"(?:pub\s+)?struct\s+(\w+)") {
            for caps in re.captures_iter(content) {
                if let Some(name) = caps.get(1) {
                    let mut class = ClassInfo::new(name.as_str().to_string());
                    class.metadata.insert("type".to_string(), "struct".to_string());
                    items.push(class);
                }
            }
        }
        
        // Extract enums
        if let Ok(re) = regex::Regex::new(r"(?:pub\s+)?enum\s+(\w+)") {
            for caps in re.captures_iter(content) {
                if let Some(name) = caps.get(1) {
                    let mut class = ClassInfo::new(name.as_str().to_string());
                    class.metadata.insert("type".to_string(), "enum".to_string());
                    items.push(class);
                }
            }
        }
        
        // Extract traits
        if let Ok(re) = regex::Regex::new(r"(?:pub\s+)?trait\s+(\w+)") {
            for caps in re.captures_iter(content) {
                if let Some(name) = caps.get(1) {
                    let mut class = ClassInfo::new(name.as_str().to_string());
                    class.metadata.insert("type".to_string(), "trait".to_string());
                    items.push(class);
                }
            }
        }
        
        items
    }
    
    fn extract_imports(&self, content: &str) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        
        if let Ok(re) = regex::Regex::new(r"use\s+([^;]+);") {
            for caps in re.captures_iter(content) {
                if let Some(use_path) = caps.get(1) {
                    let path = use_path.as_str().trim();
                    if !path.is_empty() {
                        imports.push(ImportInfo::new(ImportType::RustUse, path.to_string()));
                    }
                }
            }
        }
        
        imports
    }
    
    /// Build AST from analysis results (following C++ adapter pattern)
    fn build_ast_from_analysis(&self, functions: &[FunctionInfo], classes: &[ClassInfo], content: &str) -> crate::core::ast::ASTNode {
        use crate::core::ast::{ASTBuilder, ASTNodeType};
        
        let mut builder = ASTBuilder::new();
        let lines: Vec<&str> = content.lines().collect();
        
        // Classes/structs and their methods (following C++ pattern)
        for class in classes {
            let start_line = self.find_line_number(&lines, &class.name, "struct").unwrap_or(class.start_line);
            builder.enter_scope(ASTNodeType::Class, class.name.clone(), start_line);
            
            // Methods within the struct body (from impl blocks)
            for method in &class.methods {
                let method_start = method.start_line.max(start_line);
                builder.add_node(ASTNodeType::Method, method.name.clone(), method_start);
            }
            
            let class_end = class.end_line.max(start_line + 1);
            builder.exit_scope(class_end);
        }
        
        // Top-level functions (exclude impl methods)
        for func in functions {
            if matches!(func.metadata.get("is_method"), Some(v) if v == "true") {
                continue; // Skip impl methods, they're already added above
            }
            
            let func_start = self.find_line_number(&lines, &func.name, "fn")
                .unwrap_or(func.start_line.max(1));
            builder.add_node(ASTNodeType::Function, func.name.clone(), func_start);
        }
        
        builder.build()
    }
    
    /// Find line number for a given symbol name and keyword
    fn find_line_number(&self, lines: &[&str], name: &str, keyword: &str) -> Option<u32> {
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if keyword.is_empty() {
                // For functions, just check if the line contains the name
                if trimmed.contains(name) && !trimmed.starts_with("//") && !trimmed.starts_with("/*") {
                    return Some((i + 1) as u32);
                }
            } else {
                // For structs/fns, check for keyword and name
                if trimmed.starts_with(keyword) && trimmed.contains(name) {
                    return Some((i + 1) as u32);
                }
            }
        }
        None
    }
}

#[async_trait]
impl LanguageAnalyzer for RustAnalyzer {
    fn get_language(&self) -> Language {
        Language::Rust
    }
    
    fn get_language_name(&self) -> &'static str {
        "Rust"
    }
    
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec![".rs"]
    }
    
    async fn analyze(&mut self, content: &str, filename: &str) -> Result<AnalysisResult> {
        let file_path = std::path::PathBuf::from(filename);
        let mut file_info = FileInfo::new(file_path);
        file_info.total_lines = content.lines().count() as u32;
        
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                file_info.empty_lines += 1;
            } else if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                file_info.comment_lines += 1;
            } else {
                file_info.code_lines += 1;
            }
        }
        
        file_info.code_ratio = if file_info.total_lines > 0 {
            file_info.code_lines as f64 / file_info.total_lines as f64
        } else {
            0.0
        };
        
        let mut result = AnalysisResult::new(file_info, Language::Rust);
        
        result.functions = self.extract_functions(content);
        result.classes = self.extract_structs(content);
        result.imports = self.extract_imports(content);
        
        // Build AST from analysis results (following C++ pattern)
        if !result.functions.is_empty() || !result.classes.is_empty() {
            let ast_root = self.build_ast_from_analysis(&result.functions, &result.classes, content);
            let mut ast_stats = crate::core::ast::ASTStatistics::default();
            ast_stats.update_from_root(&ast_root);
            result.ast_root = Some(ast_root);
            result.ast_statistics = Some(ast_stats);
        }
        
        result.complexity = self.calculate_complexity(content);
        
        result.update_statistics();
        Ok(result)
    }
}

impl Default for RustAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}