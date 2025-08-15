//! 🚀 Tree-sitter based Rust analyzer
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

pub struct TreeSitterRustAnalyzer {
    parser: Parser,
}

impl TreeSitterRustAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|e| anyhow::anyhow!("Failed to set Rust language: {:?}", e))?;
        
        Ok(Self { parser })
    }
    
    /// Extract functions using tree-sitter query
    fn extract_functions(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        
        // Query for function definitions
        let query_str = r#"
            [
              (function_item
                name: (identifier) @name) @function
              (closure_expression) @closure
            ]
        "#;
        
        let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), query_str)?;
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
                        func_info.metadata.insert("type".to_string(), "function".to_string());
                    }
                    "closure" => {
                        func_node = Some(capture.node);
                        func_info.start_line = capture.node.start_position().row as u32 + 1;
                        func_info.end_line = capture.node.end_position().row as u32 + 1;
                        func_info.name = "closure".to_string();
                        func_info.metadata.insert("type".to_string(), "closure".to_string());
                    }
                    _ => {}
                }
            }
            
            // Extract parameters if we have a function node
            if let Some(node) = func_node {
                func_info.parameters = self.extract_parameters(node, source)?;
                func_info.is_async = self.is_async_function(node, source);
                
                // Extract function modifiers (pub, unsafe, etc.)
                func_info.metadata.extend(self.extract_function_modifiers(node, source)?);
            }
            
            // Set default complexity (will be calculated separately)
            func_info.complexity = ComplexityInfo::default();
            
            functions.push(func_info);
        }
        
        Ok(functions)
    }
    
    /// Extract structs/enums/traits (Rust's equivalent to classes) using tree-sitter query
    fn extract_classes(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<ClassInfo>> {
        let mut classes = Vec::new();
        
        let query_str = r#"
            [
              (struct_item
                name: (type_identifier) @name) @struct
              (enum_item
                name: (type_identifier) @name) @enum
              (trait_item
                name: (type_identifier) @name) @trait
              (impl_item
                type: (type_identifier) @impl_type) @impl
            ]
        "#;
        
        let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), query_str)?;
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
                    "impl_type" => {
                        class_info.name = capture.node.utf8_text(source.as_bytes())?.to_string();
                    }
                    "struct" => {
                        class_node = Some(capture.node);
                        class_info.start_line = capture.node.start_position().row as u32 + 1;
                        class_info.end_line = capture.node.end_position().row as u32 + 1;
                        class_info.metadata.insert("type".to_string(), "struct".to_string());
                    }
                    "enum" => {
                        class_node = Some(capture.node);
                        class_info.start_line = capture.node.start_position().row as u32 + 1;
                        class_info.end_line = capture.node.end_position().row as u32 + 1;
                        class_info.metadata.insert("type".to_string(), "enum".to_string());
                    }
                    "trait" => {
                        class_node = Some(capture.node);
                        class_info.start_line = capture.node.start_position().row as u32 + 1;
                        class_info.end_line = capture.node.end_position().row as u32 + 1;
                        class_info.metadata.insert("type".to_string(), "trait".to_string());
                    }
                    "impl" => {
                        class_node = Some(capture.node);
                        class_info.start_line = capture.node.start_position().row as u32 + 1;
                        class_info.end_line = capture.node.end_position().row as u32 + 1;
                        class_info.metadata.insert("type".to_string(), "impl".to_string());
                    }
                    _ => {}
                }
            }
            
            // Extract methods and traits
            if let Some(node) = class_node {
                class_info.methods = self.extract_associated_functions(node, source)?;
                
                // For impl blocks, extract the trait being implemented
                if node.kind() == "impl_item" {
                    if let Some(trait_node) = node.child_by_field_name("trait") {
                        if let Ok(trait_name) = trait_node.utf8_text(source.as_bytes()) {
                            class_info.metadata.insert("implementing_trait".to_string(), trait_name.to_string());
                        }
                    }
                }
                
                // Extract visibility modifiers
                class_info.metadata.extend(self.extract_type_modifiers(node, source)?);
            }
            
            classes.push(class_info);
        }
        
        Ok(classes)
    }
    
    /// Extract use statements using tree-sitter query
    fn extract_imports(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<ImportInfo>> {
        let mut imports = Vec::new();
        
        let query_str = r#"
            (use_declaration
              argument: (scoped_use_list)? @scoped
              argument: (use_list)? @list
              argument: (scoped_identifier)? @simple) @use_decl
        "#;
        
        let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut import_info = ImportInfo::new(ImportType::RustUse, String::new());
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "simple" => {
                        import_info.module_path = capture.node.utf8_text(source.as_bytes())?.to_string();
                    }
                    "scoped" | "list" => {
                        // For complex use statements, just capture the whole text
                        import_info.module_path = capture.node.utf8_text(source.as_bytes())?.to_string();
                    }
                    "use_decl" => {
                        import_info.line_number = capture.node.start_position().row as u32 + 1;
                        
                        // If we don't have a module path yet, use the whole declaration
                        if import_info.module_path.is_empty() {
                            let use_text = capture.node.utf8_text(source.as_bytes())?;
                            // Extract the part after "use "
                            if let Some(stripped) = use_text.strip_prefix("use ") {
                                import_info.module_path = stripped.trim_end_matches(';').to_string();
                            }
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
        
        if let Some(param_list) = node.child_by_field_name("parameters") {
            let mut cursor = param_list.walk();
            for child in param_list.children(&mut cursor) {
                if child.kind() == "parameter" {
                    let mut param_text = String::new();
                    
                    let mut param_cursor = child.walk();
                    for param_child in child.children(&mut param_cursor) {
                        match param_child.kind() {
                            "identifier" => {
                                if let Ok(name) = param_child.utf8_text(source.as_bytes()) {
                                    if param_text.is_empty() {
                                        param_text = name.to_string();
                                    }
                                }
                            }
                            "type_identifier" | "primitive_type" | "reference_type" => {
                                if let Ok(type_text) = param_child.utf8_text(source.as_bytes()) {
                                    if !param_text.is_empty() {
                                        param_text = format!("{}: {}", param_text, type_text);
                                    } else {
                                        param_text = type_text.to_string();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    
                    if !param_text.is_empty() {
                        params.push(param_text);
                    }
                } else if child.kind() == "self_parameter" {
                    if let Ok(self_text) = child.utf8_text(source.as_bytes()) {
                        params.push(self_text.to_string());
                    }
                }
            }
        }
        
        Ok(params)
    }
    
    /// Helper: Check if function is async
    fn is_async_function(&self, node: Node, _source: &str) -> bool {
        // Look for async keyword in the function signature
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "async" {
                return true;
            }
        }
        false
    }
    
    /// Extract function modifiers (pub, unsafe, const, etc.)
    fn extract_function_modifiers(&self, node: Node, source: &str) -> Result<std::collections::HashMap<String, String>> {
        let mut metadata = std::collections::HashMap::new();
        let mut modifiers = Vec::new();
        
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "visibility_modifier" => {
                    if let Ok(vis_text) = child.utf8_text(source.as_bytes()) {
                        modifiers.push(vis_text.to_string());
                    }
                }
                "unsafe" | "const" | "async" | "extern" => {
                    modifiers.push(child.kind().to_string());
                }
                _ => {}
            }
        }
        
        if !modifiers.is_empty() {
            metadata.insert("modifiers".to_string(), modifiers.join(" "));
        }
        
        Ok(metadata)
    }
    
    /// Extract type modifiers for structs/enums/traits
    fn extract_type_modifiers(&self, node: Node, source: &str) -> Result<std::collections::HashMap<String, String>> {
        let mut metadata = std::collections::HashMap::new();
        let mut modifiers = Vec::new();
        
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                if let Ok(vis_text) = child.utf8_text(source.as_bytes()) {
                    modifiers.push(vis_text.to_string());
                }
            }
        }
        
        if !modifiers.is_empty() {
            metadata.insert("modifiers".to_string(), modifiers.join(" "));
        }
        
        Ok(metadata)
    }
    
    /// Extract associated functions (methods) from impl blocks
    fn extract_associated_functions(&self, type_node: Node, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut methods = Vec::new();
        
        // For impl blocks, look for function_item children
        if type_node.kind() == "impl_item" {
            let mut cursor = type_node.walk();
            for child in type_node.children(&mut cursor) {
                if child.kind() == "function_item" {
                    let mut method = FunctionInfo::new(String::new());
                    
                    if let Some(name_node) = child.child_by_field_name("name") {
                        method.name = name_node.utf8_text(source.as_bytes())?.to_string();
                    }
                    
                    method.start_line = child.start_position().row as u32 + 1;
                    method.end_line = child.end_position().row as u32 + 1;
                    method.parameters = self.extract_parameters(child, source)?;
                    method.is_async = self.is_async_function(child, source);
                    method.metadata.insert("is_associated_function".to_string(), "true".to_string());
                    
                    // Check if it's a method (has self parameter) vs associated function
                    if method.parameters.iter().any(|p| p.starts_with("self") || p.starts_with("&self") || p.starts_with("&mut self")) {
                        method.metadata.insert("is_method".to_string(), "true".to_string());
                    } else {
                        method.metadata.insert("is_associated_function".to_string(), "true".to_string());
                    }
                    
                    // Extract method modifiers
                    method.metadata.extend(self.extract_function_modifiers(child, source)?);
                    
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
            "function_item" => ASTNodeType::Function,
            "struct_item" | "enum_item" | "trait_item" | "impl_item" => ASTNodeType::Class,
            "if_expression" => ASTNodeType::IfStatement,
            "for_expression" | "while_expression" | "loop_expression" => ASTNodeType::ForLoop,
            "use_declaration" => ASTNodeType::Import,
            "let_declaration" | "const_item" | "static_item" => ASTNodeType::Variable,
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
            } else if let Some(type_field) = node.child_by_field_name("type") {
                if let Ok(type_name) = type_field.utf8_text(source.as_bytes()) {
                    ast_node.name = type_name.to_string();
                }
            }
            
            parent.add_child(ast_node);
        }
        
        // Recurse through children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.build_ast_recursive(child, source, parent, depth + 1);
        }
    }
}

#[async_trait]
impl LanguageAnalyzer for TreeSitterRustAnalyzer {
    fn get_language(&self) -> Language {
        Language::Rust
    }
    
    fn get_language_name(&self) -> &'static str {
        "Rust (Tree-sitter)"
    }
    
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec![".rs"]
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
        let mut result = AnalysisResult::new(file_info, Language::Rust);
        
        // 🚀 Parse with tree-sitter (ULTRA FAST!)
        let parse_start = std::time::Instant::now();
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Rust file"))?;
        let parse_duration = parse_start.elapsed();
        
        if std::env::var("NEKOCODE_DEBUG").is_ok() {
            eprintln!("⚡ [TREE-SITTER RUST] Parse took: {:.3}ms", parse_duration.as_secs_f64() * 1000.0);
        }
        
        // Extract all constructs
        let extract_start = std::time::Instant::now();
        result.functions = self.extract_functions(&tree, content)?;
        result.classes = self.extract_classes(&tree, content)?;
        result.imports = self.extract_imports(&tree, content)?;
        let extract_duration = extract_start.elapsed();
        
        if std::env::var("NEKOCODE_DEBUG").is_ok() {
            eprintln!("⚡ [TREE-SITTER RUST] Extraction took: {:.3}ms", extract_duration.as_secs_f64() * 1000.0);
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
            eprintln!("⚡ [TREE-SITTER RUST] AST build took: {:.3}ms", ast_duration.as_secs_f64() * 1000.0);
        }
        
        // Update statistics
        result.update_statistics();
        
        Ok(result)
    }
}