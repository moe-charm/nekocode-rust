use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::tempdir;

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("git must be available for the CLI fixture");
    assert!(
        output.status.success(),
        "git {args:?} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn context(root: &Path, extra_args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nekocode"))
        .arg("context")
        .arg(root)
        .arg("--compare-ref")
        .arg("HEAD")
        .arg("--working-tree")
        .arg("--budget")
        .arg("20000")
        .args(extra_args)
        .output()
        .expect("nekocode CLI should run")
}

#[test]
fn summary_is_readable_and_json_remains_the_default_contract() {
    let directory = tempdir().expect("temporary workspace");
    let root = directory.path();
    fs::create_dir(root.join("src")).expect("src directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"summary-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), "pub fn value() -> u32 { 1 }\n").expect("source");

    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "summary@example.invalid"]);
    git(root, &["config", "user.name", "Summary Fixture"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "initial"]);
    fs::write(root.join("src/lib.rs"), "pub fn value() -> u32 { 2 }\n").expect("modified source");

    let summary_output = context(root, &["--format", "summary"]);
    assert!(
        summary_output.status.success(),
        "summary failed:\n{}",
        String::from_utf8_lossy(&summary_output.stderr)
    );
    let summary = String::from_utf8(summary_output.stdout).expect("summary must be UTF-8");
    assert!(summary.starts_with("NekoCode change summary\n"));
    assert!(summary.contains("Changes: 1 file (1 Rust), 1 hunk"));
    assert!(summary.contains("Visible patch: +1/-1 lines"));
    assert!(summary.contains("- M src/lib.rs [summary-fixture] (1 hunk)"));
    assert!(summary.contains("Diagnostics: not run"));
    assert!(!summary.trim_start().starts_with('{'));

    let json_output = context(root, &[]);
    assert!(
        json_output.status.success(),
        "JSON failed:\n{}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let json: Value = serde_json::from_slice(&json_output.stdout).expect("default must stay JSON");
    assert_eq!(json["contract_version"], "context-v1");
    assert_eq!(json["artifact_kind"], "context");
}
