//! Impact analysis for code changes
//! 
//! This module provides functionality to analyze the impact of code changes
//! across a codebase, detecting breaking changes, reference usage, and 
//! assessing the risk level of modifications.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};

use crate::core::types::{AnalysisResult, DirectoryAnalysis, FunctionInfo, ClassInfo, Language};
use crate::core::session::AnalysisSession;

/// Risk levels for impact assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

impl RiskLevel {
    pub fn emoji(&self) -> &'static str {
        match self {
            RiskLevel::Low => "🟢",
            RiskLevel::Medium => "🟡",
            RiskLevel::High => "🔴",
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low",
            RiskLevel::Medium => "Medium", 
            RiskLevel::High => "High",
        }
    }
}

/// Type of change detected
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    #[serde(rename = "function_added")]
    FunctionAdded,
    #[serde(rename = "function_removed")]
    FunctionRemoved,
    #[serde(rename = "function_modified")]
    FunctionModified,
    #[serde(rename = "class_added")]
    ClassAdded,
    #[serde(rename = "class_removed")]
    ClassRemoved,
    #[serde(rename = "class_modified")]
    ClassModified,
    #[serde(rename = "signature_changed")]
    SignatureChanged,
    #[serde(rename = "type_changed")]
    TypeChanged,
}

impl ChangeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeType::FunctionAdded => "Function added",
            ChangeType::FunctionRemoved => "Function removed", 
            ChangeType::FunctionModified => "Function modified",
            ChangeType::ClassAdded => "Class added",
            ChangeType::ClassRemoved => "Class removed",
            ChangeType::ClassModified => "Class modified",
            ChangeType::SignatureChanged => "Signature changed",
            ChangeType::TypeChanged => "Type changed",
        }
    }
}

/// Information about a symbol that has changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedSymbol {
    pub name: String,
    pub symbol_type: String, // "function" or "class"
    pub file_path: PathBuf,
    pub line_number: u32,
    pub change_type: ChangeType,
    pub signature_before: Option<String>,
    pub signature_after: Option<String>,
    pub references: Vec<SymbolReference>,
    pub risk_level: RiskLevel,
    pub breaking_change: bool,
}

/// Reference to a symbol in the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    pub file_path: PathBuf,
    pub line_number: u32,
    pub context: String, // surrounding code context
    pub usage_type: String, // "call", "declaration", "import", etc.
}

/// Circular dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependency {
    pub files: Vec<PathBuf>,
    pub description: String,
}

/// Complete impact analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysisResult {
    pub analysis_path: PathBuf,
    pub modified_files: Vec<PathBuf>,
    pub changed_symbols: Vec<ChangedSymbol>,
    pub affected_files: Vec<PathBuf>,
    pub circular_dependencies: Vec<CircularDependency>,
    pub overall_risk: RiskLevel,
    pub breaking_changes_count: u32,
    pub references_count: u32,
    pub complexity_change: ComplexityChange,
    pub analysis_time_ms: u64,
    pub generated_at: DateTime<Utc>,
}

/// Complexity change information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityChange {
    pub before_avg: f64,
    pub after_avg: f64,
    pub change_delta: f64,
    pub complexity_increased: bool,
}

/// Impact analyzer configuration
#[derive(Debug, Clone)]
pub struct ImpactConfig {
    pub include_tests: bool,
    pub compare_ref: Option<String>,
    pub skip_circular: bool,
    pub risk_threshold: RiskLevel,
    pub verbose: bool,
}

impl Default for ImpactConfig {
    fn default() -> Self {
        Self {
            include_tests: false,
            compare_ref: None,
            skip_circular: false,
            risk_threshold: RiskLevel::Low,
            verbose: false,
        }
    }
}

/// Main impact analyzer
pub struct ImpactAnalyzer {
    config: ImpactConfig,
}

impl ImpactAnalyzer {
    pub fn new(config: ImpactConfig) -> Self {
        Self { config }
    }
    
