use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};
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
    /// Build bounded Git context, investigate a symbol, or read a saved packet.
    Context(Box<ContextArgs>),
}

#[derive(Args, Debug)]
#[command(
    group(ArgGroup::new("symbol_mode").args(["at", "symbol", "packet"])),
    group(ArgGroup::new("symbol_query").args(["at", "symbol"]))
)]
pub struct ContextArgs {
    /// Workspace, nested directory or source file; defaults to the current directory.
    pub path: Option<PathBuf>,
    /// Compare committed changes from REF...HEAD; add --working-tree for local edits.
    #[arg(long, conflicts_with = "symbol_mode")]
    pub compare_ref: Option<String>,
    /// Advisory token budget; the compact JSON byte limit is four times this value.
    #[arg(long, default_value = "8000")]
    pub budget: usize,
    /// Collect compiler diagnostics (trusted workspace; may execute build code).
    #[arg(long, conflicts_with = "symbol_mode")]
    pub diagnostics: bool,
    #[arg(long, value_enum, default_value_t = DiagnosticProducerArg::CargoCheck, conflicts_with = "symbol_mode")]
    pub diagnostic_producer: DiagnosticProducerArg,
    /// Include staged, unstaged and untracked Git observations.
    #[arg(long, conflicts_with = "symbol_mode")]
    pub working_tree: bool,
    /// Read untracked contents; requires --working-tree.
    #[arg(long, conflicts_with = "symbol_mode")]
    pub include_untracked_content: bool,
    /// Enable all features for diagnostics or a live symbol investigation.
    #[arg(long)]
    pub all_features: bool,
    /// Source lines around each Git hunk (0 to 200).
    #[arg(long, default_value = "8", conflicts_with = "symbol_mode")]
    pub excerpt_lines: usize,
    /// Saved diagnostic snapshot; use the same features and producer for comparison.
    #[arg(long, conflicts_with = "symbol_mode")]
    pub baseline: Option<PathBuf>,
    /// Investigate a source position: workspace-relative FILE:LINE[:COLUMN], 1-based.
    #[arg(long)]
    pub at: Option<String>,
    /// Resolve an exact symbol name; ambiguous names return candidates.
    #[arg(long)]
    pub symbol: Option<String>,
    /// Read a saved symbol packet without starting Cargo or rust-analyzer.
    #[arg(long, conflicts_with = "all_features")]
    pub packet: Option<PathBuf>,
    /// Compare the earlier saved packet with --packet, without starting analysis.
    #[arg(long, requires = "packet", conflicts_with = "item")]
    pub compare_packet: Option<PathBuf>,
    /// Save captured evidence for later paging and item expansion.
    #[arg(long, requires = "symbol_query")]
    pub save_packet: Option<PathBuf>,
    /// Expand one captured item; requires --packet.
    #[arg(long, requires = "packet", conflicts_with = "cursor")]
    pub item: Option<String>,
    /// Read the next page using the packet's continuation cursor.
    #[arg(long, requires = "packet")]
    pub cursor: Option<String>,
    /// Maximum symbol evidence items per response.
    #[arg(long, default_value = "8", requires = "symbol_mode")]
    pub max_items: usize,
    /// Maximum time for live backend observation (1..600 seconds; default 60).
    #[arg(long, default_value = "60", requires = "symbol_query")]
    pub timeout_seconds: u64,
    /// Enable build scripts and proc-macro preparation in a trusted workspace.
    #[arg(long, requires = "symbol_query")]
    pub allow_build_scripts: bool,
    #[arg(long, value_enum, default_value_t = OutputFormatArg::Json)]
    pub format: OutputFormatArg,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}
