//! Dead code detection module
//! 
//! Integrates with external tools to detect unused code across multiple languages

use nekocode_core::{Result, Session, Language};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

pub mod external;
pub mod rust;
pub mod python; 
pub mod report;

/// Main dead code analyzer
pub struct DeadCodeAnalyzer<'a> {
    session: &'a Session,
    use_external: bool,
}

impl<'a> DeadCodeAnalyzer<'a> {
    /// Create new analyzer from session
    pub fn new(session: &'a Session, use_external: bool) -> Self {
        Self {
            session,
            use_external,
        }
    }

    /// Analyze dead code in the session
    pub async fn analyze(&self) -> Result<DeadCodeReport> {
        // Step 1: Collect all symbols from nekocode analysis
        let all_symbols = self.collect_all_symbols()?;
        
        // Step 2: Analyze references based on mode
        let dead_items = if self.use_external {
            self.analyze_with_external_tools().await?
        } else {
            self.analyze_internal_references(&all_symbols)?
        };
        
        // Step 3: Generate report
        Ok(DeadCodeReport {
            session_id: self.session.id().to_string(),
            total_symbols: all_symbols.len(),
            dead_items,
            tool_used: if self.use_external { "external".to_string() } else { "internal".to_string() },
            confidence: self.calculate_confidence(),
            timestamp: chrono::Utc::now(),
            original_dead_count: None,  // Will be set when filtering
            filter_confidence: None,  // Will be set when filtering
        })
    }

    /// Collect all symbols from session analysis results
    fn collect_all_symbols(&self) -> Result<Vec<SymbolRef>> {
        let mut symbols = Vec::new();
        
        for result in &self.session.info.analysis_results {
            // Add functions
            for func in &result.functions {
                symbols.push(SymbolRef {
                    name: func.symbol.name.clone(),
                    symbol_type: SymbolType::Function,
                    file_path: if func.symbol.file_path.as_os_str().is_empty() {
                        result.file_info.path.clone()
                    } else {
                        func.symbol.file_path.clone()
                    },
                    line_start: func.symbol.line_start,
                    line_end: func.symbol.line_end,
                    language: result.file_info.language,
                });
            }
            
            // Add classes/structs
            for class in &result.classes {
                symbols.push(SymbolRef {
                    name: class.symbol.name.clone(),
                    symbol_type: SymbolType::Class,
                    file_path: if class.symbol.file_path.as_os_str().is_empty() {
                        result.file_info.path.clone()
                    } else {
                        class.symbol.file_path.clone()
                    },
                    line_start: class.symbol.line_start,
                    line_end: class.symbol.line_end,
                    language: result.file_info.language,
                });
            }
        }
        
        Ok(symbols)
    }

    /// Analyze with external tools (cargo clippy, vulture, etc.)
    async fn analyze_with_external_tools(&self) -> Result<Vec<DeadItem>> {
        let mut dead_items = Vec::new();
        
        // Group files by language
        let mut language_files: HashMap<Language, Vec<PathBuf>> = HashMap::new();
        for result in &self.session.info.analysis_results {
            language_files.entry(result.file_info.language)
                .or_insert_with(Vec::new)
                .push(result.file_info.path.clone());
        }
        
        // Analyze each language with appropriate tool
        for (language, files) in language_files {
            let mut items = match language {
                Language::Rust => {
                    rust::RustDeadCodeAnalyzer::analyze_comprehensive(&files).await?
                }
                Language::Python => {
                    python::PythonDeadCodeAnalyzer::analyze_with_vulture(&files).await?
                }
                Language::JavaScript | Language::TypeScript => {
                    // Use internal analysis for JS/TS for now
                    Vec::new()
                }
                _ => Vec::new(),
            };
            dead_items.append(&mut items);
        }
        
        Ok(dead_items)
    }

