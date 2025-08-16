//! Common traits for NekoCode tools

use async_trait::async_trait;
use crate::types::{AnalysisResult, Language};
use crate::error::Result;

/// Trait for analysis providers (language-specific analyzers)
#[async_trait]
pub trait AnalysisProvider: Send + Sync {
    /// Get supported language
    fn get_language(&self) -> Language;
    
    /// Get human-readable name
    fn get_name(&self) -> &'static str;
    
    /// Get supported file extensions
    fn get_extensions(&self) -> &'static [&'static str];
    
    /// Analyze file content
    async fn analyze(&mut self, content: &str, filename: &str) -> Result<AnalysisResult>;
    
    /// Check if can analyze file
    fn can_analyze(&self, extension: &str) -> bool {
        self.get_extensions()
            .iter()
            .any(|&ext| ext.eq_ignore_ascii_case(extension))
    }
}

/// Trait for tools that work with language analysis
pub trait LanguageSupport {
    /// Get supported languages
    fn supported_languages() -> &'static [Language];
    
    /// Check if language is supported
    fn supports_language(language: Language) -> bool {
        Self::supported_languages().contains(&language)
    }
}

/// Trait for command processors
#[async_trait]
pub trait CommandProcessor: Send + Sync {
    type Config;
    type Result;
    
    /// Process command with given configuration
    async fn process(&mut self, config: Self::Config) -> Result<Self::Result>;
    
    /// Get tool name
    fn tool_name(&self) -> &'static str;
    
    /// Get tool version
    fn tool_version(&self) -> &'static str {
        crate::VERSION
    }
}