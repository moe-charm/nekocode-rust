//! 🚀 Tree-sitter based Python analyzer
//! 100x faster than PEST implementation!

use anyhow::Result;
use tree_sitter::{Parser, Query, QueryCursor, Node};
use async_trait::async_trait;

use crate::core::types::{
    AnalysisResult, ClassInfo, FileInfo, FunctionInfo, ImportInfo, 
    Language, ComplexityInfo, ImportType
};
use crate::core::ast::{ASTNode, ASTNodeType, ASTStatistics};
use crate::analyzers::traits::LanguageAnalyzer;

pub struct TreeSitterPythonAnalyzer {
    parser: Parser,
}

impl TreeSitterPythonAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|e| anyhow::anyhow!("Failed to set Python language: {:?}", e))?;
        
        Ok(Self { parser })
    }
    
    /// Extract functions using tree-sitter query
    fn extract_functions(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        
        // Query for function definitions and lambda functions
        let query_str = r#"
            [
              (function_definition
                name: (identifier) @name) @function
              (lambda 
                parameters: (parameters)? @params) @function
            ]
        "#;
        
        let query = Query::new(&tree_sitter_python::LANGUAGE.into(), query_str)?;
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
                        
                        // Handle lambda functions (which don't have names)
                        if capture.node.kind() == "lambda" && func_info.name.is_empty() {
                            func_info.name = "lambda".to_string();
                            func_info.metadata.insert("is_lambda".to_string(), "true".to_string());
                        }
                    }
                    _ => {}
                }
            }
            
            // Extract parameters if we have a function node
            if let Some(node) = func_node {
                func_info.parameters = self.extract_parameters(node, source)?;
                func_info.is_async = self.is_async_function(node, source);
                
                // Check for decorators
                if let Some(parent) = node.parent() {
                    func_info.metadata.extend(self.extract_decorators(parent, source)?);
                }
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
            (class_definition
              name: (identifier) @name
              superclasses: (argument_list)? @superclasses) @class
        "#;
        
        let query = Query::new(&tree_sitter_python::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut class_info = ClassInfo::new(String::new());
            let mut class_node = None;
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "name" => {
                        class_info.name = capture.node.utf8_text(source.as_bytes())?.to_string();
                    }
                    "superclasses" => {
                        let parent_classes = self.extract_superclasses(capture.node, source)?;
                        if !parent_classes.is_empty() {
                            class_info.parent_class = Some(parent_classes[0].clone());
                            if parent_classes.len() > 1 {
                                class_info.metadata.insert("multiple_inheritance".to_string(), parent_classes.join(", "));
                            }
                        }
                    }
                    "class" => {
                        class_node = Some(capture.node);
                        class_info.start_line = capture.node.start_position().row as u32 + 1;
                        class_info.end_line = capture.node.end_position().row as u32 + 1;
                    }
                    _ => {}
                }
            }
            
            // Extract methods and decorators
            if let Some(node) = class_node {
                class_info.methods = self.extract_class_methods(node, source)?;
                
                // Check for decorators
                if let Some(parent) = node.parent() {
                    class_info.metadata.extend(self.extract_decorators(parent, source)?);
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
            [
              (import_statement
                name: (dotted_name) @module) @import
              (import_from_statement
                module_name: (dotted_name)? @module
                name: (import_list)? @names) @from_import
            ]
        "#;
        
        let query = Query::new(&tree_sitter_python::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut import_info = ImportInfo::new(ImportType::PythonImport, String::new());
            let mut is_from_import = false;
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "module" => {
                        import_info.module_path = capture.node.utf8_text(source.as_bytes())?.to_string();
                    }
                    "names" => {
                        import_info.imported_names = self.extract_imported_names(capture.node, source)?;
                    }
                    "from_import" => {
                        is_from_import = true;
                        import_info.import_type = ImportType::PythonFromImport;
                        import_info.line_number = capture.node.start_position().row as u32 + 1;
                    }
                    "import" => {
                        if !is_from_import {
                            import_info.line_number = capture.node.start_position().row as u32 + 1;
                        }
                    }
                    _ => {}
                }
            }
            
            imports.push(import_info);
        }
        
        Ok(imports)
    }
    
    /// Helper: Extract parameters from a function node
    fn extract_parameters(&self, node: Node, source: &str) -> Result<Vec<String>> {
        let mut params = Vec::new();
        
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                match child.kind() {
                    "identifier" => {
                        if let Ok(param_text) = child.utf8_text(source.as_bytes()) {
                            params.push(param_text.to_string());
                        }
                    }
                    "typed_parameter" => {
                        if let Some(name_child) = child.child_by_field_name("name") {
                            if let Ok(param_text) = name_child.utf8_text(source.as_bytes()) {
                                params.push(param_text.to_string());
                            }
                        }
                    }
                    "default_parameter" => {
                        if let Some(name_child) = child.child_by_field_name("name") {
                            if let Ok(param_text) = name_child.utf8_text(source.as_bytes()) {
                                params.push(param_text.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        Ok(params)
    }
    
    /// Helper: Check if function is async
    fn is_async_function(&self, node: Node, _source: &str) -> bool {
        // Check if there's an async keyword before the function
        if let Some(prev_sibling) = node.prev_sibling() {
            if prev_sibling.kind() == "async" {
                return true;
            }
        }
        false
    }
    
    /// Extract superclasses from argument list
    fn extract_superclasses(&self, node: Node, source: &str) -> Result<Vec<String>> {
        let mut superclasses = Vec::new();
        
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "attribute" {
                if let Ok(class_name) = child.utf8_text(source.as_bytes()) {
                    superclasses.push(class_name.to_string());
                }
            }
        }
        
        Ok(superclasses)
    }
    
    /// Extract methods from a class node
    fn extract_class_methods(&self, class_node: Node, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut methods = Vec::new();
        
        if let Some(body) = class_node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "function_definition" {
                    let mut method = FunctionInfo::new(String::new());
                    
                    if let Some(name_node) = child.child_by_field_name("name") {
                        method.name = name_node.utf8_text(source.as_bytes())?.to_string();
                    }
                    
                    method.start_line = child.start_position().row as u32 + 1;
                    method.end_line = child.end_position().row as u32 + 1;
                    method.parameters = self.extract_parameters(child, source)?;
                    method.is_async = self.is_async_function(child, source);
                    method.metadata.insert("is_class_method".to_string(), "true".to_string());
                    
                    // Check for special methods
                    if method.name.starts_with("__") && method.name.ends_with("__") {
                        method.metadata.insert("is_dunder_method".to_string(), "true".to_string());
                    }
                    
                    // Check for decorators
                    if let Some(parent) = child.parent() {
                        method.metadata.extend(self.extract_decorators(parent, source)?);
                    }
                    
                    methods.push(method);
                }
            }
        }
        
        Ok(methods)
    }
    
    /// Extract decorators from a decorated definition
    fn extract_decorators(&self, node: Node, source: &str) -> Result<std::collections::HashMap<String, String>> {
        let mut metadata = std::collections::HashMap::new();
        let mut decorators = Vec::new();
        
        if node.kind() == "decorated_definition" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "decorator" {
                    if let Ok(decorator_text) = child.utf8_text(source.as_bytes()) {
                        decorators.push(decorator_text.to_string());
                    }
                }
            }
        }
        
        if !decorators.is_empty() {
            metadata.insert("decorators".to_string(), decorators.join(", "));
        }
        
        Ok(metadata)
    }
    
    /// Extract imported names from import list
    fn extract_imported_names(&self, node: Node, source: &str) -> Result<Vec<String>> {
        let mut names = Vec::new();
        
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" | "dotted_name" => {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        names.push(name.to_string());
                    }
                }
                "aliased_import" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                            names.push(name.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(names)
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
            "function_definition" => ASTNodeType::Function,
            "class_definition" => ASTNodeType::Class,
            "if_statement" => ASTNodeType::IfStatement,
            "for_statement" | "while_statement" => ASTNodeType::ForLoop,
            "import_statement" | "import_from_statement" => ASTNodeType::Import,
            "assignment" => ASTNodeType::Variable,
            "lambda" => ASTNodeType::Function,
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
impl LanguageAnalyzer for TreeSitterPythonAnalyzer {
    fn get_language(&self) -> Language {
        Language::Python
    }
    
    fn get_language_name(&self) -> &'static str {
        "Python (Tree-sitter)"
    }
    
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec![".py", ".pyw", ".pyi"]
    }
    
    async fn analyze(&mut self, content: &str, filename: &str) -> Result<AnalysisResult> {
        // Create file info
        let file_path = std::path::PathBuf::from(filename);
        let mut file_info = FileInfo::new(file_path);
        file_info.total_lines = content.lines().count() as u32;
        
        // Calculate basic line statistics
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                file_info.empty_lines += 1;
            } else if trimmed.starts_with("#") {
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
        let mut result = AnalysisResult::new(file_info, Language::Python);
        
        // 🚀 Parse with tree-sitter (ULTRA FAST!)
        let parse_start = std::time::Instant::now();
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Python file"))?;
        let parse_duration = parse_start.elapsed();
        
        if std::env::var("NEKOCODE_DEBUG").is_ok() {
            eprintln!("⚡ [TREE-SITTER PYTHON] Parse took: {:.3}ms", parse_duration.as_secs_f64() * 1000.0);
        }
        
        // Extract all constructs
        let extract_start = std::time::Instant::now();
        result.functions = self.extract_functions(&tree, content)?;
        result.classes = self.extract_classes(&tree, content)?;
        result.imports = self.extract_imports(&tree, content)?;
        let extract_duration = extract_start.elapsed();
        
        if std::env::var("NEKOCODE_DEBUG").is_ok() {
            eprintln!("⚡ [TREE-SITTER PYTHON] Extraction took: {:.3}ms", extract_duration.as_secs_f64() * 1000.0);
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
            eprintln!("⚡ [TREE-SITTER PYTHON] AST build took: {:.3}ms", ast_duration.as_secs_f64() * 1000.0);
        }
        
        // Update statistics
        result.update_statistics();
        
        Ok(result)
    }
}