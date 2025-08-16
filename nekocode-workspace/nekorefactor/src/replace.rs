//! Text replacement functionality

use std::path::{Path, PathBuf};
use std::fs;
use regex::Regex;

use nekocode_core::{Result, NekocodeError};
use crate::preview::{MatchInfo, PreviewOperation};

/// Options for replace operations
#[derive(Debug, Clone)]
pub struct ReplaceOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub multiline: bool,
}

impl Default for ReplaceOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            whole_word: false,
            use_regex: false,
            multiline: false,
        }
    }
}

/// Replace engine for text replacement operations
pub struct ReplaceEngine {
    options: ReplaceOptions,
}

impl ReplaceEngine {
    /// Create new replace engine
    pub fn new(options: ReplaceOptions) -> Self {
        Self { options }
    }
    
    /// Create new with default options
    pub fn default() -> Self {
        Self::new(ReplaceOptions::default())
    }
    
    /// Find matches in a file
    pub fn find_matches(
        &self,
        file: &Path,
        pattern: &str
    ) -> Result<Vec<MatchInfo>> {
        let content = fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        self.find_matches_in_content(&content, pattern)
    }
    
    /// Find matches in content
    pub fn find_matches_in_content(
        &self,
        content: &str,
        pattern: &str
    ) -> Result<Vec<MatchInfo>> {
        let mut matches = Vec::new();
        
        if self.options.use_regex {
            self.find_regex_matches(content, pattern, &mut matches)?;
        } else {
            self.find_literal_matches(content, pattern, &mut matches);
        }
        
        Ok(matches)
    }
    