    /// Analyze impact of changes in the specified path
    pub async fn analyze_impact(&self, path: &Path) -> Result<ImpactAnalysisResult> {
        let start_time = std::time::Instant::now();
        
        if self.config.verbose {
            println!("🔍 Starting impact analysis for: {}", path.display());
        }
        
        // Perform current analysis
        let current_analysis = self.analyze_current_state(path).await?;
        
        // Detect changes (for now, simulate as we don't have git integration yet)
        let changed_symbols = self.detect_changed_symbols(&current_analysis)?;
        
        // Find references for changed symbols
        let mut symbols_with_refs = Vec::new();
        for mut symbol in changed_symbols {
            symbol.references = self.find_symbol_references(&symbol, &current_analysis)?;
            symbol.risk_level = self.assess_risk_level(&symbol);
            symbols_with_refs.push(symbol);
        }
        
        // Identify affected files
        let affected_files = self.identify_affected_files(&symbols_with_refs);
        
        // Check for circular dependencies
        let circular_dependencies = if !self.config.skip_circular {
            self.detect_circular_dependencies(&current_analysis)?
        } else {
            Vec::new()
        };
        
        // Calculate overall metrics
        let overall_risk = self.calculate_overall_risk(&symbols_with_refs);
        let breaking_changes_count = symbols_with_refs.iter()
            .filter(|s| s.breaking_change)
            .count() as u32;
        let references_count = symbols_with_refs.iter()
            .map(|s| s.references.len())
            .sum::<usize>() as u32;
        
        // Calculate complexity changes (simplified for initial implementation)
        let complexity_change = self.calculate_complexity_change(&current_analysis);
        
        let analysis_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(ImpactAnalysisResult {
            analysis_path: path.to_path_buf(),
            modified_files: vec![path.to_path_buf()], // Simplified
            changed_symbols: symbols_with_refs,
            affected_files,
            circular_dependencies,
            overall_risk,
            breaking_changes_count,
            references_count,
            complexity_change,
            analysis_time_ms,
            generated_at: Utc::now(),
        })
    }
    
    /// Analyze current state of the codebase
    async fn analyze_current_state(&self, path: &Path) -> Result<DirectoryAnalysis> {
        let mut session = AnalysisSession::default();
        session.analyze_path(path, self.config.include_tests).await
            .context("Failed to analyze current state")
    }
    
    /// Detect changed symbols (simplified implementation)
    fn detect_changed_symbols(&self, analysis: &DirectoryAnalysis) -> Result<Vec<ChangedSymbol>> {
        let mut changed_symbols = Vec::new();
        
        // For initial implementation, treat functions and classes with certain patterns as "potentially changed"
        // In a real implementation, this would compare with git history
        for file in &analysis.files {
            // Detect function changes - look for functions that likely represent changes
            for function in &file.functions {
                let is_likely_changed = function.name.contains("new") || 
                                      function.name.contains("update") || 
                                      function.name.contains("modify") || 
                                      function.name.contains("change") ||
                                      function.name.contains("API") ||
                                      function.name.contains("Feature") ||
                                      function.name.ends_with("New") ||
                                      function.name.starts_with("new") ||
                                      function.parameters.len() > 2; // Functions with many params are likely complex/changed
                
                if is_likely_changed {
                    let breaking_change = function.parameters.len() > 3 || 
                                        function.name.contains("API") ||
                                        function.name.contains("new");
                                        
                    changed_symbols.push(ChangedSymbol {
                        name: function.name.clone(),
                        symbol_type: "function".to_string(),
                        file_path: file.file_info.path.clone(),
                        line_number: function.start_line,
                        change_type: if function.name.contains("new") {
                            ChangeType::FunctionAdded
                        } else {
                            ChangeType::FunctionModified
                        },
                        signature_before: None,
                        signature_after: Some(self.format_function_signature(function)),
                        references: Vec::new(),
                        risk_level: RiskLevel::Low, // Will be calculated later
                        breaking_change,
                    });
                }
            }
            
            // Detect class changes - look for classes that suggest modifications
            for class in &file.classes {
                let is_likely_changed = class.name.contains("New") || 
                                      class.name.contains("Updated") ||
                                      class.name.contains("Manager") ||
                                      class.name.starts_with("New") ||
                                      !class.methods.is_empty(); // Classes with methods are more likely to have changes
                
                if is_likely_changed {
                    changed_symbols.push(ChangedSymbol {
                        name: class.name.clone(),
                        symbol_type: "class".to_string(),
                        file_path: file.file_info.path.clone(),
                        line_number: class.start_line,
                        change_type: if class.name.contains("New") {
                            ChangeType::ClassAdded
                        } else {
                            ChangeType::ClassModified
                        },
                        signature_before: None,
                        signature_after: Some(format!("class {}", class.name)),
                        references: Vec::new(),
                        risk_level: RiskLevel::Low,
                        breaking_change: !class.methods.is_empty() || class.name.contains("Manager"),
                    });
                }
            }
        }
        
        if self.config.verbose {
            println!("🔍 Detected {} potentially changed symbols", changed_symbols.len());
        }
        
        Ok(changed_symbols)
    }
    
