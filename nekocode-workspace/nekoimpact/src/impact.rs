//! Core impact analysis functionality

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use chrono::{DateTime, Utc};

use nekocode_core::{
    AnalysisResult, SessionManager,
    FunctionInfo, Result
};

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
    
    pub fn from_change_count(count: usize) -> Self {
        match count {
            0..=2 => RiskLevel::Low,
            3..=10 => RiskLevel::Medium,
            _ => RiskLevel::High,
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
    
    pub fn is_breaking(&self) -> bool {
        matches!(self, 
            ChangeType::FunctionRemoved | 
            ChangeType::ClassRemoved | 
            ChangeType::SignatureChanged |
            ChangeType::TypeChanged
        )
    }
}

/// Information about a symbol that has changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedSymbol {
    pub name: String,
    pub symbol_type: String,
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
    pub reference_type: String,
    pub context: String,
}

/// Result of impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactResult {
    pub changed_symbols: Vec<ChangedSymbol>,
    pub affected_files: HashSet<PathBuf>,
    pub total_references: usize,
    pub risk_assessment: RiskAssessment,
    pub breaking_changes: Vec<BreakingChange>,
    pub analyzed_at: DateTime<Utc>,
}

/// Risk assessment summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub high_risk_count: usize,
    pub medium_risk_count: usize,
    pub low_risk_count: usize,
    pub breaking_change_count: usize,
    pub affected_file_count: usize,
    pub recommendation: String,
}

/// Breaking change information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    pub symbol: String,
    pub change_type: ChangeType,
    pub file_path: PathBuf,
    pub line_number: u32,
    pub description: String,
    pub affected_files: Vec<PathBuf>,
}

/// Impact analyzer
pub struct ImpactAnalyzer {
    session_manager: SessionManager,
}

impl ImpactAnalyzer {
    /// Create new impact analyzer
    pub fn new() -> Result<Self> {
        Ok(Self {
            session_manager: SessionManager::new()?,
        })
    }

