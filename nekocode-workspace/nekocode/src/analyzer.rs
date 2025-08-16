//! Language analyzers using Tree-sitter

use std::path::Path;
use tree_sitter::{Parser, Query, QueryCursor, Node, Tree};
use async_trait::async_trait;

use nekocode_core::{
    Result, NekocodeError,
    types::{
        AnalysisResult, FileInfo, FunctionInfo, ClassInfo,
        ImportInfo, ExportInfo, Language, CodeMetrics,
        SymbolInfo, SymbolType, Visibility, ParameterInfo
    }
};

/// Analyzer configuration
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    pub extract_functions: bool,
    pub extract_classes: bool,
    pub extract_imports: bool,
    pub extract_exports: bool,
    pub calculate_complexity: bool,
    pub build_ast: bool,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            extract_functions: true,
            extract_classes: true,
            extract_imports: true,
            extract_exports: true,
            calculate_complexity: true,
            build_ast: false,
        }
    }
}

/// Base trait for language analyzers
#[async_trait]
pub trait Analyzer: Send + Sync {
    /// Analyze a file
    async fn analyze(&mut self, path: &Path, content: &str) -> Result<AnalysisResult>;
    
    /// Get the language this analyzer supports
    fn language(&self) -> Language;
    
    /// Check if this analyzer can handle the given file
    fn can_analyze(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match self.language() {
                Language::JavaScript => matches!(ext, "js" | "jsx" | "mjs" | "cjs"),
                Language::TypeScript => matches!(ext, "ts" | "tsx"),
                Language::Python => matches!(ext, "py" | "pyw" | "pyi"),
                Language::Rust => matches!(ext, "rs"),
                Language::Cpp => matches!(ext, "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" | "c" | "h"),
                Language::Go => matches!(ext, "go"),
                Language::CSharp => matches!(ext, "cs"),
                _ => false
            }
        } else {
            false
        }
    }
}

/// JavaScript analyzer using Tree-sitter
pub struct JavaScriptAnalyzer {
    parser: Parser,
    config: AnalyzerConfig,
}

impl JavaScriptAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_javascript::language())
            .map_err(|e| NekocodeError::Analysis(format!("Failed to set JavaScript language: {:?}", e)))?;
        
        Ok(Self {
            parser,
            config: AnalyzerConfig::default(),
        })
    }
    
    fn extract_functions(&self, tree: &Tree, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        
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
        
        let query = Query::new(tree_sitter_javascript::language(), query_str)
            .map_err(|e| NekocodeError::Analysis(format!("Query error: {}", e)))?;
        
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut func_info = FunctionInfo {
                symbol: SymbolInfo {
                    id: String::new(),
                    name: String::new(),
                    symbol_type: SymbolType::Function,
                    file_path: std::path::PathBuf::new(),
                    line_start: 0,
                    line_end: 0,
                    column_start: 0,
                    column_end: 0,
                    language: Language::JavaScript,
                    visibility: Some(Visibility::Public),
                    parent_id: None,
                    metadata: std::collections::HashMap::new(),
                },
                parameters: Vec::new(),
                return_type: None,
                is_async: false,
                is_static: false,
                is_generic: false,
                complexity: None,
            };
            
            for capture in mat.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                match capture_name.as_str() {
                    "name" => {
                        if let Ok(text) = capture.node.utf8_text(source.as_bytes()) {
                            func_info.symbol.name = text.to_string();
                        }
                    }
                    "function" => {
                        func_info.symbol.line_start = capture.node.start_position().row as u32 + 1;
                        func_info.symbol.line_end = capture.node.end_position().row as u32 + 1;
                    }
                    _ => {}
                }
            }
            
            if !func_info.symbol.name.is_empty() || func_info.symbol.line_start > 0 {
                functions.push(func_info);
            }
        }
        
        Ok(functions)
    }
    
    fn extract_classes(&self, tree: &Tree, source: &str) -> Result<Vec<ClassInfo>> {
        let mut classes = Vec::new();
        
        let query_str = r#"
            (class_declaration
              name: (identifier) @name) @class
        "#;
        
        let query = Query::new(tree_sitter_javascript::language(), query_str)
            .map_err(|e| NekocodeError::Analysis(format!("Query error: {}", e)))?;
        
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut class_info = ClassInfo {
                symbol: SymbolInfo {
                    id: String::new(),
                    name: String::new(),
                    symbol_type: SymbolType::Class,
                    file_path: std::path::PathBuf::new(),
                    line_start: 0,
                    line_end: 0,
                    column_start: 0,
                    column_end: 0,
                    language: Language::JavaScript,
                    visibility: Some(Visibility::Public),
                    parent_id: None,
                    metadata: std::collections::HashMap::new(),
                },
                base_classes: Vec::new(),
                interfaces: Vec::new(),
                methods: Vec::new(),
                fields: Vec::new(),
                is_abstract: false,
                is_interface: false,
            };
            
            for capture in mat.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                match capture_name.as_str() {
                    "name" => {
                        if let Ok(text) = capture.node.utf8_text(source.as_bytes()) {
                            class_info.symbol.name = text.to_string();
                        }
                    }
                    "class" => {
                        class_info.symbol.line_start = capture.node.start_position().row as u32 + 1;
                        class_info.symbol.line_end = capture.node.end_position().row as u32 + 1;
                    }
                    _ => {}
                }
            }
            
            if !class_info.symbol.name.is_empty() {
                classes.push(class_info);
            }
        }
        
        Ok(classes)
    }
}

