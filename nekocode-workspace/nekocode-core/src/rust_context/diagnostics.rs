//! Cargo/Clippy diagnostic parsing, status, and exact multiset deltas.

use super::snapshot;
use super::summary;
use super::*;
use snapshot::*;
use summary::*;

pub(crate) fn diagnostics_status(diagnostics: Option<&RustDiagnosticRun>) -> ArtifactStatus {
    diagnostics.map_or(ArtifactStatus::CompletedClean, diagnostic_status)
}

pub(crate) fn diagnostic_status(run: &RustDiagnosticRun) -> ArtifactStatus {
    match run.status.as_str() {
        "timed_out" => ArtifactStatus::TimedOut,
        "output_limited" => ArtifactStatus::OutputLimited,
        "failed" if run.messages.is_empty() => ArtifactStatus::ToolFailed,
        "success" if run.messages.is_empty() => ArtifactStatus::CompletedClean,
        _ => ArtifactStatus::CompletedWithDiagnostics,
    }
}

pub(crate) fn diagnostic_run_is_complete(run: &RustDiagnosticRun) -> bool {
    artifact_status_is_complete(diagnostic_status(run))
}

/// A non-zero compiler exit can still produce useful diagnostics, so the
/// observation remains evidence. It is not, however, a complete workspace
/// observation for baseline comparison: Cargo may stop before checking every
/// package or target. Only a successful producer run is safe for exact delta
/// conclusions.
pub(crate) fn diagnostic_comparison_is_complete(run: &RustDiagnosticRun) -> bool {
    run.status == "success"
}

pub(crate) fn artifact_status_is_complete(status: ArtifactStatus) -> bool {
    matches!(
        status,
        ArtifactStatus::CompletedClean | ArtifactStatus::CompletedWithDiagnostics
    )
}

pub(crate) fn evidence_for_artifact_status(status: ArtifactStatus) -> EvidenceLevel {
    if artifact_status_is_complete(status) {
        EvidenceLevel::ToolConfirmed
    } else {
        EvidenceLevel::Incomplete
    }
}

/// Derive the machine-readable comparison basis from one workspace
/// observation and its diagnostic invocation markers.
pub(crate) fn comparison_basis_for_run(
    workspace: &RustWorkspaceSnapshot,
    run: &RustDiagnosticRun,
) -> RustDiagnosticComparisonBasis {
    let root = workspace.workspace_root.clone();
    let mut members = workspace.workspace_members.clone();
    members.sort();
    let normalized_members = normalized_json(&members, &root);
    let mut normalized_packages = workspace
        .packages
        .iter()
        .map(|package| {
            serde_json::json!({
                "id": package.id,
                "name": package.name,
                "version": package.version,
                "manifest_path": package.manifest_path,
                "edition": package.edition,
            })
        })
        .collect::<Vec<_>>();
    let mut normalized_package_targets = workspace
        .packages
        .iter()
        .map(|package| {
            serde_json::json!({
                "id": package.id,
                "targets": package.targets,
                "target_details": package.target_details,
            })
        })
        .collect::<Vec<_>>();
    let mut normalized_features = workspace
        .packages
        .iter()
        .map(|package| {
            serde_json::json!({
                "id": package.id,
                "feature_definitions": package.feature_definitions,
            })
        })
        .collect::<Vec<_>>();
    sort_json_values(&mut normalized_packages);
    sort_json_values(&mut normalized_package_targets);
    sort_json_values(&mut normalized_features);

    RustDiagnosticComparisonBasis {
        workspace_members_sha256: digest_json(&normalized_members),
        package_set_sha256: digest_json(&normalized_json(&normalized_packages, &root)),
        package_targets_sha256: digest_json(&normalized_json(&normalized_package_targets, &root)),
        feature_definitions_sha256: digest_json(&normalized_json(&normalized_features, &root)),
        workspace_inputs_sha256: digest_json(&normalized_json(&workspace.inputs, &root)),
        compiler_config: compiler_config_observation(workspace, &root),
        target_coverage: if run.all_targets {
            "workspace_all_targets".to_string()
        } else {
            "workspace_default_targets".to_string()
        },
        feature_coverage: if run.all_features {
            "all_features".to_string()
        } else {
            "default_features".to_string()
        },
        toolchain: workspace.toolchain.clone(),
        producer: run.producer,
        profile: run.profile,
        producer_version: run.producer_version.clone(),
        complete: diagnostic_comparison_is_complete(run),
    }
}

