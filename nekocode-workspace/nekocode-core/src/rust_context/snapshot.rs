//! Snapshot persistence, hashing, and public path redaction.

use super::*;
use super::{diagnostics, summary};
use super::{execution, workspace};
use diagnostics::*;
use execution::*;
use summary::*;
use workspace::index_rust_workspace;

pub fn build_rust_snapshot(
    path: impl AsRef<Path>,
    include_diagnostics: bool,
    all_features: bool,
) -> Result<RustContextSnapshot> {
    build_rust_snapshot_with_mode(path, include_diagnostics, all_features)
}

/// Build a snapshot with explicit analysis mode semantics.
pub fn build_rust_snapshot_with_mode(
    path: impl AsRef<Path>,
    include_diagnostics: bool,
    all_features: bool,
) -> Result<RustContextSnapshot> {
    let analysis = if include_diagnostics {
        AnalysisMode::CargoCheck
    } else {
        AnalysisMode::MetadataOnly
    };
    build_rust_snapshot_with_analysis(path, analysis, all_features)
}

/// Build a snapshot using metadata, cargo check, or the explicit Clippy
/// producer. The latter two modes execute trusted workspace code.
pub fn build_rust_snapshot_with_analysis(
    path: impl AsRef<Path>,
    analysis: AnalysisMode,
    all_features: bool,
) -> Result<RustContextSnapshot> {
    let path = path.as_ref();
    let initial_workspace = index_rust_workspace(path)?;
    let producer = match analysis {
        AnalysisMode::MetadataOnly => None,
        AnalysisMode::CargoCheck => Some(DiagnosticProducer::CargoCheck),
        AnalysisMode::Clippy => Some(DiagnosticProducer::Clippy),
    };
    let mut diagnostics = producer
        .map(|producer| {
            run_diagnostic_with_options(&initial_workspace.root, producer, all_features)
        })
        .transpose()?;
    // A compiler observation can create Cargo.lock in a workspace that did
    // not have one before the run. Re-read metadata after the observation so
    // the snapshot describes the post-run inputs and repeated snapshots do
    // not differ only because NekoCode created that lockfile.
    let workspace = if producer.is_some() {
        index_rust_workspace(&initial_workspace.root)?
    } else {
        initial_workspace
    };
    if let Some(run) = diagnostics.as_mut() {
        run.comparison_basis = Some(comparison_basis_for_run(&workspace, run));
    }
    let status = diagnostics
        .as_ref()
        .map_or(ArtifactStatus::CompletedClean, diagnostic_status);
    let mut limitations = if let Some(producer) = producer {
        vec![format!(
            "{} may execute trusted workspace build scripts and procedural macros.",
            producer.command_name()
        )]
    } else {
        vec!["Compiler diagnostics were not requested.".to_string()]
    };
    if producer.is_some() && !artifact_status_is_complete(status) {
        limitations.push(format!(
            "{} did not produce a complete diagnostic observation (status: {}).",
            producer
                .map(DiagnosticProducer::command_name)
                .unwrap_or("compiler diagnostics"),
            artifact_status_name(status)
        ));
    }
    let mut snapshot = RustContextSnapshot {
        contract_version: SNAPSHOT_CONTRACT_VERSION.to_string(),
        artifact_kind: "snapshot".to_string(),
        status,
        analysis_mode: analysis,
        schema_version: SCHEMA_VERSION,
        evidence: evidence_for_artifact_status(status),
        execution_policy: producer
            .map_or_else(metadata_execution_policy, diagnostic_execution_policy),
        generated_at: chrono::Utc::now().to_rfc3339(),
        workspace,
        diagnostics,
        canonical_hash: None,
        limitations,
        omissions: Vec::new(),
    };
    snapshot.canonical_hash = Some(canonical_snapshot_hash(&snapshot)?);
    Ok(snapshot)
}

/// Atomically replace an explicit snapshot path with pretty JSON.
pub fn write_rust_snapshot(path: impl AsRef<Path>, snapshot: &RustContextSnapshot) -> Result<()> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| NekocodeError::Config("snapshot path must name a file".to_string()))?;
    let temporary = parent.join(format!(
        ".{}.tmp-{}",
        file_name.to_string_lossy(),
        std::process::id()
    ));
    let bytes = serde_json::to_vec_pretty(snapshot)?;
    if let Err(error) = std::fs::write(&temporary, bytes) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.into());
    }
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