#[async_trait]
impl Analyzer for JavaScriptAnalyzer {
    async fn analyze(&mut self, path: &Path, content: &str) -> Result<AnalysisResult> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| NekocodeError::Analysis("Failed to parse JavaScript".to_string()))?;
        
        let mut result = AnalysisResult {
            file_info: FileInfo::new(path.to_path_buf()),
            symbols: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            dependencies: Vec::new(),
            metrics: CodeMetrics::default(),
            errors: Vec::new(),
        };
        
        // Update file info with content data
        result.file_info.size_bytes = content.len() as u64;
        result.file_info.total_lines = content.lines().count() as u32;
        
        if self.config.extract_functions {
            result.functions = self.extract_functions(&tree, content)?;
        }
        
        if self.config.extract_classes {
            result.classes = self.extract_classes(&tree, content)?;
        }
        
        Ok(result)
    }
    
    fn language(&self) -> Language {
        Language::JavaScript
    }
}

/// TypeScript analyzer
pub struct TypeScriptAnalyzer {
    parser: Parser,
    config: AnalyzerConfig,
}

impl TypeScriptAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript())
            .map_err(|e| NekocodeError::Analysis(format!("Failed to set TypeScript language: {:?}", e)))?;
        
        Ok(Self {
            parser,
            config: AnalyzerConfig::default(),
        })
    }
}

#[async_trait]
impl Analyzer for TypeScriptAnalyzer {
    async fn analyze(&mut self, path: &Path, content: &str) -> Result<AnalysisResult> {
        // Similar to JavaScript but with TypeScript-specific features
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| NekocodeError::Analysis("Failed to parse TypeScript".to_string()))?;
        
        let mut result = AnalysisResult {
            file_info: FileInfo::new(path.to_path_buf()),
            symbols: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            dependencies: Vec::new(),
            metrics: CodeMetrics::default(),
            errors: Vec::new(),
        };
        
        result.file_info.size_bytes = content.len() as u64;
        result.file_info.total_lines = content.lines().count() as u32;
        
        Ok(result)
    }
    
    fn language(&self) -> Language {
        Language::TypeScript
    }
}

/// Python analyzer
pub struct PythonAnalyzer {
    parser: Parser,
    config: AnalyzerConfig,
}

