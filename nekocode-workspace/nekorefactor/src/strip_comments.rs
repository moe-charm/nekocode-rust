//! Comment removal functionality using Tree-sitter AST
//! 
//! Safely removes comments from source code while preserving:
//! - License headers
//! - Documentation comments (optional)
//! - Important directives
//! - String literals that look like comments

use anyhow::{anyhow, Result};
use tree_sitter::{Node, Parser, Range};

/// Options for comment stripping
#[derive(Debug, Clone)]
pub struct StripOptions {
    /// Keep documentation comments (JSDoc, docstrings, etc.)
    pub keep_docs: bool,
    
    /// Keep license headers
    pub keep_license: bool,
    
    /// Keep important markers (@important, WARNING, FIXME, etc.)
    pub keep_important: bool,
    
    /// Keep directive comments (eslint-disable, @ts-ignore, etc.)
    pub keep_directives: bool,
    
    /// Only remove inline comments (//)
    pub inline_only: bool,
    
    /// Only remove block comments (/* */)
    pub block_only: bool,
    
    /// Only remove trailing comments
    pub trailing_only: bool,
    
    /// Preview mode - don't modify, just show what would be removed
    pub preview: bool,
    
    /// Show statistics only
    pub stats_only: bool,
}

impl Default for StripOptions {
    fn default() -> Self {
        Self {
            keep_docs: false,
            keep_license: true,  // Default: keep license headers
            keep_important: false,
            keep_directives: true,  // Default: keep directives
            inline_only: false,
            block_only: false,
            trailing_only: false,
            preview: false,
            stats_only: false,
        }
    }
}

/// Language-specific comment formats
#[derive(Debug, Clone)]
pub struct CommentFormat {
    pub line_comment: Vec<&'static str>,
    pub block_comment_start: Vec<&'static str>,
    pub block_comment_end: Vec<&'static str>,
    pub doc_line: Vec<&'static str>,
    pub doc_block_start: Vec<&'static str>,
    pub doc_block_end: Vec<&'static str>,
}

impl CommentFormat {
    /// Get comment format for a specific language
    pub fn for_language(language: &str) -> Self {
        match language.to_lowercase().as_str() {
            "javascript" | "js" | "jsx" | "typescript" | "ts" | "tsx" => Self {
                line_comment: vec!["//"],
                block_comment_start: vec!["/*"],
                block_comment_end: vec!["*/"],
                doc_line: vec!["///"],
                doc_block_start: vec!["/**"],
                doc_block_end: vec!["*/"],
            },
            
            "python" | "py" => Self {
                line_comment: vec!["#"],
                block_comment_start: vec!["\"\"\"", "'''"],
                block_comment_end: vec!["\"\"\"", "'''"],
                doc_line: vec!["#"],
                doc_block_start: vec!["\"\"\"", "'''"],
                doc_block_end: vec!["\"\"\"", "'''"],
            },
            
            "rust" | "rs" => Self {
                line_comment: vec!["//"],
                block_comment_start: vec!["/*"],
                block_comment_end: vec!["*/"],
                doc_line: vec!["///", "//!"],
                doc_block_start: vec!["/**", "/*!"],
                doc_block_end: vec!["*/"],
            },
            
            "c" | "cpp" | "c++" | "cc" | "cxx" | "h" | "hpp" => Self {
                line_comment: vec!["//"],
                block_comment_start: vec!["/*"],
                block_comment_end: vec!["*/"],
                doc_line: vec![],
                doc_block_start: vec!["/**"],
                doc_block_end: vec!["*/"],
            },
            
            "go" => Self {
                line_comment: vec!["//"],
                block_comment_start: vec!["/*"],
                block_comment_end: vec!["*/"],
                doc_line: vec!["//"],
                doc_block_start: vec!["/*"],
                doc_block_end: vec!["*/"],
            },
            
            "csharp" | "cs" => Self {
                line_comment: vec!["//"],
                block_comment_start: vec!["/*"],
                block_comment_end: vec!["*/"],
                doc_line: vec!["///"],
                doc_block_start: vec!["/**"],
                doc_block_end: vec!["*/"],
            },
            
            _ => Self {
                line_comment: vec!["//", "#"],
                block_comment_start: vec!["/*"],
                block_comment_end: vec!["*/"],
                doc_line: vec![],
                doc_block_start: vec![],
                doc_block_end: vec![],
            }
        }
    }
}

/// Comment removal statistics
#[derive(Debug, Default)]
pub struct StripStats {
    pub total_comments: usize,
    pub removed_comments: usize,
    pub kept_comments: usize,
    pub inline_removed: usize,
    pub block_removed: usize,
    pub doc_kept: usize,
    pub license_kept: usize,
    pub directive_kept: usize,
    pub important_kept: usize,
    pub bytes_before: usize,
    pub bytes_after: usize,
}

impl StripStats {
    pub fn reduction_percentage(&self) -> f64 {
        if self.bytes_before == 0 {
            return 0.0;
        }
        ((self.bytes_before - self.bytes_after) as f64 / self.bytes_before as f64) * 100.0
    }
    