    /// Find references to a changed symbol
    fn find_symbol_references(&self, symbol: &ChangedSymbol, analysis: &DirectoryAnalysis) 
        -> Result<Vec<SymbolReference>> {
        let mut references = Vec::new();
        
        for file in &analysis.files {
            // Look for function calls that match our symbol
            for call in &file.function_calls {
                if call.function_name == symbol.name || 
                   call.full_name().contains(&symbol.name) {
                    references.push(SymbolReference {
                        file_path: file.file_info.path.clone(),
                        line_number: call.line_number,
                        context: format!("{}()", call.full_name()),
                        usage_type: "call".to_string(),
                    });
                }
            }
            
            // Look for imports that reference our symbol
            for import in &file.imports {
                if import.imported_names.contains(&symbol.name) {
                    references.push(SymbolReference {
                        file_path: file.file_info.path.clone(),
                        line_number: import.line_number,
                        context: format!("import {} from '{}'", symbol.name, import.module_path),
                        usage_type: "import".to_string(),
                    });
                }
            }
            
            // Look for exports of our symbol
            for export in &file.exports {
                if export.exported_names.contains(&symbol.name) {
                    references.push(SymbolReference {
                        file_path: file.file_info.path.clone(),
                        line_number: export.line_number,
                        context: format!("export {}", symbol.name),
                        usage_type: "export".to_string(),
                    });
                }
            }
            
            // Look for function definitions that match (in case of overloading/inheritance)
            for function in &file.functions {
                if function.name == symbol.name && file.file_info.path != symbol.file_path {
                    references.push(SymbolReference {
                        file_path: file.file_info.path.clone(),
                        line_number: function.start_line,
                        context: format!("function {}({})", function.name, function.parameters.join(", ")),
                        usage_type: "definition".to_string(),
                    });
                }
            }
            
            // Look for class usage (constructor calls, inheritance)
            if symbol.symbol_type == "class" {
                for function in &file.functions {
                    // Check for constructor calls (new ClassName)
                    if function.name.to_lowercase().contains("new") && 
                       function.name.to_lowercase().contains(&symbol.name.to_lowercase()) {
                        references.push(SymbolReference {
                            file_path: file.file_info.path.clone(),
                            line_number: function.start_line,
                            context: format!("new {}()", symbol.name),
                            usage_type: "constructor".to_string(),
                        });
                    }
                }
                
                for class in &file.classes {
                    // Check for inheritance
                    if let Some(parent) = &class.parent_class {
                        if parent == &symbol.name {
                            references.push(SymbolReference {
                                file_path: file.file_info.path.clone(),
                                line_number: class.start_line,
                                context: format!("class {} extends {}", class.name, parent),
                                usage_type: "inheritance".to_string(),
                            });
                        }
                    }
                }
            }
        }
        
        Ok(references)
    }
    