fn compiler_config_observation(
    workspace: &RustWorkspaceSnapshot,
    root: &Path,
) -> RustCompilerConfigObservation {
    let cargo_home = std::env::var_os("CARGO_HOME").map(PathBuf::from);
    let configs = workspace
        .inputs
        .iter()
        .filter(|input| is_cargo_config_path(&input.path, cargo_home.as_deref()))
        .collect::<Vec<_>>();
    if !configs.is_empty() {
        return RustCompilerConfigObservation {
            status: ComparisonObservationStatus::Observed,
            sha256: Some(digest_json(&normalized_json(&configs, root))),
        };
    }

    let global_config_location_is_known = std::env::var_os("CARGO_HOME").is_some()
        || std::env::var_os("HOME").is_some()
        || std::env::var_os("USERPROFILE").is_some();
    RustCompilerConfigObservation {
        status: if global_config_location_is_known {
            ComparisonObservationStatus::Absent
        } else {
            ComparisonObservationStatus::Unknown
        },
        sha256: None,
    }
}

fn is_cargo_config_path(path: &Path, cargo_home: Option<&Path>) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.ends_with("/.cargo/config")
        || normalized.ends_with("/.cargo/config.toml")
        || cargo_home
            .is_some_and(|home| path == home.join("config") || path == home.join("config.toml"))
}

fn normalized_json<T: serde::Serialize>(value: &T, root: &Path) -> serde_json::Value {
    let mut value = serde_json::to_value(value).unwrap_or_default();
    let root = root.to_string_lossy().replace('\\', "/");
    sanitize_public_json(&mut value, None, &root);
    value
}

fn digest_json(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn sort_json_values(values: &mut [serde_json::Value]) {
    values.sort_by_key(|value| serde_json::to_string(value).unwrap_or_default());
}

pub(crate) fn diagnostic_fingerprint(diagnostic: &RustDiagnostic) -> String {
    diagnostic_fingerprint_with_context(
        diagnostic,
        DiagnosticProducer::CargoCheck,
        DiagnosticProfile::CargoCheckV1,
    )
}

pub(crate) fn diagnostic_fingerprint_with_context(
    diagnostic: &RustDiagnostic,
    producer: DiagnosticProducer,
    profile: DiagnosticProfile,
) -> String {
    let primary_span = diagnostic
        .spans
        .iter()
        .find(|span| span.is_primary)
        .or_else(|| diagnostic.spans.first());
    let file = primary_span
        .and_then(|span| span.file.as_ref())
        .or(diagnostic.file.as_ref())
        .map(|path| path.to_string_lossy().replace('\\', "/"));
    let message = diagnostic
        .message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let identity = serde_json::json!({
        "producer": producer.display_name(),
        "profile": diagnostic_profile_name(profile),
        "package_id": diagnostic.package_id.as_deref(),
        "target": diagnostic.target.as_deref(),
        "code": diagnostic.code.as_deref().unwrap_or(&diagnostic.level),
        "message": message,
        "primary_file": file,
        "primary_line_start": primary_span.and_then(|span| span.line_start).or(diagnostic.line),
        "primary_column_start": primary_span.and_then(|span| span.column_start).or(diagnostic.column),
        "primary_line_end": primary_span.and_then(|span| span.line_end),
        "primary_column_end": primary_span.and_then(|span| span.column_end),
        "primary_label": primary_span.and_then(|span| span.label.as_deref()),
    });
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&identity).unwrap_or_default());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn diagnostic_with_fingerprint(mut diagnostic: RustDiagnostic) -> RustDiagnostic {
    if diagnostic.fingerprint.is_empty() {
        diagnostic.fingerprint = diagnostic_fingerprint(&diagnostic);
    }
    diagnostic
}

