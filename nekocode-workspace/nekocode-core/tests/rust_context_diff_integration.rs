use nekocode_core::{
    build_rust_context_with_config, build_rust_snapshot, format_context_summary,
    sanitize_context_for_output, sanitize_snapshot_for_output, AnalysisMode, ComparisonStatus,
    EvidenceLevel, GitChangeScope, LineCountStatus, RustContextOptions,
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
    assert_eq!(pack.evidence, EvidenceLevel::ToolConfirmed);
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
    let scopes = &pack.diff.as_ref().expect("diff metadata").change_scopes;
    let revision = scopes
        .iter()
        .find(|scope| scope.scope == GitChangeScope::Revision)
        .expect("revision scope");
    assert_eq!(revision.file_count, 0);
    let staged = scopes
        .iter()
        .find(|scope| scope.scope == GitChangeScope::Staged)
        .expect("staged scope");
    assert_eq!(staged.file_count, 0);
    let unstaged = scopes
        .iter()
        .find(|scope| scope.scope == GitChangeScope::Unstaged)
        .expect("unstaged scope");
    assert_eq!((unstaged.file_count, unstaged.rust_file_count), (1, 1));
    assert_eq!((unstaged.additions, unstaged.deletions), (1, 1));
    assert_eq!(unstaged.counted_files, 1);
    let untracked = scopes
        .iter()
        .find(|scope| scope.scope == GitChangeScope::Untracked)
        .expect("untracked scope");
    assert_eq!((untracked.file_count, untracked.rust_file_count), (1, 1));
    assert_eq!(untracked.not_read_files, 1);
    let tracked_file = pack
        .changed_files
        .iter()
        .find(|file| file.path == Path::new("src/lib.rs"))
        .expect("tracked file");
    assert_eq!(tracked_file.scope_changes.len(), 1);
    assert_eq!(
        tracked_file.scope_changes[0].scope,
        GitChangeScope::Unstaged
    );
    assert_eq!(
        tracked_file.scope_changes[0].line_count_status,
        LineCountStatus::Counted
    );
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
    assert!(summary.contains("Change scopes (pre-budget totals):"));
    assert!(summary.contains("- unstaged: 1 file (1 Rust), +1/-1 counted lines across 1 file"));
    assert!(summary.contains("- untracked: 1 file (1 Rust), +0/-0 counted lines across 0 files; unknown line counts: 1 not read"));
    assert!(summary.contains("Visible patch: +1/-1 lines"));
    assert!(summary.contains("- M src/lib.rs [context-fixture] (1 hunk)"));
    assert!(summary.contains("- ?? src/untracked.rs"));
    assert!(summary.contains("Diagnostics: not run"));
    assert_eq!(summary, format_context_summary(&pack));

    let mut omitted_patch_pack = pack.clone();
    let omitted_diff = omitted_patch_pack
        .diff
        .as_mut()
        .expect("working-tree context should contain diff metadata");
    omitted_diff.patch.clear();
    omitted_diff.patch_truncated = true;
    omitted_diff.omitted_patch_bytes = 12_345;
    let omitted_summary = format_context_summary(&omitted_patch_pack);
    assert!(omitted_summary.contains("Visible patch: omitted to fit budget; 12345 bytes omitted"));
    assert!(!omitted_summary.contains("Visible patch: +0/-0 lines"));

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
fn change_scopes_preserve_mixed_index_binary_rename_and_budget_evidence() {
    let directory = tempdir().expect("temporary repository");
    let root = directory.path();
    fs::create_dir_all(root.join("src")).expect("src directory");
    fs::create_dir_all(root.join("assets")).expect("assets directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"scope-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    fs::write(
        root.join("src/lib.rs"),
        "pub mod both;\npub mod deleted;\npub mod old;\npub mod revision;\n",
    )
    .expect("lib source");
    fs::write(root.join("src/both.rs"), "pub fn value() -> u32 { 1 }\n").expect("mixed source");
    fs::write(root.join("src/old.rs"), "pub fn moved() {}\n").expect("rename source");
    fs::write(root.join("src/deleted.rs"), "pub fn removed() {}\n").expect("deleted source");
    fs::write(
        root.join("src/revision.rs"),
        "pub fn committed() -> u32 { 1 }\n",
    )
    .expect("revision source");
    fs::write(root.join("assets/blob.bin"), [0_u8, 1, 2, 3]).expect("binary fixture");

    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "context@example.invalid"]);
    git(root, &["config", "user.name", "Context Fixture"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "initial"]);

    fs::write(
        root.join("src/revision.rs"),
        "pub fn committed() -> u32 { 2 }\n",
    )
    .expect("committed revision source");
    git(root, &["add", "src/revision.rs"]);
    git(root, &["commit", "-qm", "revision change"]);

    fs::write(root.join("src/both.rs"), "pub fn value() -> u32 { 2 }\n").expect("staged source");
    git(root, &["add", "src/both.rs"]);
    fs::write(
        root.join("src/both.rs"),
        "pub fn value() -> u32 { 3 }\npub fn extra() {}\n",
    )
    .expect("unstaged source");
    git(root, &["mv", "src/old.rs", "src/new.rs"]);
    fs::write(
        root.join("src/new.rs"),
        "pub fn moved() { let _changed = true; }\n",
    )
    .expect("unstaged edit after staged rename");
    git(root, &["rm", "-q", "src/deleted.rs"]);
    fs::write(root.join("assets/blob.bin"), [0_u8, 1, 2, 4, 5]).expect("modified binary");
    fs::write(root.join("src/\u{8ffd}\u{52a0}.rs"), "pub fn added() {}\n")
        .expect("untracked Unicode source");

    let mut options = RustContextOptions::new(Some("HEAD~1".to_string()), 20_000);
    options.include_working_tree = true;
    let pack = build_rust_context_with_config(root, options).expect("context should build");
    let diff = pack.diff.as_ref().expect("working-tree diff");

    let revision = diff
        .change_scopes
        .iter()
        .find(|scope| scope.scope == GitChangeScope::Revision)
        .expect("revision scope");
    assert_eq!((revision.file_count, revision.rust_file_count), (1, 1));
    assert_eq!((revision.additions, revision.deletions), (1, 1));
    assert_eq!(revision.counted_files, 1);

    let staged = diff
        .change_scopes
        .iter()
        .find(|scope| scope.scope == GitChangeScope::Staged)
        .expect("staged scope");
    assert_eq!((staged.file_count, staged.rust_file_count), (3, 3));
    assert_eq!((staged.additions, staged.deletions), (1, 2));
    assert_eq!(staged.counted_files, 3);

    let unstaged = diff
        .change_scopes
        .iter()
        .find(|scope| scope.scope == GitChangeScope::Unstaged)
        .expect("unstaged scope");
    assert_eq!((unstaged.file_count, unstaged.rust_file_count), (3, 2));
    assert_eq!((unstaged.additions, unstaged.deletions), (3, 2));
    assert_eq!((unstaged.counted_files, unstaged.binary_files), (2, 1));

    let untracked = diff
        .change_scopes
        .iter()
        .find(|scope| scope.scope == GitChangeScope::Untracked)
        .expect("untracked scope");
    assert_eq!((untracked.file_count, untracked.rust_file_count), (1, 1));
    assert_eq!(untracked.not_read_files, 1);

    let both = pack
        .changed_files
        .iter()
        .find(|file| file.path == Path::new("src/both.rs"))
        .expect("mixed staged and unstaged path");
    assert_eq!(both.scope_changes.len(), 2);
    assert!(both
        .scope_changes
        .iter()
        .any(|change| change.scope == GitChangeScope::Staged));
    assert!(both
        .scope_changes
        .iter()
        .any(|change| change.scope == GitChangeScope::Unstaged));
    let renamed = pack
        .changed_files
        .iter()
        .find(|file| file.path == Path::new("src/new.rs"))
        .expect("renamed path");
    assert_eq!(renamed.old_path.as_deref(), Some(Path::new("src/old.rs")));
    assert_eq!(renamed.scope_changes.len(), 2);
    assert!(renamed.scope_changes.iter().any(|change| {
        change.scope == GitChangeScope::Staged && change.status.starts_with('R')
    }));
    assert!(renamed
        .scope_changes
        .iter()
        .any(|change| { change.scope == GitChangeScope::Unstaged && change.status == "M" }));
    let deleted = pack
        .changed_files
        .iter()
        .find(|file| file.path == Path::new("src/deleted.rs"))
        .expect("deleted path");
    assert_eq!(deleted.status, "D");
    assert_eq!(deleted.hunks.len(), 1);
    assert_eq!(
        deleted.scope_changes[0].deletions,
        Some(1),
        "deletion numstat should remain attached to the deleted path"
    );
    assert_eq!(
        both.hunks.len(),
        2,
        "staged and unstaged hunks should not absorb the deletion hunk"
    );
    let binary = pack
        .changed_files
        .iter()
        .find(|file| file.path == Path::new("assets/blob.bin"))
        .expect("binary path");
    assert_eq!(
        binary.scope_changes[0].line_count_status,
        LineCountStatus::Binary
    );

    let expected_scopes = diff.change_scopes.clone();
    let mut tiny_options = RustContextOptions::new(Some("HEAD~1".to_string()), 1);
    tiny_options.include_working_tree = true;
    let tiny = build_rust_context_with_config(root, tiny_options).expect("bounded context");
    assert!(tiny.diff.as_ref().expect("bounded diff").patch_truncated);
    assert!(tiny.changed_files.is_empty());
    assert_eq!(
        tiny.diff.as_ref().expect("bounded diff").change_scopes,
        expected_scopes
    );
}