    /// Analyze impact against a Git reference for a given session (diff-based)
    pub async fn diff_against_ref(
        &mut self,
        session_id: &str,
        compare_ref: &str,
        include_working: bool,
    ) -> Result<ImpactResult> {
        use std::process::Command;

        // Load session info and analysis results
        let (base_path, analysis_results) = {
            let session = self.session_manager.get_session_mut(session_id)?;
            (session.info.path.clone(), session.info.analysis_results.clone())
        };

        // 1) Collect changed file sets, statuses, and changed line ranges via git
        let mut changed_files: Vec<std::path::PathBuf> = Vec::new();
        let mut changes_by_file: std::collections::HashMap<std::path::PathBuf, Vec<(u32, u32)>> = std::collections::HashMap::new();
        let mut added_files: std::collections::HashSet<std::path::PathBuf> = std::collections::HashSet::new();
        let mut deleted_files: std::collections::HashSet<std::path::PathBuf> = std::collections::HashSet::new();

        // Helper to parse unified diff and populate ranges (new file side)
        fn parse_unified_ranges(diff_text: &str) -> Vec<(u32, u32)> {
            let mut ranges = Vec::new();
            for line in diff_text.lines() {
                if line.starts_with("@@") {
                    // Example: @@ -12,0 +13,5 @@
                    if let Some(pos_plus) = line.find("+") {
                        let rest = &line[pos_plus + 1..];
                        // rest like: 13,5 @@
                        let parts: Vec<&str> = rest.split_whitespace().collect();
                        if let Some(h) = parts.first() {
                            let nums: Vec<&str> = h.split(',').collect();
                            if !nums.is_empty() {
                                let start: u32 = nums[0].parse().unwrap_or(0);
                                let count: u32 = if nums.len() > 1 { nums[1].parse().unwrap_or(1) } else { 1 };
                                let end = if count == 0 { start } else { start.saturating_add(count.saturating_sub(1)) };
                                if start > 0 { ranges.push((start, end)); }
                            }
                        }
                    }
                }
            }
            ranges
        }

        // Get ref..HEAD changed files + ranges
        let out_names = Command::new("git")
            .current_dir(&base_path)
            .args(["diff", &format!("{}..HEAD", compare_ref), "--name-only"])
            .output().ok();
        if let Some(out) = out_names {
            for l in String::from_utf8_lossy(&out.stdout).lines() {
                let t = l.trim();
                if !t.is_empty() { changed_files.push(base_path.join(t)); }
            }
        }
        // Name-status for ref..HEAD
        let out_status = Command::new("git")
            .current_dir(&base_path)
            .args(["diff", &format!("{}..HEAD", compare_ref), "--name-status"])
            .output().ok();
        if let Some(out) = out_status {
            for l in String::from_utf8_lossy(&out.stdout).lines() {
                // e.g., "A\tsrc/foo.rs" or "D\tsrc/bar.rs" or "R100\told\tnew"
                let parts: Vec<&str> = l.split('\t').collect();
                if parts.is_empty() { continue; }
                let code = parts[0];
                if code.starts_with('A') && parts.len() >= 2 {
                    added_files.insert(base_path.join(parts[1]));
                } else if code.starts_with('D') && parts.len() >= 2 {
                    deleted_files.insert(base_path.join(parts[1]));
                } else if code.starts_with('R') && parts.len() >= 3 {
                    // Rename: treat as modified of new path
                    changed_files.push(base_path.join(parts[2]));
                }
            }
        }

        let out_unified = Command::new("git")
            .current_dir(&base_path)
            .args(["diff", &format!("{}..HEAD", compare_ref), "--unified=0"])
            .output().ok();
        let unified_text = out_unified.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
        // Split diff by file markers
        let mut current_file: Option<std::path::PathBuf> = None;
        for line in unified_text.lines() {
            if line.starts_with("+++ b/") {
                let path = line.trim_start_matches("+++ b/").trim();
                current_file = Some(base_path.join(path));
            } else if line.starts_with("@@") {
                if let Some(cf) = current_file.clone() {
                    let ranges = parse_unified_ranges(line);
                    if !ranges.is_empty() {
                        changes_by_file.entry(cf).or_default().extend(ranges);
                    }
                }
            }
        }

        // Include working tree changes if requested (unstaged + staged)
        if include_working {
            let out_w_names = Command::new("git").current_dir(&base_path)
                .args(["diff", "--name-only"]).output().ok();
            if let Some(out) = out_w_names {
                for l in String::from_utf8_lossy(&out.stdout).lines() {
                    let t = l.trim();
                    if !t.is_empty() { changed_files.push(base_path.join(t)); }
                }
            }
            let out_wc_names = Command::new("git").current_dir(&base_path)
                .args(["diff", "--cached", "--name-only"]).output().ok();
            if let Some(out) = out_wc_names {
                for l in String::from_utf8_lossy(&out.stdout).lines() {
                    let t = l.trim();
                    if !t.is_empty() { changed_files.push(base_path.join(t)); }
                }
            }
            let out_w_unified = Command::new("git").current_dir(&base_path)
                .args(["diff", "--unified=0"]).output().ok();
            let w_text = out_w_unified.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
            let mut current_file2: Option<std::path::PathBuf> = None;
            for line in w_text.lines() {
                if line.starts_with("+++ b/") {
                    let path = line.trim_start_matches("+++ b/").trim();
                    current_file2 = Some(base_path.join(path));
                } else if line.starts_with("@@") {
                    if let Some(cf) = current_file2.clone() {
                        let ranges = parse_unified_ranges(line);
                        if !ranges.is_empty() {
                            changes_by_file.entry(cf).or_default().extend(ranges);
                        }
                    }
                }
            }
        }

        // 2) Map changed files to session analysis results
        let mut changed_symbols = Vec::new();
        let mut affected_files = HashSet::new();
        let mut breaking_changes = Vec::new();
        let changed_set: HashSet<_> = changed_files.iter().collect();

        for result in &analysis_results {
            if changed_set.contains(&result.file_info.path) {
                affected_files.insert(result.file_info.path.clone());
                // If we have line ranges, only mark overlapping functions
                let ranges = changes_by_file.get(&result.file_info.path);
                if let Some(ranges) = ranges {
                    for func in &result.functions {
                        let f_start = func.symbol.line_start;
                        let f_end = func.symbol.line_end;
                        let overlaps = ranges.iter().any(|(s, e)| !(f_end < *s || f_start > *e));
                        if overlaps {
                            // Heuristic: if header line overlapped, treat as signature change
                            let header_hit = ranges.iter().any(|(s, e)| *s <= f_start && f_start <= *e);
                            let ctype = if header_hit { ChangeType::SignatureChanged } else { ChangeType::FunctionModified };
                            let change = self.create_changed_symbol(func, ctype);
                            if change.breaking_change {
                                breaking_changes.push(BreakingChange {
                                    symbol: change.name.clone(),
                                    change_type: change.change_type.clone(),
                                    file_path: change.file_path.clone(),
                                    line_number: change.line_number,
                                    description: match change.change_type {
                                        ChangeType::SignatureChanged => format!("{} signature changed", change.name),
                                        ChangeType::FunctionRemoved => format!("Function {} was removed", change.name),
                                        _ => format!("{} was modified", change.name),
                                    },
                                    affected_files: vec![],
                                });
                            }
                            changed_symbols.push(change);
                        }
                    }
                    // Also check class/struct ranges
                    for class in &result.classes {
                        let c_start = class.symbol.line_start;
                        let c_end = class.symbol.line_end;
                        let overlaps = ranges.iter().any(|(s, e)| !(c_end < *s || c_start > *e));
                        if overlaps {
                            changed_symbols.push(ChangedSymbol {
                                name: class.symbol.name.clone(),
                                symbol_type: "class".to_string(),
                                file_path: result.file_info.path.clone(),
                                line_number: class.symbol.line_start,
                                change_type: ChangeType::ClassModified,
                                signature_before: None,
                                signature_after: None,
                                references: vec![],
                                risk_level: if class.is_public { RiskLevel::Medium } else { RiskLevel::Low },
                                breaking_change: false,
                            });
                        }
                    }
                } else {
                    // Fallback: mark all functions
                    for func in &result.functions {
                        let change = self.create_changed_symbol(func, ChangeType::FunctionModified);
                        if change.breaking_change {
                            breaking_changes.push(BreakingChange {
                                symbol: change.name.clone(),
                                change_type: change.change_type.clone(),
                                file_path: change.file_path.clone(),
                                line_number: change.line_number,
                                description: format!("{} was modified", change.name),
                                affected_files: vec![],
                            });
                        }
                        changed_symbols.push(change);
                    }
                    // Fallback: mark class modified
                    for class in &result.classes {
                        changed_symbols.push(ChangedSymbol {
                            name: class.symbol.name.clone(),
                            symbol_type: "class".to_string(),
                            file_path: result.file_info.path.clone(),
                            line_number: class.symbol.line_start,
                            change_type: ChangeType::ClassModified,
                            signature_before: None,
                            signature_after: None,
                            references: vec![],
                            risk_level: if class.is_public { RiskLevel::Medium } else { RiskLevel::Low },
                            breaking_change: false,
                        });
                    }
                }
            }
            // Mark additions: if file is added in this diff and exists in session
            if added_files.contains(&result.file_info.path) {
                affected_files.insert(result.file_info.path.clone());
                for func in &result.functions {
                    changed_symbols.push(ChangedSymbol {
                        name: func.symbol.name.clone(),
                        symbol_type: "function".to_string(),
                        file_path: result.file_info.path.clone(),
                        line_number: func.symbol.line_start,
                        change_type: ChangeType::FunctionAdded,
                        signature_before: None,
                        signature_after: Some(self.get_function_signature(func)),
                        references: vec![],
                        risk_level: if func.is_public { RiskLevel::Low } else { RiskLevel::Low },
                        breaking_change: false,
                    });
                }
            }
        }

        // Mark deletions: files present in deleted set but not in session results
        // We cannot parse base file structurally here; report file-level breaking changes
        for d in deleted_files {
            affected_files.insert(d.clone());
            breaking_changes.push(BreakingChange {
                symbol: d.file_name().and_then(|s| s.to_str()).unwrap_or("<deleted>").to_string(),
                change_type: ChangeType::FunctionRemoved,
                file_path: d.clone(),
                line_number: 0,
                description: format!("File removed: {} (potential API removals)", d.display()),
                affected_files: vec![],
            });
        }

        // 3) Aggregate
        let total_references = changed_symbols
            .iter()
            .map(|s| s.references.len())
            .sum();

        let mut risk_assessment = self.assess_risk(&changed_symbols, &breaking_changes);
        risk_assessment.affected_file_count = affected_files.len();

        Ok(ImpactResult {
            changed_symbols,
            affected_files,
            total_references,
            risk_assessment,
            breaking_changes,
            analyzed_at: Utc::now(),
        })
    }
    