pub(crate) fn is_delta_diagnostic(diagnostic: &RustDiagnostic) -> bool {
    matches!(diagnostic.level.as_str(), "error" | "warning")
}

#[cfg(test)]
pub(crate) fn diagnostic_multimap(
    diagnostics: &[RustDiagnostic],
) -> BTreeMap<String, Vec<RustDiagnostic>> {
    diagnostic_multimap_with_context(
        diagnostics,
        DiagnosticProducer::CargoCheck,
        DiagnosticProfile::CargoCheckV1,
    )
}

pub(crate) fn diagnostic_multimap_with_context(
    diagnostics: &[RustDiagnostic],
    producer: DiagnosticProducer,
    profile: DiagnosticProfile,
) -> BTreeMap<String, Vec<RustDiagnostic>> {
    let mut grouped = BTreeMap::<String, Vec<RustDiagnostic>>::new();
    for diagnostic in diagnostics
        .iter()
        .filter(|diagnostic| is_delta_diagnostic(diagnostic))
        .cloned()
        .map(|mut diagnostic| {
            diagnostic.fingerprint =
                diagnostic_fingerprint_with_context(&diagnostic, producer, profile);
            diagnostic
        })
    {
        grouped
            .entry(diagnostic.fingerprint.clone())
            .or_default()
            .push(diagnostic);
    }
    grouped
}

#[derive(Debug, Default)]
pub(crate) struct DiagnosticChanges {
    pub(crate) added: Vec<RustDiagnostic>,
    pub(crate) resolved: Vec<RustDiagnostic>,
    pub(crate) persisting: Vec<RustDiagnostic>,
}

#[cfg(test)]
pub(crate) fn compare_diagnostic_multisets(
    baseline: &[RustDiagnostic],
    current: &[RustDiagnostic],
) -> DiagnosticChanges {
    let mut changes = DiagnosticChanges::default();
    let mut baseline = diagnostic_multimap(baseline);
    let current = diagnostic_multimap(current);

    compare_diagnostic_groups(&mut changes, &mut baseline, current);
    changes
}

pub(crate) fn compare_diagnostic_multisets_for_runs(
    baseline: &RustDiagnosticRun,
    current: &RustDiagnosticRun,
) -> DiagnosticChanges {
    let mut changes = DiagnosticChanges::default();
    let mut baseline =
        diagnostic_multimap_with_context(&baseline.messages, baseline.producer, baseline.profile);
    let current =
        diagnostic_multimap_with_context(&current.messages, current.producer, current.profile);

    compare_diagnostic_groups(&mut changes, &mut baseline, current);
    changes
}

pub(crate) fn compare_diagnostic_groups(
    changes: &mut DiagnosticChanges,
    baseline: &mut BTreeMap<String, Vec<RustDiagnostic>>,
    current: BTreeMap<String, Vec<RustDiagnostic>>,
) {
    for (fingerprint, current_group) in current {
        let baseline_group = baseline.remove(&fingerprint).unwrap_or_default();
        let persisting_count = baseline_group.len().min(current_group.len());
        changes
            .persisting
            .extend(current_group.iter().take(persisting_count).cloned());
        changes
            .added
            .extend(current_group.into_iter().skip(persisting_count));
        changes
            .resolved
            .extend(baseline_group.into_iter().skip(persisting_count));
    }
    for baseline_group in std::mem::take(baseline).into_values() {
        changes.resolved.extend(baseline_group);
    }
}