    /// Find literal string matches
    fn find_literal_matches(
        &self,
        content: &str,
        pattern: &str,
        matches: &mut Vec<MatchInfo>
    ) {
        let search_pattern = if self.options.case_sensitive {
            pattern.to_string()
        } else {
            pattern.to_lowercase()
        };
        
        for (line_idx, line) in content.lines().enumerate() {
            let search_line = if self.options.case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };
            
            let mut start = 0;
            while let Some(pos) = search_line[start..].find(&search_pattern) {
                let actual_pos = start + pos;
                
                // Check whole word if needed
                if self.options.whole_word {
                    if !self.is_word_boundary(&search_line, actual_pos, actual_pos + search_pattern.len()) {
                        start = actual_pos + 1;
                        continue;
                    }
                }
                
                matches.push(MatchInfo {
                    line_number: (line_idx + 1) as u32,
                    column_start: actual_pos as u32,
                    column_end: (actual_pos + pattern.len()) as u32,
                    matched_text: line[actual_pos..actual_pos + pattern.len()].to_string(),
                    line_content: line.to_string(),
                });
                
                start = actual_pos + pattern.len();
            }
        }
    }
    
    /// Find regex matches
    fn find_regex_matches(
        &self,
        content: &str,
        pattern: &str,
        matches: &mut Vec<MatchInfo>
    ) -> Result<()> {
        let regex = if self.options.case_sensitive {
            Regex::new(pattern)
        } else {
            Regex::new(&format!("(?i){}", pattern))
        }.map_err(|e| NekocodeError::Other(anyhow::anyhow!("Invalid regex: {}", e)))?;
        
        if self.options.multiline {
            // Multiline regex matching
            for mat in regex.find_iter(content) {
                let start = mat.start();
                let end = mat.end();
                let matched = mat.as_str();
                
                // Find line number and column
                let (line_num, col_start) = self.byte_pos_to_line_col(content, start);
                let (_, col_end) = self.byte_pos_to_line_col(content, end);
                
                // Get the full line content
                let line_content = content.lines()
                    .nth(line_num - 1)
                    .unwrap_or("")
                    .to_string();
                
                matches.push(MatchInfo {
                    line_number: line_num as u32,
                    column_start: col_start as u32,
                    column_end: col_end as u32,
                    matched_text: matched.to_string(),
                    line_content,
                });
            }
        } else {
            // Line-by-line regex matching
            for (line_idx, line) in content.lines().enumerate() {
                for mat in regex.find_iter(line) {
                    matches.push(MatchInfo {
                        line_number: (line_idx + 1) as u32,
                        column_start: mat.start() as u32,
                        column_end: mat.end() as u32,
                        matched_text: mat.as_str().to_string(),
                        line_content: line.to_string(),
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if position is at word boundary
    fn is_word_boundary(&self, text: &str, start: usize, end: usize) -> bool {
        let chars: Vec<char> = text.chars().collect();
        
        // Check start boundary
        if start > 0 {
            let prev = chars[start - 1];
            if prev.is_alphanumeric() || prev == '_' {
                return false;
            }
        }
        
        // Check end boundary
        if end < chars.len() {
            let next = chars[end];
            if next.is_alphanumeric() || next == '_' {
                return false;
            }
        }
        
        true
    }
    
    /// Convert byte position to line and column
    fn byte_pos_to_line_col(&self, content: &str, byte_pos: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        let mut current_pos = 0;
        
        for ch in content.chars() {
            if current_pos >= byte_pos {
                break;
            }
            
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            
            current_pos += ch.len_utf8();
        }
        
        (line, col)
    }
    
    /// Create preview operation for replacement
    pub fn create_preview(
        &self,
        file: PathBuf,
        pattern: String,
        replacement: String
    ) -> Result<PreviewOperation> {
        let matches = self.find_matches(&file, &pattern)?;
        
        Ok(PreviewOperation::Replace {
            file,
            pattern,
            replacement,
            matches,
        })
    }
    
    /// Perform replacement in a file
    pub fn replace_in_file(
        &self,
        file: &Path,
        pattern: &str,
        replacement: &str
    ) -> Result<usize> {
        let content = fs::read_to_string(file)
            .map_err(|e| NekocodeError::Io(e))?;
        
        let (new_content, count) = self.replace_in_content(&content, pattern, replacement)?;
        
        if count > 0 {
            fs::write(file, new_content)
                .map_err(|e| NekocodeError::Io(e))?;
        }
        
        Ok(count)
    }
    
    /// Perform replacement in content
    pub fn replace_in_content(
        &self,
        content: &str,
        pattern: &str,
        replacement: &str
    ) -> Result<(String, usize)> {
        if self.options.use_regex {
            self.replace_regex(content, pattern, replacement)
        } else {
            Ok(self.replace_literal(content, pattern, replacement))
        }
    }
    
    /// Replace literal string
    fn replace_literal(
        &self,
        content: &str,
        pattern: &str,
        replacement: &str
    ) -> (String, usize) {
        if self.options.case_sensitive {
            let parts: Vec<&str> = content.split(pattern).collect();
            let count = parts.len() - 1;
            (parts.join(replacement), count)
        } else {
            // Case-insensitive replacement is more complex
            let mut result = String::new();
            let mut count = 0;
            let mut last_end = 0;
            
            let search_pattern = pattern.to_lowercase();
            let search_content = content.to_lowercase();
            
            let mut start = 0;
            while let Some(pos) = search_content[start..].find(&search_pattern) {
                let actual_pos = start + pos;
                
                // Check whole word if needed
                if self.options.whole_word {
                    if !self.is_word_boundary(&search_content, actual_pos, actual_pos + search_pattern.len()) {
                        start = actual_pos + 1;
                        continue;
                    }
                }
                
                result.push_str(&content[last_end..actual_pos]);
                result.push_str(replacement);
                count += 1;
                
                last_end = actual_pos + pattern.len();
                start = last_end;
            }
            
            result.push_str(&content[last_end..]);
            (result, count)
        }
    }
    
    /// Replace using regex
    fn replace_regex(
        &self,
        content: &str,
        pattern: &str,
        replacement: &str
    ) -> Result<(String, usize)> {
        let regex = if self.options.case_sensitive {
            Regex::new(pattern)
        } else {
            Regex::new(&format!("(?i){}", pattern))
        }.map_err(|e| NekocodeError::Other(anyhow::anyhow!("Invalid regex: {}", e)))?;
        
        let mut count = 0;
        let result = regex.replace_all(content, |_: &regex::Captures| {
            count += 1;
            replacement.to_string()
        });
        
        Ok((result.to_string(), count))
    }
}