impl PythonAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_python::language())
            .map_err(|e| NekocodeError::Analysis(format!("Failed to set Python language: {:?}", e)))?;
        
        Ok(Self {
            parser,
            config: AnalyzerConfig::default(),
        })
    }
    
    fn extract_functions(&self, tree: &Tree, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        
        let query_str = r#"
            [
              (function_definition) @function
              (lambda) @function
            ]
        "#;
        
        let query = Query::new(tree_sitter_python::language(), query_str)
            .map_err(|e| NekocodeError::Analysis(format!("Failed to create query: {:?}", e)))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            for capture in mat.captures {
                let func_node = capture.node;
                let start_line = func_node.start_position().row as u32 + 1;
                let end_line = func_node.end_position().row as u32 + 1;
                
                let mut func_name = String::new();
                if func_node.kind() == "lambda" {
                    func_name = "lambda".to_string();
                } else {
                    // Extract function name
                    let mut node_cursor = func_node.walk();
                    for child in func_node.children(&mut node_cursor) {
                        if child.kind() == "identifier" && func_name.is_empty() {
                            if let Ok(name) = child.utf8_text(source.as_bytes()) {
                                func_name = name.to_string();
                                break;
                            }
                        }
                    }
                }
                
                if func_name.is_empty() {
                    func_name = "<anonymous>".to_string();
                }
                
                // Create SymbolInfo
                let symbol = SymbolInfo {
                    id: format!("python_func_{}", func_name),
                    name: func_name,
                    symbol_type: SymbolType::Function,
                    file_path: std::path::PathBuf::new(),  // Will be filled by caller
                    line_start: start_line,
                    line_end: end_line,
                    column_start: func_node.start_position().column as u32,
                    column_end: func_node.end_position().column as u32,
                    language: Language::Python,
                    visibility: Some(Visibility::Public),
                    parent_id: None,
                    metadata: std::collections::HashMap::new(),
                };
                
                // Extract parameters
                let mut parameters = Vec::new();
                if let Some(params_node) = func_node.child_by_field_name("parameters") {
                    let mut param_cursor = params_node.walk();
                    for child in params_node.children(&mut param_cursor) {
                        if child.kind() == "identifier" {
                            if let Ok(param) = child.utf8_text(source.as_bytes()) {
                                parameters.push(ParameterInfo {
                                    name: param.to_string(),
                                    param_type: None,
                                    default_value: None,
                                    is_optional: false,
                                    is_variadic: false,
                                });
                            }
                        }
                    }
                }
                
                let func_info = FunctionInfo {
                    symbol,
                    parameters,
                    return_type: None,
                    is_async: false,
                    is_static: false,
                    is_generic: false,
                    complexity: None,
                };
                
                functions.push(func_info);
            }
        }
        
        Ok(functions)
    }
    
    fn extract_classes(&self, tree: &Tree, source: &str) -> Result<Vec<ClassInfo>> {
        let mut classes = Vec::new();
        
        let query_str = r#"
            (class_definition) @class
        "#;
        
        let query = Query::new(tree_sitter_python::language(), query_str)
            .map_err(|e| NekocodeError::Analysis(format!("Failed to create query: {:?}", e)))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            for capture in mat.captures {
                let class_node = capture.node;
                let start_line = class_node.start_position().row as u32 + 1;
                let end_line = class_node.end_position().row as u32 + 1;
                
                let mut class_name = String::new();
                // Extract class name
                let mut cursor = class_node.walk();
                for child in class_node.children(&mut cursor) {
                    if child.kind() == "identifier" && class_name.is_empty() {
                        if let Ok(name) = child.utf8_text(source.as_bytes()) {
                            class_name = name.to_string();
                        }
                    }
                }
                
                if class_name.is_empty() {
                    class_name = "<anonymous>".to_string();
                }
                
                // Create SymbolInfo
                let symbol = SymbolInfo {
                    id: format!("python_class_{}", class_name),
                    name: class_name,
                    symbol_type: SymbolType::Class,
                    file_path: std::path::PathBuf::new(),  // Will be filled by caller
                    line_start: start_line,
                    line_end: end_line,
                    column_start: class_node.start_position().column as u32,
                    column_end: class_node.end_position().column as u32,
                    language: Language::Python,
                    visibility: Some(Visibility::Public),
                    parent_id: None,
                    metadata: std::collections::HashMap::new(),
                };
                
                // Extract methods
                let methods = self.extract_class_methods(class_node, source)?;
                
                let class_info = ClassInfo {
                    symbol,
                    base_classes: Vec::new(),
                    interfaces: Vec::new(),
                    methods,
                    fields: Vec::new(),
                    is_abstract: false,
                    is_interface: false,
                };
                
                classes.push(class_info);
            }
        }
        
        Ok(classes)
    }
    
    fn extract_class_methods(&self, class_node: Node, source: &str) -> Result<Vec<String>> {
        let mut methods = Vec::new();
        
        let mut cursor = class_node.walk();
        for child in class_node.children(&mut cursor) {
            if child.kind() == "block" {
                let mut block_cursor = child.walk();
                for stmt in child.children(&mut block_cursor) {
                    if stmt.kind() == "function_definition" {
                        let mut func_cursor = stmt.walk();
                        for func_child in stmt.children(&mut func_cursor) {
                            if func_child.kind() == "identifier" {
                                if let Ok(name) = func_child.utf8_text(source.as_bytes()) {
                                    methods.push(name.to_string());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(methods)
    }
    
    fn extract_imports(&self, tree: &Tree, source: &str) -> Result<Vec<ImportInfo>> {
        let mut imports = Vec::new();
        
        let query_str = r#"
            [
              (import_statement) @import
              (import_from_statement) @from_import
            ]
        "#;
        
        let query = Query::new(tree_sitter_python::language(), query_str)
            .map_err(|e| NekocodeError::Analysis(format!("Failed to create query: {:?}", e)))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            for capture in mat.captures {
                let import_node = capture.node;
                let line_number = import_node.start_position().row as u32 + 1;
                
                if let Ok(import_text) = import_node.utf8_text(source.as_bytes()) {
                    let module_name = import_text.replace("import ", "").replace("from ", "");
                    let import_info = ImportInfo {
                        module: module_name,
                        imported_names: Vec::new(),
                        alias: None,
                        is_default: false,
                        is_namespace: false,
                        line: line_number,
                    };
                    imports.push(import_info);
                }
            }
        }
        
        Ok(imports)
    }
}

#[async_trait]
impl Analyzer for PythonAnalyzer {
    async fn analyze(&mut self, path: &Path, content: &str) -> Result<AnalysisResult> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| NekocodeError::Analysis("Failed to parse Python".to_string()))?;
        
        let mut result = AnalysisResult {
            file_info: FileInfo::new(path.to_path_buf()),
            symbols: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            dependencies: Vec::new(),
            metrics: CodeMetrics::default(),
            errors: Vec::new(),
        };
        
        // Update file info with content data
        result.file_info.size_bytes = content.len() as u64;
        result.file_info.total_lines = content.lines().count() as u32;
        
        // Extract functions, classes, and imports
        if self.config.extract_functions {
            result.functions = self.extract_functions(&tree, content)?;
        }
        
        if self.config.extract_classes {
            result.classes = self.extract_classes(&tree, content)?;
        }
        
        if self.config.extract_imports {
            result.imports = self.extract_imports(&tree, content)?;
        }
        
        // Update metrics based on extracted information
        result.metrics.lines_of_code = result.file_info.code_lines;
        result.metrics.blank_lines = result.file_info.empty_lines;
        
        Ok(result)
    }
    
    fn language(&self) -> Language {
        Language::Python
    }
}

/// Rust analyzer
pub struct RustAnalyzer {
    parser: Parser,
    config: AnalyzerConfig,
}

impl RustAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language())
            .map_err(|e| NekocodeError::Analysis(format!("Failed to set Rust language: {:?}", e)))?;
        
        Ok(Self {
            parser,
            config: AnalyzerConfig::default(),
        })
    }
    
    fn extract_functions(&self, tree: &Tree, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        
        let query_str = r#"
            [
              (function_item
                name: (identifier) @name) @function
              (closure_expression) @closure
            ]
        "#;
        
        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| NekocodeError::Analysis(format!("Failed to create query: {:?}", e)))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut func_name = String::new();
            let mut func_node = None;
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "name" => {
                        if let Ok(name) = capture.node.utf8_text(source.as_bytes()) {
                            func_name = name.to_string();
                        }
                    }
                    "function" | "closure" => {
                        func_node = Some(capture.node);
                        if capture.node.kind() == "closure_expression" && func_name.is_empty() {
                            func_name = "closure".to_string();
                        }
                    }
                    _ => {}
                }
            }
            
            if let Some(node) = func_node {
                let symbol = SymbolInfo {
                    id: format!("rust_func_{}", func_name),
                    name: func_name.clone(),
                    symbol_type: SymbolType::Function,
                    file_path: std::path::PathBuf::new(),
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    column_start: node.start_position().column as u32,
                    column_end: node.end_position().column as u32,
                    language: Language::Rust,
                    visibility: Some(Visibility::Public),
                    parent_id: None,
                    metadata: std::collections::HashMap::new(),
                };
                
                let func_info = FunctionInfo {
                    symbol,
                    parameters: Vec::new(),
                    return_type: None,
                    is_async: false,
                    is_static: false,
                    is_generic: false,
                    complexity: None,
                };
                
                functions.push(func_info);
            }
        }
        
        Ok(functions)
    }
    
    fn extract_classes(&self, tree: &Tree, source: &str) -> Result<Vec<ClassInfo>> {
        let mut classes = Vec::new();
        
        let query_str = r#"
            [
              (struct_item
                name: (type_identifier) @name) @struct
              (enum_item
                name: (type_identifier) @name) @enum
              (trait_item
                name: (type_identifier) @name) @trait
            ]
        "#;
        
        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| NekocodeError::Analysis(format!("Failed to create query: {:?}", e)))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        
        for mat in matches {
            let mut class_name = String::new();
            let mut class_node = None;
            
            for capture in mat.captures {
                match query.capture_names()[capture.index as usize].as_ref() {
                    "name" => {
                        if let Ok(name) = capture.node.utf8_text(source.as_bytes()) {
                            class_name = name.to_string();
                        }
                    }
                    "struct" | "enum" | "trait" => {
                        class_node = Some(capture.node);
                    }
                    _ => {}
                }
            }
            
            if let Some(node) = class_node {
                let symbol = SymbolInfo {
                    id: format!("rust_type_{}", class_name),
                    name: class_name.clone(),
                    symbol_type: SymbolType::Class,
                    file_path: std::path::PathBuf::new(),
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    column_start: node.start_position().column as u32,
                    column_end: node.end_position().column as u32,
                    language: Language::Rust,
                    visibility: Some(Visibility::Public),
                    parent_id: None,
                    metadata: std::collections::HashMap::new(),
                };
                
                let class_info = ClassInfo {
                    symbol,
                    base_classes: Vec::new(),
                    interfaces: Vec::new(),
                    methods: Vec::new(),
                    fields: Vec::new(),
                    is_abstract: false,
                    is_interface: node.kind() == "trait_item",
                };
                
                classes.push(class_info);
            }
        }
        
        Ok(classes)
    }
}

