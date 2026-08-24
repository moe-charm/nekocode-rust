use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "nekocode",
    author,
    version,
    about = "Rust-first evidence-backed code context layer"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum AnalysisArg {
    MetadataOnly,
    CargoCheck,
    Clippy,
}

#[derive(Clone, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum DiagnosticProducerArg {
    CargoCheck,
    Clippy,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum OutputFormatArg {
    /// Versioned JSON artifact for machines, MCP, and durable storage.
    Json,
    /// Deterministic plain-text explanation of the collected evidence.
    Summary,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Snapshot a Rust workspace using Cargo metadata.
    #[command(name = "snapshot")]
    Snapshot {
        path: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = AnalysisArg::MetadataOnly)]
        analysis: AnalysisArg,
        #[arg(long)]
        all_features: bool,
    },
    /// Build a bounded Git-diff-focused Rust context artifact.
    Context {
        path: PathBuf,
        #[arg(long)]
        compare_ref: Option<String>,
        #[arg(long, default_value = "8000")]
        budget: usize,
        #[arg(long)]
        diagnostics: bool,
        #[arg(long, value_enum, default_value_t = DiagnosticProducerArg::CargoCheck)]
        diagnostic_producer: DiagnosticProducerArg,
        #[arg(long)]
        working_tree: bool,
        #[arg(long)]
        include_untracked_content: bool,
        #[arg(long)]
        all_features: bool,
        #[arg(long, default_value = "8")]
        excerpt_lines: usize,
        #[arg(long)]
        baseline: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormatArg::Json)]
        format: OutputFormatArg,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}