/// Read and validate an explicit Rust context snapshot.
pub fn read_rust_snapshot(path: impl AsRef<Path>) -> Result<RustContextSnapshot> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    let snapshot: RustContextSnapshot = serde_json::from_slice(&bytes)?;
    if snapshot.contract_version != SNAPSHOT_CONTRACT_VERSION {
        return Err(NekocodeError::Config(format!(
            "unsupported Rust snapshot contract version {} (expected {})",
            snapshot.contract_version, SNAPSHOT_CONTRACT_VERSION
        )));
    }
    if snapshot.artifact_kind != "snapshot" {
        return Err(NekocodeError::Config(
            "Rust baseline artifact_kind must be snapshot".to_string(),
        ));
    }
    if snapshot.schema_version != SCHEMA_VERSION {
        return Err(NekocodeError::Config(format!(
            "unsupported Rust snapshot schema version {} (expected {})",
            snapshot.schema_version, SCHEMA_VERSION
        )));
    }
    if let Some(expected_hash) = snapshot.canonical_hash.as_deref() {
        let actual_hash = canonical_snapshot_hash(&snapshot)?;
        if expected_hash != actual_hash {
            return Err(NekocodeError::Config(format!(
                "Rust snapshot canonical hash mismatch (expected {expected_hash}, calculated {actual_hash})"
            )));
        }
    }
    Ok(snapshot)
}

pub(crate) fn canonical_snapshot_hash(snapshot: &RustContextSnapshot) -> Result<String> {
    let mut value = serde_json::to_value(snapshot)?;
    let workspace_root = snapshot.workspace.workspace_root.clone();
    normalize_hash_value(&mut value, None, &workspace_root);
    let bytes = serde_json::to_vec(&value)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

/// Return the public snapshot view without machine-specific absolute paths.
pub fn sanitize_snapshot_for_output(snapshot: &SnapshotV1) -> Result<SnapshotV1> {
    sanitize_artifact_paths(snapshot, &snapshot.workspace.workspace_root)
}

/// Return the public context view without machine-specific absolute paths.
pub fn sanitize_context_for_output(context: &ContextV1) -> Result<ContextV1> {
    sanitize_artifact_paths(context, &context.workspace.workspace_root)
}

pub(crate) fn sanitize_artifact_paths<T>(value: &T, workspace_root: &Path) -> Result<T>
where
    T: Serialize + DeserializeOwned,
{
    let mut json = serde_json::to_value(value)?;
    let normalized_root = workspace_root.to_string_lossy().replace('\\', "/");
    sanitize_public_json(&mut json, None, &normalized_root);
    Ok(serde_json::from_value(json)?)
}

pub(crate) fn sanitize_public_json(
    value: &mut serde_json::Value,
    key: Option<&str>,
    workspace_root: &str,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (child_key, child) in map.iter_mut() {
                sanitize_public_json(child, Some(child_key), workspace_root);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                sanitize_public_json(item, key, workspace_root);
            }
        }
        serde_json::Value::String(text) => {
            let normalized = text.replace('\\', "/");
            if normalized.contains(workspace_root) {
                *text = normalized.replace(workspace_root, "$WORKSPACE");
            } else if is_path_field(key)
                && (normalized.starts_with('/') || is_windows_absolute(&normalized))
            {
                *text = "$EXTERNAL".to_string();
            }
        }
        _ => {}
    }
}

pub(crate) fn is_path_field(key: Option<&str>) -> bool {
    matches!(
        key,
        Some(
            "root"
                | "workspace_root"
                | "cwd"
                | "manifest_path"
                | "src_path"
                | "file"
                | "file_name"
                | "filename"
                | "path"
                | "old_path"
                | "baseline"
                | "baseline_path"
        )
    )
}

pub(crate) fn is_windows_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/'
}

pub(crate) fn normalize_hash_value(
    value: &mut serde_json::Value,
    key: Option<&str>,
    workspace_root: &Path,
) {
    match value {
        serde_json::Value::Object(map) => {
            map.remove("generated_at");
            map.remove("canonical_hash");
            // Cargo writes elapsed-time text such as `Finished ... in 0.14s`
            // to stderr. Keep that observation in the artifact, but exclude
            // raw process stderr from the identity hash so repeated runs are
            // comparable. Structured diagnostics, status, exit code, and
            // provenance remain hashed separately.
            map.remove("stderr");
            for (child_key, child) in map.iter_mut() {
                normalize_hash_value(child, Some(child_key), workspace_root);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                normalize_hash_value(item, key, workspace_root);
            }
        }
        serde_json::Value::String(text) => {
            let normalized = text.replace('\\', "/");
            let root_text = workspace_root.to_string_lossy().replace('\\', "/");
            if normalized.contains(&root_text) {
                *text = normalized.replace(&root_text, "$WORKSPACE");
            } else if is_path_field(key) {
                let candidate = Path::new(text.as_str());
                if candidate.is_absolute() || is_windows_absolute(&normalized) {
                    *text = "$EXTERNAL".to_string();
                }
            }
        }
        _ => {}
    }
}