pub(crate) fn build_diagnostic_delta(
    baseline_path: &Path,
    baseline_run: Option<&RustDiagnosticRun>,
    current_run: &RustDiagnosticRun,
    baseline_integrity_verified: bool,
) -> RustDiagnosticDelta {
    let mut delta = RustDiagnosticDelta {
        baseline_path: baseline_path.to_path_buf(),
        status: ComparisonStatus::Comparable,
        compatible: true,
        added: Vec::new(),
        resolved: Vec::new(),
        persisting: Vec::new(),
        reasons: Vec::new(),
        limitations: Vec::new(),
    };

    let Some(baseline_run) = baseline_run else {
        delta.status = ComparisonStatus::BaselineMissing;
        delta.compatible = false;
        push_reason(
            &mut delta,
            ComparisonReasonCode::BaselineMissing,
            "baseline_diagnostic_observation",
        );
        delta
            .limitations
            .push("baseline snapshot has no cargo check diagnostics".to_string());
        return delta;
    };

    if !baseline_integrity_verified {
        delta.compatible = false;
        push_reason(
            &mut delta,
            ComparisonReasonCode::BaselineIntegrityUnavailable,
            "baseline_canonical_hash",
        );
        delta.limitations.push(
            "baseline snapshot has no verified canonical hash; diagnostic comparison is unavailable"
                .to_string(),
        );
    }

    let baseline_basis = baseline_run.comparison_basis.as_ref();
    if baseline_basis.is_none() {
        delta.compatible = false;
        push_reason(
            &mut delta,
            ComparisonReasonCode::ComparisonBasisUnavailable,
            "baseline_comparison_basis",
        );
        delta
            .limitations
            .push("baseline diagnostic observation has no comparison basis".to_string());
    }
    let current_basis = current_run.comparison_basis.as_ref();
    if current_basis.is_none() {
        delta.compatible = false;
        push_reason(
            &mut delta,
            ComparisonReasonCode::ComparisonBasisUnavailable,
            "current_comparison_basis",
        );
        delta
            .limitations
            .push("current diagnostic observation has no comparison basis".to_string());
    }

    if let (Some(baseline_basis), Some(current_basis)) = (baseline_basis, current_basis) {
        compare_basis(&mut delta, baseline_basis, current_basis);
    }
    if baseline_run.provenance.tool != current_run.provenance.tool
        || baseline_run.provenance.version != current_run.provenance.version
    {
        delta.compatible = false;
        push_reason(
            &mut delta,
            ComparisonReasonCode::ToolProvenanceMismatch,
            "tool_provenance",
        );
        delta
            .limitations
            .push("baseline and current diagnostic tool provenance differs".to_string());
    }
    let baseline_complete = diagnostic_comparison_is_complete(baseline_run);
    let current_complete = diagnostic_comparison_is_complete(current_run);
    if !baseline_complete {
        delta.compatible = false;
        push_reason(
            &mut delta,
            ComparisonReasonCode::BaselineObservationIncomplete,
            "baseline_observation",
        );
    }
    if !current_complete {
        delta.compatible = false;
        push_reason(
            &mut delta,
            ComparisonReasonCode::CurrentObservationIncomplete,
            "current_observation",
        );
    }
    if !baseline_complete || !current_complete {
        delta.limitations.push(
            "diagnostic delta requires complete baseline and current observations".to_string(),
        );
    }

    if !delta.compatible {
        return finish_incompatible_delta(delta, baseline_run, current_run);
    }

    let changes = compare_diagnostic_multisets_for_runs(baseline_run, current_run);
    delta.added = changes.added;
    delta.resolved = changes.resolved;
    delta.persisting = changes.persisting;
    delta.status = ComparisonStatus::Comparable;
    delta
}

pub(crate) fn invalid_baseline_delta(
    baseline_path: &Path,
    limitation: String,
) -> RustDiagnosticDelta {
    RustDiagnosticDelta {
        baseline_path: baseline_path.to_path_buf(),
        status: ComparisonStatus::NotComparable,
        compatible: false,
        added: Vec::new(),
        resolved: Vec::new(),
        persisting: Vec::new(),
        reasons: vec![ComparisonReason {
            code: ComparisonReasonCode::BaselineIntegrityMismatch,
            dimension: "baseline_artifact".to_string(),
        }],
        limitations: vec![limitation],
    }
}