#[test]
fn working_tree_context_supports_an_unborn_head() {
    let directory = tempdir().expect("temporary repository");
    let root = directory.path();
    fs::create_dir_all(root.join("src")).expect("src directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"unborn-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), "pub fn staged() -> u32 { 1 }\n").expect("staged source");

    git(root, &["init", "-q"]);
    git(root, &["add", "Cargo.toml", "src/lib.rs"]);
    fs::write(root.join("src/lib.rs"), "pub fn unstaged() -> u32 { 2 }\n")
        .expect("unstaged source");
    fs::write(root.join("src/new.rs"), "pub fn untracked() {}\n").expect("untracked source");

    let mut options = RustContextOptions::new(None, 8_000);
    options.include_working_tree = true;
    let pack = build_rust_context_with_config(root, options)
        .expect("an unborn HEAD should compare the index with the empty tree");
    let diff = pack.diff.as_ref().expect("working-tree diff");
    assert_eq!(diff.resolved_head, None);

    let lib = pack
        .changed_files
        .iter()
        .find(|file| file.path == Path::new("src/lib.rs"))
        .expect("source should be both staged and unstaged");
    let staged = lib
        .scope_changes
        .iter()
        .find(|change| change.scope == GitChangeScope::Staged)
        .expect("staged source observation");
    assert_eq!((staged.additions, staged.deletions), (Some(1), Some(0)));
    let unstaged = lib
        .scope_changes
        .iter()
        .find(|change| change.scope == GitChangeScope::Unstaged)
        .expect("unstaged source observation");
    assert_eq!((unstaged.additions, unstaged.deletions), (Some(1), Some(1)));
    assert!(pack.changed_files.iter().any(|file| {
        file.path == Path::new("src/new.rs")
            && file
                .scope_changes
                .iter()
                .any(|change| change.scope == GitChangeScope::Untracked)
    }));
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
    fs::create_dir_all(root.join("src/nested")).expect("nested src directory");
    fs::write(root.join("src/lib.rs"), "pub fn stable() {}\n").expect("source");

    let first = build_rust_snapshot(root, false, false).expect("snapshot should build");
    let second = build_rust_snapshot(root, false, false).expect("snapshot should build twice");
    assert_eq!(first.contract_version, "snapshot-v1");
    assert_eq!(first.analysis_mode, AnalysisMode::MetadataOnly);
    assert_eq!(first.canonical_hash, second.canonical_hash);

    let from_src = build_rust_snapshot(root.join("src"), false, false)
        .expect("a nested source directory should discover its Cargo workspace");
    let from_file = build_rust_snapshot(root.join("src/lib.rs"), false, false)
        .expect("a Rust source file should discover its Cargo workspace");
    let canonical_root = root.canonicalize().expect("workspace should canonicalize");
    assert_eq!(from_src.workspace.root, canonical_root);
    assert_eq!(from_file.workspace.root, canonical_root);
    assert_eq!(from_src.workspace.workspace_root, canonical_root);
    assert_eq!(from_file.workspace.workspace_root, canonical_root);

    let public = sanitize_snapshot_for_output(&first).expect("public view should serialize");
    assert_eq!(public.workspace.root, std::path::Path::new("$WORKSPACE"));
    assert!(!serde_json::to_string(&public)
        .expect("public snapshot should serialize")
        .contains(root.to_string_lossy().as_ref()));
}