    pub fn display(&self) -> String {
        format!(
            r#"📊 Comment Removal Statistics
================================
Total comments found: {}
Comments removed: {}
Comments kept: {}
  - Documentation: {}
  - License headers: {}
  - Directives: {}
  - Important markers: {}

Type breakdown:
  - Inline comments removed: {}
  - Block comments removed: {}

Size reduction:
  - Before: {} bytes
  - After: {} bytes
  - Reduction: {:.1}%"#,
            self.total_comments,
            self.removed_comments,
            self.kept_comments,
            self.doc_kept,
            self.license_kept,
            self.directive_kept,
            self.important_kept,
            self.inline_removed,
            self.block_removed,
            self.bytes_before,
            self.bytes_after,
            self.reduction_percentage()
        )
    }
}

/// Main comment stripper
pub struct CommentStripper {
    parser: Parser,
    language: String,
    format: CommentFormat,
    options: StripOptions,
}

impl CommentStripper {
    /// Create a new comment stripper for a specific language
    pub fn new(language: &str, options: StripOptions) -> Result<Self> {
        let mut parser = Parser::new();
        
        // Set language based on input
        let language_fn = match language.to_lowercase().as_str() {
            "javascript" | "js" | "jsx" => tree_sitter_javascript::language(),
            "typescript" | "ts" | "tsx" => tree_sitter_typescript::language_typescript(),
            "python" | "py" => tree_sitter_python::language(),
            "rust" | "rs" => tree_sitter_rust::language(),
            "c" => tree_sitter_c::language(),
            "cpp" | "c++" | "cc" | "cxx" => tree_sitter_cpp::language(),
            "go" => tree_sitter_go::language(),
            "csharp" | "cs" => tree_sitter_c_sharp::language(),
            _ => return Err(anyhow!("Unsupported language: {}", language)),
        };
        
        parser.set_language(language_fn)?;
        
        Ok(Self {
            parser,
            language: language.to_string(),
            format: CommentFormat::for_language(language),
            options,
        })
    }
    
    /// Strip comments from source code
    pub fn strip(&mut self, source: &str) -> Result<(String, StripStats)> {
        let tree = self.parser.parse(source, None)
            .ok_or_else(|| anyhow!("Failed to parse source code"))?;
        
        let root_node = tree.root_node();
        let mut stats = StripStats::default();
        stats.bytes_before = source.len();
        
        // Collect all comment nodes
        let mut comments_to_remove = Vec::new();
        self.collect_comments(&root_node, source, &mut comments_to_remove, &mut stats)?;

        // Sort comments by position (reverse order for safe removal)
        comments_to_remove.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

        // Build result string (always compute the processed content for consistent stats/preview)
        let mut result = source.to_string();

        // Removed/kept counts are based on detection above
        stats.removed_comments = comments_to_remove.len();
        // Note: kept_comments is already set in collect_comments

        // Always compute the processed content in-memory to ensure
        // stats/preview/real-apply report identical size deltas
        for comment_range in comments_to_remove {
            let start = comment_range.start_byte;
            let end = comment_range.end_byte;

            // Replace comment with appropriate whitespace
            let replacement = self.get_replacement(&source[start..end]);
            result.replace_range(start..end, &replacement);
        }
        
        stats.bytes_after = result.len();
        
        Ok((result, stats))
    }
    
    /// Recursively collect comment nodes
    fn collect_comments(
        &self,
        node: &Node,
        source: &str,
        comments_to_remove: &mut Vec<Range>,
        stats: &mut StripStats,
    ) -> Result<()> {
        // Check if this is a comment node
        if self.is_comment_node(node) {
            stats.total_comments += 1;
            
            let start = node.start_byte();
            let end = node.end_byte();
            let comment_text = &source[start..end];
            
            // Determine if we should keep this comment
            if self.should_keep_comment(comment_text, node) {
                stats.kept_comments += 1;
                
                // Track what type we kept
                if self.is_doc_comment(comment_text) {
                    stats.doc_kept += 1;
                } else if self.is_license_comment(comment_text) {
                    stats.license_kept += 1;
                } else if self.is_directive_comment(comment_text) {
                    stats.directive_kept += 1;
                } else if self.is_important_comment(comment_text) {
                    stats.important_kept += 1;
                }
            } else {
                // Mark for removal
                comments_to_remove.push(node.range());
                
                // Track what type we're removing
                if comment_text.starts_with("//") || comment_text.starts_with("#") {
                    stats.inline_removed += 1;
                } else {
                    stats.block_removed += 1;
                }
            }
        }
        
        // Recurse through children
        for child in node.children(&mut node.walk()) {
            self.collect_comments(&child, source, comments_to_remove, stats)?;
        }
        
        Ok(())
    }
    
