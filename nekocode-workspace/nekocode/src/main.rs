//! Canonical NekoCode CLI: two Rust-first use cases over `nekocode-core`.

mod cli;

use clap::Parser;
use cli::{AnalysisArg, Cli, Commands, OutputFormatArg};
use nekocode_core::{AnalysisMode, ContextRequest, NekocodeError, Result, SnapshotRequest};
use std::fs;

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::Snapshot {
            path,
            output,
            analysis,
            all_features,
        } => snapshot(
            SnapshotRequest {
                path,
                analysis: if matches!(analysis, AnalysisArg::CargoCheck) {
                    AnalysisMode::CargoCheck
                } else {
                    AnalysisMode::MetadataOnly
                },
                all_features,
            },
            output,
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
            format,
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
            format,
            output,
        ),
    }
}

fn snapshot(request: SnapshotRequest, output: Option<std::path::PathBuf>) -> Result<()> {
    if request.all_features && request.analysis == AnalysisMode::MetadataOnly {
        return Err(NekocodeError::Config(
            "--all-features requires --analysis cargo-check".into(),
        ));
    }
    let artifact =
        nekocode_core::sanitize_snapshot_for_output(&nekocode_core::build_snapshot(&request)?)?;
    let json = serde_json::to_string_pretty(&artifact)?;
    if let Some(path) = output {
        nekocode_core::write_rust_snapshot(&path, &artifact)?;
        eprintln!("Rust snapshot written to {}", path.display());
    }
    println!("{json}");
    Ok(())
}

fn context(
    request: ContextRequest,
    format: OutputFormatArg,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
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
    if request.all_features && !request.diagnostics {
        return Err(NekocodeError::Config(
            "--all-features requires --diagnostics".into(),
        ));
    }
    let artifact =
        nekocode_core::sanitize_context_for_output(&nekocode_core::build_context(&request)?)?;
    let rendered = match format {
        OutputFormatArg::Json => serde_json::to_string_pretty(&artifact)?,
        OutputFormatArg::Summary => nekocode_core::format_context_summary(&artifact),
    };
    if let Some(path) = output {
        fs::write(&path, &rendered)?;
        eprintln!("Rust context written to {}", path.display());
    }
    print!("{rendered}");
    if !rendered.ends_with('\n') {
        println!();
    }
    Ok(())
}