    /// Analyze impact of changes in a session
    pub async fn analyze_session(&mut self, session_id: &str) -> Result<ImpactResult> {
        // Get session data and clone what we need
        let analysis_results = {
            let session = self.session_manager.get_session_mut(session_id)?;
            session.info.analysis_results.clone()
        };
        
        self.analyze_results_internal(&analysis_results)
    }
    
    /// Analyze impact between two sessions
    pub async fn compare_sessions(
        &mut self, 
        base_session_id: &str,
        head_session_id: &str
    ) -> Result<ImpactResult> {
        // Get both sessions' data and clone what we need
        let base_results = {
            let session = self.session_manager.get_session_mut(base_session_id)?;
            session.info.analysis_results.clone()
        };
        
        let head_results = {
            let session = self.session_manager.get_session_mut(head_session_id)?;
            session.info.analysis_results.clone()
        };
        
        self.compare_analysis_results(&base_results, &head_results)
    }
    
    /// Internal analysis implementation
    fn analyze_results_internal(&self, analysis_results: &[AnalysisResult]) -> Result<ImpactResult> {
        let mut changed_symbols = Vec::new();
        let mut affected_files = HashSet::new();
        let mut breaking_changes = Vec::new();
        
        // Analyze each file in results
        for result in analysis_results {
            // Check for changes (simplified for now)
            for function in &result.functions {
                if self.has_function_changed(&function) {
                    let change = self.create_changed_symbol(function, ChangeType::FunctionModified);
                    
                    if change.breaking_change {
                        breaking_changes.push(BreakingChange {
                            symbol: change.name.clone(),
                            change_type: change.change_type.clone(),
                            file_path: change.file_path.clone(),
                            line_number: change.line_number,
                            description: format!("{} was modified", change.name),
                            affected_files: vec![],
                        });
                    }
                    
                    changed_symbols.push(change);
                }
            }
            
            affected_files.insert(result.file_info.path.clone());
        }
        
        let total_references = changed_symbols
            .iter()
            .map(|s| s.references.len())
            .sum();
        
        let risk_assessment = self.assess_risk(&changed_symbols, &breaking_changes);
        
        Ok(ImpactResult {
            changed_symbols,
            affected_files,
            total_references,
            risk_assessment,
            breaking_changes,
            analyzed_at: Utc::now(),
        })
    }
    
