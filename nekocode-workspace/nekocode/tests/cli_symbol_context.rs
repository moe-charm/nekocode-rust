use std::process::Command;

#[test]
fn incompatible_symbol_modes_fail_before_loading_a_workspace() {
    let cases: &[&[&str]] = &[
        &["--at", "src/lib.rs:1", "--symbol", "value"],
        &["--at", "src/lib.rs:1", "--packet", "saved.json"],
        &["--at", "src/lib.rs:1", "--compare-ref", "HEAD"],
        &["--at", "src/lib.rs:1", "--diagnostics"],
        &[
            "--at",
            "src/lib.rs:1",
            "--diagnostic-producer",
            "cargo-check",
        ],
        &["--at", "src/lib.rs:1", "--working-tree"],
        &["--at", "src/lib.rs:1", "--excerpt-lines", "8"],
        &["--item", "item-1"],
        &["--cursor", "cursor-1"],
        &["--max-items", "4"],
        &["--save-packet", "saved.json"],
        &["--packet", "saved.json", "--allow-build-scripts"],
        &["--packet", "saved.json", "--all-features"],
        &["--packet", "saved.json", "--timeout-seconds", "60"],
        &[
            "--packet",
            "saved.json",
            "--item",
            "item-1",
            "--cursor",
            "cursor-1",
        ],
    ];
    for args in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_nekocode"))
            .arg("context")
            .args(*args)
            .output()
            .expect("CLI starts");
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn missing_backend_is_an_artifact_and_old_context_defaults_still_work() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let root = directory.path();
    std::fs::create_dir(root.join("src")).expect("source directory");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"symbol-cli-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "pub fn value() -> u32 { 1 }\n").expect("source");
    let output = Command::new(env!("CARGO_BIN_EXE_nekocode"))
        .current_dir(root)
        .args(["context", "--at", "src/lib.rs:1:8"])
        .env("NEKOCODE_RUST_ANALYZER_PATH", root.join("missing-analyzer"))
        .output()
        .expect("CLI starts");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact: serde_json::Value = serde_json::from_slice(&output.stdout).expect("symbol JSON");
    assert_eq!(artifact["contract_version"], "symbol-context-v1");
    assert!(artifact["items"].as_array().unwrap().is_empty());
    assert_ne!(artifact["status"], "complete");

    let output = Command::new(env!("CARGO_BIN_EXE_nekocode"))
        .current_dir(root)
        .args(["context"])
        .output()
        .expect("CLI starts");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Git context JSON");
    assert_eq!(artifact["contract_version"], "context-v1");
}

#[cfg(unix)]
#[test]
fn saved_navigation_preserves_code_without_backend_executables() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().expect("fixture directory");
    let root = directory.path();
    std::fs::create_dir(root.join("src")).expect("source directory");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"symbol-cli-paging\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    let source = r#"pub fn target(value: u32) -> u32 {
    let _url = "https://example.test/a/b";
    let _path = r"C:\work\item";
    value / 2
}
pub fn caller() -> u32 { target(8) }
#[test]
fn target_test() { assert_eq!(target(4), 2); }
"#;
    std::fs::write(root.join("src/lib.rs"), source).expect("source");
    let analyzer = root.join("fake-ra");
    std::fs::write(
        &analyzer,
        include_str!("../../nekocode-core/src/symbol_context/fixtures/fake-ra.py"),
    )
    .expect("fake LSP fixture");
    std::fs::set_permissions(&analyzer, std::fs::Permissions::from_mode(0o755))
        .expect("executable fixture");
    let saved = root.join("packet.json");
    let output = Command::new(env!("CARGO_BIN_EXE_nekocode"))
        .current_dir(root)
        .args(["context", "--at", "src/lib.rs:1:8", "--max-items", "1"])
        .arg("--save-packet")
        .arg(&saved)
        .env("NEKOCODE_RUST_ANALYZER_PATH", &analyzer)
        .output()
        .expect("investigation starts");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let initial: serde_json::Value = serde_json::from_slice(&output.stdout).expect("initial JSON");
    assert_eq!(initial["items"].as_array().unwrap().len(), 1);
    let cursor = initial["continuation"]["next_cursor"]
        .as_str()
        .expect("saved first page has a cursor");
    let item = initial["items"][0]["id"].as_str().expect("item ID");
    std::fs::remove_file(analyzer).expect("remove fake backend");

    for (flag, value) in [("--cursor", cursor), ("--item", item)] {
        let output = Command::new(env!("CARGO_BIN_EXE_nekocode"))
            .args(["context", "--packet"])
            .arg(&saved)
            .args([flag, value, "--max-items", "1"])
            .env("PATH", root.join("no-executables"))
            .env("NEKOCODE_RUST_ANALYZER_PATH", root.join("missing-analyzer"))
            .output()
            .expect("replay starts");
        assert!(
            output.status.success(),
            "{flag}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let replay: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("replay JSON");
        assert_eq!(replay["packet_id"], initial["packet_id"]);
        assert_eq!(replay["items"].as_array().unwrap().len(), 1);
        if flag == "--item" {
            let code = replay["items"][0]["code"].as_str().expect("captured code");
            assert!(code.contains("https://example.test/a/b"));
            assert!(code.contains("value / 2"));
            assert!(code.contains(r"C:\work\item"));
        } else {
            assert_ne!(replay["items"][0]["id"], initial["items"][0]["id"]);
        }
    }
}
