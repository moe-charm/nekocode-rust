//! NekoCode Rust - High-performance code analysis tool
//! 
//! This is a complete Rust port of the NekoCode C++ analyzer, providing
//! fast, accurate analysis of source code for multiple programming languages.

pub mod core;
pub mod analyzers;

pub use core::types::*;
pub use core::session::AnalysisSession;
pub use core::commands::{Command, CommandProcessor, CommandResult};
pub use analyzers::traits::LanguageAnalyzer;
pub use analyzers::javascript::JavaScriptAnalyzer;