//! Analyzer traits and common interfaces
//! 
//! This module defines the common interfaces that all language analyzers
//! must implement, providing a consistent API across different languages.

use anyhow::Result;
use async_trait::async_trait;

use crate::core::types::{AnalysisResult, Language};

/// Trait that all language analyzers must implement
#[async_trait]
pub trait LanguageAnalyzer: Send + Sync {
    /// Get the language this analyzer supports
    fn get_language(&self) -> Language;
    
    /// Get the human-readable name of this analyzer
    fn get_language_name(&self) -> &'static str;
    
    /// Get the file extensions this analyzer supports
    fn get_supported_extensions(&self) -> Vec<&'static str>;
    
    /// Analyze source code content and return analysis results
    async fn analyze(&mut self, content: &str, filename: &str) -> Result<AnalysisResult>;
    
    /// Check if this analyzer can handle the given file extension
    fn can_analyze_extension(&self, extension: &str) -> bool {
        self.get_supported_extensions()
            .iter()
            .any(|&ext| ext.eq_ignore_ascii_case(extension))
    }
    
    /// Check if this analyzer can handle the given language
    fn can_analyze_language(&self, language: Language) -> bool {
        self.get_language() == language
    }
}