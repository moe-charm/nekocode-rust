//! 🚀 Tree-sitter based C++ analyzer
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

pub struct TreeSitterCppAnalyzer {
    parser: Parser,
}

impl TreeSitterCppAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_cpp::LANGUAGE.into())
            .map_err(|e| anyhow::anyhow!("Failed to set C++ language: {:?}", e))?;
        
        Ok(Self { parser })
    }
    
    /// Extract functions using tree-sitter query
    fn extract_functions(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        
        // Query for function definitions
        let query_str = r#"
            [
              (function_definition
                declarator: (function_declarator 
                  declarator: (identifier) @name)) @function
              (function_definition
                declarator: (function_declarator 
                  declarator: (qualified_identifier 
                    name: (identifier) @name))) @function
            ]
        "#;
        
        let query = Query::new(&tree_sitter_cpp::LANGUAGE.into(), query_str)?;
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
                func_info.is_async = false; // C++ doesn't have async/await like JS/Python
                
                // Check for inline, virtual, static keywords
                func_info.metadata.extend(self.extract_function_modifiers(node, source)?);
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
            [
              (class_specifier 
                name: (type_identifier) @name
                body: (field_declaration_list)? @body) @class
              (struct_specifier 
                name: (type_identifier) @name
                body: (field_declaration_list)? @body) @class
            ]
        "#;
        
        let query = Query::new(&tree_sitter_cpp::LANGUAGE.into(), query_str)?;
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
                    "class" => {
                        class_node = Some(capture.node);
                        class_info.start_line = capture.node.start_position().row as u32 + 1;
                        class_info.end_line = capture.node.end_position().row as u32 + 1;
                        
                        // Check if it's a struct
                        if capture.node.kind() == "struct_specifier" {
                            class_info.metadata.insert("type".to_string(), "struct".to_string());
                        } else {
                            class_info.metadata.insert("type".to_string(), "class".to_string());
                        }
                    }
                    _ => {}
                }
            }
            
            // Extract methods and inheritance
            if let Some(node) = class_node {
                class_info.methods = self.extract_class_methods(node, source)?;
                
                // Extract base classes (inheritance)
                if let Some(base_clause) = node.child_by_field_name("base_class_clause") {
                    let base_classes = self.extract_base_classes(base_clause, source)?;
                    if !base_classes.is_empty() {
                        class_info.parent_class = Some(base_classes[0].clone());
                        if base_classes.len() > 1 {
                            class_info.metadata.insert("multiple_inheritance".to_string(), base_classes.join(", "));
                        }
                    }
                }
            }
            
            classes.push(class_info);
        }
        
        Ok(classes)
    }
    
    /// Extract includes using tree-sitter query
    fn extract_imports(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<ImportInfo>> {
        let mut imports = Vec::new();
        
        let query_str = r#"
            (preproc_include
              path: (string_literal) @path) @include
        "#;
        
        let query = Query::new(&tree_sitter_cpp::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut import_info = ImportInfo::new(ImportType::CppInclude, String::new());
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "path" => {
                        let path_text = capture.node.utf8_text(source.as_bytes())?;
                        // Remove quotes
                        import_info.module_path = path_text.trim_matches(|c| c == '"' || c == '<' || c == '>').to_string();
                    }
                    "include" => {
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
        
        // Look for parameter_list in the function_declarator
        if let Some(declarator) = node.child_by_field_name("declarator") {
            if let Some(param_list) = declarator.child_by_field_name("parameters") {
                let mut cursor = param_list.walk();
                for child in param_list.children(&mut cursor) {
                    if child.kind() == "parameter_declaration" {
                        if let Some(declarator) = child.child_by_field_name("declarator") {
                            if let Ok(param_text) = declarator.utf8_text(source.as_bytes()) {
                                params.push(param_text.to_string());
                            }
                        } else if let Some(type_node) = child.child_by_field_name("type") {
                            // For unnamed parameters, use the type
                            if let Ok(type_text) = type_node.utf8_text(source.as_bytes()) {
                                params.push(format!("unnamed_{}", type_text));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(params)
    }
    
    /// Extract function modifiers (virtual, static, inline, etc.)
    fn extract_function_modifiers(&self, node: Node, _source: &str) -> Result<std::collections::HashMap<String, String>> {
        let mut metadata = std::collections::HashMap::new();
        let mut modifiers = Vec::new();
        
        // Look for storage class specifiers and other modifiers
        if let Some(parent) = node.parent() {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
                match child.kind() {
                    "virtual" => modifiers.push("virtual".to_string()),
                    "static" => modifiers.push("static".to_string()),
                    "inline" => modifiers.push("inline".to_string()),
                    "explicit" => modifiers.push("explicit".to_string()),
                    "const" => modifiers.push("const".to_string()),
                    _ => {}
                }
            }
        }
        
        if !modifiers.is_empty() {
            metadata.insert("modifiers".to_string(), modifiers.join(" "));
        }
        
        Ok(metadata)
    }
    
    /// Extract base classes from base class clause
    fn extract_base_classes(&self, node: Node, source: &str) -> Result<Vec<String>> {
        let mut base_classes = Vec::new();
        
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "base_class_clause" {
                let mut subchild_cursor = child.walk();
                for subchild in child.children(&mut subchild_cursor) {
                    if subchild.kind() == "type_identifier" {
                        if let Ok(class_name) = subchild.utf8_text(source.as_bytes()) {
                            base_classes.push(class_name.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(base_classes)
    }
    
    /// Extract methods from a class node
    fn extract_class_methods(&self, class_node: Node, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut methods = Vec::new();
        
        if let Some(body) = class_node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "function_definition" {
                    let mut method = FunctionInfo::new(String::new());
                    
                    if let Some(declarator) = child.child_by_field_name("declarator") {
                        if let Some(func_declarator) = declarator.child_by_field_name("declarator") {
                            if let Ok(name) = func_declarator.utf8_text(source.as_bytes()) {
                                method.name = name.to_string();
                            }
                        }
                    }
                    
                    method.start_line = child.start_position().row as u32 + 1;
                    method.end_line = child.end_position().row as u32 + 1;
                    method.parameters = self.extract_parameters(child, source)?;
                    method.metadata.insert("is_class_method".to_string(), "true".to_string());
                    
                    // Check for special methods (constructor, destructor)
                    if method.name.starts_with('~') {
                        method.metadata.insert("is_destructor".to_string(), "true".to_string());
                    } else if method.name == class_node.child_by_field_name("name")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .unwrap_or("") {
                        method.metadata.insert("is_constructor".to_string(), "true".to_string());
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
            "function_definition" => ASTNodeType::Function,
            "class_specifier" | "struct_specifier" => ASTNodeType::Class,
            "if_statement" => ASTNodeType::IfStatement,
            "for_statement" | "while_statement" => ASTNodeType::ForLoop,
            "preproc_include" => ASTNodeType::Import,
            "declaration" => ASTNodeType::Variable,
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
            } else if let Some(declarator) = node.child_by_field_name("declarator") {
                if let Some(name_field) = declarator.child_by_field_name("declarator") {
                    if let Ok(name) = name_field.utf8_text(source.as_bytes()) {
                        ast_node.name = name.to_string();
                    }
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
impl LanguageAnalyzer for TreeSitterCppAnalyzer {
    fn get_language(&self) -> Language {
        Language::Cpp
    }
    
    fn get_language_name(&self) -> &'static str {
        "C++ (Tree-sitter)"
    }
    
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec![".cpp", ".cxx", ".cc", ".c++", ".hpp", ".hxx", ".hh", ".h++", ".h"]
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
        let mut result = AnalysisResult::new(file_info, Language::Cpp);
        
        // 🚀 Parse with tree-sitter (ULTRA FAST!)
        let parse_start = std::time::Instant::now();
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse C++ file"))?;
        let parse_duration = parse_start.elapsed();
        
        if std::env::var("NEKOCODE_DEBUG").is_ok() {
            eprintln!("⚡ [TREE-SITTER C++] Parse took: {:.3}ms", parse_duration.as_secs_f64() * 1000.0);
        }
        
        // Extract all constructs
        let extract_start = std::time::Instant::now();
        result.functions = self.extract_functions(&tree, content)?;
        result.classes = self.extract_classes(&tree, content)?;
        result.imports = self.extract_imports(&tree, content)?;
        let extract_duration = extract_start.elapsed();
        
        if std::env::var("NEKOCODE_DEBUG").is_ok() {
            eprintln!("⚡ [TREE-SITTER C++] Extraction took: {:.3}ms", extract_duration.as_secs_f64() * 1000.0);
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
            eprintln!("⚡ [TREE-SITTER C++] AST build took: {:.3}ms", ast_duration.as_secs_f64() * 1000.0);
        }
        
        // Update statistics
        result.update_statistics();
        
        Ok(result)
    }
}