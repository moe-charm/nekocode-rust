use nekocode_core::{build_rust_context_with_config, RustContextOptions};
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
}