    /// Compare analysis results between base and head
    fn compare_analysis_results(
        &self,
        base_results: &[AnalysisResult],
        head_results: &[AnalysisResult]
    ) -> Result<ImpactResult> {
        let mut changed_symbols = Vec::new();
        let mut affected_files = HashSet::new();
        let mut breaking_changes = Vec::new();
        
        // Build maps for comparison
        let base_map: HashMap<PathBuf, &AnalysisResult> = base_results
            .iter()
            .map(|r| (r.file_info.path.clone(), r))
            .collect();
        
        let head_map: HashMap<PathBuf, &AnalysisResult> = head_results
            .iter()
            .map(|r| (r.file_info.path.clone(), r))
            .collect();
        
        // Check for removed files
        for (path, base_result) in &base_map {
            if !head_map.contains_key(path) {
                // File was removed
                for function in &base_result.functions {
                    let change = ChangedSymbol {
                        name: function.symbol.name.clone(),
                        symbol_type: "function".to_string(),
                        file_path: path.clone(),
                        line_number: function.symbol.line_start,
                        change_type: ChangeType::FunctionRemoved,
                        signature_before: Some(self.get_function_signature(function)),
                        signature_after: None,
                        references: vec![],
                        risk_level: RiskLevel::High,
                        breaking_change: true,
                    };
                    
                    breaking_changes.push(BreakingChange {
                        symbol: change.name.clone(),
                        change_type: change.change_type.clone(),
                        file_path: path.clone(),
                        line_number: change.line_number,
                        description: format!("Function {} was removed", change.name),
                        affected_files: vec![],
                    });
                    
                    changed_symbols.push(change);
                }
                
                affected_files.insert(path.clone());
            }
        }
        
        // Check for added and modified files
        for (path, head_result) in &head_map {
            if let Some(base_result) = base_map.get(path) {
                // File exists in both - check for modifications
                let changes = self.compare_file_results(base_result, head_result);
                for change in changes {
                    if change.breaking_change {
                        breaking_changes.push(BreakingChange {
                            symbol: change.name.clone(),
                            change_type: change.change_type.clone(),
                            file_path: change.file_path.clone(),
                            line_number: change.line_number,
                            description: format!("{} {}", change.name, change.change_type.as_str()),
                            affected_files: vec![],
                        });
                    }
                    changed_symbols.push(change);
                }
                
                if !changed_symbols.is_empty() {
                    affected_files.insert(path.clone());
                }
            } else {
                // File was added
                for function in &head_result.functions {
                    let change = ChangedSymbol {
                        name: function.symbol.name.clone(),
                        symbol_type: "function".to_string(),
                        file_path: path.clone(),
                        line_number: function.symbol.line_start,
                        change_type: ChangeType::FunctionAdded,
                        signature_before: None,
                        signature_after: Some(self.get_function_signature(function)),
                        references: vec![],
                        risk_level: RiskLevel::Low,
                        breaking_change: false,
                    };
                    changed_symbols.push(change);
                }
                
                affected_files.insert(path.clone());
            }
        }
        
        let total_references = changed_symbols
            .iter()
            .map(|s| s.references.len())
            .sum();
        
        let risk_assessment = self.assess_risk(&changed_symbols, &breaking_changes);
        
        Ok(ImpactResult {
            changed_symbols,
            affected_files,
            total_references,
            risk_assessment,
            breaking_changes,
            analyzed_at: Utc::now(),
        })
    }
    
