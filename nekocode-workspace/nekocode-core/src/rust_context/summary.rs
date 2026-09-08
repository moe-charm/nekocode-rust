//! Deterministic human-readable projection of a completed context artifact.

use super::diagnostics;
use super::*;
use diagnostics::*;
use std::fmt::Write as FmtWrite;
use std::path::Path;

pub fn format_context_summary(pack: &ContextV1) -> String {
    let mut output = String::new();
    let rust_files = pack
        .changed_files
        .iter()
        .filter(|file| file.is_rust)
        .count();
    let hunk_count: usize = pack.changed_files.iter().map(|file| file.hunks.len()).sum();
    let changed_file_count = item_count(pack.changed_files.len(), "file", "files");
    let changed_hunk_count = item_count(hunk_count, "hunk", "hunks");
    let (additions, deletions) = pack
        .diff
        .as_ref()
        .map(|diff| count_patch_lines(&diff.patch))
        .unwrap_or((0, 0));

    writeln!(output, "NekoCode change summary").expect("write to String");
    writeln!(output, "Status: {}", artifact_status_name(pack.status)).expect("write to String");
    writeln!(
        output,
        "Comparison: {}",
        comparison_status_name(pack.comparison_status)
    )
    .expect("write to String");
    writeln!(output, "Evidence: {}", evidence_level_name(pack.evidence)).expect("write to String");
    writeln!(
        output,
        "Execution: {}; workspace_trust={}; process_network_isolation={}",
        analysis_mode_name(pack.execution_policy.mode),
        pack.execution_policy.workspace_trust,
        pack.execution_policy.process_network_isolation
    )
    .expect("write to String");
    writeln!(
        output,
        "Changes: {changed_file_count} ({rust_files} Rust), {changed_hunk_count}",
    )
    .expect("write to String");
    if let Some(diff) = &pack.diff {
        if !diff.change_scopes.is_empty() {
            writeln!(output, "Change scopes (pre-budget totals):").expect("write to String");
            for scope in &diff.change_scopes {
                let mut unknowns = Vec::new();
                if scope.binary_files > 0 {
                    unknowns.push(format!("{} binary", scope.binary_files));
                }
                if scope.not_read_files > 0 {
                    unknowns.push(format!("{} not read", scope.not_read_files));
                }
                let unknowns = if unknowns.is_empty() {
                    String::new()
                } else {
                    format!("; unknown line counts: {}", unknowns.join(", "))
                };
                writeln!(
                    output,
                    "- {}: {} ({} Rust), +{}/-{} counted lines across {}{}",
                    git_change_scope_name(scope.scope),
                    item_count(scope.file_count, "file", "files"),
                    scope.rust_file_count,
                    scope.additions,
                    scope.deletions,
                    item_count(scope.counted_files, "file", "files"),
                    unknowns
                )
                .expect("write to String");
            }
        }
        if diff.patch_truncated && diff.patch.trim().is_empty() {
            writeln!(
                output,
                "Visible patch: omitted to fit budget; {} bytes omitted",
                diff.omitted_patch_bytes
            )
            .expect("write to String");
        } else {
            let truncation = if diff.patch_truncated {
                format!("; truncated, {} bytes omitted", diff.omitted_patch_bytes)
            } else {
                String::new()
            };
            writeln!(
                output,
                "Visible patch: +{additions}/-{deletions} lines{truncation}"
            )
            .expect("write to String");
        }
    } else {
        writeln!(output, "Visible patch: not requested").expect("write to String");
    }

    if pack.changed_files.is_empty() {
        writeln!(output, "Files: none").expect("write to String");
    } else {
        writeln!(output, "Files:").expect("write to String");
        for file in pack.changed_files.iter().take(SUMMARY_FILE_LIMIT) {
            let old_path = file
                .old_path
                .as_ref()
                .map(|path| format!(" <- {}", display_path(path)))
                .unwrap_or_default();
            let package = file
                .package
                .as_ref()
                .map(|name| format!(" [{name}]"))
                .unwrap_or_default();
            writeln!(
                output,
                "- {} {}{}{} ({})",
                file.status,
                display_path(&file.path),
                old_path,
                package,
                item_count(file.hunks.len(), "hunk", "hunks")
            )
            .expect("write to String");
        }
        if pack.changed_files.len() > SUMMARY_FILE_LIMIT {
            writeln!(
                output,
                "- ... {} more included files not displayed",
                pack.changed_files.len() - SUMMARY_FILE_LIMIT
            )
            .expect("write to String");
        }
    }

    format_diagnostics(&mut output, pack);

    writeln!(
        output,
        "Budget: {}/{} bytes; {}",
        pack.budget.serialized_bytes,
        pack.budget.max_bytes,
        if pack.budget.exceeded {
            "limit exceeded"
        } else if pack.omissions.is_empty() {
            "within limit"
        } else {
            "within limit with omissions"
        }
    )
    .expect("write to String");

    if pack.omissions.is_empty() {
        writeln!(output, "Omissions: none").expect("write to String");
    } else {
        writeln!(output, "Omissions:").expect("write to String");
        for omission in &pack.omissions {
            writeln!(
                output,
                "- {}: {} omitted ({}, priority={})",
                omission.kind, omission.omitted_count, omission.reason, omission.priority
            )
            .expect("write to String");
        }
    }

    if !pack.limitations.is_empty() {
        writeln!(output, "Limitations:").expect("write to String");
        for limitation in pack.limitations.iter().take(SUMMARY_LIMITATION_LIMIT) {
            writeln!(output, "- {}", single_line(limitation)).expect("write to String");
        }
        if pack.limitations.len() > SUMMARY_LIMITATION_LIMIT {
            writeln!(
                output,
                "- ... {} more limitations not displayed",
                pack.limitations.len() - SUMMARY_LIMITATION_LIMIT
            )
            .expect("write to String");
        }
    }

    output
}