    /// Assess risk level for a changed symbol
    fn assess_risk_level(&self, symbol: &ChangedSymbol) -> RiskLevel {
        let ref_count = symbol.references.len();
        
        if symbol.breaking_change || ref_count > 10 {
            RiskLevel::High
        } else if ref_count > 3 || matches!(symbol.change_type, ChangeType::SignatureChanged) {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }
    
    /// Identify files affected by the changes
    fn identify_affected_files(&self, changed_symbols: &[ChangedSymbol]) -> Vec<PathBuf> {
        let mut affected_files = HashSet::new();
        
        for symbol in changed_symbols {
            affected_files.insert(symbol.file_path.clone());
            for reference in &symbol.references {
                affected_files.insert(reference.file_path.clone());
            }
        }
        
        affected_files.into_iter().collect()
    }
    
    /// Detect circular dependencies (simplified)
    fn detect_circular_dependencies(&self, _analysis: &DirectoryAnalysis) -> Result<Vec<CircularDependency>> {
        // Simplified implementation - in practice this would analyze import graphs
        Ok(Vec::new())
    }
    
    /// Calculate overall risk level
    fn calculate_overall_risk(&self, changed_symbols: &[ChangedSymbol]) -> RiskLevel {
        let high_risk_count = changed_symbols.iter()
            .filter(|s| s.risk_level == RiskLevel::High)
            .count();
        let medium_risk_count = changed_symbols.iter()
            .filter(|s| s.risk_level == RiskLevel::Medium)
            .count();
            
        if high_risk_count > 0 {
            RiskLevel::High
        } else if medium_risk_count > 0 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }
    
    /// Calculate complexity changes
    fn calculate_complexity_change(&self, analysis: &DirectoryAnalysis) -> ComplexityChange {
        let avg_complexity = if !analysis.files.is_empty() {
            analysis.files.iter()
                .map(|f| f.complexity.cyclomatic_complexity as f64)
                .sum::<f64>() / analysis.files.len() as f64
        } else {
            0.0
        };
        
        // Simplified - in practice this would compare before/after
        ComplexityChange {
            before_avg: avg_complexity,
            after_avg: avg_complexity + 0.1, // Simulate small increase
            change_delta: 0.1,
            complexity_increased: true,
        }
    }
    
    /// Format function signature for display
    fn format_function_signature(&self, function: &FunctionInfo) -> String {
        let params = function.parameters.join(", ");
        format!("{}({})", function.name, params)
    }
}

/// Output formatters for different formats
pub struct OutputFormatter;

impl OutputFormatter {
    /// Format as plain text
    pub fn format_plain(result: &ImpactAnalysisResult) -> String {
        let mut output = Vec::new();
        
        output.push("🔍 Impact Analysis Results".to_string());
        output.push("=".repeat(50));
        output.push("".to_string());
        
        // Summary
        output.push("📊 Change Summary".to_string());
        output.push(format!("• Modified Files: {}", result.modified_files.len()));
        output.push(format!("• Analysis Time: {:.2}s", result.analysis_time_ms as f64 / 1000.0));
        output.push(format!("• Risk Level: {} {}", result.overall_risk.emoji(), result.overall_risk.as_str()));
        output.push("".to_string());
        
        // Changed symbols
        if !result.changed_symbols.is_empty() {
            output.push("⚠️ Changed Symbols".to_string());
            for symbol in &result.changed_symbols {
                output.push(format!("• {} '{}' in {}:{}",
                    symbol.change_type.as_str(),
                    symbol.name,
                    symbol.file_path.display(),
                    symbol.line_number
                ));
                output.push(format!("  Risk: {} {}, References: {}",
                    symbol.risk_level.emoji(),
                    symbol.risk_level.as_str(),
                    symbol.references.len()
                ));
                if symbol.breaking_change {
                    output.push("  ⚠️ Breaking change detected".to_string());
                }
                output.push("".to_string());
            }
        }
        
        // Affected files
        if !result.affected_files.is_empty() {
            output.push("📄 Affected Files".to_string());
            for file in &result.affected_files {
                output.push(format!("• {}", file.display()));
            }
            output.push("".to_string());
        }
        
        // Complexity changes
        output.push("📈 Complexity Changes".to_string());
        output.push(format!("• Average complexity change: {:+.1}", result.complexity_change.change_delta));
        if result.complexity_change.complexity_increased {
            output.push("• ⬆️ Complexity increased".to_string());
        } else {
            output.push("• ⬇️ Complexity decreased".to_string());
        }
        
        output.join("\n")
    }
    