    /// Compare file results for changes
    fn compare_file_results(
        &self,
        base: &AnalysisResult,
        head: &AnalysisResult
    ) -> Vec<ChangedSymbol> {
        let mut changes = Vec::new();
        
        // Compare functions
        let base_funcs: HashMap<String, &FunctionInfo> = base.functions
            .iter()
            .map(|f| (f.symbol.name.clone(), f))
            .collect();
        
        let head_funcs: HashMap<String, &FunctionInfo> = head.functions
            .iter()
            .map(|f| (f.symbol.name.clone(), f))
            .collect();
        
        // Check for removed functions
        for (name, base_func) in &base_funcs {
            if !head_funcs.contains_key(name) {
                changes.push(ChangedSymbol {
                    name: name.clone(),
                    symbol_type: "function".to_string(),
                    file_path: base.file_info.path.clone(),
                    line_number: base_func.symbol.line_start,
                    change_type: ChangeType::FunctionRemoved,
                    signature_before: Some(self.get_function_signature(base_func)),
                    signature_after: None,
                    references: vec![],
                    risk_level: RiskLevel::High,
                    breaking_change: true,
                });
            }
        }
        
        // Check for added and modified functions
        for (name, head_func) in &head_funcs {
            if let Some(base_func) = base_funcs.get(name) {
                // Check if signature changed
                let base_sig = self.get_function_signature(base_func);
                let head_sig = self.get_function_signature(head_func);
                
                if base_sig != head_sig {
                    changes.push(ChangedSymbol {
                        name: name.clone(),
                        symbol_type: "function".to_string(),
                        file_path: head.file_info.path.clone(),
                        line_number: head_func.symbol.line_start,
                        change_type: ChangeType::SignatureChanged,
                        signature_before: Some(base_sig),
                        signature_after: Some(head_sig),
                        references: vec![],
                        risk_level: RiskLevel::High,
                        breaking_change: true,
                    });
                }
            } else {
                // Function was added
                changes.push(ChangedSymbol {
                    name: name.clone(),
                    symbol_type: "function".to_string(),
                    file_path: head.file_info.path.clone(),
                    line_number: head_func.symbol.line_start,
                    change_type: ChangeType::FunctionAdded,
                    signature_before: None,
                    signature_after: Some(self.get_function_signature(head_func)),
                    references: vec![],
                    risk_level: RiskLevel::Low,
                    breaking_change: false,
                });
            }
        }
        
        changes
    }
    
