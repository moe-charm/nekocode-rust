//! Core types for NekoCode Rust
//! 
//! This module contains all the fundamental data structures used throughout
//! the analysis system, ported from the C++ types.hpp file.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

use crate::core::ast::{ASTNode, ASTStatistics};

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "javascript")]
    JavaScript,
    #[serde(rename = "typescript")]
    TypeScript,
    #[serde(rename = "cpp")]
    Cpp,
    #[serde(rename = "c")]
    C,
    #[serde(rename = "python")]
    Python,
    #[serde(rename = "csharp")]
    CSharp,
    #[serde(rename = "go")]
    Go,
    #[serde(rename = "rust")]
    Rust,
    #[serde(rename = "unknown")]
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            ".js" | ".mjs" | ".jsx" | ".cjs" => Language::JavaScript,
            ".ts" | ".tsx" => Language::TypeScript,
            ".cpp" | ".cxx" | ".cc" | ".hpp" | ".hxx" | ".hh" => Language::Cpp,
            ".c" | ".h" => Language::C,
            ".py" | ".pyw" | ".pyi" => Language::Python,
            ".cs" => Language::CSharp,
            ".go" => Language::Go,
            ".rs" => Language::Rust,
            _ => Language::Unknown,
        }
    }
}

/// File information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub total_lines: u32,
    pub code_lines: u32,
    pub comment_lines: u32,
    pub empty_lines: u32,
    pub code_ratio: f64,
    pub analyzed_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl FileInfo {
    pub fn new(path: PathBuf) -> Self {
        Self {
            name: path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            path,
            size_bytes: 0,
            total_lines: 0,
            code_lines: 0,
            comment_lines: 0,
            empty_lines: 0,
            code_ratio: 0.0,
            analyzed_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

/// Complexity rating levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplexityRating {
    #[serde(rename = "simple")]
    Simple,      // <= 10
    #[serde(rename = "moderate")]
    Moderate,    // 11-20
    #[serde(rename = "complex")]
    Complex,     // 21-50
    #[serde(rename = "very_complex")]
    VeryComplex, // > 50
}

/// Complexity analysis information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityInfo {
    pub cyclomatic_complexity: u32,
    pub max_nesting_depth: u32,
    pub cognitive_complexity: u32,
    pub rating: ComplexityRating,
    pub rating_emoji: String,
}

impl ComplexityInfo {
    pub fn new() -> Self {
        let mut info = Self {
            cyclomatic_complexity: 1,
            max_nesting_depth: 0,
            cognitive_complexity: 0,
            rating: ComplexityRating::Simple,
            rating_emoji: "🟢".to_string(),
        };
        info.update_rating();
        info
    }
    
    pub fn update_rating(&mut self) {
        let complexity = self.cyclomatic_complexity;
        self.rating = if complexity <= 10 {
            self.rating_emoji = "🟢".to_string();
            ComplexityRating::Simple
        } else if complexity <= 20 {
            self.rating_emoji = "🟡".to_string();
            ComplexityRating::Moderate
        } else if complexity <= 50 {
            self.rating_emoji = "🟠".to_string();
            ComplexityRating::Complex
        } else {
            self.rating_emoji = "🔴".to_string();
            ComplexityRating::VeryComplex
        };
    }
    
    pub fn to_string(&self) -> String {
        match self.rating {
            ComplexityRating::Simple => format!("Simple {}", self.rating_emoji),
            ComplexityRating::Moderate => format!("Moderate {}", self.rating_emoji),
            ComplexityRating::Complex => format!("Complex {}", self.rating_emoji),
            ComplexityRating::VeryComplex => format!("Very Complex {}", self.rating_emoji),
        }
    }
}

impl Default for ComplexityInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Function information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub parameters: Vec<String>,
    pub is_async: bool,
    pub is_arrow_function: bool,
    pub complexity: ComplexityInfo,
    pub metadata: HashMap<String, String>,
}