#[async_trait]
impl Analyzer for RustAnalyzer {
    async fn analyze(&mut self, path: &Path, content: &str) -> Result<AnalysisResult> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| NekocodeError::Analysis("Failed to parse Rust".to_string()))?;
        
        let mut result = AnalysisResult {
            file_info: FileInfo::new(path.to_path_buf()),
            symbols: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            dependencies: Vec::new(),
            metrics: CodeMetrics::default(),
            errors: Vec::new(),
        };
        
        // Update file info with content data
        result.file_info.size_bytes = content.len() as u64;
        result.file_info.total_lines = content.lines().count() as u32;
        
        // Extract functions and classes
        if self.config.extract_functions {
            result.functions = self.extract_functions(&tree, content)?;
        }
        
        if self.config.extract_classes {
            result.classes = self.extract_classes(&tree, content)?;
        }
        
        Ok(result)
    }
    
    fn language(&self) -> Language {
        Language::Rust
    }
}

/// C++ analyzer
pub struct CppAnalyzer {
    parser: Parser,
    config: AnalyzerConfig,
}

impl CppAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_cpp::language())
            .map_err(|e| NekocodeError::Analysis(format!("Failed to set C++ language: {:?}", e)))?;
        
        Ok(Self {
            parser,
            config: AnalyzerConfig::default(),
        })
    }
    
    fn extract_simple(&self, tree: &Tree, source: &str) -> (Vec<FunctionInfo>, Vec<ClassInfo>) {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        
        // Recursive search for function_definition and class_specifier nodes
        self.traverse_cpp_nodes(tree.root_node(), source, &mut functions, &mut classes);
        
        (functions, classes)
    }
    
    fn traverse_cpp_nodes(&self, node: Node, source: &str, functions: &mut Vec<FunctionInfo>, classes: &mut Vec<ClassInfo>) {
        // Check current node
        if node.kind() == "function_definition" {
            // Extract function name
            let mut func_name = String::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "function_declarator" {
                    // Look deeper for identifier
                    let mut decl_cursor = child.walk();
                    for decl_child in child.children(&mut decl_cursor) {
                        if decl_child.kind() == "identifier" {
                            if let Ok(name) = decl_child.utf8_text(source.as_bytes()) {
                                func_name = name.to_string();
                                break;
                            }
                        }
                    }
                    if !func_name.is_empty() { break; }
                }
            }
            
            if func_name.is_empty() {
                func_name = format!("function_{}", functions.len());
            }
            
            let symbol = SymbolInfo {
                id: format!("cpp_func_{}", func_name),
                name: func_name,
                symbol_type: SymbolType::Function,
                file_path: std::path::PathBuf::new(),
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                column_start: node.start_position().column as u32,
                column_end: node.end_position().column as u32,
                language: Language::Cpp,
                visibility: Some(Visibility::Public),
                parent_id: None,
                metadata: std::collections::HashMap::new(),
            };
            functions.push(FunctionInfo {
                symbol,
                parameters: Vec::new(),
                return_type: None,
                is_async: false,
                is_static: false,
                is_generic: false,
                complexity: None,
            });
        } else if node.kind() == "class_specifier" || node.kind() == "struct_specifier" {
            // Extract class name
            let mut class_name = String::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        class_name = name.to_string();
                        break;
                    }
                }
            }
            
            if class_name.is_empty() {
                class_name = format!("class_{}", classes.len());
            }
            
            let symbol = SymbolInfo {
                id: format!("cpp_class_{}", class_name),
                name: class_name,
                symbol_type: SymbolType::Class,
                file_path: std::path::PathBuf::new(),
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                column_start: node.start_position().column as u32,
                column_end: node.end_position().column as u32,
                language: Language::Cpp,
                visibility: Some(Visibility::Public),
                parent_id: None,
                metadata: std::collections::HashMap::new(),
            };
            classes.push(ClassInfo {
                symbol,
                base_classes: Vec::new(),
                interfaces: Vec::new(),
                methods: Vec::new(),
                fields: Vec::new(),
                is_abstract: false,
                is_interface: false,
            });
        }
        
        // Recursively check children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_cpp_nodes(child, source, functions, classes);
        }
    }
}

