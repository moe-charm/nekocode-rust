//! Dead code analysis report generation

use crate::deadcode::{DeadCodeReport, DeadItem, SymbolType};
use nekocode_core::Language;
use serde_json;
use std::collections::HashMap;

/// Report formatter for different output formats
pub struct ReportFormatter;

impl ReportFormatter {
    /// Format report as JSON
    pub fn format_json(report: &DeadCodeReport, pretty: bool) -> String {
        if pretty {
            serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
        } else {
            serde_json::to_string(report).unwrap_or_else(|_| "{}".to_string())
        }
    }

    /// Format report as human-readable text
    pub fn format_text(report: &DeadCodeReport) -> String {
        let mut output = String::new();
        
        // Header
        output.push_str("🔍 Dead Code Analysis Report\n");
        output.push_str("===========================\n\n");
        
        // Basic statistics
        output.push_str(&format!("📊 Session: {}\n", report.session_id));
        output.push_str(&format!("🕒 Generated: {}\n", report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
        output.push_str(&format!("🔧 Tool: {}\n", report.tool_used));
        output.push_str(&format!("🎯 Confidence: {}%\n\n", report.confidence));
        
        // Statistics
        let stats = report.statistics();
        output.push_str(&format!("📈 Statistics:\n"));
        output.push_str(&format!("  Total symbols analyzed: {}\n", stats.total_symbols));
        
        // Show filtering information if present
        if let (Some(original_count), Some(filter_conf)) = (report.original_dead_count, report.filter_confidence) {
            output.push_str(&format!("  Dead code items found: {} (filtered from {} with ≥{}% confidence)\n", 
                stats.total_dead, original_count, filter_conf));
        } else {
            output.push_str(&format!("  Dead code items found: {}\n", stats.total_dead));
        }
        
        if stats.total_dead > 0 {
            let high_conf_count = report.dead_items.iter()
                .filter(|item| item.confidence >= 80)
                .count();
            output.push_str(&format!("  High confidence items (≥80%): {}\n\n", high_conf_count));
        } else {
            output.push_str("\n");
        }
        
        // By language breakdown
        if !stats.dead_by_language.is_empty() {
            output.push_str("📋 Dead Code by Language:\n");
            for (lang, count) in &stats.dead_by_language {
                output.push_str(&format!("  {:?}: {} items\n", lang, count));
            }
            output.push('\n');
        }
        
        // Confidence distribution
        if !stats.confidence_distribution.is_empty() {
            output.push_str("🎯 Confidence Distribution:\n");
            for (range, count) in &stats.confidence_distribution {
                output.push_str(&format!("  {}: {} items\n", range, count));
            }
            output.push('\n');
        }
        
        // Detailed items
        output.push_str("🗑️ Dead Code Items:\n");
        output.push_str("==================\n\n");
        
        // Group by language for better organization
        let by_language = report.by_language();
        for (language, items) in by_language {
            output.push_str(&format!("## {:?}\n\n", language));
            
            // Group by symbol type
            let mut by_type: HashMap<SymbolType, Vec<&DeadItem>> = HashMap::new();
            for item in items {
                by_type.entry(item.symbol_type)
                    .or_insert_with(Vec::new)
                    .push(item);
            }
            
            for (symbol_type, type_items) in by_type {
                output.push_str(&format!("### {:?}s\n", symbol_type));
                
                for item in type_items {
                    output.push_str(&Self::format_dead_item(item));
                    output.push('\n');
                }
                output.push('\n');
            }
        }
        
        output
    }

    /// Format single dead code item
    fn format_dead_item(item: &DeadItem) -> String {
        format!(
            "- **{}** ({}% confidence)\n  📁 {}\n  📍 Lines {}-{}\n  💡 {}\n",
            item.name,
            item.confidence,
            item.file_path.display(),
            item.line_start,
            item.line_end,
            item.reason
        )
    }

    /// Format as GitHub comment for CI/CD
    pub fn format_github_comment(report: &DeadCodeReport) -> String {
        let mut output = String::new();
        
        // Header with emoji
        output.push_str("## 🔍 Dead Code Analysis Report\n\n");
        
        let stats = report.statistics();
        
        // Summary with emoji indicators
        if stats.total_dead == 0 {
            output.push_str("✅ **No dead code detected!**\n\n");
        } else {
            let confidence_emoji = if report.confidence >= 90 {
                "🔴"
            } else if report.confidence >= 70 {
                "🟡"
            } else {
                "🟢"
            };
            
            output.push_str(&format!(
                "{} **{} dead code items found** ({}% confidence)\n\n",
                confidence_emoji, stats.total_dead, report.confidence
            ));
        }
        
        // Key metrics
        output.push_str("### 📊 Summary\n\n");
        output.push_str(&format!("| Metric | Value |\n"));
        output.push_str(&format!("|--------|-------|\n"));
        output.push_str(&format!("| Total Symbols | {} |\n", stats.total_symbols));
        output.push_str(&format!("| Dead Items | {} |\n", stats.total_dead));
        output.push_str(&format!("| High Confidence | {} |\n", stats.high_confidence_dead));
        output.push_str(&format!("| Tool Used | {} |\n", report.tool_used));
        output.push('\n');
        
        // Language breakdown if multiple languages
        if stats.dead_by_language.len() > 1 {
            output.push_str("### 🌍 By Language\n\n");
            for (lang, count) in &stats.dead_by_language {
                let lang_emoji = Self::get_language_emoji(*lang);
                output.push_str(&format!("- {} {:?}: {} items\n", lang_emoji, lang, count));
            }
            output.push('\n');
        }
        
        // High priority items (high confidence)
        let high_conf_items = report.high_confidence(80);
        if !high_conf_items.is_empty() {
            output.push_str("### 🔥 High Priority Items\n\n");
            output.push_str("> These items have high confidence (≥80%) and should be reviewed:\n\n");
            
            for item in high_conf_items.iter().take(10) { // Limit to top 10
                output.push_str(&format!(
                    "- **{}** in `{}` ({}% confidence)\n",
                    item.name,
                    item.file_path.display(),
                    item.confidence
                ));
            }
            
            if high_conf_items.len() > 10 {
                output.push_str(&format!("\n... and {} more\n", high_conf_items.len() - 10));
            }
            
            output.push('\n');
        }
        
        // Recommendations
        output.push_str("### 💡 Recommendations\n\n");
        if stats.total_dead == 0 {
            output.push_str("✨ Great job! No dead code found in this codebase.\n\n");
        } else {
            if stats.high_confidence_dead > 0 {
                output.push_str("1. 🎯 **Start with high confidence items** - these are most likely safe to remove\n");
            }
            output.push_str("2. 🧪 **Test thoroughly** - run your full test suite before removing code\n");
            output.push_str("3. 🔍 **Review manually** - some items might be used dynamically or in other ways\n");
            if report.tool_used == "internal" {
                output.push_str("4. 🛠️ **Consider external tools** - for more accurate analysis (cargo clippy, vulture, etc.)\n");
            }
        }
        
        // Footer
        output.push_str(&format!(
            "\n---\n*Generated by NekoCode deadcode analyzer on {}*\n",
            report.timestamp.format("%Y-%m-%d %H:%M UTC")
        ));
        
        output
    }

    /// Format as CSV for data analysis
    pub fn format_csv(report: &DeadCodeReport) -> String {
        let mut output = String::new();
        
        // CSV header
        output.push_str("name,type,language,file,line_start,line_end,confidence,reason\n");
        
        // Data rows
        for item in &report.dead_items {
            output.push_str(&format!(
                "\"{}\",\"{:?}\",\"{:?}\",\"{}\",{},{},{},\"{}\"\n",
                item.name.replace('"', "\"\""),
                item.symbol_type,
                item.language,
                item.file_path.display(),
                item.line_start,
                item.line_end,
                item.confidence,
                item.reason.replace('"', "\"\"")
            ));
        }
        
        output
    }

    /// Format as summary statistics only
    pub fn format_summary(report: &DeadCodeReport) -> String {
        let stats = report.statistics();
        
        format!(
            "Dead Code Summary: {} items found ({} high confidence) from {} total symbols using {}",
            stats.total_dead,
            stats.high_confidence_dead,
            stats.total_symbols,
            report.tool_used
        )
    }

    /// Get emoji for language
    fn get_language_emoji(language: Language) -> &'static str {
        match language {
            Language::Rust => "🦀",
            Language::Python => "🐍",
            Language::JavaScript => "🟨",
            Language::TypeScript => "🔷",
            Language::Go => "🐹",
            Language::Cpp => "⚙️",
            Language::C => "🔧",
            Language::CSharp => "💎",
            _ => "📄",
        }
    }
}

/// Output format options
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Json,
    JsonPretty,
    Text,
    GitHubComment,
    Csv,
    Summary,
}

impl OutputFormat {
    /// Parse format from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "json-pretty" | "pretty" => Some(Self::JsonPretty),
            "text" | "txt" => Some(Self::Text),
            "github" | "github-comment" => Some(Self::GitHubComment),
            "csv" => Some(Self::Csv),
            "summary" => Some(Self::Summary),
            _ => None,
        }
    }

    /// Format report using this format
    pub fn format(self, report: &DeadCodeReport) -> String {
        match self {
            Self::Json => ReportFormatter::format_json(report, false),
            Self::JsonPretty => ReportFormatter::format_json(report, true),
            Self::Text => ReportFormatter::format_text(report),
            Self::GitHubComment => ReportFormatter::format_github_comment(report),
            Self::Csv => ReportFormatter::format_csv(report),
            Self::Summary => ReportFormatter::format_summary(report),
        }
    }
}