impl FunctionInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            start_line: 0,
            end_line: 0,
            parameters: Vec::new(),
            is_async: false,
            is_arrow_function: false,
            complexity: ComplexityInfo::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Member variable information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberVariable {
    pub name: String,
    pub var_type: String,
    pub declaration_line: u32,
    pub is_static: bool,
    pub is_const: bool,
    pub access_modifier: String,
    pub used_by_methods: Vec<String>,
    pub modified_by_methods: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl MemberVariable {
    pub fn new(name: String, var_type: String, declaration_line: u32) -> Self {
        Self {
            name,
            var_type,
            declaration_line,
            is_static: false,
            is_const: false,
            access_modifier: "private".to_string(),
            used_by_methods: Vec::new(),
            modified_by_methods: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Class information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub parent_class: Option<String>,
    pub start_line: u32,
    pub end_line: u32,
    pub methods: Vec<FunctionInfo>,
    pub properties: Vec<String>,
    pub member_variables: Vec<MemberVariable>,
    pub metadata: HashMap<String, String>,
}

impl ClassInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            parent_class: None,
            start_line: 0,
            end_line: 0,
            methods: Vec::new(),
            properties: Vec::new(),
            member_variables: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Import types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportType {
    #[serde(rename = "es6_import")]
    ES6Import,      // import ... from
    #[serde(rename = "commonjs_require")]
    CommonJSRequire, // require()
    #[serde(rename = "dynamic_import")]
    DynamicImport,   // import()
    #[serde(rename = "python_import")]
    PythonImport,   // import module
    #[serde(rename = "python_from_import")]
    PythonFromImport, // from module import ...
    #[serde(rename = "cpp_include")]
    CppInclude,     // #include
    #[serde(rename = "csharp_using")]
    CSharpUsing,    // using namespace
    #[serde(rename = "go_import")]
    GoImport,       // import "package"
    #[serde(rename = "rust_use")]
    RustUse,        // use crate::module
}

/// Export types  
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportType {
    #[serde(rename = "es6_export")]
    ES6Export,      // export ...
    #[serde(rename = "es6_default")]
    ES6Default,     // export default
    #[serde(rename = "commonjs_exports")]
    CommonJSExports, // module.exports
    #[serde(rename = "python_global")]
    PythonGlobal,   // global variables and functions (no explicit export)
    #[serde(rename = "cpp_extern")]
    CppExtern,      // extern declarations
    #[serde(rename = "csharp_public")]
    CSharpPublic,   // public classes/methods
    #[serde(rename = "go_exported")]
    GoExported,     // Capitalized names (exported)
    #[serde(rename = "rust_pub")]
    RustPub,        // pub declarations
}

/// Import information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    #[serde(rename = "type")]
    pub import_type: ImportType,
    pub module_path: String,
    pub imported_names: Vec<String>,
    pub alias: Option<String>,
    pub line_number: u32,
    pub metadata: HashMap<String, String>,
}

impl ImportInfo {
    pub fn new(import_type: ImportType, module_path: String) -> Self {
        Self {
            import_type,
            module_path,
            imported_names: Vec::new(),
            alias: None,
            line_number: 0,
            metadata: HashMap::new(),
        }
    }
}

/// Export information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportInfo {
    #[serde(rename = "type")]
    pub export_type: ExportType,
    pub exported_names: Vec<String>,
    pub is_default: bool,
    pub line_number: u32,
}

impl ExportInfo {
    pub fn new(export_type: ExportType) -> Self {
        Self {
            export_type,
            exported_names: Vec::new(),
            is_default: false,
            line_number: 0,
        }
    }
}

/// Function call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub function_name: String,
    pub object_name: Option<String>,
    pub line_number: u32,
    pub is_method_call: bool,
}

impl FunctionCall {
    pub fn new(function_name: String, line_number: u32) -> Self {
        Self {
            function_name,
            object_name: None,
            line_number,
            is_method_call: false,
        }
    }
    
    pub fn full_name(&self) -> String {
        if self.is_method_call {
            if let Some(ref obj) = self.object_name {
                format!("{}.{}", obj, self.function_name)
            } else {
                self.function_name.clone()
            }
        } else {
            self.function_name.clone()
        }
    }
}

/// Comment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentInfo {
    pub line_start: u32,
    pub line_end: u32,
    #[serde(rename = "type")]
    pub comment_type: String,
    pub content: String,
    pub looks_like_code: bool,
}

impl CommentInfo {
    pub fn new(line_start: u32, line_end: u32, comment_type: String, content: String) -> Self {
        Self {
            line_start,
            line_end,
            comment_type,
            content,
            looks_like_code: false,
        }
    }
}

/// Analysis statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub class_count: u32,
    pub function_count: u32,
    pub import_count: u32,
    pub export_count: u32,
    pub unique_calls: u32,
    pub total_calls: u32,
    pub commented_lines_count: u32,
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            class_count: 0,
            function_count: 0,
            import_count: 0,
            export_count: 0,
            unique_calls: 0,
            total_calls: 0,
            commented_lines_count: 0,
        }
    }
}

/// Complete analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    // Basic information
    pub file_info: FileInfo,
    pub language: Language,
    
    // Structure information
    pub classes: Vec<ClassInfo>,
    pub functions: Vec<FunctionInfo>,
    
    // Dependencies
    pub imports: Vec<ImportInfo>,
    pub exports: Vec<ExportInfo>,
    
    // Function calls
    pub function_calls: Vec<FunctionCall>,
    pub call_frequency: HashMap<String, u32>,
    
    // Complexity
    pub complexity: ComplexityInfo,
    
    // Comments
    pub commented_lines: Vec<CommentInfo>,
    
    // Extension metadata
    pub metadata: HashMap<String, String>,
    
    // Statistics
    pub stats: Statistics,
    
    // 🌳 AST Information (new addition for AST revolution)
    pub ast_root: Option<ASTNode>,
    pub ast_statistics: Option<ASTStatistics>,
    
    // Generation timestamp
    pub generated_at: DateTime<Utc>,
}

