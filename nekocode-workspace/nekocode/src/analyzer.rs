//! Language analyzers using Tree-sitter

use std::path::Path;
use tree_sitter::{Parser, Query, QueryCursor, Node, Tree};
use async_trait::async_trait;

use nekocode_core::{
    Result, NekocodeError,
    types::{
        AnalysisResult, FileInfo, FunctionInfo, ClassInfo,
        ImportInfo, ExportInfo, Language, CodeMetrics,
        SymbolInfo, SymbolType, Visibility
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
        
        Ok(result)
    }
    
    fn language(&self) -> Language {
        Language::CSharp
    }
}