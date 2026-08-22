//! Rust-first, evidence-backed project context.
//!
//! This module deliberately does not reimplement Rust semantic analysis. Cargo
//! is the source of truth for workspace/package metadata; Git is the source of
//! truth for the requested change set. Later backends can enrich this snapshot
//! with rustc, Clippy, and rust-analyzer results without changing the JSON
//! contract established here.

use crate::error::{NekocodeError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

const SCHEMA_VERSION: u32 = 1;

/// Evidence level for data returned by the Rust-first context layer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceLevel {
    /// Directly reported by Cargo/Git rather than inferred from syntax.
    ToolConfirmed,
    /// Derived from a semantic backend (reserved for later backends).
    SemanticResolved,
    /// Derived from syntax only (reserved for later backends).
    SyntaxOnly,
    /// The requested backend could not provide complete information.
    Incomplete,
}

/// Tool versions used to produce a workspace snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustToolchainInfo {
    pub rustc_version: Option<String>,
    pub cargo_version: Option<String>,
    pub host: Option<String>,
}

/// A Cargo package in the indexed workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustPackage {
    pub name: String,
    pub version: String,
    pub manifest_path: PathBuf,
    pub targets: Vec<String>,
    pub features: Vec<String>,
}

/// A stable, serializable snapshot of Cargo workspace structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustWorkspaceSnapshot {
    pub schema_version: u32,
    pub evidence: EvidenceLevel,
    pub root: PathBuf,
    pub workspace_root: PathBuf,
    pub toolchain: RustToolchainInfo,
    pub packages: Vec<RustPackage>,
}

/// A file reported by `git diff --name-status`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedRustFile {
    pub status: String,
    pub path: PathBuf,
}

/// One compiler diagnostic extracted from Cargo's JSON message stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiagnostic {
    pub level: String,
    pub message: String,
    pub code: Option<String>,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

/// Result of one `cargo check` invocation, including failed checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiagnosticRun {
    pub command: String,
    pub status: String,
    pub messages: Vec<RustDiagnostic>,
}

/// Compact context pack intended for MCP/AI consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustContextPack {
    pub schema_version: u32,
    pub evidence: EvidenceLevel,
    pub root: PathBuf,
    pub workspace: RustWorkspaceSnapshot,
    pub compare_ref: Option<String>,
    pub changed_files: Vec<ChangedRustFile>,
    pub diagnostics: Option<RustDiagnosticRun>,
    pub budget_tokens: usize,
    pub omitted_changed_files: usize,
    pub omitted_diagnostics: usize,
    pub limitations: Vec<String>,
}

/// Read Cargo workspace metadata without attempting to parse Rust semantics.
pub fn index_rust_workspace(path: impl AsRef<Path>) -> Result<RustWorkspaceSnapshot> {
    let root = normalize_workspace_root(path.as_ref())?;
    let output = Command::new("cargo")
        .current_dir(&root)
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .map_err(|error| {
            NekocodeError::External(format!("failed to run cargo metadata: {error}"))
        })?;

    if !output.status.success() {
        return Err(NekocodeError::External(format_command_failure(
            "cargo metadata",
            &output.stderr,
        )));
    }

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let workspace_root = metadata
        .get("workspace_root")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| {
            NekocodeError::External("cargo metadata omitted workspace_root".to_string())
        })?;

    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| NekocodeError::External("cargo metadata omitted packages".to_string()))?
        .iter()
        .map(parse_package)
        .collect::<Result<Vec<_>>>()?;
    let toolchain = detect_toolchain();

    Ok(RustWorkspaceSnapshot {
        schema_version: SCHEMA_VERSION,
        evidence: EvidenceLevel::ToolConfirmed,
        root,
        workspace_root,
        toolchain,
        packages,
    })
}

/// Build a change-focused context pack from Cargo metadata and Git.
pub fn build_rust_context(
    path: impl AsRef<Path>,
    compare_ref: Option<&str>,
    budget_tokens: usize,
) -> Result<RustContextPack> {
    build_rust_context_with_options(path, compare_ref, budget_tokens, false)
}

