//! Compact session format - Optimized for size and speed
//! 
//! This module provides compact JSON serialization that reduces
//! session file sizes by 20-30% through short key names.

use serde_json::{json, Value};
use std::collections::HashMap;

use crate::types::{
    AnalysisResult, FileInfo, SymbolInfo, FunctionInfo, ClassInfo, 
    ImportInfo, ExportInfo, CodeMetrics, Language, SymbolType, 
    Visibility, ParameterInfo
};
use crate::session::SessionInfo;

/// Output format for session serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Human-readable format with full key names (backward compatible)
    Human,
    /// Compact format with 2-letter keys (20-30% smaller)
    Compact,
}

impl Default for OutputMode {
    fn default() -> Self {
        OutputMode::Human  // Keep backward compatibility
    }
}

/// Compact session serializer
pub struct CompactSerializer;

impl CompactSerializer {
    /// Convert SessionInfo to compact JSON
    pub fn to_compact_json(session: &SessionInfo) -> Value {
        json!({
            "id": session.id,  // Keep ID readable
            "pt": session.path,  // path
            "ca": session.created_at.timestamp(),  // created_at
            "la": session.last_accessed.timestamp(),  // last_accessed
            "lm": session.last_modified.timestamp(),  // last_modified
            "md": session.metadata,  // metadata (already short)
            "ar": session.analysis_results.iter().map(|r| {
                Self::analysis_result_to_compact(r)
            }).collect::<Vec<_>>(),  // analysis_results
            "fc": session.file_count,  // file_count
            "tl": session.total_lines,  // total_lines
            "lg": session.languages.iter().map(|(lang, count)| {
                (Self::language_to_compact(lang), count)
            }).collect::<HashMap<_, _>>(),  // languages
            "fh": session.file_hashes,  // file_hashes
            "ls": session.last_scan_time.map(|t| t.timestamp()),  // last_scan_time
            "vr": session.version,  // version
            "dr": session.is_dirty,  // is_dirty
        })
    }
    
    /// Convert AnalysisResult to compact format
    fn analysis_result_to_compact(result: &AnalysisResult) -> Value {
        json!({
            "fi": Self::file_info_to_compact(&result.file_info),
            "sy": result.symbols.iter().map(|s| Self::symbol_to_compact(s)).collect::<Vec<_>>(),
            "fn": result.functions.iter().map(|f| Self::function_to_compact(f)).collect::<Vec<_>>(),
            "cl": result.classes.iter().map(|c| Self::class_to_compact(c)).collect::<Vec<_>>(),
            "im": result.imports.iter().map(|i| Self::import_to_compact(i)).collect::<Vec<_>>(),
            "ex": result.exports.iter().map(|e| Self::export_to_compact(e)).collect::<Vec<_>>(),
            "dp": result.dependencies,
            "mt": Self::metrics_to_compact(&result.metrics),
            "er": result.errors,
        })
    }
    
    /// Convert FileInfo to compact format
    fn file_info_to_compact(info: &FileInfo) -> Value {
        json!({
            "nm": info.name,  // name
            "pt": info.path,  // path
            "lg": Self::language_to_compact(&info.language),  // language
            "sb": info.size_bytes,  // size_bytes
            "tl": info.total_lines,  // total_lines
            "cd": info.code_lines,  // code_lines
            "cm": info.comment_lines,  // comment_lines
            "em": info.empty_lines,  // empty_lines
        })
    }
    
    /// Convert SymbolInfo to compact format
    fn symbol_to_compact(symbol: &SymbolInfo) -> Value {
        json!({
            "id": symbol.id,  // Keep ID readable
            "nm": symbol.name,  // name
            "tp": Self::symbol_type_to_compact(&symbol.symbol_type),  // type
            "fp": symbol.file_path,  // file_path
            "ls": symbol.line_start,  // line_start
            "le": symbol.line_end,  // line_end
            "cs": symbol.column_start,  // column_start
            "ce": symbol.column_end,  // column_end
            "lg": Self::language_to_compact(&symbol.language),  // language
            "vi": symbol.visibility.as_ref().map(|v| Self::visibility_to_compact(v)),  // visibility
            "pr": symbol.parent_id,  // parent_id
            "md": symbol.metadata,  // metadata
        })
    }
    
    /// Convert FunctionInfo to compact format
    fn function_to_compact(func: &FunctionInfo) -> Value {
        json!({
            "sy": Self::symbol_to_compact(&func.symbol),  // symbol
            "pm": func.parameters.iter().map(|p| Self::parameter_to_compact(p)).collect::<Vec<_>>(),  // parameters
            "rt": func.return_type,  // return_type
            "ay": func.is_async,  // is_async
            "st": func.is_static,  // is_static
            "gn": func.is_generic,  // is_generic
            "cx": func.complexity,  // complexity
        })
    }
    
    /// Convert ParameterInfo to compact format
    fn parameter_to_compact(param: &ParameterInfo) -> Value {
        json!({
            "nm": param.name,  // name
            "tp": param.param_type,  // param_type
            "dv": param.default_value,  // default_value
            "op": param.is_optional,  // is_optional
            "va": param.is_variadic,  // is_variadic
        })
    }
    
    /// Convert ClassInfo to compact format
    fn class_to_compact(class: &ClassInfo) -> Value {
        json!({
            "sy": Self::symbol_to_compact(&class.symbol),  // symbol
            "bc": class.base_classes,  // base_classes
            "if": class.interfaces,  // interfaces
            "mt": class.methods,  // methods
            "fd": class.fields,  // fields
            "ab": class.is_abstract,  // is_abstract
            "in": class.is_interface,  // is_interface
        })
    }
    