pub(crate) fn format_diagnostics(output: &mut String, pack: &ContextV1) {
    if let Some(run) = &pack.diagnostics {
        let unique = unique_primary_diagnostics(&run.messages);
        let errors = unique
            .iter()
            .filter(|diagnostic| diagnostic.level == "error")
            .count();
        let warnings = unique
            .iter()
            .filter(|diagnostic| diagnostic.level == "warning")
            .count();
        writeln!(
            output,
            "Diagnostics: {}; producer_status={}; producer={}; profile={}; {} ({} errors, {} warnings; {} raw messages)",
            artifact_status_name(diagnostic_status(run)),
            run.status,
            run.producer.display_name(),
            diagnostic_profile_name(run.profile),
            item_count(
                unique.len(),
                "unique primary diagnostic",
                "unique primary diagnostics"
            ),
            errors,
            warnings,
            run.messages.len()
        )
        .expect("write to String");
    } else {
        writeln!(output, "Diagnostics: not run").expect("write to String");
    }

    if let Some(delta) = &pack.diagnostic_delta {
        let added = unique_primary_diagnostics(&delta.added);
        let resolved = unique_primary_diagnostics(&delta.resolved);
        let persisting = unique_primary_diagnostics(&delta.persisting);
        writeln!(
            output,
            "Diagnostic delta: {}; {} new, {} resolved, {} persisting (unique errors/warnings)",
            comparison_status_name(delta.status),
            added.len(),
            resolved.len(),
            persisting.len()
        )
        .expect("write to String");
        if !delta.reasons.is_empty() {
            writeln!(output, "Diagnostic comparison reasons:").expect("write to String");
            for reason in &delta.reasons {
                writeln!(
                    output,
                    "- {} ({})",
                    comparison_reason_code_name(reason.code),
                    single_line(&reason.dimension)
                )
                .expect("write to String");
            }
        }
        format_diagnostic_items(output, "NEW", &added);
        format_diagnostic_items(output, "RESOLVED", &resolved);
        let raw_delta_count = delta.added.len() + delta.resolved.len() + delta.persisting.len();
        let unique_delta_count = added.len() + resolved.len() + persisting.len();
        if raw_delta_count != unique_delta_count {
            writeln!(
                output,
                "Diagnostic detail: {raw_delta_count} raw error/warning observations condensed to {unique_delta_count} unique diagnostics"
            )
            .expect("write to String");
        }
        if delta.status != ComparisonStatus::Comparable {
            if let Some(run) = &pack.diagnostics {
                let current = unique_primary_diagnostics(&run.messages);
                format_diagnostic_items(output, "CURRENT", &current);
            }
        }
    } else if let Some(run) = &pack.diagnostics {
        writeln!(
            output,
            "Diagnostic delta: {}",
            comparison_status_name(pack.comparison_status)
        )
        .expect("write to String");
        let current = unique_primary_diagnostics(&run.messages);
        format_diagnostic_items(output, "CURRENT", &current);
    } else {
        writeln!(output, "Diagnostic delta: not requested").expect("write to String");
    }
}