impl AnalysisResult {
    pub fn new(file_info: FileInfo, language: Language) -> Self {
        Self {
            file_info,
            language,
            classes: Vec::new(),
            functions: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            function_calls: Vec::new(),
            call_frequency: HashMap::new(),
            complexity: ComplexityInfo::new(),
            commented_lines: Vec::new(),
            metadata: HashMap::new(),
            stats: Statistics::default(),
            ast_root: None,
            ast_statistics: None,
            generated_at: Utc::now(),
        }
    }
    
    pub fn update_statistics(&mut self) {
        self.stats.class_count = self.classes.len() as u32;
        self.stats.function_count = self.functions.len() as u32;
        self.stats.import_count = self.imports.len() as u32;
        self.stats.export_count = self.exports.len() as u32;
        self.stats.unique_calls = self.call_frequency.len() as u32;
        self.stats.total_calls = self.function_calls.len() as u32;
        self.stats.commented_lines_count = self.commented_lines.len() as u32;
    }
}

/// Directory analysis summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryAnalysis {
    pub directory_path: PathBuf,
    pub files: Vec<AnalysisResult>,
    pub summary: DirectorySummary,
    pub generated_at: DateTime<Utc>,
}

/// Directory analysis summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorySummary {
    pub total_files: u32,
    pub total_lines: u32,
    pub total_size: u64,
    pub large_files: u32,  // >500 lines
    pub complex_files: u32, // Complex rating or higher
    
    // Code structure statistics
    pub total_classes: u32,
    pub total_functions: u32,
    
    // Overall complexity statistics
    pub total_complexity: u32,
    pub average_complexity: f64,
    pub max_complexity: u32,
    pub most_complex_file: String,
}

impl Default for DirectorySummary {
    fn default() -> Self {
        Self {
            total_files: 0,
            total_lines: 0,
            total_size: 0,
            large_files: 0,
            complex_files: 0,
            total_classes: 0,
            total_functions: 0,
            total_complexity: 0,
            average_complexity: 0.0,
            max_complexity: 0,
            most_complex_file: String::new(),
        }
    }
}

impl DirectoryAnalysis {
    pub fn new(directory_path: PathBuf) -> Self {
        Self {
            directory_path,
            files: Vec::new(),
            summary: DirectorySummary::default(),
            generated_at: Utc::now(),
        }
    }
    
    pub fn update_summary(&mut self) {
        let mut summary = DirectorySummary::default();
        summary.total_files = self.files.len() as u32;
        
        for file in &self.files {
            summary.total_lines += file.file_info.total_lines;
            summary.total_size += file.file_info.size_bytes;
            summary.total_classes += file.stats.class_count;
            summary.total_functions += file.stats.function_count;
            
            if file.file_info.total_lines > 500 {
                summary.large_files += 1;
            }
            
            if matches!(file.complexity.rating, ComplexityRating::Complex | ComplexityRating::VeryComplex) {
                summary.complex_files += 1;
            }
            
            summary.total_complexity += file.complexity.cyclomatic_complexity;
            
            if file.complexity.cyclomatic_complexity > summary.max_complexity {
                summary.max_complexity = file.complexity.cyclomatic_complexity;
                summary.most_complex_file = file.file_info.name.clone();
            }
        }
        
        summary.average_complexity = if summary.total_files > 0 {
            summary.total_complexity as f64 / summary.total_files as f64
        } else {
            0.0
        };
        
        self.summary = summary;
    }
}

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub included_extensions: Vec<String>,
    pub excluded_patterns: Vec<String>,
    pub analyze_complexity: bool,
    pub analyze_dependencies: bool,
    pub analyze_function_calls: bool,
    pub include_test_files: bool,
    pub complete_analysis: bool,
    pub enable_parallel_processing: bool,
    pub max_threads: usize,
    pub verbose_output: bool,
    pub include_line_numbers: bool,
    /// 🚀 Parser type: "pest" (default) or "tree-sitter" (100x faster!)
    pub parser_type: String,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            included_extensions: vec![
                // JavaScript/TypeScript
                ".js".to_string(), ".mjs".to_string(), ".jsx".to_string(), 
                ".ts".to_string(), ".tsx".to_string(),
                // C++
                ".cpp".to_string(), ".cxx".to_string(), ".cc".to_string(), ".C".to_string(),
                ".hpp".to_string(), ".hxx".to_string(), ".hh".to_string(), ".H".to_string(),
                // C
                ".c".to_string(), ".h".to_string(),
                // Python
                ".py".to_string(), ".pyw".to_string(), ".pyi".to_string(),
                // C#
                ".cs".to_string(),
                // Go
                ".go".to_string(),
                // Rust
                ".rs".to_string(),
            ],
            excluded_patterns: vec![
                "node_modules".to_string(), ".git".to_string(), "dist".to_string(), 
                "build".to_string(), "__pycache__".to_string(), "target".to_string(),
            ],
            analyze_complexity: true,
            analyze_dependencies: true,
            analyze_function_calls: true,
            include_test_files: false,
            complete_analysis: false,
            enable_parallel_processing: true, // Re-enabled for parallel processing test
            max_threads: 0, // auto-detect
            verbose_output: false,
            include_line_numbers: true,
            parser_type: "pest".to_string(), // Default to PEST for backward compatibility
        }
    }
}