    /// Check if a node is a comment
    fn is_comment_node(&self, node: &Node) -> bool {
        let kind = node.kind();
        
        // Comments only - NO string literals
        let is_comment = matches!(kind, 
            "comment" | "line_comment" | "block_comment" |
            "doc_comment" | "documentation"
        );
        
        // Special handling for docstrings (Python triple quotes at start of function/class)
        let is_docstring = if kind == "string" || kind == "string_literal" {
            self.is_docstring_node(node)
        } else {
            false
        };
        
        is_comment || is_docstring
    }
    
    /// Determine if a comment should be kept
    fn should_keep_comment(&self, text: &str, node: &Node) -> bool {
        // Check various keep conditions
        if self.options.keep_docs && self.is_doc_comment(text) {
            return true;
        }
        
        if self.options.keep_license && self.is_license_comment(text) {
            return true;
        }
        
        if self.options.keep_directives && self.is_directive_comment(text) {
            return true;
        }
        
        if self.options.keep_important && self.is_important_comment(text) {
            return true;
        }
        
        // Check filter options (these return true to KEEP comments that don't match the filter)
        if self.options.inline_only && !self.is_inline_comment(text) {
            return true;
        }
        
        if self.options.block_only && !self.is_block_comment(text) {
            return true;
        }
        
        if self.options.trailing_only && !self.is_trailing_comment(node) {
            return true;
        }
        
        false
    }
    
    /// Check if comment is documentation
    fn is_doc_comment(&self, text: &str) -> bool {
        // Check for doc patterns
        text.starts_with("/**") || 
        text.starts_with("///") ||
        text.starts_with("//!") ||
        text.starts_with("/*!") ||
        (self.language == "python" && (text.starts_with("\"\"\"") || text.starts_with("'''")))
    }
    
    /// Check if comment contains license information
    fn is_license_comment(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        lower.contains("copyright") ||
        lower.contains("license") ||
        lower.contains("(c)") ||
        lower.contains("©") ||
        lower.contains("mit ") ||
        lower.contains("apache") ||
        lower.contains("bsd") ||
        lower.contains("gpl")
    }
    
    /// Check if comment is a directive
    fn is_directive_comment(&self, text: &str) -> bool {
        text.contains("eslint-disable") ||
        text.contains("@ts-ignore") ||
        text.contains("@ts-nocheck") ||
        text.contains("istanbul ignore") ||
        text.contains("pragma:") ||
        text.contains("#region") ||
        text.contains("#endregion") ||
        text.contains("prettier-ignore") ||
        text.contains("tslint:disable") ||
        text.contains("rubocop:disable") ||
        text.contains("pylint:") ||
        text.contains("type:") ||
        text.contains("noqa") ||
        text.contains("fmt:")
    }
    
    /// Check if comment contains important markers
    fn is_important_comment(&self, text: &str) -> bool {
        let upper = text.to_uppercase();
        upper.contains("@IMPORTANT") ||
        upper.contains("WARNING") ||
        upper.contains("DANGER") ||
        upper.contains("FIXME") ||
        upper.contains("HACK") ||
        upper.contains("XXX") ||
        upper.contains("SECURITY") ||
        upper.contains("SAFETY")
    }
    
    /// Check if this is an inline comment
    fn is_inline_comment(&self, text: &str) -> bool {
        text.starts_with("//") || text.starts_with("#")
    }
    
    /// Check if this is a block comment
    fn is_block_comment(&self, text: &str) -> bool {
        text.starts_with("/*") || 
        (self.language == "python" && (text.starts_with("\"\"\"") || text.starts_with("'''")))
    }
    
    /// Check if comment is trailing (end of line)
    fn is_trailing_comment(&self, node: &Node) -> bool {
        // A trailing comment is one that starts on the same line as code
        let start_point = node.start_position();
        
        // Check if there's any non-whitespace before this comment on the same line
        // This would require access to the full source context
        // For now, we'll use a heuristic based on column position
        
        start_point.column > 0
    }
    
    /// Check if a string node is actually a docstring (Python)
    fn is_docstring_node(&self, node: &Node) -> bool {
        // Only Python has docstrings that are string literals
        if self.language != "python" {
            return false;
        }
        
        // Get the text to check if it's triple-quoted
        if let Some(parent) = node.parent() {
            let start = node.start_byte();
            let end = node.end_byte();
            
            // For now, we'll be conservative and only treat Python triple-quoted strings
            // at the beginning of functions/classes as docstrings
            let parent_kind = parent.kind();
            
            // Check if this string is at the start of a function or class body
            matches!(parent_kind, 
                "function_definition" | "class_definition" | 
                "method_definition" | "async_function_definition"
            )
        } else {
            false
        }
    }
    
    /// Get replacement string for removed comment
    fn get_replacement(&self, comment_text: &str) -> String {
        // Count newlines in the comment to preserve line structure
        let newline_count = comment_text.matches('\n').count();
        
        if newline_count > 0 {
            // Preserve newlines to maintain line numbers
            "\n".repeat(newline_count)
        } else if comment_text.starts_with("//") || comment_text.starts_with("#") {
            // Inline comment - replace with empty string
            String::new()
        } else {
            // Block comment on single line - replace with space
            " ".to_string()
        }
    }
}
