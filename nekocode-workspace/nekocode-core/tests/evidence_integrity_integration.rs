use nekocode_core::{
    build_context, build_snapshot, sanitize_snapshot_for_output, write_rust_snapshot, AnalysisMode,
    ComparisonStatus, ContextRequest, GitChangeScope, SnapshotRequest,
};
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn fixture(root: &Path, source: &str) {
    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"evidence-integrity\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), source).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn saved_public_snapshot_preserves_unchanged_warning_identity() {
    let directory = tempdir().unwrap();
    let root = directory.path();
    fixture(root, "pub fn value() -> u32 { let unused = 3; 42 }\n");
    let snapshot = build_snapshot(&SnapshotRequest {
        path: root.into(),
        analysis: AnalysisMode::CargoCheck,
        all_features: false,
    })
    .unwrap();
    let run = snapshot.diagnostics.as_ref().unwrap();
    assert_eq!(run.status, "success");
    let warnings = run.messages.iter().filter(|m| m.level == "warning").count();
    assert!(warnings > 0);
    let baseline = root.join("baseline.json");
    write_rust_snapshot(&baseline, &sanitize_snapshot_for_output(&snapshot).unwrap()).unwrap();
    let mut request = ContextRequest::new(root, 30_000);
    request.diagnostics = true;
    request.baseline = Some(baseline);
    request.budget = 30_000;
    let context = build_context(&request).unwrap();
    let delta = context.diagnostic_delta.unwrap();
    assert_eq!(delta.status, ComparisonStatus::Comparable);
    assert!(delta.added.is_empty());
    assert!(delta.resolved.is_empty());
    assert_eq!(delta.persisting.len(), warnings);
}

#[test]
fn revision_index_and_disk_excerpts_keep_their_own_source() {
    let directory = tempdir().unwrap();
    let root = directory.path();
    fixture(root, "pub fn value() -> u32 { 1 }\n");
    let spaced = root.join("src/file with space.rs");
    fs::write(&spaced, "pub fn value() -> u32 { 1 }\n").unwrap();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "Evidence Fixture"]);
    git(root, &["config", "user.email", "fixture@example.invalid"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "first"]);
    for number in 2..=4 {
        for path in [root.join("src/lib.rs"), spaced.clone()] {
            fs::write(path, format!("pub fn value() -> u32 {{ {number} }}\n")).unwrap();
        }
        if number < 4 {
            git(root, &["add", "."]);
        }
        if number == 2 {
            git(root, &["commit", "-qm", "second"]);
        }
    }
    let mut request = ContextRequest::new(root, 30_000);
    request.compare_ref = Some("HEAD~1".into());
    request.budget = 30_000;
    let revision = build_context(&request).unwrap();
    assert!(!revision.source_excerpts.is_empty());
    for excerpt in &revision.source_excerpts {
        assert_eq!(excerpt.scope, Some(GitChangeScope::Revision));
        assert!(excerpt.content.contains("{ 2 }"));
        assert!(!excerpt.content.contains("{ 4 }"));
    }
    request.working_tree = true;
    let mixed = build_context(&request).unwrap();
    let file = mixed
        .changed_files
        .iter()
        .find(|f| f.path == Path::new("src/file with space.rs"))
        .unwrap();
    assert_eq!(file.hunks.len(), 3);
    for (scope, number) in [
        (GitChangeScope::Revision, 2),
        (GitChangeScope::Staged, 3),
        (GitChangeScope::Unstaged, 4),
    ] {
        let excerpt = mixed
            .source_excerpts
            .iter()
            .find(|e| e.path == file.path && e.scope == Some(scope))
            .unwrap();
        assert!(
            excerpt.content.contains(&format!("{{ {number} }}")),
            "{excerpt:?}"
        );
    }
}