#[async_trait]
impl Analyzer for CppAnalyzer {
    async fn analyze(&mut self, path: &Path, content: &str) -> Result<AnalysisResult> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| NekocodeError::Analysis("Failed to parse C++".to_string()))?;
        
        let mut result = AnalysisResult {
            file_info: FileInfo::new(path.to_path_buf()),
            symbols: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            dependencies: Vec::new(),
            metrics: CodeMetrics::default(),
            errors: Vec::new(),
        };
        
        // Update file info with content data
        result.file_info.size_bytes = content.len() as u64;
        result.file_info.total_lines = content.lines().count() as u32;
        
        // Extract functions and classes
        let (functions, classes) = self.extract_simple(&tree, content);
        result.functions = functions;
        result.classes = classes;
        
        Ok(result)
    }
    
    fn language(&self) -> Language {
        Language::Cpp
    }
}

/// Go analyzer
pub struct GoAnalyzer {
    parser: Parser,
    config: AnalyzerConfig,
}

impl GoAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_go::language())
            .map_err(|e| NekocodeError::Analysis(format!("Failed to set Go language: {:?}", e)))?;
        
        Ok(Self {
            parser,
            config: AnalyzerConfig::default(),
        })
    }
    
    fn extract_simple(&self, tree: &Tree, _source: &str) -> (Vec<FunctionInfo>, Vec<ClassInfo>) {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        
        let root = tree.root_node();
        let mut cursor = root.walk();
        
        for node in root.children(&mut cursor) {
            if node.kind() == "function_declaration" || node.kind() == "method_declaration" {
                let symbol = SymbolInfo {
                    id: format!("go_func_{}", functions.len()),
                    name: format!("function_{}", functions.len()),
                    symbol_type: SymbolType::Function,
                    file_path: std::path::PathBuf::new(),
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    column_start: node.start_position().column as u32,
                    column_end: node.end_position().column as u32,
                    language: Language::Go,
                    visibility: Some(Visibility::Public),
                    parent_id: None,
                    metadata: std::collections::HashMap::new(),
                };
                functions.push(FunctionInfo {
                    symbol,
                    parameters: Vec::new(),
                    return_type: None,
                    is_async: false,
                    is_static: false,
                    is_generic: false,
                    complexity: None,
                });
            } else if node.kind() == "type_declaration" {
                let symbol = SymbolInfo {
                    id: format!("go_type_{}", classes.len()),
                    name: format!("type_{}", classes.len()),
                    symbol_type: SymbolType::Class,
                    file_path: std::path::PathBuf::new(),
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    column_start: node.start_position().column as u32,
                    column_end: node.end_position().column as u32,
                    language: Language::Go,
                    visibility: Some(Visibility::Public),
                    parent_id: None,
                    metadata: std::collections::HashMap::new(),
                };
                classes.push(ClassInfo {
                    symbol,
                    base_classes: Vec::new(),
                    interfaces: Vec::new(),
                    methods: Vec::new(),
                    fields: Vec::new(),
                    is_abstract: false,
                    is_interface: false,
                });
            }
        }
        
        (functions, classes)
    }
}

