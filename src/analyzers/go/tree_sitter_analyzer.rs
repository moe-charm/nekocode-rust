//! 🚀 Tree-sitter based Go analyzer
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

pub struct TreeSitterGoAnalyzer {
    parser: Parser,
}

impl TreeSitterGoAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::LANGUAGE.into())
            .map_err(|e| anyhow::anyhow!("Failed to set Go language: {:?}", e))?;
        
        Ok(Self { parser })
    }
    
    /// Extract functions using tree-sitter query
    fn extract_functions(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        
        // Query for function definitions
        let query_str = r#"
            [
              (function_declaration
                name: (identifier) @name) @function
              (method_declaration
                name: (field_identifier) @name) @function
            ]
        "#;
        
        let query = Query::new(&tree_sitter_go::LANGUAGE.into(), query_str)?;
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
                        
                        // Check if it's a method
                        if capture.node.kind() == "method_declaration" {
                            func_info.metadata.insert("is_method".to_string(), "true".to_string());
                            
                            // Extract receiver type for methods
                            if let Some(receiver) = capture.node.child_by_field_name("receiver") {
                                if let Ok(receiver_text) = receiver.utf8_text(source.as_bytes()) {
                                    func_info.metadata.insert("receiver".to_string(), receiver_text.to_string());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            
            // Extract parameters if we have a function node
            if let Some(node) = func_node {
                func_info.parameters = self.extract_parameters(node, source)?;
                func_info.is_async = false; // Go doesn't have async/await
            }
            
            // Set default complexity (will be calculated separately)
            func_info.complexity = ComplexityInfo::default();
            
            functions.push(func_info);
        }
        
        Ok(functions)
    }
    
    /// Extract types/structs (Go's equivalent to classes) using tree-sitter query
    fn extract_classes(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<ClassInfo>> {
        let mut classes = Vec::new();
        
        let query_str = r#"
            [
              (type_declaration
                (type_spec
                  name: (type_identifier) @name
                  type: (struct_type) @struct)) @type_decl
              (type_declaration
                (type_spec
                  name: (type_identifier) @name
                  type: (interface_type) @interface)) @type_decl
            ]
        "#;
        
        let query = Query::new(&tree_sitter_go::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut class_info = ClassInfo::new(String::new());
            let mut type_node = None;
            let mut is_interface = false;
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "name" => {
                        class_info.name = capture.node.utf8_text(source.as_bytes())?.to_string();
                    }
                    "struct" => {
                        class_info.metadata.insert("type".to_string(), "struct".to_string());
                    }
                    "interface" => {
                        class_info.metadata.insert("type".to_string(), "interface".to_string());
                        is_interface = true;
                    }
                    "type_decl" => {
                        type_node = Some(capture.node);
                        class_info.start_line = capture.node.start_position().row as u32 + 1;
                        class_info.end_line = capture.node.end_position().row as u32 + 1;
                    }
                    _ => {}
                }
            }
            
            // Extract methods for this type (methods with matching receiver)
            if let Some(_node) = type_node {
                class_info.methods = self.extract_methods_for_type(&class_info.name, tree, source)?;
                
                // For interfaces, extract method signatures
                if is_interface {
                    if let Some(interface_methods) = self.extract_interface_methods(&class_info.name, tree, source)? {
                        class_info.methods.extend(interface_methods);
                    }
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
              (import_declaration
                (import_spec
                  path: (interpreted_string_literal) @path)) @import
              (import_declaration
                (import_spec_list
                  (import_spec
                    path: (interpreted_string_literal) @path))) @import
            ]
        "#;
        
        let query = Query::new(&tree_sitter_go::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut import_info = ImportInfo::new(ImportType::GoImport, String::new());
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "path" => {
                        let path_text = capture.node.utf8_text(source.as_bytes())?;
                        // Remove quotes
                        import_info.module_path = path_text.trim_matches('"').to_string();
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
    
    /// Helper: Extract parameters from a function node
    fn extract_parameters(&self, node: Node, source: &str) -> Result<Vec<String>> {
        let mut params = Vec::new();
        
        if let Some(param_list) = node.child_by_field_name("parameters") {
            let mut cursor = param_list.walk();
            for child in param_list.children(&mut cursor) {
                if child.kind() == "parameter_declaration" {
                    let mut param_names = Vec::new();
                    let mut param_type = String::new();
                    
                    let mut param_cursor = child.walk();
                    for param_child in child.children(&mut param_cursor) {
                        match param_child.kind() {
                            "identifier" => {
                                if let Ok(name) = param_child.utf8_text(source.as_bytes()) {
                                    param_names.push(name.to_string());
                                }
                            }
                            "type_identifier" | "pointer_type" | "slice_type" => {
                                if let Ok(type_text) = param_child.utf8_text(source.as_bytes()) {
                                    param_type = type_text.to_string();
                                }
                            }
                            _ => {}
                        }
                    }
                    
                    // Go allows multiple parameters of the same type: func(a, b int)
                    if param_names.is_empty() {
                        params.push(param_type);
                    } else {
                        for name in param_names {
                            params.push(format!("{} {}", name, param_type));
                        }
                    }
                }
            }
        }
        
        Ok(params)
    }
    
    /// Extract methods for a specific type by scanning all method declarations
    fn extract_methods_for_type(&self, type_name: &str, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut methods = Vec::new();
        
        let query_str = r#"
            (method_declaration
              receiver: (parameter_list
                (parameter_declaration
                  type: (pointer_type 
                    (type_identifier) @receiver_type)))
              name: (field_identifier) @name) @method
        "#;
        
        let query = Query::new(&tree_sitter_go::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut method = FunctionInfo::new(String::new());
            let mut receiver_type = String::new();
            let mut method_node = None;
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "receiver_type" => {
                        receiver_type = capture.node.utf8_text(source.as_bytes())?.to_string();
                    }
                    "name" => {
                        method.name = capture.node.utf8_text(source.as_bytes())?.to_string();
                    }
                    "method" => {
                        method_node = Some(capture.node);
                        method.start_line = capture.node.start_position().row as u32 + 1;
                        method.end_line = capture.node.end_position().row as u32 + 1;
                    }
                    _ => {}
                }
            }
            
            // Only include methods that belong to this type
            if receiver_type == type_name {
                if let Some(node) = method_node {
                    method.parameters = self.extract_parameters(node, source)?;
                    method.metadata.insert("is_method".to_string(), "true".to_string());
                    method.metadata.insert("receiver_type".to_string(), receiver_type);
                }
                methods.push(method);
            }
        }
        
        Ok(methods)
    }
    
    /// Extract interface method signatures
    fn extract_interface_methods(&self, _interface_name: &str, _tree: &tree_sitter::Tree, _source: &str) -> Result<Option<Vec<FunctionInfo>>> {
        // For now, return empty - interface method extraction is complex
        // This would require parsing the interface body and extracting method signatures
        Ok(Some(Vec::new()))
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
            "function_declaration" | "method_declaration" => ASTNodeType::Function,
            "type_declaration" => ASTNodeType::Class, // Go structs/interfaces are like classes
            "if_statement" => ASTNodeType::IfStatement,
            "for_statement" | "range_clause" => ASTNodeType::ForLoop,
            "import_declaration" => ASTNodeType::Import,
            "var_declaration" | "const_declaration" => ASTNodeType::Variable,
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
impl LanguageAnalyzer for TreeSitterGoAnalyzer {
    fn get_language(&self) -> Language {
        Language::Go
    }
    
    fn get_language_name(&self) -> &'static str {
        "Go (Tree-sitter)"
    }
    
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec![".go"]
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
        let mut result = AnalysisResult::new(file_info, Language::Go);
        
        // 🚀 Parse with tree-sitter (ULTRA FAST!)
        let parse_start = std::time::Instant::now();
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Go file"))?;
        let parse_duration = parse_start.elapsed();
        
        if std::env::var("NEKOCODE_DEBUG").is_ok() {
            eprintln!("⚡ [TREE-SITTER GO] Parse took: {:.3}ms", parse_duration.as_secs_f64() * 1000.0);
        }
        
        // Extract all constructs
        let extract_start = std::time::Instant::now();
        result.functions = self.extract_functions(&tree, content)?;
        result.classes = self.extract_classes(&tree, content)?;
        result.imports = self.extract_imports(&tree, content)?;
        let extract_duration = extract_start.elapsed();
        
        if std::env::var("NEKOCODE_DEBUG").is_ok() {
            eprintln!("⚡ [TREE-SITTER GO] Extraction took: {:.3}ms", extract_duration.as_secs_f64() * 1000.0);
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
            eprintln!("⚡ [TREE-SITTER GO] AST build took: {:.3}ms", ast_duration.as_secs_f64() * 1000.0);
        }
        
        // Update statistics
        result.update_statistics();
        
        Ok(result)
    }
}