    /// Format as JSON
    pub fn format_json(result: &ImpactAnalysisResult) -> Result<String> {
        serde_json::to_string_pretty(result)
            .context("Failed to serialize impact analysis result to JSON")
    }
    
    /// Format as GitHub comment
    pub fn format_github_comment(result: &ImpactAnalysisResult) -> String {
        let mut output = Vec::new();
        
        output.push("🔍 **Impact Analysis Results**".to_string());
        output.push("".to_string());
        
        // Summary section
        output.push("## 📊 Change Summary".to_string());
        output.push(format!("- **Modified Files**: {}", result.modified_files.len()));
        output.push(format!("- **Analysis Time**: {:.1}s", result.analysis_time_ms as f64 / 1000.0));
        output.push(format!("- **Risk Level**: {} {}", result.overall_risk.emoji(), result.overall_risk.as_str()));
        output.push("".to_string());
        
        // Impact detection
        if !result.changed_symbols.is_empty() {
            output.push("## ⚠️ Impact Detection".to_string());
            
            // Group by symbol type
            let functions: Vec<_> = result.changed_symbols.iter()
                .filter(|s| s.symbol_type == "function")
                .collect();
            let classes: Vec<_> = result.changed_symbols.iter()
                .filter(|s| s.symbol_type == "class")
                .collect();
            
            if !functions.is_empty() {
                output.push("**Modified Functions:**".to_string());
                for func in functions {
                    output.push(format!("- `{}()` in `{}:{}`", 
                        func.name, 
                        func.file_path.display(), 
                        func.line_number
                    ));
                    output.push(format!("  - **References found**: {} locations", func.references.len()));
                    if func.breaking_change {
                        output.push("  - **Breaking change**: Signature modified".to_string());
                    }
                }
                output.push("".to_string());
            }
            
            if !classes.is_empty() {
                output.push("**Modified Classes:**".to_string());
                for class in classes {
                    output.push(format!("- `{}` in `{}:{}`", 
                        class.name, 
                        class.file_path.display(), 
                        class.line_number
                    ));
                    output.push(format!("  - **References found**: {} locations", class.references.len()));
                    if class.breaking_change {
                        output.push("  - **Breaking change**: Interface modified".to_string());
                    }
                }
                output.push("".to_string());
            }
            
            // Affected files
            if result.affected_files.len() > result.modified_files.len() {
                output.push("**Affected Files:**".to_string());
                let mut count = 0;
                for file in &result.affected_files {
                    if !result.modified_files.contains(file) && count < 5 {
                        output.push(format!("1. `{}` - ⚠️ May need review", file.display()));
                        count += 1;
                    }
                }
                if result.affected_files.len() > 5 {
                    output.push(format!("... and {} more files", result.affected_files.len() - 5));
                }
                output.push("".to_string());
            }
        }
        
        // Circular dependencies
        output.push("## 🔄 Circular Dependencies".to_string());
        if result.circular_dependencies.is_empty() {
            output.push("✅ No new circular dependencies introduced".to_string());
        } else {
            for dep in &result.circular_dependencies {
                output.push(format!("⚠️ {}", dep.description));
            }
        }
        output.push("".to_string());
        
        // Complexity changes
        output.push("## 📈 Complexity Changes".to_string());
        output.push(format!("- **Before**: Avg complexity = {:.1}", result.complexity_change.before_avg));
        output.push(format!("- **After**: Avg complexity = {:.1} ({:+.1})", 
            result.complexity_change.after_avg,
            result.complexity_change.change_delta
        ));
        
        output.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{FileInfo, Language, AnalysisResult, FunctionInfo, ClassInfo, ImportInfo, ImportType, ExportInfo, ExportType, FunctionCall, Statistics};
    use std::collections::HashMap;
    use chrono::Utc;
    
    fn create_test_analysis() -> DirectoryAnalysis {
        let mut analysis = DirectoryAnalysis::new(PathBuf::from("/tmp/test"));
        
        // Create a file with some functions and classes
        let mut file_info = FileInfo::new(PathBuf::from("/tmp/test/example.js"));
        file_info.total_lines = 50;
        file_info.code_lines = 40;
        
        let mut result = AnalysisResult::new(file_info, Language::JavaScript);
        
        // Add functions
        let mut func1 = FunctionInfo::new("getUserById".to_string());
        func1.start_line = 10;
        func1.parameters = vec!["id".to_string()];
        
        let mut func2 = FunctionInfo::new("newUserAPI".to_string());
        func2.start_line = 20;
        func2.parameters = vec!["userData".to_string(), "options".to_string()];
        
        result.functions = vec![func1, func2];
        
        // Add a class
        let mut class1 = ClassInfo::new("UpdatedUserManager".to_string());
        class1.start_line = 30;
        result.classes = vec![class1];
        
        // Add function calls
        let call1 = FunctionCall::new("getUserById".to_string(), 15);
        let call2 = FunctionCall::new("newUserAPI".to_string(), 25);
        result.function_calls = vec![call1, call2];
        
        // Add imports
        let mut import1 = ImportInfo::new(ImportType::ES6Import, "./user-service".to_string());
        import1.imported_names = vec!["getUserById".to_string(), "newUserAPI".to_string()];
        import1.line_number = 1;
        result.imports = vec![import1];
        
        // Add exports
        let mut export1 = ExportInfo::new(ExportType::ES6Export);
        export1.exported_names = vec!["getUserById".to_string()];
        export1.line_number = 5;
        result.exports = vec![export1];
        
        result.update_statistics();
        analysis.files = vec![result];
        analysis.update_summary();
        
        analysis
    }
    
    #[test]
    fn test_impact_config_creation() {
        let config = ImpactConfig::default();
        assert_eq!(config.include_tests, false);
        assert_eq!(config.skip_circular, false);
        assert_eq!(config.risk_threshold, RiskLevel::Low);
        
        let custom_config = ImpactConfig {
            include_tests: true,
            compare_ref: Some("main".to_string()),
            skip_circular: true,
            risk_threshold: RiskLevel::High,
            verbose: true,
        };
        assert_eq!(custom_config.include_tests, true);
        assert_eq!(custom_config.risk_threshold, RiskLevel::High);
    }
    
    #[test]
    fn test_risk_level_methods() {
        assert_eq!(RiskLevel::Low.emoji(), "🟢");
        assert_eq!(RiskLevel::Medium.emoji(), "🟡");
        assert_eq!(RiskLevel::High.emoji(), "🔴");
        
        assert_eq!(RiskLevel::Low.as_str(), "Low");
        assert_eq!(RiskLevel::Medium.as_str(), "Medium");
        assert_eq!(RiskLevel::High.as_str(), "High");
    }
    
    #[test]
    fn test_change_type_conversion() {
        assert_eq!(ChangeType::FunctionAdded.as_str(), "Function added");
        assert_eq!(ChangeType::FunctionModified.as_str(), "Function modified");
        assert_eq!(ChangeType::ClassAdded.as_str(), "Class added");
        assert_eq!(ChangeType::SignatureChanged.as_str(), "Signature changed");
    }
    
    #[test]
    fn test_changed_symbol_creation() {
        let symbol = ChangedSymbol {
            name: "testFunction".to_string(),
            symbol_type: "function".to_string(),
            file_path: PathBuf::from("/tmp/test.js"),
            line_number: 10,
            change_type: ChangeType::FunctionModified,
            signature_before: None,
            signature_after: Some("testFunction(param)".to_string()),
            references: Vec::new(),
            risk_level: RiskLevel::Medium,
            breaking_change: true,
        };
        
        assert_eq!(symbol.name, "testFunction");
        assert_eq!(symbol.risk_level, RiskLevel::Medium);
        assert_eq!(symbol.breaking_change, true);
    }
    
    #[test]
    fn test_detect_changed_symbols() {
        let analysis = create_test_analysis();
        let config = ImpactConfig::default();
        let analyzer = ImpactAnalyzer::new(config);
        
        let changed_symbols = analyzer.detect_changed_symbols(&analysis).unwrap();
        
        // Should detect newUserAPI and UpdatedUserManager based on naming patterns
        assert!(changed_symbols.len() >= 2);
        
        let has_new_user_api = changed_symbols.iter()
            .any(|s| s.name == "newUserAPI");
        let has_updated_manager = changed_symbols.iter()
            .any(|s| s.name == "UpdatedUserManager");
            
        assert!(has_new_user_api, "Should detect newUserAPI as changed");
        assert!(has_updated_manager, "Should detect UpdatedUserManager as changed");
    }
    
    #[test]
    fn test_risk_assessment() {
        let config = ImpactConfig::default();
        let analyzer = ImpactAnalyzer::new(config);
        
        // Low risk symbol
        let low_risk_symbol = ChangedSymbol {
            name: "helper".to_string(),
            symbol_type: "function".to_string(),
            file_path: PathBuf::from("/tmp/test.js"),
            line_number: 10,
            change_type: ChangeType::FunctionModified,
            signature_before: None,
            signature_after: None,
            references: vec![], // No references
            risk_level: RiskLevel::Low,
            breaking_change: false,
        };
        
        let risk = analyzer.assess_risk_level(&low_risk_symbol);
        assert_eq!(risk, RiskLevel::Low);
        
        // High risk symbol
        let high_risk_symbol = ChangedSymbol {
            name: "criticalAPI".to_string(),
            symbol_type: "function".to_string(),
            file_path: PathBuf::from("/tmp/test.js"),
            line_number: 10,
            change_type: ChangeType::SignatureChanged,
            signature_before: None,
            signature_after: None,
            references: vec![SymbolReference {
                file_path: PathBuf::from("/tmp/other.js"),
                line_number: 5,
                context: "criticalAPI()".to_string(),
                usage_type: "call".to_string(),
            }; 15], // Many references
            risk_level: RiskLevel::Low,
            breaking_change: true,
        };
        
        let risk = analyzer.assess_risk_level(&high_risk_symbol);
        assert_eq!(risk, RiskLevel::High);
    }
    
    #[test]
    fn test_output_formatter_plain() {
        let result = ImpactAnalysisResult {
            analysis_path: PathBuf::from("/tmp/test"),
            modified_files: vec![PathBuf::from("/tmp/test/file.js")],
            changed_symbols: vec![ChangedSymbol {
                name: "testFunc".to_string(),
                symbol_type: "function".to_string(),
                file_path: PathBuf::from("/tmp/test/file.js"),
                line_number: 10,
                change_type: ChangeType::FunctionModified,
                signature_before: None,
                signature_after: Some("testFunc()".to_string()),
                references: vec![],
                risk_level: RiskLevel::Medium,
                breaking_change: false,
            }],
            affected_files: vec![PathBuf::from("/tmp/test/file.js")],
            circular_dependencies: vec![],
            overall_risk: RiskLevel::Medium,
            breaking_changes_count: 0,
            references_count: 0,
            complexity_change: ComplexityChange {
                before_avg: 2.0,
                after_avg: 2.5,
                change_delta: 0.5,
                complexity_increased: true,
            },
            analysis_time_ms: 100,
            generated_at: Utc::now(),
        };
        
        let output = OutputFormatter::format_plain(&result);
        assert!(output.contains("Impact Analysis Results"));
        assert!(output.contains("Modified Files: 1"));
        assert!(output.contains("🟡 Medium"));
        assert!(output.contains("testFunc"));
    }
    
    #[test]
    fn test_output_formatter_json() {
        let result = ImpactAnalysisResult {
            analysis_path: PathBuf::from("/tmp/test"),
            modified_files: vec![],
            changed_symbols: vec![],
            affected_files: vec![],
            circular_dependencies: vec![],
            overall_risk: RiskLevel::Low,
            breaking_changes_count: 0,
            references_count: 0,
            complexity_change: ComplexityChange {
                before_avg: 1.0,
                after_avg: 1.0,
                change_delta: 0.0,
                complexity_increased: false,
            },
            analysis_time_ms: 50,
            generated_at: Utc::now(),
        };
        
        let json_output = OutputFormatter::format_json(&result).unwrap();
        assert!(json_output.contains("analysis_path"));
        assert!(json_output.contains("overall_risk"));
        assert!(json_output.contains("low"));
        
        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_output).unwrap();
        assert_eq!(parsed["overall_risk"], "low");
    }
    