fn compare_basis(
    delta: &mut RustDiagnosticDelta,
    baseline: &RustDiagnosticComparisonBasis,
    current: &RustDiagnosticComparisonBasis,
) {
    if baseline.toolchain != current.toolchain {
        mismatch(
            delta,
            ComparisonReasonCode::ToolchainMismatch,
            "toolchain",
            "baseline and current Rust toolchains differ",
        );
    }
    if baseline.workspace_members_sha256 != current.workspace_members_sha256
        || baseline.package_set_sha256 != current.package_set_sha256
    {
        mismatch(
            delta,
            ComparisonReasonCode::PackageSetMismatch,
            "package_set",
            "baseline and current workspace packages differ",
        );
    }
    if baseline.package_targets_sha256 != current.package_targets_sha256 {
        mismatch(
            delta,
            ComparisonReasonCode::TargetCoverageMismatch,
            "package_targets",
            "baseline and current package targets differ",
        );
    }
    if baseline.target_coverage != current.target_coverage {
        mismatch(
            delta,
            ComparisonReasonCode::TargetCoverageMismatch,
            "target_coverage",
            "baseline and current target coverage differ",
        );
    }
    if baseline.feature_definitions_sha256 != current.feature_definitions_sha256 {
        mismatch(
            delta,
            ComparisonReasonCode::FeatureDefinitionMismatch,
            "feature_definitions",
            "baseline and current feature definitions differ",
        );
    }
    if baseline.feature_coverage != current.feature_coverage {
        mismatch(
            delta,
            ComparisonReasonCode::FeatureCoverageMismatch,
            "feature_coverage",
            "baseline and current feature coverage differ",
        );
    }
    if baseline.workspace_inputs_sha256 != current.workspace_inputs_sha256 {
        mismatch(
            delta,
            ComparisonReasonCode::WorkspaceInputsMismatch,
            "workspace_inputs",
            "baseline and current workspace inputs differ",
        );
    }
    match (
        &baseline.compiler_config.status,
        &current.compiler_config.status,
        &baseline.compiler_config.sha256,
        &current.compiler_config.sha256,
    ) {
        (ComparisonObservationStatus::Unknown, _, _, _)
        | (_, ComparisonObservationStatus::Unknown, _, _) => mismatch(
            delta,
            ComparisonReasonCode::CompilerConfigUnknown,
            "compiler_config",
            "compiler-affecting Cargo configuration could not be fully observed",
        ),
        (left_status, right_status, left_digest, right_digest)
            if left_status != right_status || left_digest != right_digest =>
        {
            mismatch(
                delta,
                ComparisonReasonCode::CompilerConfigMismatch,
                "compiler_config",
                "baseline and current compiler-affecting Cargo configuration differs",
            )
        }
        _ => {}
    }
    if baseline.producer != current.producer {
        mismatch(
            delta,
            ComparisonReasonCode::ProducerMismatch,
            "producer",
            "baseline and current diagnostic producers differ",
        );
    }
    if baseline.profile != current.profile {
        mismatch(
            delta,
            ComparisonReasonCode::AnalysisProfileMismatch,
            "analysis_profile",
            "baseline and current diagnostic profiles differ",
        );
    }
    match (&baseline.producer_version, &current.producer_version) {
        (None, _) | (_, None) => mismatch(
            delta,
            ComparisonReasonCode::ProducerVersionUnknown,
            "producer_version",
            "diagnostic producer version is unavailable",
        ),
        (Some(left), Some(right)) if left != right => mismatch(
            delta,
            ComparisonReasonCode::ProducerVersionMismatch,
            "producer_version",
            "baseline and current diagnostic producer versions differ",
        ),
        _ => {}
    }
}

fn mismatch(
    delta: &mut RustDiagnosticDelta,
    code: ComparisonReasonCode,
    dimension: &str,
    limitation: &str,
) {
    delta.compatible = false;
    push_reason(delta, code, dimension);
    delta.limitations.push(limitation.to_string());
}

fn push_reason(delta: &mut RustDiagnosticDelta, code: ComparisonReasonCode, dimension: &str) {
    let reason = ComparisonReason {
        code,
        dimension: dimension.to_string(),
    };
    if !delta.reasons.contains(&reason) {
        delta.reasons.push(reason);
    }
}