    /// Analyze references internally using AST data
    fn analyze_internal_references(&self, all_symbols: &[SymbolRef]) -> Result<Vec<DeadItem>> {
        // Simple heuristic: if a symbol is not referenced in imports/exports, it might be dead
        // This is a basic implementation - external tools are more accurate
        
        let mut referenced = std::collections::HashSet::new();
        
        // Collect all references from imports/exports
        for result in &self.session.info.analysis_results {
            for import in &result.imports {
                referenced.insert(import.module.clone());
            }
            for export in &result.exports {
                referenced.insert(export.name.clone());
            }
        }
        
        // Find symbols that are not referenced
        let dead_items: Vec<DeadItem> = all_symbols
            .iter()
            .filter(|symbol| !referenced.contains(&symbol.name))
            .filter(|symbol| !self.is_entry_point(symbol)) // Keep main functions etc.
            .map(|symbol| DeadItem {
                name: symbol.name.clone(),
                symbol_type: symbol.symbol_type,
                file_path: symbol.file_path.clone(),
                line_start: symbol.line_start,
                line_end: symbol.line_end,
                language: symbol.language,
                confidence: 60, // Lower confidence for internal analysis
                reason: "Not referenced in imports/exports".to_string(),
            })
            .collect();
            
        Ok(dead_items)
    }

    /// Check if symbol is an entry point (main, test, etc.)
    fn is_entry_point(&self, symbol: &SymbolRef) -> bool {
        matches!(symbol.name.as_str(), "main" | "test" | "setup" | "teardown") ||
        symbol.name.starts_with("test_") ||
        symbol.name.starts_with("bench_")
    }

    /// Calculate confidence based on analysis method
    fn calculate_confidence(&self) -> u8 {
        if self.use_external {
            90 // External tools are more accurate
        } else {
            60 // Internal analysis is basic
        }
    }
}

/// Reference to a symbol in the codebase
#[derive(Debug, Clone)]
pub struct SymbolRef {
    pub name: String,
    pub symbol_type: SymbolType,
    pub file_path: PathBuf,
    pub line_start: u32,
    pub line_end: u32,
    pub language: Language,
}

/// Type of symbol
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum SymbolType {
    Function,
    Class,
    Variable,
    Constant,
    Module,
}

/// Dead code item found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadItem {
    pub name: String,
    pub symbol_type: SymbolType,
    pub file_path: PathBuf,
    pub line_start: u32,
    pub line_end: u32,
    pub language: Language,
    pub confidence: u8, // 0-100
    pub reason: String,
}

/// Complete dead code analysis report
#[derive(Debug, Serialize, Deserialize)]
pub struct DeadCodeReport {
    pub session_id: String,
    pub total_symbols: usize,
    pub dead_items: Vec<DeadItem>,
    pub tool_used: String,
    pub confidence: u8,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_dead_count: Option<usize>,  // Count before filtering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_confidence: Option<u8>,  // Confidence threshold used for filtering
}

impl DeadCodeReport {
    /// Get dead items by language
    pub fn by_language(&self) -> HashMap<Language, Vec<&DeadItem>> {
        let mut result = HashMap::new();
        for item in &self.dead_items {
            result.entry(item.language)
                .or_insert_with(Vec::new)
                .push(item);
        }
        result
    }

    /// Get high confidence dead items (>= threshold)
    pub fn high_confidence(&self, threshold: u8) -> Vec<&DeadItem> {
        self.dead_items
            .iter()
            .filter(|item| item.confidence >= threshold)
            .collect()
    }

    /// Calculate statistics
    pub fn statistics(&self) -> DeadCodeStats {
        let by_lang = self.by_language();
        let high_conf = self.high_confidence(80);
        
        DeadCodeStats {
            total_symbols: self.total_symbols,
            total_dead: self.dead_items.len(),
            high_confidence_dead: high_conf.len(),
            dead_by_language: by_lang.into_iter()
                .map(|(lang, items)| (lang, items.len()))
                .collect(),
            confidence_distribution: self.confidence_distribution(),
        }
    }

    fn confidence_distribution(&self) -> HashMap<String, usize> {
        let mut dist = HashMap::new();
        for item in &self.dead_items {
            let bucket = match item.confidence {
                90..=100 => "90-100%",
                80..=89 => "80-89%", 
                70..=79 => "70-79%",
                60..=69 => "60-69%",
                _ => "50-59%",
            };
            *dist.entry(bucket.to_string()).or_insert(0) += 1;
        }
        dist
    }
}

/// Statistics about dead code analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct DeadCodeStats {
    pub total_symbols: usize,
    pub total_dead: usize,
    pub high_confidence_dead: usize,
    pub dead_by_language: HashMap<Language, usize>,
    pub confidence_distribution: HashMap<String, usize>,
}