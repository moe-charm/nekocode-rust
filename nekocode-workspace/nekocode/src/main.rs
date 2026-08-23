//! Canonical NekoCode CLI: two Rust-first use cases over `nekocode-core`.

mod cli;

use clap::Parser;
use cli::{AnalysisArg, Cli, Commands};
use nekocode_core::{AnalysisMode, ContextRequest, NekocodeError, Result, SnapshotRequest};
use std::fs;

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::Snapshot {
            path,
            output,
            analysis,
            legacy_snapshot,
            diagnostics,
            all_features,
        } => snapshot(
            SnapshotRequest {
                path,
                analysis: if diagnostics || matches!(analysis, AnalysisArg::CargoCheck) {
                    AnalysisMode::CargoCheck
                } else {
                    AnalysisMode::MetadataOnly
                },
                all_features,
            },
            output,
            legacy_snapshot,
        ),
        Commands::Context {
            path,
            compare_ref,
            budget,
            diagnostics,
            working_tree,
            include_untracked_content,
            all_features,
            excerpt_lines,
            baseline,
            output,
        } => context(
            ContextRequest {
                path,
                compare_ref,
                budget,
                diagnostics,
                working_tree,
                include_untracked_content,
                all_features,
                excerpt_lines,
                baseline,
            },
            output,
        ),
    }
}

fn snapshot(
    request: SnapshotRequest,
    output: Option<std::path::PathBuf>,
    legacy_output: Option<std::path::PathBuf>,
) -> Result<()> {
    if output.is_some() && legacy_output.is_some() {
        return Err(NekocodeError::Config(
            "--snapshot and --output cannot be used together".into(),
        ));
    }
    if request.all_features && request.analysis == AnalysisMode::MetadataOnly {
        return Err(NekocodeError::Config(
            "--all-features requires --analysis cargo-check".into(),
        ));
    }
    let artifact =
        nekocode_core::sanitize_snapshot_for_output(&nekocode_core::build_snapshot(&request)?)?;
    let json = serde_json::to_string_pretty(&artifact)?;
    if let Some(path) = output.or(legacy_output) {
        nekocode_core::write_rust_snapshot(&path, &artifact)?;
        eprintln!("Rust snapshot written to {}", path.display());
    }
    println!("{json}");
    Ok(())
}

fn context(request: ContextRequest, output: Option<std::path::PathBuf>) -> Result<()> {
    if request.excerpt_lines > 200 {
        return Err(NekocodeError::Config(
            "--excerpt-lines must be between 0 and 200".into(),
        ));
    }
    if request.include_untracked_content && !request.working_tree {
        return Err(NekocodeError::Config(
            "--include-untracked-content requires --working-tree".into(),
        ));
    }
    let artifact =
        nekocode_core::sanitize_context_for_output(&nekocode_core::build_context(&request)?)?;
    let json = serde_json::to_string_pretty(&artifact)?;
    if let Some(path) = output {
        fs::write(&path, &json)?;
        eprintln!("Rust context written to {}", path.display());
    }
    println!("{json}");
    Ok(())
}
