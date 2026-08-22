use nekocode_core::{index_rust_workspace, EvidenceLevel};

#[test]
fn indexes_the_workspace_that_builds_the_core_crate() {
    let snapshot = index_rust_workspace(env!("CARGO_MANIFEST_DIR"))
        .expect("the core crate must remain inside a valid Cargo workspace");

    assert_eq!(snapshot.evidence, EvidenceLevel::ToolConfirmed);
    assert!(snapshot.workspace_root.ends_with("nekocode-workspace"));
    assert!(snapshot
        .toolchain
        .rustc_version
        .as_deref()
        .map(|version| version.starts_with("rustc "))
        .unwrap_or(false));
    assert!(snapshot
        .toolchain
        .cargo_version
        .as_deref()
        .map(|version| version.starts_with("cargo "))
        .unwrap_or(false));
    assert!(snapshot
        .packages
        .iter()
        .any(|package| package.name == "nekocode-core"));
}