#[test]
fn unicode_git_paths_are_preserved_in_context_and_summary() {
    let directory = tempdir().expect("temporary repository");
    let root = directory.path();
    fs::create_dir(root.join("src")).expect("src directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"unicode-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), "pub fn stable() {}\n").expect("source");
    fs::write(root.join("src/予定.rs"), "pub fn before() -> u32 { 1 }\n")
        .expect("tracked Unicode source");

    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "context@example.invalid"]);
    git(root, &["config", "user.name", "Context Fixture"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "initial"]);

    fs::write(root.join("src/予定.rs"), "pub fn after() -> u32 { 2 }\n")
        .expect("modified Unicode source");
    fs::write(root.join("src/追加.rs"), "pub fn added() {}\n").expect("untracked Unicode source");

    let mut options = RustContextOptions::new(Some("HEAD".to_string()), 8_000);
    options.include_working_tree = true;
    let pack = build_rust_context_with_config(root, options).expect("context should build");

    let tracked = pack
        .changed_files
        .iter()
        .find(|file| file.path == Path::new("src/予定.rs"))
        .expect("tracked Unicode path should be preserved");
    assert_eq!(tracked.status, "M");
    assert!(!tracked.hunks.is_empty());
    assert!(pack.changed_files.iter().any(|file| {
        file.path == Path::new("src/追加.rs") && file.status == "??" && file.is_rust
    }));
    assert!(pack
        .diff
        .as_ref()
        .is_some_and(|diff| diff.patch.contains("+++ b/src/予定.rs")));

    let summary = format_context_summary(&pack);
    assert!(summary.contains("src/予定.rs"));
    assert!(summary.contains("src/追加.rs"));
    assert!(!summary.contains("\\344"));
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
