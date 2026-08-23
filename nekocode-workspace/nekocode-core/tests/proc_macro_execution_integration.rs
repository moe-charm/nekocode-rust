use nekocode_core::{build_rust_context_with_options, ArtifactStatus, ComparisonStatus};
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rust_proc_macro_workspace")
}

#[test]
fn proc_macro_execution_is_recorded_as_compiler_evidence() {
    let pack = build_rust_context_with_options(fixture_root(), None, 20_000, true)
        .expect("proc-macro fixture should return an observation");
    let diagnostics = pack.diagnostics.expect("diagnostics were requested");

    assert_eq!(pack.status, ArtifactStatus::CompletedWithDiagnostics);
    assert_eq!(pack.comparison_status, ComparisonStatus::BaselineMissing);
    assert_eq!(diagnostics.status, "failed");
    assert!(diagnostics.messages.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("nekocode proc macro execution sentinel")
    }));
    assert_eq!(pack.execution_policy.workspace_trust, "required");
    assert_eq!(
        pack.execution_policy.process_network_isolation,
        "not_enforced"
    );
}
