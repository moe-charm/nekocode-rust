//! Context request orchestration and bounded artifact construction.

use super::workspace;
use super::*;
use super::{budget, diagnostics, execution, git};
use budget::*;
use diagnostics::*;
use execution::*;
use git::*;
use workspace::*;

pub fn build_rust_context(
    path: impl AsRef<Path>,
    compare_ref: Option<&str>,
    budget_tokens: usize,
) -> Result<RustContextPack> {
    build_rust_context_with_options(path, compare_ref, budget_tokens, false)
}

/// Build a context pack, optionally including compiler diagnostics.
pub fn build_rust_context_with_options(
    path: impl AsRef<Path>,
    compare_ref: Option<&str>,
    budget_tokens: usize,
    include_diagnostics: bool,
) -> Result<RustContextPack> {
    let mut options = RustContextOptions::new(compare_ref.map(str::to_string), budget_tokens);
    options.include_diagnostics = include_diagnostics;
    build_rust_context_with_config(path, options)
}

/// Build a context pack with explicit working-tree, feature, and diff options.
pub fn build_rust_context_with_config(
    path: impl AsRef<Path>,
    options: RustContextOptions,
) -> Result<RustContextPack> {
    if options.budget_tokens == 0 {
        return Err(NekocodeError::Config(
            "context budget must be greater than zero".to_string(),
        ));
    }
    if options.all_features && !options.include_diagnostics {
        return Err(NekocodeError::Config(
            "--all-features requires --diagnostics".to_string(),
        ));
    }
    if options.diagnostic_producer != DiagnosticProducer::CargoCheck && !options.include_diagnostics
    {
        return Err(NekocodeError::Config(
            "--diagnostic-producer requires --diagnostics".to_string(),
        ));
    }

    let initial_workspace = index_rust_workspace(path)?;
    let workspace_root = initial_workspace.root.clone();
    let wants_git =
        options.include_diff || options.compare_ref.is_some() || options.include_working_tree;
    let include_patch = options.include_diff
        || (options.excerpt_lines > 0
            && (options.compare_ref.is_some() || options.include_working_tree));
    let (mut changed_files, diff) = if wants_git {
        let (files, diff) = git_context(
            &workspace_root,
            options.compare_ref.as_deref(),
            options.include_working_tree,
            options.include_untracked_content,
            include_patch,
        )?;
        (files, Some(diff))
    } else {
        (Vec::new(), None)
    };
    annotate_changed_files(
        &mut changed_files,
        &workspace_root,
        &initial_workspace.packages,
    );
    let source_excerpts = build_source_excerpts(
        &workspace_root,
        &changed_files,
        options.excerpt_lines,
        diff.as_ref().and_then(|diff| diff.resolved_head.as_deref()),
    );
    let mut diagnostics = if options.include_diagnostics {
        Some(run_diagnostic_with_options(
            &workspace_root,
            options.diagnostic_producer,
            options.all_features,
        )?)
    } else {
        None
    };
    // A compiler observation can update Cargo's effective input set (for
    // example through a generated lockfile or build configuration). Refresh
    // metadata before computing a diagnostic delta and serializing the pack.
    let workspace = if options.include_diagnostics {
        index_rust_workspace(&workspace_root)?
    } else {
        initial_workspace
    };
    if let Some(run) = diagnostics.as_mut() {
        run.comparison_basis = Some(comparison_basis_for_run(&workspace, run));
    }
    let mut extra_limitations = Vec::new();
    if options.include_working_tree && !options.include_untracked_content {
        extra_limitations.push(
            "Untracked files are reported as markers; use --include-untracked-content to read their contents."
                .to_string(),
        );
    }
    if options.include_untracked_content && !options.include_working_tree {
        extra_limitations
            .push("--include-untracked-content has no effect without --working-tree.".to_string());
    }
    let mut comparison_status = if options.include_diagnostics {
        ComparisonStatus::BaselineMissing
    } else {
        ComparisonStatus::Comparable
    };
    let diagnostic_delta = match options.baseline.as_ref() {
        None => {
            if options.include_diagnostics {
                let delta = build_diagnostic_delta(
                    Path::new(""),
                    None,
                    diagnostics
                        .as_ref()
                        .expect("diagnostics are present when include_diagnostics is true"),
                    true,
                );
                comparison_status = delta.status;
                extra_limitations.extend(delta.limitations.iter().cloned());
                Some(delta)
            } else {
                None
            }
        }
        Some(baseline_path) => match diagnostics.as_ref() {
            None => {
                extra_limitations.push(
                    "Diagnostic baseline was supplied without --diagnostics; no delta was computed."
                        .to_string(),
                );
                comparison_status = ComparisonStatus::BaselineMissing;
                None
            }
            Some(current_run) => match read_rust_snapshot(baseline_path) {
                Ok(baseline) => {
                    let baseline_run = baseline.diagnostics.as_ref();
                    if baseline_run.is_none() {
                        extra_limitations.push(
                            "Diagnostic baseline does not contain a saved diagnostic run; comparison status is baseline_missing."
                                .to_string(),
                        );
                    }
                    let delta = build_diagnostic_delta(
                        baseline_path,
                        baseline_run,
                        current_run,
                        baseline.canonical_hash.is_some(),
                    );
                    comparison_status = delta.status;
                    extra_limitations.extend(delta.limitations.iter().cloned());
                    Some(delta)
                }
                Err(error) => {
                    let delta = invalid_baseline_delta(
                        baseline_path,
                        format!(
                            "Diagnostic baseline could not be read; no delta was computed: {error}"
                        ),
                    );
                    comparison_status = delta.status;
                    extra_limitations.extend(delta.limitations.iter().cloned());
                    Some(delta)
                }
            },
        },
    };
    if let Some(run) = diagnostics
        .as_ref()
        .filter(|run| !diagnostic_run_is_complete(run))
    {
        extra_limitations.push(format!(
            "{} did not produce a complete diagnostic observation (status: {}); diagnostic delta is incomplete.",
            run.producer.command_name(),
            run.status,
        ));
    }

    // Keep the pack bounded even before semantic symbol data is added. The
    // estimate is intentionally conservative: JSON is usually a few bytes per
    // token, and the caller can request a larger budget when needed.
    let byte_budget = options.budget_tokens.saturating_mul(4);
    let mut pack = RustContextPack {
        contract_version: CONTEXT_CONTRACT_VERSION.to_string(),
        artifact_kind: "context".to_string(),
        status: diagnostics_status(diagnostics.as_ref()),
        comparison_status,
        schema_version: SCHEMA_VERSION,
        evidence: EvidenceLevel::ToolConfirmed,
        execution_policy: if options.include_diagnostics {
            diagnostic_execution_policy(options.diagnostic_producer)
        } else {
            metadata_execution_policy()
        },
        root: workspace.root.clone(),
        workspace,
        compare_ref: options.compare_ref.clone(),
        changed_files,
        diff,
        source_excerpts,
        diagnostics,
        diagnostic_producer: options
            .include_diagnostics
            .then_some(options.diagnostic_producer),
        diagnostic_profile: options
            .include_diagnostics
            .then_some(options.diagnostic_producer.profile()),
        baseline: options.baseline.clone(),
        diagnostic_delta,
        budget: BudgetReport {
            requested_tokens: options.budget_tokens,
            max_bytes: byte_budget,
            serialized_bytes: 0,
            exceeded: false,
        },
        budget_tokens: options.budget_tokens,
        estimated_tokens: 0,
        serialized_bytes: 0,
        budget_exceeded: false,
        include_working_tree: options.include_working_tree,
        include_untracked_content: options.include_untracked_content,
        all_features: options.all_features,
        omitted_changed_files: 0,
        omitted_excerpts: 0,
        omitted_diagnostics: 0,
        omitted_delta_items: 0,
        omitted_diff_bytes: 0,
        truncation_order: vec![
            "diff.patch".to_string(),
            "source_excerpts".to_string(),
            "diagnostic_delta".to_string(),
            "diagnostics.messages".to_string(),
            "changed_files".to_string(),
        ],
        limitations: limitations(
            options.include_diagnostics,
            options.include_working_tree,
            &extra_limitations,
            false,
        ),
        omissions: Vec::new(),
    };

    // Patch text is the largest and least structured field. Keep a bounded
    // prefix before trimming individual diagnostics/files so the result stays
    // useful for AI/PR consumers.
    if let Some(diff) = pack.diff.as_mut() {
        let patch_budget = byte_budget / 2;
        if diff.patch.len() > patch_budget {
            let omitted = truncate_utf8(&mut diff.patch, patch_budget);
            diff.patch.push_str("\n... [diff truncated]\n");
            diff.patch_truncated = true;
            diff.omitted_patch_bytes = omitted;
            pack.omitted_diff_bytes = omitted;
        }
    }

    // Trim the least stable, most verbose parts first so the advertised budget
    // remains useful even when cargo emits hundreds of warnings. Workspace
    // metadata is retained as the structural baseline whenever possible.
    while serialized_size(&pack)? > byte_budget {
        if let Some(diff) = pack.diff.as_mut() {
            if !diff.patch.is_empty() {
                let old_len = diff.patch.len();
                let new_len = if old_len > 64 { old_len / 2 } else { 0 };
                if new_len == 0 {
                    diff.patch.clear();
                    let omitted = old_len;
                    diff.patch_truncated = true;
                    diff.omitted_patch_bytes += omitted;
                    pack.omitted_diff_bytes += omitted;
                    continue;
                } else {
                    let omitted = truncate_utf8(&mut diff.patch, new_len);
                    diff.patch.push_str("\n... [diff truncated]\n");
                    diff.patch_truncated = true;
                    diff.omitted_patch_bytes += omitted;
                    pack.omitted_diff_bytes += omitted;
                    continue;
                }
            }
        }
        if pack.source_excerpts.pop().is_some() {
            pack.omitted_excerpts += 1;
            continue;
        }
        if let Some(delta) = pack.diagnostic_delta.as_mut() {
            if pop_diagnostic_delta_item(delta) {
                pack.omitted_delta_items += 1;
                continue;
            }
        }
        if let Some(run) = pack.diagnostics.as_mut() {
            if run.messages.pop().is_some() {
                pack.omitted_diagnostics += 1;
                continue;
            }
        }
        // Keep the diagnostic run envelope (basis, producer, status, and
        // provenance) even when every message has been omitted. Dropping the
        // whole run would hide the conditions needed to interpret a delta and
        // would double-count the omitted diagnostics container.
        if pack.changed_files.pop().is_some() {
            pack.omitted_changed_files += 1;
            continue;
        }
        break;
    }

    pack.omissions = omissions_for_pack(&pack);
    pack.serialized_bytes = serialized_size(&pack)?;
    pack.estimated_tokens = pack.serialized_bytes.div_ceil(4);
    pack.budget_exceeded = pack.serialized_bytes > byte_budget;
    pack.budget.serialized_bytes = pack.serialized_bytes;
    pack.budget.exceeded = pack.budget_exceeded;
    if pack.budget_exceeded {
        pack.status = ArtifactStatus::OutputLimited;
        pack.omissions = omissions_for_pack(&pack);
        pack.serialized_bytes = serialized_size(&pack)?;
        pack.estimated_tokens = pack.serialized_bytes.div_ceil(4);
        pack.budget.serialized_bytes = pack.serialized_bytes;
    }
    let diagnostic_failed = pack
        .diagnostics
        .as_ref()
        .is_some_and(|run| !diagnostic_run_is_complete(run));
    let comparison_incomplete = pack.comparison_status != ComparisonStatus::Comparable
        || pack
            .diagnostic_delta
            .as_ref()
            .is_some_and(|delta| !delta.compatible);
    if pack.omitted_changed_files > 0
        || pack.omitted_excerpts > 0
        || pack.omitted_diagnostics > 0
        || pack.omitted_delta_items > 0
        || pack.omitted_diff_bytes > 0
        || pack.budget_exceeded
        || diagnostic_failed
        || comparison_incomplete
    {
        pack.evidence = EvidenceLevel::Incomplete;
    }
    pack.limitations = limitations(
        options.include_diagnostics,
        options.include_working_tree,
        &extra_limitations,
        pack.omitted_changed_files > 0
            || pack.omitted_excerpts > 0
            || pack.omitted_diagnostics > 0
            || pack.omitted_delta_items > 0
            || pack.omitted_diff_bytes > 0
            || pack.budget_exceeded,
    );
    Ok(pack)
}
