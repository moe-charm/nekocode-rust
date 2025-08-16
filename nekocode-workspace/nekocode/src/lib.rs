//! NekoCode - Core analysis engine with Tree-sitter support

pub mod analyzer;
pub mod ast;
pub mod cli;
pub mod session;

pub use analyzer::{
    Analyzer, AnalyzerConfig,
    JavaScriptAnalyzer, TypeScriptAnalyzer,
    PythonAnalyzer, RustAnalyzer,
    CppAnalyzer, GoAnalyzer, CSharpAnalyzer
};

pub use ast::{ASTBuilder, ASTNode, ASTNodeType, ASTStatistics};
pub use session::{SessionCommands, SessionUpdater};
pub use cli::Cli;