/// Build a context pack, optionally including compiler diagnostics.
pub fn build_rust_context_with_options(
    path: impl AsRef<Path>,
    compare_ref: Option<&str>,
    budget_tokens: usize,
    include_diagnostics: bool,
) -> Result<RustContextPack> {
    if budget_tokens == 0 {
        return Err(NekocodeError::Config(
            "context budget must be greater than zero".to_string(),
        ));
    }

    let workspace = index_rust_workspace(path)?;
    let changed_files = match compare_ref {
        Some(reference) => git_changed_files(&workspace.root, reference)?,
        None => Vec::new(),
    };
    let diagnostics = if include_diagnostics {
        Some(run_cargo_check(&workspace.root)?)
    } else {
        None
    };

    // Keep the pack bounded even before semantic symbol data is added. The
    // estimate is intentionally conservative: JSON is usually a few bytes per
    // token, and the caller can request a larger budget when needed.
    let byte_budget = budget_tokens.saturating_mul(4);
    let mut pack = RustContextPack {
        schema_version: SCHEMA_VERSION,
        evidence: EvidenceLevel::ToolConfirmed,
        root: workspace.root.clone(),
        workspace,
        compare_ref: compare_ref.map(str::to_string),
        changed_files,
        diagnostics,
        budget_tokens,
        omitted_changed_files: 0,
        omitted_diagnostics: 0,
        limitations: limitations(include_diagnostics, false),
    };

    // Trim the least stable, most verbose parts first so the advertised budget
    // remains useful even when cargo emits hundreds of warnings. Workspace
    // metadata is retained as the structural baseline whenever possible.
    while serde_json::to_vec(&pack)?.len() > byte_budget {
        if let Some(run) = pack.diagnostics.as_mut() {
            if run.messages.pop().is_some() {
                pack.omitted_diagnostics += 1;
                continue;
            }
        }
        if pack.diagnostics.is_some() {
            pack.diagnostics = None;
            pack.omitted_diagnostics += 1;
            continue;
        }
        if pack.changed_files.pop().is_some() {
            pack.omitted_changed_files += 1;
            continue;
        }
        break;
    }

    if pack.omitted_changed_files > 0 || pack.omitted_diagnostics > 0 {
        pack.evidence = EvidenceLevel::Incomplete;
    }
    pack.limitations = limitations(
        include_diagnostics,
        pack.omitted_changed_files > 0 || pack.omitted_diagnostics > 0,
    );
    Ok(pack)
}

fn normalize_workspace_root(path: &Path) -> Result<PathBuf> {
    let candidate = path.canonicalize()?;
    let root = if candidate.is_file() {
        candidate
            .parent()
            .ok_or_else(|| NekocodeError::Config("Cargo.toml has no parent directory".to_string()))?
            .to_path_buf()
    } else {
        candidate
    };

    if !root.join("Cargo.toml").is_file() {
        return Err(NekocodeError::Config(format!(
            "Rust workspace not found: {}",
            root.display()
        )));
    }
    Ok(root)
}

fn parse_package(value: &serde_json::Value) -> Result<RustPackage> {
    let name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| NekocodeError::External("cargo metadata package has no name".to_string()))?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| NekocodeError::External(format!("package {name} has no version")))?;
    let manifest_path = value
        .get("manifest_path")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| NekocodeError::External(format!("package {name} has no manifest_path")))?;
    let targets = value
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .map(|targets| {
            targets
                .iter()
                .filter_map(|target| target.get("name").and_then(serde_json::Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let mut features = value
        .get("features")
        .and_then(serde_json::Value::as_object)
        .map(|features| features.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    features.sort();

    Ok(RustPackage {
        name: name.to_string(),
        version: version.to_string(),
        manifest_path,
        targets,
        features,
    })
}

fn detect_toolchain() -> RustToolchainInfo {
    let rustc_output = Command::new("rustc").args(["-Vv"]).output().ok();
    let mut rustc_version = None;
    let mut host = None;
    if let Some(output) = rustc_output.filter(|output| output.status.success()) {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut lines = text.lines();
        rustc_version = lines.next().map(str::to_string);
        host = text
            .lines()
            .find_map(|line| line.strip_prefix("host: ").map(str::to_string));
    }

    RustToolchainInfo {
        rustc_version,
        cargo_version: command_version("cargo"),
        host,
    }
}

fn command_version(command: &str) -> Option<String> {
    let output = Command::new(command).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::to_string)
}

fn git_changed_files(root: &Path, compare_ref: &str) -> Result<Vec<ChangedRustFile>> {
    if compare_ref.trim().is_empty() {
        return Err(NekocodeError::Config(
            "compare ref must not be empty".to_string(),
        ));
    }

    let output = Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-status", &format!("{compare_ref}...HEAD")])
        .output()
        .map_err(|error| NekocodeError::External(format!("failed to run git diff: {error}")))?;

    if !output.status.success() {
        return Err(NekocodeError::External(format_command_failure(
            "git diff",
            &output.stderr,
        )));
    }

    parse_name_status(&String::from_utf8_lossy(&output.stdout))
}

fn parse_name_status(text: &str) -> Result<Vec<ChangedRustFile>> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut fields = line.split('\t');
            let status = fields.next().ok_or_else(|| {
                NekocodeError::External("malformed git name-status output".to_string())
            })?;
            let path = fields
                .next_back()
                .ok_or_else(|| NekocodeError::External("git change has no path".to_string()))?;
            Ok(ChangedRustFile {
                status: status.to_string(),
                path: PathBuf::from(path),
            })
        })
        .collect()
}

