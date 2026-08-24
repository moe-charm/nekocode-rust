use nekocode_core::{
    build_rust_context_with_config, build_rust_snapshot_with_analysis, AnalysisMode,
    ArtifactStatus, ComparisonStatus, DiagnosticProducer, DiagnosticProfile, RustContextOptions,
};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_package(root: &Path, source: &str, build_script: Option<&str>) {
    fs::create_dir_all(root.join("src")).expect("src directory");
    let build_line = build_script.map_or(String::new(), |_| "build = \"build.rs\"\n".to_string());
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"clippy-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{build_line}"
        ),
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), source).expect("source");
    if let Some(build_script) = build_script {
        fs::write(root.join("build.rs"), build_script).expect("build script");
    }
}

#[test]
fn clippy_clean_and_warning_observations_have_explicit_markers() {
    let clean_directory = tempdir().expect("clean fixture directory");
    write_package(
        clean_directory.path(),
        "pub fn value() -> u32 { 1 }\n",
        None,
    );
    let clean =
        build_rust_snapshot_with_analysis(clean_directory.path(), AnalysisMode::Clippy, false)
            .expect("clean Clippy fixture should run");
    let clean_run = clean.diagnostics.as_ref().expect("Clippy run");
    assert_eq!(clean.analysis_mode, AnalysisMode::Clippy);
    assert_eq!(clean.execution_policy.mode, AnalysisMode::Clippy);
    assert_eq!(clean_run.producer, DiagnosticProducer::Clippy);
    assert_eq!(clean_run.profile, DiagnosticProfile::ClippyDefaultV1);
    assert!(clean_run.producer_version.is_some());
    assert_eq!(clean_run.status, "success");
    assert_eq!(clean.status, ArtifactStatus::CompletedClean);

    let warning_directory = tempdir().expect("warning fixture directory");
    write_package(
        warning_directory.path(),
        "#![warn(clippy::needless_return)]\npub fn value() -> u32 { return 1; }\n",
        None,
    );
    let warning =
        build_rust_snapshot_with_analysis(warning_directory.path(), AnalysisMode::Clippy, false)
            .expect("warning Clippy fixture should run");
    let warning_run = warning.diagnostics.as_ref().expect("Clippy warning run");
    assert_eq!(warning_run.producer, DiagnosticProducer::Clippy);
    assert_eq!(warning_run.status, "success");
    assert_eq!(warning.status, ArtifactStatus::CompletedWithDiagnostics);
    assert!(warning_run
        .messages
        .iter()
        .any(|diagnostic| diagnostic.code.as_deref() == Some("clippy::needless_return")));
}

#[test]
fn repeated_clippy_snapshots_have_stable_canonical_hashes() {
    let directory = tempdir().expect("stable-hash fixture directory");
    write_package(directory.path(), "pub fn value() -> u32 { 1 }\n", None);

    let first = build_rust_snapshot_with_analysis(directory.path(), AnalysisMode::Clippy, false)
        .expect("first Clippy snapshot should run");
    let second = build_rust_snapshot_with_analysis(directory.path(), AnalysisMode::Clippy, false)
        .expect("second Clippy snapshot should run");

    assert_eq!(first.canonical_hash, second.canonical_hash);
}

#[test]
fn clippy_compiler_errors_remain_evidence() {
    let directory = tempdir().expect("compiler-error fixture directory");
    write_package(
        directory.path(),
        "pub fn value() -> u32 { \"not a number\" }\n",
        None,
    );
    let snapshot = build_rust_snapshot_with_analysis(directory.path(), AnalysisMode::Clippy, false)
        .expect("compiler errors should remain a Clippy observation");
    let run = snapshot.diagnostics.as_ref().expect("Clippy run");
    assert_eq!(run.producer, DiagnosticProducer::Clippy);
    assert_eq!(run.status, "failed");
    assert_eq!(snapshot.status, ArtifactStatus::CompletedWithDiagnostics);
    assert!(run
        .messages
        .iter()
        .any(|diagnostic| diagnostic.code.as_deref() == Some("E0308")));
}

#[test]
fn clippy_tool_failure_is_not_reported_as_a_clean_run() {
    let directory = tempdir().expect("tool-failure fixture directory");
    write_package(
        directory.path(),
        "pub fn value() -> u32 { 1 }\n",
        Some("fn main() { std::process::exit(9); }\n"),
    );
    let snapshot = build_rust_snapshot_with_analysis(directory.path(), AnalysisMode::Clippy, false)
        .expect("a failing build script should remain an observation");
    let run = snapshot.diagnostics.as_ref().expect("Clippy run");
    assert_eq!(run.producer, DiagnosticProducer::Clippy);
    assert_eq!(run.status, "failed");
    assert!(run.messages.is_empty());
    assert_eq!(snapshot.status, ArtifactStatus::ToolFailed);
    assert_eq!(snapshot.evidence, nekocode_core::EvidenceLevel::Incomplete);
}

#[test]
fn cargo_and_clippy_profiles_are_not_comparable() {
    let directory = tempdir().expect("profile fixture directory");
    let root = directory.path();
    write_package(root, "pub fn value() -> u32 { 1 }\n", None);

    let baseline = build_rust_snapshot_with_analysis(root, AnalysisMode::CargoCheck, false)
        .expect("cargo-check baseline should run");
    let baseline_path = directory.path().join("baseline.json");
    nekocode_core::write_rust_snapshot(&baseline_path, &baseline).expect("write baseline");

    let mut options = RustContextOptions::new(None, 20_000);
    options.include_diagnostics = true;
    options.diagnostic_producer = DiagnosticProducer::Clippy;
    options.baseline = Some(baseline_path);
    let context = build_rust_context_with_config(root, options)
        .expect("Clippy context should preserve profile mismatch evidence");
    assert_eq!(
        context.diagnostic_producer,
        Some(DiagnosticProducer::Clippy)
    );
    assert_eq!(
        context.diagnostic_profile,
        Some(DiagnosticProfile::ClippyDefaultV1)
    );
    let delta = context.diagnostic_delta.expect("profile delta");
    assert_eq!(delta.status, ComparisonStatus::NotComparable);
    assert!(!delta.compatible);
    assert!(delta
        .limitations
        .iter()
        .any(|limitation| limitation.contains("producers differ")));

    let mut tiny_options = RustContextOptions::new(None, 1);
    tiny_options.include_diagnostics = true;
    tiny_options.diagnostic_producer = DiagnosticProducer::Clippy;
    let tiny = build_rust_context_with_config(root, tiny_options)
        .expect("a tiny budget should still preserve the producer marker");
    assert_eq!(tiny.diagnostic_producer, Some(DiagnosticProducer::Clippy));
    assert_eq!(
        tiny.diagnostic_profile,
        Some(DiagnosticProfile::ClippyDefaultV1)
    );
    assert_eq!(tiny.status, ArtifactStatus::OutputLimited);
    assert!(tiny.diagnostics.is_none());
}
