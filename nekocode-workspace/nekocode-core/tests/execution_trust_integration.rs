use nekocode_core::build_rust_snapshot;
use std::fs;
use tempfile::tempdir;

#[test]
fn cargo_check_is_explicit_and_records_the_execution_boundary() {
    let directory = tempdir().expect("temporary workspace");
    let root = directory.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"execution-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = \"build.rs\"\n",
    )
    .expect("manifest");
    fs::create_dir(root.join("src")).expect("src directory");
    fs::write(root.join("src/lib.rs"), "pub fn stable() {}\n").expect("source");
    fs::write(
        root.join("build.rs"),
        r#"use std::{env, fs, path::Path};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    let report = format!(
        "offline={}\ntarget={}\nwrapper={}\n",
        env::var("CARGO_NET_OFFLINE").unwrap_or_default(),
        env::var("CARGO_TARGET_DIR").unwrap_or_default(),
        env::var("RUSTC_WRAPPER").unwrap_or_default(),
    );
    fs::write(Path::new(&manifest_dir).join("BUILD_SCRIPT_RAN"), report).expect("sentinel");
    println!("cargo:rerun-if-changed=build.rs");
}
"#,
    )
    .expect("build script");
    fs::create_dir(root.join(".cargo")).expect("cargo config directory");
    fs::write(
        root.join(".cargo/config.toml"),
        "[build]\nrustc-wrapper = \"missing-wrapper\"\nrustc-workspace-wrapper = \"missing-workspace-wrapper\"\n",
    )
    .expect("cargo config");

    let sentinel = root.join("BUILD_SCRIPT_RAN");
    let metadata_snapshot = build_rust_snapshot(root, false, false)
        .expect("metadata-only snapshot should observe the workspace");
    assert_eq!(
        metadata_snapshot.analysis_mode,
        nekocode_core::AnalysisMode::MetadataOnly
    );
    assert!(
        !sentinel.exists(),
        "metadata-only mode must not execute build.rs"
    );

    let snapshot =
        build_rust_snapshot(root, true, false).expect("cargo-check should return an observation");
    assert_eq!(
        snapshot.analysis_mode,
        nekocode_core::AnalysisMode::CargoCheck
    );
    assert_eq!(snapshot.execution_policy.workspace_trust, "required");
    assert_eq!(snapshot.execution_policy.cargo_registry_network, "offline");

    assert!(
        sentinel.is_file(),
        "build.rs should execute in explicit mode"
    );
    let report = fs::read_to_string(sentinel).expect("sentinel report");
    assert!(report.contains("offline=true"));
    assert!(report.contains("nekocode-rust-first-target"));
    assert!(report.contains("wrapper=\n"));
}
