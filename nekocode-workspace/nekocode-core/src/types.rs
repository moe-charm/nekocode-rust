//! Core types shared across all NekoCode tools
//! 
//! This module contains fundamental data structures that are language-agnostic
//! and don't depend on Tree-sitter or any specific parser implementation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
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
            "js" | "mjs" | "jsx" | "cjs" => Language::JavaScript,
            "ts" | "tsx" => Language::TypeScript,
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" | "h++" | "c++" => Language::Cpp,
            "c" | "h" => Language::C,
            "py" | "pyw" | "pyi" => Language::Python,
            "cs" => Language::CSharp,
            "go" => Language::Go,
            "rs" => Language::Rust,
            _ => Language::Unknown,
        }
    }
    
    pub fn from_path(path: &std::path::Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(Self::from_extension)
            .unwrap_or(Language::Unknown)
    }
    
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::JavaScript => &["js", "mjs", "jsx", "cjs"],
            Language::TypeScript => &["ts", "tsx"],
            Language::Cpp => &["cpp", "cxx", "cc", "hpp", "hxx", "hh", "h++", "c++"],
            Language::C => &["c", "h"],
            Language::Python => &["py", "pyw", "pyi"],
            Language::CSharp => &["cs"],
            Language::Go => &["go"],
            Language::Rust => &["rs"],
            Language::Unknown => &[],
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Cpp => "C++",
            Language::C => "C",
            Language::Python => "Python",
            Language::CSharp => "C#",
            Language::Go => "Go",
            Language::Rust => "Rust",
            Language::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// File information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub language: Language,
    pub size_bytes: u64,
    pub total_lines: u32,
    pub code_lines: u32,
    pub comment_lines: u32,
    pub empty_lines: u32,
    pub code_ratio: f64,
    pub analyzed_at: DateTime<Utc>,
    pub hash: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl FileInfo {
    pub fn new(path: PathBuf) -> Self {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let language = Language::from_path(&path);
        
        Self {
            name,
            path,
            language,
            size_bytes: 0,
            total_lines: 0,
            code_lines: 0,
            comment_lines: 0,
            empty_lines: 0,
            code_ratio: 0.0,
            analyzed_at: Utc::now(),
            hash: None,
            metadata: HashMap::new(),
        }
    }
}

/// Symbol type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Class,
    Method,
    Variable,
    Constant,
    Interface,
    Namespace,
    Module,
    Struct,
    Enum,
    Trait,
    Type,
}

/// Visibility/access modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Package,
}

/// Symbol information (function, class, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub id: String,
    pub name: String,
    pub symbol_type: SymbolType,
    pub file_path: PathBuf,
    pub line_start: u32,
    pub line_end: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub language: Language,
    pub visibility: Option<Visibility>,
    pub parent_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Function parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
    pub is_optional: bool,
    pub is_variadic: bool,
}

/// Function-specific information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub symbol: SymbolInfo,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_static: bool,
    pub is_generic: bool,
    pub complexity: Option<u32>,
}

/// Class-specific information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub symbol: SymbolInfo,
    pub base_classes: Vec<String>,
    pub interfaces: Vec<String>,
    pub methods: Vec<String>,  // Symbol IDs
    pub fields: Vec<String>,   // Symbol IDs
    pub is_abstract: bool,
    pub is_interface: bool,
}

/// Import information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub module: String,
    pub imported_names: Vec<String>,
    pub alias: Option<String>,
    pub is_default: bool,
    pub is_namespace: bool,
    pub line: u32,
}

/// Export information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportInfo {
    pub name: String,
    pub export_type: SymbolType,
    pub is_default: bool,
    pub is_reexport: bool,
    pub source_module: Option<String>,
    pub line: u32,
}

/// Code metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    pub lines_of_code: u32,
    pub lines_with_comments: u32,
    pub blank_lines: u32,
    pub cyclomatic_complexity: Option<u32>,
    pub halstead_volume: Option<f64>,
    pub maintainability_index: Option<f64>,
}

impl Default for CodeMetrics {
    fn default() -> Self {
        Self {
            lines_of_code: 0,
            lines_with_comments: 0,
            blank_lines: 0,
            cyclomatic_complexity: None,
            halstead_volume: None,
            maintainability_index: None,
        }
    }
}

/// Analysis result structure (Tree-sitter independent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub file_info: FileInfo,
    pub symbols: Vec<SymbolInfo>,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<ImportInfo>,
    pub exports: Vec<ExportInfo>,
    pub dependencies: Vec<String>,
    pub metrics: CodeMetrics,
    pub errors: Vec<String>,
}

impl AnalysisResult {
    pub fn new(file_info: FileInfo) -> Self {
        Self {
            file_info,
            symbols: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            dependencies: Vec::new(),
            metrics: CodeMetrics::default(),
            errors: Vec::new(),
        }
    }
    
    /// Get all symbols of a specific type
    pub fn get_symbols_by_type(&self, symbol_type: SymbolType) -> Vec<&SymbolInfo> {
        self.symbols
            .iter()
            .filter(|s| s.symbol_type == symbol_type)
            .collect()
    }
    
    /// Get symbol by ID
    pub fn get_symbol_by_id(&self, id: &str) -> Option<&SymbolInfo> {
        self.symbols.iter().find(|s| s.id == id)
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

impl ComplexityRating {
    pub fn from_score(score: u32) -> Self {
        match score {
            0..=10 => ComplexityRating::Simple,
            11..=20 => ComplexityRating::Moderate,
            21..=50 => ComplexityRating::Complex,
            _ => ComplexityRating::VeryComplex,
        }
    }
}