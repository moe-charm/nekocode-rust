use nekocode_core::{
    build_rust_context_with_config, build_rust_snapshot, format_context_summary,
    sanitize_context_for_output, sanitize_snapshot_for_output, AnalysisMode, ComparisonStatus,
    RustContextOptions,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("git must be available for the diff fixture");
    assert!(
        output.status.success(),
        "git {:?} failed:\n{}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn context_contains_hunks_packages_and_working_tree_files() {
    let directory = tempdir().expect("temporary workspace");
    let root = directory.path();
    fs::create_dir(root.join("src")).expect("src directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"context-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), "pub fn before() -> u32 { 1 }\n").expect("source");

    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "context@example.invalid"]);
    git(root, &["config", "user.name", "Context Fixture"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "initial"]);

    fs::write(root.join("src/lib.rs"), "pub fn after() -> u32 { 2 }\n").expect("modified source");
    fs::write(root.join("src/untracked.rs"), "pub fn new_file() {}\n").expect("untracked source");

    let mut options = RustContextOptions::new(Some("HEAD".to_string()), 8_000);
    options.include_working_tree = true;
    let pack = build_rust_context_with_config(root, options).expect("context should build");

    assert_eq!(pack.contract_version, "context-v1");
    assert_eq!(pack.artifact_kind, "context");
    assert_eq!(pack.comparison_status, ComparisonStatus::Comparable);
    assert_eq!(pack.execution_policy.workspace_trust, "not_required");
    assert_eq!(
        pack.execution_policy.process_network_isolation,
        "not_applicable"
    );
    assert_eq!(pack.budget.requested_tokens, 8_000);
    assert!(pack.budget.max_bytes >= pack.serialized_bytes);
    assert!(pack
        .diff
        .as_ref()
        .is_some_and(|diff| diff.include_working_tree));
    assert!(pack
        .diff
        .as_ref()
        .is_some_and(|diff| diff.resolved_base.is_some() && diff.resolved_head.is_some()));
    assert!(pack
        .changed_files
        .iter()
        .any(|file| file.path == Path::new("src/lib.rs") && !file.hunks.is_empty()));
    assert!(pack.changed_files.iter().any(|file| {
        file.path == Path::new("src/untracked.rs") && file.status == "??" && file.is_rust
    }));
    assert!(pack
        .changed_files
        .iter()
        .find(|file| file.path == Path::new("src/lib.rs"))
        .and_then(|file| file.package.as_deref())
        .is_some_and(|package| package == "context-fixture"));
    assert!(pack
        .diff
        .as_ref()
        .is_some_and(|diff| diff.patch.contains("after")));
    assert!(pack
        .diff
        .as_ref()
        .is_some_and(|diff| !diff.patch.contains("new_file")));
    let excerpt = pack
        .source_excerpts
        .iter()
        .find(|excerpt| excerpt.path == Path::new("src/lib.rs"))
        .expect("modified Rust source should have an excerpt");
    assert_eq!(excerpt.source, "git-diff-hunk");
    assert!(excerpt.start_line <= 1);
    assert!(excerpt.end_line >= 1);
    assert!(excerpt.content.contains("after"));

    let summary = format_context_summary(&pack);
    assert!(summary.starts_with("NekoCode change summary\n"));
    assert!(summary.contains("Changes: 2 files (2 Rust), 1 hunk"));
    assert!(summary.contains("Visible patch: +1/-1 lines"));
    assert!(summary.contains("- M src/lib.rs [context-fixture] (1 hunk)"));
    assert!(summary.contains("- ?? src/untracked.rs"));
    assert!(summary.contains("Diagnostics: not run"));
    assert_eq!(summary, format_context_summary(&pack));

    let mut content_options = RustContextOptions::new(Some("HEAD".to_string()), 8_000);
    content_options.include_working_tree = true;
    content_options.include_untracked_content = true;
    let content_pack =
        build_rust_context_with_config(root, content_options).expect("untracked content works");
    assert!(content_pack
        .diff
        .as_ref()
        .is_some_and(|diff| diff.include_untracked_content && diff.patch.contains("new_file")));
}