fn run_cargo_check(root: &Path) -> Result<RustDiagnosticRun> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args([
            "check",
            "--workspace",
            "--all-targets",
            "--message-format=json",
        ])
        .output()
        .map_err(|error| NekocodeError::External(format!("failed to run cargo check: {error}")))?;

    Ok(RustDiagnosticRun {
        command: "cargo check --workspace --all-targets --message-format=json".to_string(),
        status: if output.status.success() {
            "success".to_string()
        } else {
            "failed".to_string()
        },
        messages: parse_cargo_diagnostics(&String::from_utf8_lossy(&output.stdout)),
    })
}

fn parse_cargo_diagnostics(text: &str) -> Vec<RustDiagnostic> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| {
            value.get("reason").and_then(serde_json::Value::as_str) == Some("compiler-message")
        })
        .filter_map(|value| {
            let message = value.get("message")?;
            let level = message.get("level")?.as_str()?.to_string();
            let text = message.get("message")?.as_str()?.to_string();
            let code = message
                .get("code")
                .and_then(|code| code.get("code"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let span = message
                .get("spans")
                .and_then(serde_json::Value::as_array)
                .and_then(|spans| {
                    spans
                        .iter()
                        .find(|span| {
                            span.get("is_primary").and_then(serde_json::Value::as_bool)
                                == Some(true)
                        })
                        .or_else(|| spans.first())
                });
            let file = span
                .and_then(|span| span.get("file_name"))
                .and_then(serde_json::Value::as_str)
                .map(PathBuf::from);
            let line = span
                .and_then(|span| span.get("line_start"))
                .and_then(serde_json::Value::as_u64)
                .map(|line| line as u32);
            let column = span
                .and_then(|span| span.get("column_start"))
                .and_then(serde_json::Value::as_u64)
                .map(|column| column as u32);

            Some(RustDiagnostic {
                level,
                message: text,
                code,
                file,
                line,
                column,
            })
        })
        .collect()
}

fn format_command_failure(command: &str, stderr: &[u8]) -> String {
    let detail = String::from_utf8_lossy(stderr).trim().to_string();
    if detail.is_empty() {
        format!("{command} failed")
    } else {
        format!("{command} failed: {detail}")
    }
}

fn limitations(include_diagnostics: bool, budget_truncated: bool) -> Vec<String> {
    let mut limitations = Vec::new();
    if !include_diagnostics {
        limitations.push("Compiler diagnostics were not requested; use --diagnostics.".to_string());
    }
    limitations
        .push("Symbol references and public API impact require a semantic backend.".to_string());
    limitations
        .push("No breaking-change conclusion is emitted from this snapshot alone.".to_string());
    if budget_truncated {
        limitations
            .push("Some diagnostics or changed files were omitted to fit the budget.".to_string());
    }
    limitations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cargo_package_metadata() {
        let value = serde_json::json!({
            "name": "demo",
            "version": "0.1.0",
            "manifest_path": "/tmp/demo/Cargo.toml",
            "targets": [{"name": "demo", "kind": ["bin"]}],
            "features": {"default": [], "full": ["dep:full"]}
        });

        let package = parse_package(&value).expect("package should parse");
        assert_eq!(package.name, "demo");
        assert_eq!(package.targets, vec!["demo"]);
        assert_eq!(package.features, vec!["default", "full"]);
    }

    #[test]
    fn parses_git_name_status_and_uses_new_path_for_rename() {
        let files = parse_name_status("M\tsrc/lib.rs\nR100\tsrc/old.rs\tsrc/new.rs\n")
            .expect("name-status should parse");

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].status, "M");
        assert_eq!(files[1].status, "R100");
        assert_eq!(files[1].path, PathBuf::from("src/new.rs"));
    }

    #[test]
    fn rejects_empty_compare_ref() {
        let error = git_changed_files(Path::new("."), " ").expect_err("empty ref must fail");
        assert!(error.to_string().contains("compare ref"));
    }

    #[test]
    fn rejects_zero_budget() {
        let error = build_rust_context(".", None, 0).expect_err("zero budget must fail");
        assert!(error.to_string().contains("budget"));
    }

    #[test]
    fn parses_primary_cargo_diagnostic_span() {
        let text = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused function","code":{"code":"dead_code"},"spans":[{"file_name":"src/lib.rs","line_start":3,"column_start":5,"is_primary":true}]}}"#;
        let diagnostics = parse_cargo_diagnostics(text);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_deref(), Some("dead_code"));
        assert_eq!(diagnostics[0].file, Some(PathBuf::from("src/lib.rs")));
        assert_eq!(diagnostics[0].line, Some(3));
    }
}