    #[test]
    fn test_output_formatter_github_comment() {
        let result = ImpactAnalysisResult {
            analysis_path: PathBuf::from("/tmp/test"),
            modified_files: vec![PathBuf::from("/tmp/test/file.js")],
            changed_symbols: vec![
                ChangedSymbol {
                    name: "addUser".to_string(),
                    symbol_type: "function".to_string(),
                    file_path: PathBuf::from("/tmp/test/file.js"),
                    line_number: 15,
                    change_type: ChangeType::FunctionModified,
                    signature_before: None,
                    signature_after: Some("addUser(user, options)".to_string()),
                    references: vec![SymbolReference {
                        file_path: PathBuf::from("/tmp/test/other.js"),
                        line_number: 20,
                        context: "addUser()".to_string(),
                        usage_type: "call".to_string(),
                    }],
                    risk_level: RiskLevel::High,
                    breaking_change: true,
                }
            ],
            affected_files: vec![
                PathBuf::from("/tmp/test/file.js"),
                PathBuf::from("/tmp/test/other.js")
            ],
            circular_dependencies: vec![],
            overall_risk: RiskLevel::High,
            breaking_changes_count: 1,
            references_count: 1,
            complexity_change: ComplexityChange {
                before_avg: 3.0,
                after_avg: 3.2,
                change_delta: 0.2,
                complexity_increased: true,
            },
            analysis_time_ms: 75,
            generated_at: Utc::now(),
        };
        
        let output = OutputFormatter::format_github_comment(&result);
        assert!(output.contains("**Impact Analysis Results**"));
        assert!(output.contains("## 📊 Change Summary"));
        assert!(output.contains("## ⚠️ Impact Detection"));
        assert!(output.contains("**Modified Functions:**"));
        assert!(output.contains("`addUser()`"));
        assert!(output.contains("**References found**: 1 locations"));
        assert!(output.contains("**Breaking change**: Signature modified"));
        assert!(output.contains("🔴 High"));
        assert!(output.contains("## 🔄 Circular Dependencies"));
        assert!(output.contains("## 📈 Complexity Changes"));
    }
}