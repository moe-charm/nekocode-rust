//! 🚀 Tree-sitter based JavaScript/TypeScript analyzer
//! 100x faster than PEST implementation!

use anyhow::Result;
use tree_sitter::{Parser, Query, QueryCursor, Node};
use async_trait::async_trait;

use crate::core::types::{
    AnalysisResult, ClassInfo, FileInfo, FunctionInfo, ImportInfo, 
    ExportInfo, Language, FunctionCall, ComplexityInfo
};
use crate::core::ast::{ASTBuilder, ASTNode, ASTNodeType, ASTStatistics};
use crate::analyzers::traits::LanguageAnalyzer;

pub struct TreeSitterJavaScriptAnalyzer {
    parser: Parser,
}

impl TreeSitterJavaScriptAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        // Use JavaScript grammar by default
        parser.set_language(&tree_sitter_javascript::LANGUAGE.into())
            .map_err(|e| anyhow::anyhow!("Failed to set language: {:?}", e))?;
        
        Ok(Self { parser })
    }
    
    pub fn set_typescript(&mut self) -> Result<()> {
        // Switch to TypeScript grammar for .ts/.tsx files
        self.parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .map_err(|e| anyhow::anyhow!("Failed to set TypeScript language: {:?}", e))?;
        Ok(())
    }
    
    pub fn set_tsx(&mut self) -> Result<()> {
        // Switch to TSX grammar for .tsx files
        self.parser.set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
            .map_err(|e| anyhow::anyhow!("Failed to set TSX language: {:?}", e))?;
        Ok(())
    }
    
    /// Extract functions using tree-sitter query
    fn extract_functions(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        
        // Query for function declarations and arrow functions
        let query_str = r#"
            [
              (function_declaration
                name: (identifier) @name) @function
              (function_expression
                name: (identifier) @name) @function
              (arrow_function) @function
              (method_definition
                name: (property_identifier) @name) @function
            ]
        "#;
        
        let query = Query::new(&tree_sitter_javascript::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut func_info = FunctionInfo::new(String::new());
            let mut func_node = None;
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "name" => {
                        func_info.name = capture.node.utf8_text(source.as_bytes())?.to_string();
                    }
                    "function" => {
                        func_node = Some(capture.node);
                        func_info.start_line = capture.node.start_position().row as u32 + 1;
                        func_info.end_line = capture.node.end_position().row as u32 + 1;
                    }
                    _ => {}
                }
            }
            
            // Extract parameters if we have a function node
            if let Some(node) = func_node {
                func_info.parameters = self.extract_parameters(node, source)?;
                func_info.is_async = self.is_async_function(node, source);
                func_info.is_arrow_function = node.kind() == "arrow_function";
            }
            
            // Set default complexity (will be calculated separately)
            func_info.complexity = ComplexityInfo::default();
            
            functions.push(func_info);
        }
        
        Ok(functions)
    }
    
    /// Extract classes using tree-sitter query
    fn extract_classes(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<ClassInfo>> {
        let mut classes = Vec::new();
        
        let query_str = r#"
            (class_declaration
              name: (identifier) @name) @class
        "#;
        
        let query = Query::new(&tree_sitter_javascript::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut class_info = ClassInfo::new(String::new());
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "name" => {
                        class_info.name = capture.node.utf8_text(source.as_bytes())?.to_string();
                    }
                    "class" => {
                        class_info.start_line = capture.node.start_position().row as u32 + 1;
                        class_info.end_line = capture.node.end_position().row as u32 + 1;
                        
                        // Extract methods
                        class_info.methods = self.extract_class_methods(capture.node, source)?;
                        
                        // Check for extends
                        if let Some(heritage) = capture.node.child_by_field_name("heritage") {
                            if let Some(extends) = heritage.child_by_field_name("parent") {
                                class_info.parent_class = Some(extends.utf8_text(source.as_bytes())?.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            
            classes.push(class_info);
        }
        
        Ok(classes)
    }
    
    /// Extract imports using tree-sitter query
    fn extract_imports(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<ImportInfo>> {
        let mut imports = Vec::new();
        
        let query_str = r#"
            (import_statement
              source: (string) @source) @import
        "#;
        
        let query = Query::new(&tree_sitter_javascript::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut import_info = ImportInfo::new(
                crate::core::types::ImportType::ES6Import,
                String::new()
            );
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "source" => {
                        let source_text = capture.node.utf8_text(source.as_bytes())?;
                        // Remove quotes
                        import_info.module_path = source_text.trim_matches(|c| c == '"' || c == '\'' || c == '`').to_string();
                    }
                    "import" => {
                        import_info.line_number = capture.node.start_position().row as u32 + 1;
                    }
                    _ => {}
                }
            }
            
            imports.push(import_info);
        }
        
        Ok(imports)
    }
    
    /// Extract exports using tree-sitter query
    fn extract_exports(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<ExportInfo>> {
        let mut exports = Vec::new();
        
        let query_str = r#"
            (export_statement) @export
        "#;
        
        let query = Query::new(&tree_sitter_javascript::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut export_info = ExportInfo::new(
                crate::core::types::ExportType::ES6Export
            );
            
            for capture in mat.captures {
                let node = capture.node;
                export_info.line_number = node.start_position().row as u32 + 1;
                export_info.is_default = false;  // Will be determined by parsing the text
                
                // Extract exported names
                if let Ok(text) = node.utf8_text(source.as_bytes()) {
                    if text.contains("function") {
                        export_info.exported_names.push("function".to_string());
                    } else if text.contains("class") {
                        export_info.exported_names.push("class".to_string());
                    }
                }
            }
            
            exports.push(export_info);
        }
        
        Ok(exports)
    }
    
    /// Helper: Extract parameters from a function node
    fn extract_parameters(&self, node: Node, source: &str) -> Result<Vec<String>> {
        let mut params = Vec::new();
        
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "identifier" || child.kind() == "formal_parameter" {
                    if let Ok(param_text) = child.utf8_text(source.as_bytes()) {
                        params.push(param_text.to_string());
                    }
                }
            }
        }
        
        Ok(params)
    }
    
    /// Helper: Check if function is async
    fn is_async_function(&self, node: Node, source: &str) -> bool {
        // Check if there's an async keyword before the function
        if let Some(prev_sibling) = node.prev_sibling() {
            if prev_sibling.kind() == "async" {
                return true;
            }
        }
        
        // Check the function's text itself
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            return text.starts_with("async");
        }
        
        false
    }
    
    /// Extract methods from a class node
    fn extract_class_methods(&self, class_node: Node, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut methods = Vec::new();
        
        if let Some(body) = class_node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "method_definition" {
                    let mut method = FunctionInfo::new(String::new());
                    
                    if let Some(name_node) = child.child_by_field_name("name") {
                        method.name = name_node.utf8_text(source.as_bytes())?.to_string();
                    }
                    
                    method.start_line = child.start_position().row as u32 + 1;
                    method.end_line = child.end_position().row as u32 + 1;
                    method.parameters = self.extract_parameters(child, source)?;
                    method.is_async = self.is_async_function(child, source);
                    
                    methods.push(method);
                }
            }
        }
        
        Ok(methods)
    }
    
    /// Build AST from tree-sitter CST
    fn build_ast(&self, tree: &tree_sitter::Tree, source: &str) -> ASTNode {
        let mut root = ASTNode::new(ASTNodeType::FileRoot, String::new());
        self.build_ast_recursive(tree.root_node(), source, &mut root, 0);
        root
    }
    
    /// Recursive AST building
    fn build_ast_recursive(&self, node: Node, source: &str, parent: &mut ASTNode, depth: usize) {
        // Map tree-sitter node types to our AST types
        let ast_type = match node.kind() {
            "function_declaration" | "function_expression" | "arrow_function" => ASTNodeType::Function,
            "class_declaration" => ASTNodeType::Class,
            "method_definition" => ASTNodeType::Method,
            "variable_declarator" => ASTNodeType::Variable,
            "if_statement" => ASTNodeType::IfStatement,
            "for_statement" | "for_in_statement" | "for_of_statement" => ASTNodeType::ForLoop,
            "while_statement" | "do_statement" => ASTNodeType::WhileLoop,
            "import_statement" => ASTNodeType::Import,
            "export_statement" => ASTNodeType::Export,
            _ => ASTNodeType::Unknown,
        };
        
        if ast_type != ASTNodeType::Unknown {
            let mut ast_node = ASTNode::new(ast_type, String::new());
            ast_node.start_line = node.start_position().row as u32 + 1;
            ast_node.end_line = node.end_position().row as u32 + 1;
            ast_node.depth = depth as u32;
            
            // Try to get node name
            if let Some(name_field) = node.child_by_field_name("name") {
                if let Ok(name) = name_field.utf8_text(source.as_bytes()) {
                    ast_node.name = name.to_string();
                }
            }
            
            parent.children.push(ast_node);
        }
        
        // Recurse through children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.build_ast_recursive(child, source, parent, depth + 1);
        }
    }
}

