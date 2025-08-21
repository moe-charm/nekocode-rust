use std::path::Path;
use nekocode_core::Result;

/// Simple language detector based on file extension
pub struct LanguageDetector;

impl LanguageDetector {
    pub fn new() -> Self {
        Self
    }
    
    pub fn detect_language(&self, file_path: &Path) -> Result<String> {
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
            
        let language = match extension.as_str() {
            "rs" => "rust",
            "py" => "python", 
            "js" => "javascript",
            "ts" => "typescript",
            "cpp" | "cc" | "cxx" => "cpp",
            "c" => "c",
            "go" => "go",
            "cs" => "csharp",
            _ => "unknown",
        };
        
        Ok(language.to_string())
    }
}