#[async_trait]
impl Analyzer for GoAnalyzer {
    async fn analyze(&mut self, path: &Path, content: &str) -> Result<AnalysisResult> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| NekocodeError::Analysis("Failed to parse Go".to_string()))?;
        
        let mut result = AnalysisResult {
            file_info: FileInfo::new(path.to_path_buf()),
            symbols: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            dependencies: Vec::new(),
            metrics: CodeMetrics::default(),
            errors: Vec::new(),
        };
        
        // Update file info with content data
        result.file_info.size_bytes = content.len() as u64;
        result.file_info.total_lines = content.lines().count() as u32;
        
        // Extract functions and classes
        let (functions, classes) = self.extract_simple(&tree, content);
        result.functions = functions;
        result.classes = classes;
        
        Ok(result)
    }
    
    fn language(&self) -> Language {
        Language::Go
    }
}

/// C# analyzer
pub struct CSharpAnalyzer {
    parser: Parser,
    config: AnalyzerConfig,
}

impl CSharpAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_c_sharp::language())
            .map_err(|e| NekocodeError::Analysis(format!("Failed to set C# language: {:?}", e)))?;
        
        Ok(Self {
            parser,
            config: AnalyzerConfig::default(),
        })
    }
    
    fn extract_simple(&self, tree: &Tree, source: &str) -> (Vec<FunctionInfo>, Vec<ClassInfo>) {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        
        // Recursive search for C# nodes
        self.traverse_csharp_nodes(tree.root_node(), source, &mut functions, &mut classes);
        
        (functions, classes)
    }
    
    fn traverse_csharp_nodes(&self, node: Node, source: &str, functions: &mut Vec<FunctionInfo>, classes: &mut Vec<ClassInfo>) {
        // Check current node
        if node.kind() == "method_declaration" {
            // Extract method name
            let mut func_name = String::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        func_name = name.to_string();
                        break;
                    }
                }
            }
            
            if func_name.is_empty() {
                func_name = format!("method_{}", functions.len());
            }
            
            let symbol = SymbolInfo {
                id: format!("csharp_func_{}", func_name),
                name: func_name,
                symbol_type: SymbolType::Function,
                file_path: std::path::PathBuf::new(),
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                column_start: node.start_position().column as u32,
                column_end: node.end_position().column as u32,
                language: Language::CSharp,
                visibility: Some(Visibility::Public),
                parent_id: None,
                metadata: std::collections::HashMap::new(),
            };
            functions.push(FunctionInfo {
                symbol,
                parameters: Vec::new(),
                return_type: None,
                is_async: false,
                is_static: false,
                is_generic: false,
                complexity: None,
            });
        } else if node.kind() == "class_declaration" || node.kind() == "interface_declaration" {
            // Extract class name
            let mut class_name = String::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        class_name = name.to_string();
                        break;
                    }
                }
            }
            
            if class_name.is_empty() {
                class_name = format!("class_{}", classes.len());
            }
            
            let symbol = SymbolInfo {
                id: format!("csharp_class_{}", class_name),
                name: class_name,
                symbol_type: SymbolType::Class,
                file_path: std::path::PathBuf::new(),
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                column_start: node.start_position().column as u32,
                column_end: node.end_position().column as u32,
                language: Language::CSharp,
                visibility: Some(Visibility::Public),
                parent_id: None,
                metadata: std::collections::HashMap::new(),
            };
            classes.push(ClassInfo {
                symbol,
                base_classes: Vec::new(),
                interfaces: Vec::new(),
                methods: Vec::new(),
                fields: Vec::new(),
                is_abstract: false,
                is_interface: node.kind() == "interface_declaration",
            });
        }
        
        // Recursively check children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_csharp_nodes(child, source, functions, classes);
        }
    }
}

#[async_trait]
impl Analyzer for CSharpAnalyzer {
    async fn analyze(&mut self, path: &Path, content: &str) -> Result<AnalysisResult> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| NekocodeError::Analysis("Failed to parse C#".to_string()))?;
        
        let mut result = AnalysisResult {
            file_info: FileInfo::new(path.to_path_buf()),
            symbols: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            dependencies: Vec::new(),
            metrics: CodeMetrics::default(),
            errors: Vec::new(),
        };
        
        // Update file info with content data
        result.file_info.size_bytes = content.len() as u64;
        result.file_info.total_lines = content.lines().count() as u32;
        
        // Extract functions and classes
        let (functions, classes) = self.extract_simple(&tree, content);
        result.functions = functions;
        result.classes = classes;
        
        Ok(result)
    }
    
    fn language(&self) -> Language {
        Language::CSharp
    }
}