    /// Get function signature as string
    fn get_function_signature(&self, func: &FunctionInfo) -> String {
        let params = func.parameters
            .iter()
            .map(|p| {
                if let Some(ref t) = p.param_type {
                    format!("{}: {}", p.name, t)
                } else {
                    p.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        
        let return_type = func.return_type
            .as_ref()
            .map(|t| format!(" -> {}", t))
            .unwrap_or_default();
        
        format!("{}({}){}", func.symbol.name, params, return_type)
    }
    
    /// Check if function has changed (placeholder)
    fn has_function_changed(&self, _func: &FunctionInfo) -> bool {
        // TODO: Implement actual change detection
        false
    }
    
    /// Create changed symbol from function info
    fn create_changed_symbol(&self, func: &FunctionInfo, change_type: ChangeType) -> ChangedSymbol {
        // Heuristic risk: public APIs are riskier even on modification
        let risk = if func.is_public {
            match change_type {
                ChangeType::FunctionRemoved | ChangeType::SignatureChanged => RiskLevel::High,
                ChangeType::FunctionModified => RiskLevel::Medium,
                _ => RiskLevel::Low,
            }
        } else {
            RiskLevel::from_change_count(0)
        };

        ChangedSymbol {
            name: func.symbol.name.clone(),
            symbol_type: "function".to_string(),
            file_path: func.symbol.file_path.clone(),
            line_number: func.symbol.line_start,
            change_type: change_type.clone(),
            signature_before: None,
            signature_after: Some(self.get_function_signature(func)),
            references: vec![],
            risk_level: risk,
            breaking_change: change_type.is_breaking(),
        }
    }
    
    /// Assess overall risk
    fn assess_risk(
        &self,
        changed_symbols: &[ChangedSymbol],
        breaking_changes: &[BreakingChange]
    ) -> RiskAssessment {
        let high_risk_count = changed_symbols
            .iter()
            .filter(|s| s.risk_level == RiskLevel::High)
            .count();
        
        let medium_risk_count = changed_symbols
            .iter()
            .filter(|s| s.risk_level == RiskLevel::Medium)
            .count();
        
        let low_risk_count = changed_symbols
            .iter()
            .filter(|s| s.risk_level == RiskLevel::Low)
            .count();
        
        let breaking_change_count = breaking_changes.len();
        
        let overall_risk = if breaking_change_count > 5 || high_risk_count > 10 {
            RiskLevel::High
        } else if breaking_change_count > 0 || high_risk_count > 0 || medium_risk_count > 5 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };
        
        let recommendation = match overall_risk {
            RiskLevel::High => "⚠️ High risk changes detected. Thorough testing and review required.",
            RiskLevel::Medium => "⚡ Moderate risk changes. Standard testing recommended.",
            RiskLevel::Low => "✅ Low risk changes. Safe to proceed with basic testing.",
        }.to_string();
        
        RiskAssessment {
            overall_risk,
            high_risk_count,
            medium_risk_count,
            low_risk_count,
            breaking_change_count,
            affected_file_count: 0, // Will be set by caller
            recommendation,
        }
    }
}
