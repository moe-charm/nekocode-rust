#[cfg(unix)]
mod unix_only {
    use nekocode_core::{build_rust_context_with_config, RustContextOptions};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .expect("git must be available");
        assert!(
            output.status.success(),
            "git {:?} failed:\n{}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn context_disables_external_diff_and_textconv_helpers() {
        let directory = tempdir().expect("temporary workspace");
        let root = directory.path();
        fs::create_dir(root.join("src")).expect("src directory");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"git-helper-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("manifest");
        fs::write(root.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").expect("source");
        fs::write(root.join(".gitattributes"), "src/lib.rs diff=sentinel\n").expect("attributes");

        let helper = root.join("helper.sh");
        let sentinel = root.join("GIT_HELPER_RAN");
        let helper_body = format!("#!/bin/sh\nprintf ran > '{}'\nexit 0\n", sentinel.display());
        fs::write(&helper, helper_body).expect("helper");
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).expect("executable");

        git(root, &["init", "-q"]);
        git(root, &["config", "user.email", "fixture@example.invalid"]);
        git(root, &["config", "user.name", "NekoCode Fixture"]);
        git(root, &["config", "diff.external", "./helper.sh"]);
        git(root, &["config", "diff.sentinel.textconv", "./helper.sh"]);
        git(root, &["add", "Cargo.toml", "src/lib.rs", ".gitattributes"]);
        git(root, &["commit", "-qm", "baseline"]);
        fs::write(root.join("src/lib.rs"), "pub fn value() -> u8 { 2 }\n")
            .expect("modified source");

        let mut options = RustContextOptions::new(Some("HEAD".to_string()), 20_000);
        options.include_working_tree = true;
        let pack = build_rust_context_with_config(root, options).expect("context should build");

        assert!(pack
            .diff
            .as_ref()
            .is_some_and(|diff| diff.patch.contains("pub fn value() -> u8 { 2 }")));
        assert!(
            !sentinel.exists(),
            "Git external diff or textconv helper must not execute"
        );
    }
}