    /// Convert ImportInfo to compact format
    fn import_to_compact(import: &ImportInfo) -> Value {
        json!({
            "md": import.module,  // module
            "in": import.imported_names,  // imported_names
            "al": import.alias,  // alias
            "df": import.is_default,  // is_default
            "ns": import.is_namespace,  // is_namespace
            "ln": import.line,  // line
        })
    }
    
    /// Convert ExportInfo to compact format
    fn export_to_compact(export: &ExportInfo) -> Value {
        json!({
            "nm": export.name,  // name
            "tp": Self::symbol_type_to_compact(&export.export_type),  // export_type
            "df": export.is_default,  // is_default
            "re": export.is_reexport,  // is_reexport
            "sm": export.source_module,  // source_module
            "ln": export.line,  // line
        })
    }
    
    /// Convert CodeMetrics to compact format
    fn metrics_to_compact(metrics: &CodeMetrics) -> Value {
        json!({
            "lc": metrics.lines_of_code,  // lines_of_code
            "cm": metrics.lines_with_comments,  // lines_with_comments
            "bl": metrics.blank_lines,  // blank_lines
            "cc": metrics.cyclomatic_complexity,  // cyclomatic_complexity
            "hv": metrics.halstead_volume,  // halstead_volume
            "mi": metrics.maintainability_index,  // maintainability_index
        })
    }
    
    /// Convert Language to compact string
    fn language_to_compact(lang: &Language) -> &'static str {
        match lang {
            Language::JavaScript => "js",
            Language::TypeScript => "ts",
            Language::Cpp => "cp",
            Language::C => "c",
            Language::Python => "py",
            Language::CSharp => "cs",
            Language::Go => "go",
            Language::Rust => "rs",
            Language::Unknown => "un",
        }
    }
    
    /// Convert SymbolType to compact string
    fn symbol_type_to_compact(symbol_type: &SymbolType) -> &'static str {
        match symbol_type {
            SymbolType::Function => "fn",
            SymbolType::Class => "cl",
            SymbolType::Method => "mt",
            SymbolType::Variable => "vr",
            SymbolType::Constant => "cn",
            SymbolType::Interface => "if",
            SymbolType::Enum => "en",
            SymbolType::Struct => "st",
            SymbolType::Namespace => "ns",
            SymbolType::Module => "md",
            SymbolType::Trait => "tr",
            SymbolType::Type => "tp",
        }
    }
    
    /// Convert Visibility to compact string
    fn visibility_to_compact(visibility: &Visibility) -> &'static str {
        match visibility {
            Visibility::Public => "pu",
            Visibility::Private => "pr",
            Visibility::Protected => "pt",
            Visibility::Internal => "in",
            Visibility::Package => "pk",
        }
    }
}

/// Human-readable formatter for compact sessions
pub struct HumanFormatter;

impl HumanFormatter {
    /// Convert compact JSON back to human-readable format for display
    pub fn format_compact(compact: &Value) -> String {
        let mut output = String::new();
        
        // Session header
        if let Some(id) = compact["id"].as_str() {
            output.push_str(&format!("📦 Session: {}\n", id));
        }
        
        if let Some(path) = compact["pt"].as_str() {
            output.push_str(&format!("📁 Path: {}\n", path));
        }
        
        if let Some(fc) = compact["fc"].as_u64() {
            output.push_str(&format!("📄 Files: {}\n", fc));
        }
        
        if let Some(tl) = compact["tl"].as_u64() {
            output.push_str(&format!("📏 Total lines: {}\n", tl));
        }
        
        // Language breakdown
        if let Some(languages) = compact["lg"].as_object() {
            output.push_str("\n📊 Languages:\n");
            for (lang_code, count) in languages {
                let lang_name = Self::expand_language_code(lang_code);
                output.push_str(&format!("   {} {}: {}\n", 
                    Self::language_emoji(lang_code), lang_name, count));
            }
        }
        
        // Analysis results summary
        if let Some(results) = compact["ar"].as_array() {
            output.push_str(&format!("\n📈 Analysis Results ({} files):\n", results.len()));
            
            // Show first few files
            for (i, result) in results.iter().take(5).enumerate() {
                if let Some(file_info) = result["fi"].as_object() {
                    let name = file_info["nm"].as_str().unwrap_or("unknown");
                    let lines = file_info["tl"].as_u64().unwrap_or(0);
                    let code = file_info["cd"].as_u64().unwrap_or(0);
                    
                    output.push_str(&format!("   {}. {} ({} lines, {} code)\n", 
                        i + 1, name, lines, code));
                }
            }
            
            if results.len() > 5 {
                output.push_str(&format!("   ... and {} more files\n", results.len() - 5));
            }
        }
        
        output
    }
    
    /// Expand language code to full name
    fn expand_language_code(code: &str) -> &'static str {
        match code {
            "js" => "JavaScript",
            "ts" => "TypeScript",
            "cp" => "C++",
            "c" => "C",
            "py" => "Python",
            "cs" => "C#",
            "go" => "Go",
            "rs" => "Rust",
            _ => "Unknown",
        }
    }
    
    /// Get emoji for language
    fn language_emoji(code: &str) -> &'static str {
        match code {
            "js" | "ts" => "🟨",
            "cp" | "c" => "🔵",
            "py" => "🐍",
            "cs" => "🟣",
            "go" => "🐹",
            "rs" => "🦀",
            _ => "📄",
        }
    }
}