fn finish_incompatible_delta(
    mut delta: RustDiagnosticDelta,
    baseline: &RustDiagnosticRun,
    current: &RustDiagnosticRun,
) -> RustDiagnosticDelta {
    delta.status = if !diagnostic_comparison_is_complete(baseline)
        || !diagnostic_comparison_is_complete(current)
    {
        ComparisonStatus::Partial
    } else {
        ComparisonStatus::NotComparable
    };
    delta
}

pub(crate) fn pop_diagnostic_delta_item(delta: &mut RustDiagnosticDelta) -> bool {
    delta
        .added
        .pop()
        .or_else(|| delta.resolved.pop())
        .or_else(|| delta.persisting.pop())
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace() -> RustWorkspaceSnapshot {
        RustWorkspaceSnapshot {
            schema_version: SCHEMA_VERSION,
            evidence: EvidenceLevel::ToolConfirmed,
            root: PathBuf::from("/tmp/comparability-matrix"),
            workspace_root: PathBuf::from("/tmp/comparability-matrix"),
            toolchain: RustToolchainInfo {
                rustc_version: Some("rustc 1.85.0".to_string()),
                cargo_version: Some("cargo 1.85.0".to_string()),
                host: Some("x86_64-unknown-linux-gnu".to_string()),
            },
            packages: vec![RustPackage {
                id: "matrix-package 0.1.0 (path+file:///tmp/comparability-matrix)".to_string(),
                name: "matrix-package".to_string(),
                version: "0.1.0".to_string(),
                manifest_path: PathBuf::from("/tmp/comparability-matrix/Cargo.toml"),
                targets: vec!["matrix-package".to_string()],
                target_details: vec![RustTarget {
                    name: "matrix-package".to_string(),
                    kind: vec!["lib".to_string()],
                    src_path: Some(PathBuf::from("/tmp/comparability-matrix/src/lib.rs")),
                    edition: Some("2021".to_string()),
                    required_features: Vec::new(),
                }],
                features: vec!["default".to_string()],
                feature_definitions: vec![RustFeatureDefinition {
                    name: "default".to_string(),
                    enables: Vec::new(),
                }],
                dependencies: Vec::new(),
                edition: Some("2021".to_string()),
            }],
            workspace_members: vec![
                "matrix-package 0.1.0 (path+file:///tmp/comparability-matrix)".to_string(),
            ],
            inputs: vec![RustInputDigest {
                path: PathBuf::from("/tmp/comparability-matrix/Cargo.toml"),
                sha256: "sha256:manifest".to_string(),
            }],
            provenance: ToolProvenance {
                tool: "cargo metadata".to_string(),
                command: "cargo metadata".to_string(),
                cwd: PathBuf::from("/tmp/comparability-matrix"),
                version: Some("cargo 1.85.0".to_string()),
                exit_code: Some(0),
            },
        }
    }

    fn diagnostic(target: &str, line: u32, label: &str) -> RustDiagnostic {
        RustDiagnostic {
            level: "error".to_string(),
            message: "mismatched types".to_string(),
            code: Some("E0308".to_string()),
            file: Some(PathBuf::from("src/lib.rs")),
            line: Some(line),
            column: Some(5),
            rendered: None,
            package_id: Some(
                "matrix-package 0.1.0 (path+file:///tmp/comparability-matrix)".to_string(),
            ),
            target: Some(target.to_string()),
            spans: vec![RustDiagnosticSpan {
                file: Some(PathBuf::from("src/lib.rs")),
                line_start: Some(line),
                column_start: Some(5),
                line_end: Some(line),
                column_end: Some(10),
                is_primary: true,
                label: Some(label.to_string()),
            }],
            fingerprint: String::new(),
        }
    }

    fn run(
        workspace: &RustWorkspaceSnapshot,
        messages: Vec<RustDiagnostic>,
        producer: DiagnosticProducer,
        status: &str,
        all_targets: bool,
        all_features: bool,
    ) -> RustDiagnosticRun {
        let mut run = RustDiagnosticRun {
            producer,
            profile: producer.profile(),
            producer_version: Some("producer 1.0".to_string()),
            command: producer.command_name().to_string(),
            status: status.to_string(),
            messages,
            stderr: None,
            all_targets,
            all_features,
            provenance: ToolProvenance {
                tool: producer.command_name().to_string(),
                command: producer.command_name().to_string(),
                cwd: workspace.workspace_root.clone(),
                version: Some("cargo 1.85.0".to_string()),
                exit_code: Some(0),
            },
            comparison_basis: None,
        };
        run.comparison_basis = Some(comparison_basis_for_run(workspace, &run));
        run
    }

    fn reason(delta: &RustDiagnosticDelta, code: ComparisonReasonCode) -> bool {
        delta.reasons.iter().any(|reason| reason.code == code)
    }

    #[test]
    fn recognizes_config_files_under_a_custom_cargo_home() {
        let cargo_home = Path::new("/opt/nekocode-cargo");
        assert!(is_cargo_config_path(
            &cargo_home.join("config.toml"),
            Some(cargo_home)
        ));
        assert!(is_cargo_config_path(
            &cargo_home.join("config"),
            Some(cargo_home)
        ));
    }

    #[test]
    fn comparability_matrix_covers_one_axis_decisions() {
        let baseline_workspace = workspace();
        let baseline_run = run(
            &baseline_workspace,
            vec![diagnostic("matrix-package", 3, "expected u32")],
            DiagnosticProducer::CargoCheck,
            "success",
            true,
            false,
        );
        assert!(baseline_run.comparison_basis.is_some());

        let same = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &baseline_run,
            true,
        );
        assert_eq!(same.status, ComparisonStatus::Comparable);
        assert!(same.reasons.is_empty());
        assert_eq!(same.persisting.len(), 1);

        let added_run = run(
            &baseline_workspace,
            vec![
                diagnostic("matrix-package", 3, "expected u32"),
                diagnostic("matrix-package", 8, "expected bool"),
            ],
            DiagnosticProducer::CargoCheck,
            "success",
            true,
            false,
        );
        let added = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &added_run,
            true,
        );
        assert_eq!(added.status, ComparisonStatus::Comparable);
        assert_eq!(added.added.len(), 1);

        let clean_run = run(
            &baseline_workspace,
            Vec::new(),
            DiagnosticProducer::CargoCheck,
            "success",
            true,
            false,
        );
        let resolved = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &clean_run,
            true,
        );
        assert_eq!(resolved.status, ComparisonStatus::Comparable);
        assert_eq!(resolved.resolved.len(), 1);

        let mut package_workspace = baseline_workspace.clone();
        package_workspace
            .workspace_members
            .push("new-member".to_string());
        let package_run = run(
            &package_workspace,
            baseline_run.messages.clone(),
            DiagnosticProducer::CargoCheck,
            "success",
            true,
            false,
        );
        let package_delta = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &package_run,
            true,
        );
        assert_eq!(package_delta.status, ComparisonStatus::NotComparable);
        assert!(reason(
            &package_delta,
            ComparisonReasonCode::PackageSetMismatch
        ));

        let target_run = run(
            &baseline_workspace,
            baseline_run.messages.clone(),
            DiagnosticProducer::CargoCheck,
            "success",
            false,
            false,
        );
        let target_delta = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &target_run,
            true,
        );
        assert!(reason(
            &target_delta,
            ComparisonReasonCode::TargetCoverageMismatch
        ));

        let feature_run = run(
            &baseline_workspace,
            baseline_run.messages.clone(),
            DiagnosticProducer::CargoCheck,
            "success",
            true,
            true,
        );
        let feature_delta = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &feature_run,
            true,
        );
        assert!(reason(
            &feature_delta,
            ComparisonReasonCode::FeatureCoverageMismatch
        ));

        let mut definition_workspace = baseline_workspace.clone();
        definition_workspace.packages[0].feature_definitions[0]
            .enables
            .push("dep:extra".to_string());
        let definition_run = run(
            &definition_workspace,
            baseline_run.messages.clone(),
            DiagnosticProducer::CargoCheck,
            "success",
            true,
            false,
        );
        let definition_delta = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &definition_run,
            true,
        );
        assert!(reason(
            &definition_delta,
            ComparisonReasonCode::FeatureDefinitionMismatch
        ));

        let mut config_workspace = baseline_workspace.clone();
        config_workspace.inputs.push(RustInputDigest {
            path: PathBuf::from("/tmp/comparability-matrix/.cargo/config.toml"),
            sha256: "sha256:config".to_string(),
        });
        let config_run = run(
            &config_workspace,
            baseline_run.messages.clone(),
            DiagnosticProducer::CargoCheck,
            "success",
            true,
            false,
        );
        let config_delta = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &config_run,
            true,
        );
        assert!(reason(
            &config_delta,
            ComparisonReasonCode::CompilerConfigMismatch
        ));

        let mut unknown_run = baseline_run.clone();
        unknown_run
            .comparison_basis
            .as_mut()
            .expect("basis")
            .compiler_config
            .status = ComparisonObservationStatus::Unknown;
        let unknown_delta = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &unknown_run,
            true,
        );
        assert!(reason(
            &unknown_delta,
            ComparisonReasonCode::CompilerConfigUnknown
        ));

        let clippy_run = run(
            &baseline_workspace,
            baseline_run.messages.clone(),
            DiagnosticProducer::Clippy,
            "success",
            true,
            false,
        );
        let producer_delta = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &clippy_run,
            true,
        );
        assert!(reason(
            &producer_delta,
            ComparisonReasonCode::ProducerMismatch
        ));

        let mut toolchain_workspace = baseline_workspace.clone();
        toolchain_workspace.toolchain.rustc_version = Some("rustc 1.86.0".to_string());
        let toolchain_run = run(
            &toolchain_workspace,
            baseline_run.messages.clone(),
            DiagnosticProducer::CargoCheck,
            "success",
            true,
            false,
        );
        let toolchain_delta = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &toolchain_run,
            true,
        );
        assert!(reason(
            &toolchain_delta,
            ComparisonReasonCode::ToolchainMismatch
        ));

        let partial_run = run(
            &baseline_workspace,
            Vec::new(),
            DiagnosticProducer::CargoCheck,
            "timed_out",
            true,
            false,
        );
        let partial_delta = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &partial_run,
            true,
        );
        assert_eq!(partial_delta.status, ComparisonStatus::Partial);
        assert!(reason(
            &partial_delta,
            ComparisonReasonCode::CurrentObservationIncomplete
        ));

        let failed_with_messages = run(
            &baseline_workspace,
            baseline_run.messages.clone(),
            DiagnosticProducer::CargoCheck,
            "failed",
            true,
            false,
        );
        assert!(
            !failed_with_messages
                .comparison_basis
                .as_ref()
                .expect("failed run basis")
                .complete
        );
        let failed_delta = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &failed_with_messages,
            true,
        );
        assert_eq!(failed_delta.status, ComparisonStatus::Partial);
        assert!(reason(
            &failed_delta,
            ComparisonReasonCode::CurrentObservationIncomplete
        ));

        let missing = build_diagnostic_delta(Path::new("baseline.json"), None, &baseline_run, true);
        assert_eq!(missing.status, ComparisonStatus::BaselineMissing);
        assert!(reason(&missing, ComparisonReasonCode::BaselineMissing));

        let unverified = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &baseline_run,
            false,
        );
        assert_eq!(unverified.status, ComparisonStatus::NotComparable);
        assert!(reason(
            &unverified,
            ComparisonReasonCode::BaselineIntegrityUnavailable
        ));

        let span_run = run(
            &baseline_workspace,
            vec![diagnostic("matrix-package", 4, "expected u32")],
            DiagnosticProducer::CargoCheck,
            "success",
            true,
            false,
        );
        let span_delta = build_diagnostic_delta(
            Path::new("baseline.json"),
            Some(&baseline_run),
            &span_run,
            true,
        );
        assert_eq!(span_delta.persisting.len(), 0);
        assert_eq!(span_delta.added.len(), 1);
        assert_eq!(span_delta.resolved.len(), 1);
    }
}