pub(crate) fn unique_primary_diagnostics(diagnostics: &[RustDiagnostic]) -> Vec<&RustDiagnostic> {
    let mut unique = BTreeMap::<String, &RustDiagnostic>::new();
    for diagnostic in diagnostics
        .iter()
        .filter(|diagnostic| is_delta_diagnostic(diagnostic))
    {
        let fingerprint = if diagnostic.fingerprint.is_empty() {
            diagnostic_fingerprint(diagnostic)
        } else {
            diagnostic.fingerprint.clone()
        };
        unique.entry(fingerprint).or_insert(diagnostic);
    }
    unique.into_values().collect()
}

pub(crate) fn format_diagnostic_items(
    output: &mut String,
    label: &str,
    diagnostics: &[&RustDiagnostic],
) {
    for diagnostic in diagnostics.iter().take(SUMMARY_DIAGNOSTIC_LIMIT) {
        let code = diagnostic
            .code
            .as_ref()
            .map(|code| format!(" [{code}]"))
            .unwrap_or_default();
        let location = diagnostic_location(diagnostic);
        writeln!(
            output,
            "- {label}{code}{location}: {}",
            truncate_chars(&single_line(&diagnostic.message), 160)
        )
        .expect("write to String");
    }
    if diagnostics.len() > SUMMARY_DIAGNOSTIC_LIMIT {
        writeln!(
            output,
            "- ... {} more {label} diagnostics not displayed",
            diagnostics.len() - SUMMARY_DIAGNOSTIC_LIMIT
        )
        .expect("write to String");
    }
}

pub(crate) fn diagnostic_location(diagnostic: &RustDiagnostic) -> String {
    let Some(path) = diagnostic.file.as_ref() else {
        return String::new();
    };
    let mut location = format!(" {}", display_path(path));
    if let Some(line) = diagnostic.line {
        write!(location, ":{line}").expect("write to String");
        if let Some(column) = diagnostic.column {
            write!(location, ":{column}").expect("write to String");
        }
    }
    location
}

pub(crate) fn count_patch_lines(patch: &str) -> (usize, usize) {
    let mut additions = 0;
    let mut deletions = 0;
    for line in patch.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }
    (additions, deletions)
}

pub(crate) fn single_line(value: &str) -> String {
    escape_terminal_controls(&value.split_whitespace().collect::<Vec<_>>().join(" "))
}

pub(crate) fn display_path(path: &Path) -> String {
    escape_terminal_controls(&path.to_string_lossy())
}

pub(crate) fn escape_terminal_controls(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                write!(escaped, "\\u{{{:x}}}", character as u32).expect("write to String");
            }
            character => escaped.push(character),
        }
    }
    escaped
}

