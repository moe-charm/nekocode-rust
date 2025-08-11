pub mod analyzer;
pub mod tree_sitter_analyzer;
// Grammar is embedded in analyzer.rs via pest_derive

pub use analyzer::JavaScriptAnalyzer;
pub use tree_sitter_analyzer::TreeSitterJavaScriptAnalyzer;