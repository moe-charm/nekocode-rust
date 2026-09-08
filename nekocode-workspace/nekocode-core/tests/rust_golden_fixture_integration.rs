use nekocode_core::{
    build_rust_context_with_config, build_rust_context_with_options, build_rust_snapshot,
    format_context_summary, index_rust_workspace, sanitize_context_for_output, write_rust_snapshot,
    ArtifactStatus, ComparisonStatus, EvidenceLevel, RustContextOptions,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rust_golden_workspace")
}

fn copy_fixture(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("fixture destination");
    for entry in fs::read_dir(source).expect("fixture directory") {
        let entry = entry.expect("fixture entry");
        if matches!(entry.file_name().to_str(), Some("target" | ".git")) {
            continue;
        }
        let target = destination.join(entry.file_name());
        if entry.file_type().expect("fixture entry type").is_dir() {
            copy_fixture(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("fixture file copy");
        }
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("git must be available for the golden fixture");
    assert!(
        output.status.success(),
        "git {args:?} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn indexes_the_complete_rust_feature_fixture() {
    let snapshot = index_rust_workspace(fixture_root()).expect("fixture must be a Cargo workspace");

    assert_eq!(snapshot.evidence, EvidenceLevel::ToolConfirmed);
    assert_eq!(snapshot.packages.len(), 3);
    assert!(snapshot
        .packages
        .iter()
        .any(|package| package.name == "golden-model" && package.features == ["feature_probe"]));
    assert!(snapshot
        .packages
        .iter()
        .any(|package| package.name == "golden-consumer"));
    assert!(snapshot
        .packages
        .iter()
        .any(|package| package.name == "golden-compile-error"));
}

#[test]
fn includes_cargo_configuration_in_input_digests() {
    for file_name in ["config.toml", "config"] {
        let directory = tempdir().expect("temporary configuration workspace");
        let workspace = directory.path().join("workspace");
        copy_fixture(&fixture_root(), &workspace);

        let config_path = workspace.join(".cargo").join(file_name);
        fs::create_dir_all(config_path.parent().expect("Cargo config parent"))
            .expect("Cargo config directory");
        fs::write(&config_path, "[term]\nverbose = false\n").expect("initial Cargo configuration");

        let initial = index_rust_workspace(&workspace).expect("workspace with Cargo config");
        let canonical_config = config_path.canonicalize().expect("canonical Cargo config");
        let initial_digest = initial
            .inputs
            .iter()
            .find(|input| input.path == canonical_config)
            .map(|input| input.sha256.clone())
            .expect("Cargo configuration must be an input digest");

        fs::write(&config_path, "[term]\nverbose = true\n").expect("updated Cargo configuration");
        let updated = index_rust_workspace(&workspace).expect("updated Cargo config workspace");
        let updated_digest = updated
            .inputs
            .iter()
            .find(|input| input.path == canonical_config)
            .map(|input| input.sha256.as_str())
            .expect("updated Cargo configuration must remain an input digest");

        assert_ne!(
            initial_digest, updated_digest,
            "Cargo config changes must be observable"
        );
    }
}

#[test]
fn includes_parent_cargo_configuration_in_input_digests() {
    let directory = tempdir().expect("temporary parent configuration workspace");
    let workspace = directory.path().join("workspace");
    copy_fixture(&fixture_root(), &workspace);

    let config_path = directory.path().join(".cargo/config.toml");
    fs::create_dir_all(config_path.parent().expect("parent Cargo config directory"))
        .expect("parent Cargo config directory");
    fs::write(&config_path, "[term]\nverbose = false\n").expect("parent Cargo configuration");

    let snapshot = index_rust_workspace(&workspace).expect("workspace with parent Cargo config");
    let canonical_config = config_path.canonicalize().expect("canonical parent config");
    assert!(snapshot
        .inputs
        .iter()
        .any(|input| input.path == canonical_config));
}

#[test]
fn validates_the_trait_impl_macro_cfg_feature_consumer() {
    let output = Command::new("cargo")
        .current_dir(fixture_root())
        .args(["check", "--package", "golden-consumer", "--all-targets"])
        .output()
        .expect("cargo must be available for Rust fixture tests");

    assert!(
        output.status.success(),
        "valid fixture package failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn captures_the_deliberate_compile_error_as_evidence() {
    let pack = build_rust_context_with_options(fixture_root(), None, 20_000, true)
        .expect("failed cargo checks are evidence, not context construction failures");
    let diagnostics = pack.diagnostics.expect("diagnostics were requested");

    assert_eq!(diagnostics.status, "failed");
    assert!(diagnostics.messages.iter().any(|diagnostic| {
        diagnostic.code.as_deref() == Some("E0308")
            && diagnostic
                .file
                .as_ref()
                .is_some_and(|file| file.ends_with("compile_error/src/lib.rs"))
    }));
    assert!(diagnostics
        .messages
        .iter()
        .filter_map(|diagnostic| diagnostic.file.as_ref())
        .all(|file| !file.is_absolute()));
}

#[test]
fn explains_a_fixed_golden_error_without_leaking_the_external_baseline_path() {
    let directory = tempdir().expect("temporary golden workspace");
    let workspace = directory.path().join("workspace");
    copy_fixture(&fixture_root(), &workspace);

    git(&workspace, &["init", "-q"]);
    git(
        &workspace,
        &["config", "user.email", "golden@example.invalid"],
    );
    git(&workspace, &["config", "user.name", "Golden Fixture"]);
    git(&workspace, &["add", "."]);
    git(&workspace, &["commit", "-qm", "broken baseline"]);

    let baseline = build_rust_snapshot(&workspace, true, false)
        .expect("the compiler error should be captured as baseline evidence");
    assert_eq!(baseline.status, ArtifactStatus::CompletedWithDiagnostics);
    assert_eq!(baseline.evidence, EvidenceLevel::ToolConfirmed);
    let baseline_path = directory.path().join("baseline.json");
    write_rust_snapshot(&baseline_path, &baseline).expect("external baseline file");

    fs::write(
        workspace.join("compile_error/src/lib.rs"),
        "// Fixed E0308 fixture.\npub fn intentional_type_error() -> u8 {\n    8\n}\n",
    )
    .expect("fixed fixture source");

    let mut options = RustContextOptions::new(Some("HEAD".to_string()), 20_000);
    options.include_working_tree = true;
    options.include_untracked_content = true;
    options.include_diagnostics = true;
    options.baseline = Some(baseline_path);
    let context = build_rust_context_with_config(&workspace, options)
        .expect("fixed fixture context should build");
    assert_eq!(context.status, ArtifactStatus::CompletedClean);
    assert_eq!(context.comparison_status, ComparisonStatus::Partial);
    assert!(context.changed_files.iter().any(|file| {
        file.path == Path::new("compile_error/src/lib.rs") && file.hunks.len() == 1
    }));
    let delta = context
        .diagnostic_delta
        .as_ref()
        .expect("fixed error should produce a diagnostic delta");
    assert!(!delta.compatible);
    assert_eq!(delta.status, ComparisonStatus::Partial);
    assert!(delta.resolved.is_empty());
    assert!(delta.reasons.iter().any(|reason| {
        reason.code == nekocode_core::ComparisonReasonCode::BaselineObservationIncomplete
    }));

    let public = sanitize_context_for_output(&context).expect("public context should sanitize");
    assert_eq!(public.baseline.as_deref(), Some(Path::new("$EXTERNAL")));
    assert_eq!(
        public
            .diagnostic_delta
            .as_ref()
            .map(|delta| delta.baseline_path.as_path()),
        Some(Path::new("$EXTERNAL"))
    );
    assert!(!serde_json::to_string(&public)
        .expect("public context JSON")
        .contains(directory.path().to_string_lossy().as_ref()));

    let summary = format_context_summary(&public);
    assert!(summary.contains("Changes: 1 file (1 Rust), 1 hunk"));
    assert!(summary.contains(
        "Diagnostic delta: partial; 0 new, 0 resolved, 0 persisting (unique errors/warnings)"
    ));
    assert!(summary.contains("- baseline_observation_incomplete (baseline_observation)"));
    assert_eq!(summary.matches("- RESOLVED [E0308]").count(), 0);
    assert!(!summary.contains("For more information about this error"));
}