pub(crate) fn truncate_chars(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let prefix: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

pub(crate) fn item_count(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

pub(crate) fn artifact_status_name(status: ArtifactStatus) -> &'static str {
    match status {
        ArtifactStatus::NotRun => "not_run",
        ArtifactStatus::CompletedClean => "completed_clean",
        ArtifactStatus::CompletedWithDiagnostics => "completed_with_diagnostics",
        ArtifactStatus::ToolFailed => "tool_failed",
        ArtifactStatus::TimedOut => "timed_out",
        ArtifactStatus::OutputLimited => "output_limited",
        ArtifactStatus::Partial => "partial",
    }
}

pub(crate) fn comparison_status_name(status: ComparisonStatus) -> &'static str {
    match status {
        ComparisonStatus::Comparable => "comparable",
        ComparisonStatus::BaselineMissing => "baseline_missing",
        ComparisonStatus::NotComparable => "not_comparable",
        ComparisonStatus::Partial => "partial",
    }
}

pub(crate) fn comparison_reason_code_name(code: ComparisonReasonCode) -> &'static str {
    match code {
        ComparisonReasonCode::BaselineMissing => "baseline_missing",
        ComparisonReasonCode::BaselineIntegrityUnavailable => "baseline_integrity_unavailable",
        ComparisonReasonCode::BaselineIntegrityMismatch => "baseline_integrity_mismatch",
        ComparisonReasonCode::ComparisonBasisUnavailable => "comparison_basis_unavailable",
        ComparisonReasonCode::ToolchainMismatch => "toolchain_mismatch",
        ComparisonReasonCode::ProducerMismatch => "producer_mismatch",
        ComparisonReasonCode::ProducerVersionUnknown => "producer_version_unknown",
        ComparisonReasonCode::ProducerVersionMismatch => "producer_version_mismatch",
        ComparisonReasonCode::AnalysisProfileMismatch => "analysis_profile_mismatch",
        ComparisonReasonCode::PackageSetMismatch => "package_set_mismatch",
        ComparisonReasonCode::TargetCoverageMismatch => "target_coverage_mismatch",
        ComparisonReasonCode::FeatureCoverageMismatch => "feature_coverage_mismatch",
        ComparisonReasonCode::FeatureDefinitionMismatch => "feature_definition_mismatch",
        ComparisonReasonCode::WorkspaceInputsMismatch => "workspace_inputs_mismatch",
        ComparisonReasonCode::CompilerConfigMismatch => "compiler_config_mismatch",
        ComparisonReasonCode::CompilerConfigUnknown => "compiler_config_unknown",
        ComparisonReasonCode::ToolProvenanceMismatch => "tool_provenance_mismatch",
        ComparisonReasonCode::BaselineObservationIncomplete => "baseline_observation_incomplete",
        ComparisonReasonCode::CurrentObservationIncomplete => "current_observation_incomplete",
    }
}

pub(crate) fn evidence_level_name(level: EvidenceLevel) -> &'static str {
    match level {
        EvidenceLevel::ToolConfirmed => "tool-confirmed",
        EvidenceLevel::SemanticResolved => "semantic-resolved",
        EvidenceLevel::SyntaxOnly => "syntax-only",
        EvidenceLevel::Incomplete => "incomplete",
    }
}

pub(crate) fn analysis_mode_name(mode: AnalysisMode) -> &'static str {
    match mode {
        AnalysisMode::MetadataOnly => "metadata_only",
        AnalysisMode::CargoCheck => "cargo_check",
        AnalysisMode::Clippy => "clippy",
    }
}

pub(crate) fn diagnostic_profile_name(profile: DiagnosticProfile) -> &'static str {
    match profile {
        DiagnosticProfile::CargoCheckV1 => "cargo_check_v1",
        DiagnosticProfile::ClippyDefaultV1 => "clippy_default_v1",
    }
}

pub(crate) fn git_change_scope_name(scope: GitChangeScope) -> &'static str {
    match scope {
        GitChangeScope::Revision => "revision",
        GitChangeScope::Staged => "staged",
        GitChangeScope::Unstaged => "unstaged",
        GitChangeScope::Untracked => "untracked",
    }
}