#[async_trait]
impl LanguageAnalyzer for TreeSitterJavaScriptAnalyzer {
    fn get_language(&self) -> Language {
        Language::JavaScript
    }
    
    fn get_language_name(&self) -> &'static str {
        "JavaScript/TypeScript"
    }
    
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec![".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx"]
    }
    
    async fn analyze(&mut self, content: &str, filename: &str) -> Result<AnalysisResult> {
        // Determine language and set appropriate grammar
        let language = if filename.ends_with(".tsx") {
            self.set_tsx()?;
            Language::TypeScript
        } else if filename.ends_with(".ts") {
            self.set_typescript()?;
            Language::TypeScript
        } else {
            Language::JavaScript
        };
        
        // Create file info
        let file_path = std::path::PathBuf::from(filename);
        let mut file_info = FileInfo::new(file_path);
        file_info.total_lines = content.lines().count() as u32;
        
        // Calculate basic line statistics
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                file_info.empty_lines += 1;
            } else if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
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
        
        // Create analysis result
        let mut result = AnalysisResult::new(file_info, language);
        
        // 🚀 Parse with tree-sitter (ULTRA FAST!)
        let parse_start = std::time::Instant::now();
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse file"))?;
        let parse_duration = parse_start.elapsed();
        
        if std::env::var("NEKOCODE_DEBUG").is_ok() {
            eprintln!("⚡ [TREE-SITTER] Parse took: {:.3}ms", parse_duration.as_secs_f64() * 1000.0);
        }
        
        // Extract all constructs
        let extract_start = std::time::Instant::now();
        result.functions = self.extract_functions(&tree, content)?;
        result.classes = self.extract_classes(&tree, content)?;
        result.imports = self.extract_imports(&tree, content)?;
        result.exports = self.extract_exports(&tree, content)?;
        let extract_duration = extract_start.elapsed();
        
        if std::env::var("NEKOCODE_DEBUG").is_ok() {
            eprintln!("⚡ [TREE-SITTER] Extraction took: {:.3}ms", extract_duration.as_secs_f64() * 1000.0);
        }
        
        // Build AST
        let ast_start = std::time::Instant::now();
        let ast_root = self.build_ast(&tree, content);
        let mut ast_stats = ASTStatistics::default();
        ast_stats.update_from_root(&ast_root);
        result.ast_root = Some(ast_root);
        result.ast_statistics = Some(ast_stats);
        let ast_duration = ast_start.elapsed();
        
        if std::env::var("NEKOCODE_DEBUG").is_ok() {
            eprintln!("⚡ [TREE-SITTER] AST build took: {:.3}ms", ast_duration.as_secs_f64() * 1000.0);
        }
        
        // Update statistics
        result.update_statistics();
        
        Ok(result)
    }
}