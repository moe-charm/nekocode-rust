use nekocode_core::{build_rust_context_with_options, index_rust_workspace, EvidenceLevel};
use std::path::PathBuf;
use std::process::Command;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rust_golden_workspace")
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
}