#[test]
fn snapshot_round_trip_and_diagnostic_delta_are_explicit() {
    let directory = tempdir().expect("temporary workspace");
    let baseline_directory = tempdir().expect("external baseline directory");
    let root = directory.path();
    fs::create_dir(root.join("src")).expect("src directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"snapshot-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), "pub fn stable() -> u32 { 1 }\n").expect("source");

    let snapshot =
        nekocode_core::build_rust_snapshot(root, true, false).expect("snapshot should build");
    assert_eq!(snapshot.schema_version, 3);
    assert_eq!(snapshot.contract_version, "snapshot-v1");
    assert_eq!(snapshot.artifact_kind, "snapshot");
    assert_eq!(snapshot.analysis_mode, AnalysisMode::CargoCheck);
    assert_eq!(snapshot.execution_policy.workspace_trust, "required");
    assert_eq!(
        snapshot.execution_policy.process_network_isolation,
        "not_enforced"
    );
    assert!(snapshot.canonical_hash.is_some());
    assert!(!snapshot.generated_at.is_empty());
    assert!(snapshot.diagnostics.is_some());

    let snapshot_path = baseline_directory.path().join("baseline.json");
    nekocode_core::write_rust_snapshot(&snapshot_path, &snapshot)
        .expect("snapshot should be written");
    let loaded = nekocode_core::read_rust_snapshot(&snapshot_path).expect("snapshot should load");
    assert_eq!(loaded, snapshot);

    let mut options = RustContextOptions::new(None, 20_000);
    options.include_diagnostics = true;
    options.baseline = Some(snapshot_path.clone());
    let pack = build_rust_context_with_config(root, options).expect("context should build");

    let public = sanitize_context_for_output(&pack).expect("public context should sanitize");
    assert_eq!(
        public.baseline.as_deref(),
        Some(std::path::Path::new("$EXTERNAL"))
    );
    assert_eq!(
        public
            .diagnostic_delta
            .as_ref()
            .map(|delta| delta.baseline_path.as_path()),
        Some(std::path::Path::new("$EXTERNAL"))
    );
    assert!(!serde_json::to_string(&public)
        .expect("public context should serialize")
        .contains(baseline_directory.path().to_string_lossy().as_ref()));

    let delta = pack
        .diagnostic_delta
        .as_ref()
        .expect("matching diagnostic baseline should produce a delta");
    assert!(delta.compatible);
    assert_eq!(delta.status, ComparisonStatus::Comparable);
    assert!(delta.added.is_empty());
    assert!(delta.resolved.is_empty());
    assert!(delta.persisting.is_empty());

    fs::write(
        root.join("src/lib.rs"),
        "pub fn broken() { let _value: u32 = \"not a number\"; }\n",
    )
    .expect("invalid source");
    let mut changed_options = RustContextOptions::new(None, 20_000);
    changed_options.include_diagnostics = true;
    changed_options.baseline = Some(snapshot_path);
    let changed = build_rust_context_with_config(root, changed_options)
        .expect("compiler errors should remain evidence");
    let changed_delta = changed
        .diagnostic_delta
        .as_ref()
        .expect("compatible baseline should produce a changed delta");
    assert!(
        changed_delta
            .added
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("E0308")),
        "unexpected diagnostic delta: {changed_delta:#?}"
    );
    let changed_summary = format_context_summary(&changed);
    assert!(changed_summary
        .contains("Diagnostics: completed_with_diagnostics; producer_status=failed;"));
    assert!(changed_summary.contains(
        "Diagnostic delta: comparable; 1 new, 0 resolved, 0 persisting (unique errors/warnings)"
    ));
    assert!(changed_summary.contains("- NEW [E0308]"));
    assert_eq!(changed_summary.matches("- NEW [E0308]").count(), 1);

    let mut missing_options = RustContextOptions::new(None, 20_000);
    missing_options.include_diagnostics = true;
    let missing = build_rust_context_with_config(root, missing_options)
        .expect("context without baseline should still build");
    assert_eq!(missing.comparison_status, ComparisonStatus::BaselineMissing);
    assert!(missing
        .limitations
        .iter()
        .any(|item| item.contains("baseline_missing")));
    let missing_summary = format_context_summary(&missing);
    assert!(missing_summary.contains("Diagnostic delta: baseline_missing"));
    assert!(missing_summary.contains("- CURRENT [E0308]"));
}

#[test]
fn metadata_snapshot_has_stable_contract_and_safe_public_paths() {
    let directory = tempdir().expect("temporary workspace");
    let root = directory.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"metadata-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    fs::create_dir(root.join("src")).expect("src directory");
    fs::write(root.join("src/lib.rs"), "pub fn stable() {}\n").expect("source");

    let first = build_rust_snapshot(root, false, false).expect("snapshot should build");
    let second = build_rust_snapshot(root, false, false).expect("snapshot should build twice");
    assert_eq!(first.contract_version, "snapshot-v1");
    assert_eq!(first.analysis_mode, AnalysisMode::MetadataOnly);
    assert_eq!(first.canonical_hash, second.canonical_hash);

    let public = sanitize_snapshot_for_output(&first).expect("public view should serialize");
    assert_eq!(public.workspace.root, std::path::Path::new("$WORKSPACE"));
    assert!(!serde_json::to_string(&public)
        .expect("public snapshot should serialize")
        .contains(root.to_string_lossy().as_ref()));
}

#[test]
fn git_paths_are_relative_to_the_nested_workspace() {
    let directory = tempdir().expect("temporary repository");
    let repository = directory.path();
    let root = repository.join("workspace");
    fs::create_dir_all(root.join("src")).expect("src directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"nested-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), "pub fn before() -> u32 { 1 }\n").expect("source");

    git(repository, &["init", "-q"]);
    git(
        repository,
        &["config", "user.email", "context@example.invalid"],
    );
    git(repository, &["config", "user.name", "Context Fixture"]);
    git(repository, &["add", "."]);
    git(repository, &["commit", "-qm", "initial"]);

    fs::write(root.join("src/lib.rs"), "pub fn after() -> u32 { 2 }\n").expect("modified source");
    let mut options = RustContextOptions::new(Some("HEAD".to_string()), 8_000);
    options.include_working_tree = true;
    let pack = build_rust_context_with_config(&root, options).expect("context should build");

    assert!(pack
        .changed_files
        .iter()
        .any(|file| file.path == Path::new("src/lib.rs")));
    assert!(pack.source_excerpts.iter().any(
        |excerpt| excerpt.path == Path::new("src/lib.rs") && excerpt.content.contains("after")
    ));
}
