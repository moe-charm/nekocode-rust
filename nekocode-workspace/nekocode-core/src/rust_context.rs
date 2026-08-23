//! Rust-first, evidence-backed project context.
//!
//! This module deliberately does not reimplement Rust semantic analysis. Cargo
//! is the source of truth for workspace/package metadata; Git is the source of
//! truth for the requested change set. Later backends can enrich this snapshot
//! with rustc, Clippy, and rust-analyzer results without changing the JSON
//! contract established here.

use crate::error::{NekocodeError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;

const SCHEMA_VERSION: u32 = 3;
const MAX_SOURCE_EXCERPT_BYTES: usize = 32 * 1024;

/// Provenance for one external tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolProvenance {
    pub tool: String,
    pub command: String,
    pub cwd: PathBuf,
    pub version: Option<String>,
    pub exit_code: Option<i32>,
}

/// A stable digest of an input file that affects Rust build meaning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustInputDigest {
    pub path: PathBuf,
    pub sha256: String,
}

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

/// One Cargo target with enough information to explain workspace shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustTarget {
    pub name: String,
    pub kind: Vec<String>,
    pub src_path: Option<PathBuf>,
    pub edition: Option<String>,
    pub required_features: Vec<String>,
}

/// A Cargo package in the indexed workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustPackage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub manifest_path: PathBuf,
    pub targets: Vec<String>,
    pub target_details: Vec<RustTarget>,
    pub features: Vec<String>,
    pub dependencies: Vec<String>,
    pub edition: Option<String>,
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
    pub workspace_members: Vec<String>,
    pub inputs: Vec<RustInputDigest>,
    pub provenance: ToolProvenance,
}

/// A complete, explicit JSON snapshot that can be used as a later baseline.
///
/// The snapshot is deliberately a file supplied by the caller. NekoCode does
/// not maintain a hidden database or silently create history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustContextSnapshot {
    pub schema_version: u32,
    pub generated_at: String,
    pub workspace: RustWorkspaceSnapshot,
    pub diagnostics: Option<RustDiagnosticRun>,
}

/// A file reported by `git diff --name-status`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedRustFile {
    pub status: String,
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub is_rust: bool,
    pub package: Option<String>,
    pub hunks: Vec<RustDiffHunk>,
}

/// One unified-diff hunk on the old and new file sides.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub header: Option<String>,
}

/// Git provenance and bounded patch content for a context request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiffSummary {
    pub compare_ref: Option<String>,
    pub resolved_base: Option<String>,
    pub resolved_head: Option<String>,
    pub include_working_tree: bool,
    pub patch: String,
    pub patch_truncated: bool,
    pub omitted_patch_bytes: usize,
    pub provenance: Option<ToolProvenance>,
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
    pub rendered: Option<String>,
    pub package_id: Option<String>,
    pub target: Option<String>,
    pub spans: Vec<RustDiagnosticSpan>,
    #[serde(default)]
    pub fingerprint: String,
}

/// A source span from rustc's structured diagnostic payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiagnosticSpan {
    pub file: Option<PathBuf>,
    pub line_start: Option<u32>,
    pub column_start: Option<u32>,
    pub line_end: Option<u32>,
    pub column_end: Option<u32>,
    pub is_primary: bool,
    pub label: Option<String>,
}

/// Result of one `cargo check` invocation, including failed checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiagnosticRun {
    pub command: String,
    pub status: String,
    pub messages: Vec<RustDiagnostic>,
    pub stderr: Option<String>,
    pub all_targets: bool,
    pub all_features: bool,
    pub provenance: ToolProvenance,
}

/// Comparison of diagnostics from a saved snapshot and the current run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustDiagnosticDelta {
    pub baseline_path: PathBuf,
    pub compatible: bool,
    pub added: Vec<RustDiagnostic>,
    pub resolved: Vec<RustDiagnostic>,
    pub persisting: Vec<RustDiagnostic>,
    pub limitations: Vec<String>,
}

/// A bounded source excerpt adjacent to a Git diff hunk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustSourceExcerpt {
    pub path: PathBuf,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
    pub source: String,
    pub truncated: bool,
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
    pub diff: Option<RustDiffSummary>,
    pub source_excerpts: Vec<RustSourceExcerpt>,
    pub diagnostics: Option<RustDiagnosticRun>,
    pub baseline: Option<PathBuf>,
    pub diagnostic_delta: Option<RustDiagnosticDelta>,
    pub budget_tokens: usize,
    pub estimated_tokens: usize,
    pub serialized_bytes: usize,
    pub budget_exceeded: bool,
    pub include_working_tree: bool,
    pub all_features: bool,
    pub omitted_changed_files: usize,
    pub omitted_excerpts: usize,
    pub omitted_diagnostics: usize,
    pub omitted_delta_items: usize,
    pub omitted_diff_bytes: usize,
    pub truncation_order: Vec<String>,
    pub limitations: Vec<String>,
}

/// Options for building a reproducible, bounded context pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustContextOptions {
    pub compare_ref: Option<String>,
    pub budget_tokens: usize,
    pub include_diagnostics: bool,
    pub include_working_tree: bool,
    pub all_features: bool,
    pub include_diff: bool,
    pub excerpt_lines: usize,
    pub baseline: Option<PathBuf>,
}

impl RustContextOptions {
    pub fn new(compare_ref: Option<String>, budget_tokens: usize) -> Self {
        Self {
            include_diff: compare_ref.is_some(),
            compare_ref,
            budget_tokens,
            include_diagnostics: false,
            include_working_tree: false,
            all_features: false,
            excerpt_lines: 8,
            baseline: None,
        }
    }
}

/// Build a complete explicit JSON snapshot for a Rust workspace.
pub fn build_rust_snapshot(
    path: impl AsRef<Path>,
    include_diagnostics: bool,
    all_features: bool,
) -> Result<RustContextSnapshot> {
    let workspace = index_rust_workspace(path)?;
    let diagnostics = if include_diagnostics {
        Some(run_cargo_check_with_options(&workspace.root, all_features)?)
    } else {
        None
    };
    Ok(RustContextSnapshot {
        schema_version: SCHEMA_VERSION,
        generated_at: chrono::Utc::now().to_rfc3339(),
        workspace,
        diagnostics,
    })
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
    if snapshot.schema_version != SCHEMA_VERSION {
        return Err(NekocodeError::Config(format!(
            "unsupported Rust snapshot schema version {} (expected {})",
            snapshot.schema_version, SCHEMA_VERSION
        )));
    }
    Ok(snapshot)
}

/// Read Cargo workspace metadata without attempting to parse Rust semantics.
pub fn index_rust_workspace(path: impl AsRef<Path>) -> Result<RustWorkspaceSnapshot> {
    let root = normalize_workspace_root(path.as_ref())?;
    let command = "cargo metadata --format-version=1 --no-deps".to_string();
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
    let workspace_members = metadata
        .get("workspace_members")
        .and_then(serde_json::Value::as_array)
        .map(|members| {
            members
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let inputs = collect_input_digests(&root, &packages);

    Ok(RustWorkspaceSnapshot {
        schema_version: SCHEMA_VERSION,
        evidence: EvidenceLevel::ToolConfirmed,
        root: root.clone(),
        workspace_root,
        toolchain,
        packages,
        workspace_members,
        inputs,
        provenance: ToolProvenance {
            tool: "cargo metadata".to_string(),
            command,
            cwd: root,
            version: command_version("cargo"),
            exit_code: Some(0),
        },
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
    let mut options = RustContextOptions::new(compare_ref.map(str::to_string), budget_tokens);
    options.include_diagnostics = include_diagnostics;
    build_rust_context_with_config(path, options)
}

/// Build a context pack with explicit working-tree, feature, and diff options.
pub fn build_rust_context_with_config(
    path: impl AsRef<Path>,
    options: RustContextOptions,
) -> Result<RustContextPack> {
    if options.budget_tokens == 0 {
        return Err(NekocodeError::Config(
            "context budget must be greater than zero".to_string(),
        ));
    }

    let workspace = index_rust_workspace(path)?;
    let wants_git =
        options.include_diff || options.compare_ref.is_some() || options.include_working_tree;
    let include_patch = options.include_diff
        || (options.excerpt_lines > 0
            && (options.compare_ref.is_some() || options.include_working_tree));
    let (mut changed_files, diff) = if wants_git {
        let (files, diff) = git_context(
            &workspace.root,
            options.compare_ref.as_deref(),
            options.include_working_tree,
            include_patch,
        )?;
        (files, Some(diff))
    } else {
        (Vec::new(), None)
    };
    annotate_changed_files(&mut changed_files, &workspace.root, &workspace.packages);
    let source_excerpts =
        build_source_excerpts(&workspace.root, &changed_files, options.excerpt_lines);
    let diagnostics = if options.include_diagnostics {
        Some(run_cargo_check_with_options(
            &workspace.root,
            options.all_features,
        )?)
    } else {
        None
    };
    let mut extra_limitations = Vec::new();
    let diagnostic_delta = match options.baseline.as_ref() {
        None => None,
        Some(baseline_path) => match diagnostics.as_ref() {
            None => {
                extra_limitations.push(
                    "Diagnostic baseline was supplied without --diagnostics; no delta was computed."
                        .to_string(),
                );
                None
            }
            Some(current_run) => match read_rust_snapshot(baseline_path) {
                Ok(baseline) => {
                    let baseline_run = baseline.diagnostics.as_ref();
                    if baseline_run.is_none() {
                        extra_limitations.push(
                            "Diagnostic baseline does not contain a saved cargo check run; no delta was computed."
                                .to_string(),
                        );
                    }
                    let delta = build_diagnostic_delta(
                        baseline_path,
                        &baseline.workspace,
                        baseline_run,
                        &workspace,
                        current_run,
                    );
                    extra_limitations.extend(delta.limitations.iter().cloned());
                    Some(delta)
                }
                Err(error) => {
                    extra_limitations.push(format!(
                        "Diagnostic baseline could not be read; no delta was computed: {error}"
                    ));
                    None
                }
            },
        },
    };
    if let Some(run) = diagnostics.as_ref().filter(|run| run.status != "success") {
        extra_limitations.push(format!(
            "cargo check did not succeed (status: {}); diagnostic delta is incomplete.",
            run.status
        ));
    }

    // Keep the pack bounded even before semantic symbol data is added. The
    // estimate is intentionally conservative: JSON is usually a few bytes per
    // token, and the caller can request a larger budget when needed.
    let byte_budget = options.budget_tokens.saturating_mul(4);
    let mut pack = RustContextPack {
        schema_version: SCHEMA_VERSION,
        evidence: EvidenceLevel::ToolConfirmed,
        root: workspace.root.clone(),
        workspace,
        compare_ref: options.compare_ref.clone(),
        changed_files,
        diff,
        source_excerpts,
        diagnostics,
        baseline: options.baseline.clone(),
        diagnostic_delta,
        budget_tokens: options.budget_tokens,
        estimated_tokens: 0,
        serialized_bytes: 0,
        budget_exceeded: false,
        include_working_tree: options.include_working_tree,
        all_features: options.all_features,
        omitted_changed_files: 0,
        omitted_excerpts: 0,
        omitted_diagnostics: 0,
        omitted_delta_items: 0,
        omitted_diff_bytes: 0,
        truncation_order: vec![
            "diff.patch".to_string(),
            "source_excerpts".to_string(),
            "diagnostic_delta".to_string(),
            "diagnostics.messages".to_string(),
            "changed_files".to_string(),
        ],
        limitations: limitations(
            options.include_diagnostics,
            options.include_working_tree,
            &extra_limitations,
            false,
        ),
    };

    // Patch text is the largest and least structured field. Keep a bounded
    // prefix before trimming individual diagnostics/files so the result stays
    // useful for AI/PR consumers.
    if let Some(diff) = pack.diff.as_mut() {
        let patch_budget = byte_budget / 2;
        if diff.patch.len() > patch_budget {
            let omitted = truncate_utf8(&mut diff.patch, patch_budget);
            diff.patch.push_str("\n... [diff truncated]\n");
            diff.patch_truncated = true;
            diff.omitted_patch_bytes = omitted;
            pack.omitted_diff_bytes = omitted;
        }
    }

    // Trim the least stable, most verbose parts first so the advertised budget
    // remains useful even when cargo emits hundreds of warnings. Workspace
    // metadata is retained as the structural baseline whenever possible.
    while serialized_size(&pack)? > byte_budget {
        if let Some(diff) = pack.diff.as_mut() {
            if !diff.patch.is_empty() {
                let old_len = diff.patch.len();
                let new_len = if old_len > 64 { old_len / 2 } else { 0 };
                if new_len == 0 {
                    diff.patch.clear();
                    let omitted = old_len;
                    diff.patch_truncated = true;
                    diff.omitted_patch_bytes += omitted;
                    pack.omitted_diff_bytes += omitted;
                    continue;
                } else {
                    let omitted = truncate_utf8(&mut diff.patch, new_len);
                    diff.patch.push_str("\n... [diff truncated]\n");
                    diff.patch_truncated = true;
                    diff.omitted_patch_bytes += omitted;
                    pack.omitted_diff_bytes += omitted;
                    continue;
                }
            }
        }
        if pack.source_excerpts.pop().is_some() {
            pack.omitted_excerpts += 1;
            continue;
        }
        if let Some(delta) = pack.diagnostic_delta.as_mut() {
            if pop_diagnostic_delta_item(delta) {
                pack.omitted_delta_items += 1;
                continue;
            }
        }
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

    pack.serialized_bytes = serialized_size(&pack)?;
    pack.estimated_tokens = pack.serialized_bytes.div_ceil(4);
    pack.budget_exceeded = pack.serialized_bytes > byte_budget;
    let diagnostic_failed = pack
        .diagnostics
        .as_ref()
        .is_some_and(|run| run.status != "success");
    let baseline_incomplete = pack
        .diagnostic_delta
        .as_ref()
        .is_some_and(|delta| !delta.compatible)
        || !extra_limitations.is_empty();
    if pack.omitted_changed_files > 0
        || pack.omitted_excerpts > 0
        || pack.omitted_diagnostics > 0
        || pack.omitted_delta_items > 0
        || pack.omitted_diff_bytes > 0
        || pack.budget_exceeded
        || diagnostic_failed
        || baseline_incomplete
    {
        pack.evidence = EvidenceLevel::Incomplete;
    }
    pack.limitations = limitations(
        options.include_diagnostics,
        options.include_working_tree,
        &extra_limitations,
        pack.omitted_changed_files > 0
            || pack.omitted_excerpts > 0
            || pack.omitted_diagnostics > 0
            || pack.omitted_delta_items > 0
            || pack.omitted_diff_bytes > 0
            || pack.budget_exceeded,
    );
    Ok(pack)
}

fn build_source_excerpts(
    root: &Path,
    changed_files: &[ChangedRustFile],
    context_lines: usize,
) -> Vec<RustSourceExcerpt> {
    if changed_files.is_empty() {
        return Vec::new();
    }

    let mut ranges: BTreeMap<PathBuf, Vec<(u32, u32)>> = BTreeMap::new();
    for file in changed_files.iter().filter(|file| file.is_rust) {
        if file.hunks.is_empty() {
            continue;
        }
        let path = root.join(&file.path);
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let line_count = content.lines().count() as u32;
        if line_count == 0 {
            continue;
        }
        let file_ranges = ranges.entry(file.path.clone()).or_default();
        for hunk in &file.hunks {
            let changed_start = hunk.new_start.max(1);
            let changed_end = if hunk.new_count == 0 {
                changed_start
            } else {
                changed_start.saturating_add(hunk.new_count.saturating_sub(1))
            };
            let start = changed_start.saturating_sub(context_lines as u32).max(1);
            let end = changed_end
                .saturating_add(context_lines as u32)
                .min(line_count);
            if start <= end {
                file_ranges.push((start, end));
            }
        }
    }

    let mut excerpts = Vec::new();
    for (path, mut file_ranges) in ranges {
        file_ranges.sort_unstable();
        let mut merged: Vec<(u32, u32)> = Vec::new();
        for (start, end) in file_ranges {
            if let Some(last) = merged.last_mut() {
                if start <= last.1.saturating_add(1) {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            merged.push((start, end));
        }

        let Ok(content) = std::fs::read_to_string(root.join(&path)) else {
            continue;
        };
        let lines = content.lines().collect::<Vec<_>>();
        for (start, end) in merged {
            let mut excerpt = lines[(start as usize - 1)..(end as usize)].join("\n");
            let mut truncated = false;
            if truncate_utf8(&mut excerpt, MAX_SOURCE_EXCERPT_BYTES) > 0 {
                truncated = true;
                excerpt.push_str("\n... [source excerpt truncated]");
            }
            excerpts.push(RustSourceExcerpt {
                path: path.clone(),
                start_line: start,
                end_line: end,
                content: excerpt,
                source: "git-diff-hunk".to_string(),
                truncated,
            });
        }
    }
    excerpts
}

fn diagnostic_fingerprint(diagnostic: &RustDiagnostic) -> String {
    let file = diagnostic
        .file
        .as_ref()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let message = diagnostic
        .message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let identity = format!(
        "{}|{}|{}|{}",
        diagnostic.code.as_deref().unwrap_or(&diagnostic.level),
        file,
        diagnostic.line.unwrap_or_default(),
        message
    );
    let mut hasher = Sha256::new();
    hasher.update(identity.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn diagnostic_with_fingerprint(mut diagnostic: RustDiagnostic) -> RustDiagnostic {
    if diagnostic.fingerprint.is_empty() {
        diagnostic.fingerprint = diagnostic_fingerprint(&diagnostic);
    }
    diagnostic
}

fn diagnostic_map(diagnostics: &[RustDiagnostic]) -> BTreeMap<String, RustDiagnostic> {
    diagnostics
        .iter()
        .cloned()
        .map(diagnostic_with_fingerprint)
        .map(|diagnostic| (diagnostic.fingerprint.clone(), diagnostic))
        .collect()
}

fn build_diagnostic_delta(
    baseline_path: &Path,
    baseline_workspace: &RustWorkspaceSnapshot,
    baseline_run: Option<&RustDiagnosticRun>,
    current_workspace: &RustWorkspaceSnapshot,
    current_run: &RustDiagnosticRun,
) -> RustDiagnosticDelta {
    let mut delta = RustDiagnosticDelta {
        baseline_path: baseline_path.to_path_buf(),
        compatible: true,
        added: Vec::new(),
        resolved: Vec::new(),
        persisting: Vec::new(),
        limitations: Vec::new(),
    };

    let Some(baseline_run) = baseline_run else {
        delta.compatible = false;
        delta
            .limitations
            .push("baseline snapshot has no cargo check diagnostics".to_string());
        return delta;
    };

    if baseline_workspace.toolchain != current_workspace.toolchain {
        delta.compatible = false;
        delta
            .limitations
            .push("baseline and current Rust toolchains differ".to_string());
    }
    if baseline_run.all_targets != current_run.all_targets {
        delta.compatible = false;
        delta
            .limitations
            .push("baseline and current target coverage differ".to_string());
    }
    if baseline_run.all_features != current_run.all_features {
        delta.compatible = false;
        delta
            .limitations
            .push("baseline and current feature coverage differ".to_string());
    }
    if baseline_run.provenance.tool != current_run.provenance.tool
        || baseline_run.provenance.version != current_run.provenance.version
    {
        delta.compatible = false;
        delta
            .limitations
            .push("baseline and current diagnostic tool provenance differs".to_string());
    }
    if baseline_run.status != "success" || current_run.status != "success" {
        delta.compatible = false;
        delta
            .limitations
            .push("diagnostic delta requires successful baseline and current checks".to_string());
    }

    if !delta.compatible {
        return delta;
    }

    let baseline = diagnostic_map(&baseline_run.messages);
    let current = diagnostic_map(&current_run.messages);
    for (fingerprint, diagnostic) in &current {
        if baseline.contains_key(fingerprint) {
            delta.persisting.push(diagnostic.clone());
        } else {
            delta.added.push(diagnostic.clone());
        }
    }
    for (fingerprint, diagnostic) in &baseline {
        if !current.contains_key(fingerprint) {
            delta.resolved.push(diagnostic.clone());
        }
    }
    delta
}

fn pop_diagnostic_delta_item(delta: &mut RustDiagnosticDelta) -> bool {
    delta
        .added
        .pop()
        .or_else(|| delta.resolved.pop())
        .or_else(|| delta.persisting.pop())
        .is_some()
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

fn collect_input_digests(root: &Path, packages: &[RustPackage]) -> Vec<RustInputDigest> {
    let mut paths = vec![root.join("Cargo.toml")];
    paths.extend(packages.iter().map(|package| package.manifest_path.clone()));
    for name in ["Cargo.lock", "rust-toolchain", "rust-toolchain.toml"] {
        let path = root.join(name);
        if path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .filter_map(|path| {
            let bytes = std::fs::read(&path).ok()?;
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            let digest = format!("{:x}", hasher.finalize());
            Some(RustInputDigest {
                path,
                sha256: digest,
            })
        })
        .collect()
}

fn annotate_changed_files(files: &mut [ChangedRustFile], root: &Path, packages: &[RustPackage]) {
    for file in files {
        file.is_rust = is_rust_path(&file.path);
        file.package = packages
            .iter()
            .filter_map(|package| {
                let package_root = Path::new(&package.manifest_path).parent()?;
                let candidate = root.join(&file.path).canonicalize().ok()?;
                let package_root = package_root.canonicalize().ok()?;
                candidate
                    .starts_with(&package_root)
                    .then(|| package.name.clone())
            })
            .next();
    }
}

fn parse_package(value: &serde_json::Value) -> Result<RustPackage> {
    let id = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
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
    let target_details: Vec<RustTarget> = value
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .map(|targets| {
            targets
                .iter()
                .filter_map(|target| {
                    let name = target.get("name")?.as_str()?.to_string();
                    let kind = target
                        .get("kind")
                        .and_then(serde_json::Value::as_array)
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let src_path = target
                        .get("src_path")
                        .and_then(serde_json::Value::as_str)
                        .map(PathBuf::from);
                    let edition = target
                        .get("edition")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string);
                    let required_features = target
                        .get("required-features")
                        .and_then(serde_json::Value::as_array)
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    Some(RustTarget {
                        name,
                        kind,
                        src_path,
                        edition,
                        required_features,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let targets = target_details
        .iter()
        .map(|target| target.name.clone())
        .collect::<Vec<_>>();
    let mut features = value
        .get("features")
        .and_then(serde_json::Value::as_object)
        .map(|features| features.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    features.sort();
    let mut dependencies = value
        .get("dependencies")
        .and_then(serde_json::Value::as_array)
        .map(|dependencies| {
            dependencies
                .iter()
                .filter_map(|dependency| dependency.get("name").and_then(serde_json::Value::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    dependencies.sort();
    dependencies.dedup();
    let edition = value
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .and_then(|targets| targets.first())
        .and_then(|target| target.get("edition"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);

    Ok(RustPackage {
        id,
        name: name.to_string(),
        version: version.to_string(),
        manifest_path,
        targets,
        target_details,
        features,
        dependencies,
        edition,
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

#[cfg(test)]
fn git_changed_files(root: &Path, compare_ref: &str) -> Result<Vec<ChangedRustFile>> {
    let (files, _) = git_context(root, Some(compare_ref), false, false)?;
    Ok(files)
}

fn parse_name_status(text: &str) -> Result<Vec<ChangedRustFile>> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            let status = fields.first().copied().ok_or_else(|| {
                NekocodeError::External("malformed git name-status output".to_string())
            })?;
            let path = fields
                .last()
                .ok_or_else(|| NekocodeError::External("git change has no path".to_string()))?;
            Ok(ChangedRustFile {
                status: status.to_string(),
                path: PathBuf::from(path),
                old_path: if (status.starts_with('R') || status.starts_with('C'))
                    && fields.len() >= 3
                {
                    Some(PathBuf::from(fields[1]))
                } else {
                    None
                },
                is_rust: is_rust_path(Path::new(path)),
                package: None,
                hunks: Vec::new(),
            })
        })
        .collect()
}

fn git_context(
    root: &Path,
    compare_ref: Option<&str>,
    include_working_tree: bool,
    include_patch: bool,
) -> Result<(Vec<ChangedRustFile>, RustDiffSummary)> {
    if compare_ref.is_some_and(|reference| reference.trim().is_empty()) {
        return Err(NekocodeError::Config(
            "compare ref must not be empty".to_string(),
        ));
    }

    let mut changed_files = Vec::new();
    let mut patch_parts = Vec::new();
    let mut commands = Vec::new();

    if let Some(reference) = compare_ref {
        let spec = format!("{reference}...HEAD");
        let args = vec![
            "diff".to_string(),
            "--relative".to_string(),
            "--name-status".to_string(),
            "--no-ext-diff".to_string(),
            spec.clone(),
        ];
        changed_files.extend(parse_name_status(&run_git(root, &args, "git diff")?)?);
        commands.push(format!("git {}", args.join(" ")));
        if include_patch {
            let patch_args = vec![
                "diff".to_string(),
                "--relative".to_string(),
                "--no-ext-diff".to_string(),
                "--unified=3".to_string(),
                spec,
            ];
            patch_parts.push(run_git(root, &patch_args, "git diff")?);
            commands.push(format!("git {}", patch_args.join(" ")));
        }
    }

    if include_working_tree {
        let base = compare_ref.unwrap_or("HEAD");
        let args = vec![
            "diff".to_string(),
            "--relative".to_string(),
            "--name-status".to_string(),
            "--no-ext-diff".to_string(),
            base.to_string(),
        ];
        changed_files.extend(parse_name_status(&run_git(root, &args, "git diff")?)?);
        commands.push(format!("git {}", args.join(" ")));
        if include_patch {
            let patch_args = vec![
                "diff".to_string(),
                "--relative".to_string(),
                "--no-ext-diff".to_string(),
                "--unified=3".to_string(),
                base.to_string(),
            ];
            patch_parts.push(run_git(root, &patch_args, "git diff")?);
            commands.push(format!("git {}", patch_args.join(" ")));
        }
        let untracked = git_untracked_files(root)?;
        for path in &untracked {
            changed_files.push(ChangedRustFile {
                status: "??".to_string(),
                path: path.clone(),
                old_path: None,
                is_rust: is_rust_path(path),
                package: None,
                hunks: Vec::new(),
            });
            if include_patch {
                patch_parts.push(untracked_patch(root, path));
            }
        }
    }

    changed_files = merge_changed_files(changed_files);
    let patch = patch_parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let hunks = parse_unified_hunks(&patch);
    for file in &mut changed_files {
        if let Some(file_hunks) = hunks.get(&file.path) {
            file.hunks = file_hunks.clone();
        }
    }

    let summary = RustDiffSummary {
        compare_ref: compare_ref.map(str::to_string),
        resolved_base: compare_ref.and_then(|reference| git_rev_parse(root, reference)),
        resolved_head: git_rev_parse(root, "HEAD"),
        include_working_tree,
        patch,
        patch_truncated: false,
        omitted_patch_bytes: 0,
        provenance: Some(ToolProvenance {
            tool: "git diff".to_string(),
            command: if commands.is_empty() {
                "git diff".to_string()
            } else {
                commands.join(" && ")
            },
            cwd: root.to_path_buf(),
            version: command_version("git"),
            exit_code: Some(0),
        }),
    };
    Ok((changed_files, summary))
}

fn run_git(root: &Path, args: &[String], label: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|error| NekocodeError::External(format!("failed to run {label}: {error}")))?;
    if !output.status.success() {
        return Err(NekocodeError::External(format_command_failure(
            label,
            &output.stderr,
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn git_rev_parse(root: &Path, reference: &str) -> Option<String> {
    let args = vec!["rev-parse".to_string(), reference.to_string()];
    run_git(root, &args, "git rev-parse")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn git_untracked_files(root: &Path) -> Result<Vec<PathBuf>> {
    let args = vec![
        "ls-files".to_string(),
        "--others".to_string(),
        "--exclude-standard".to_string(),
    ];
    Ok(run_git(root, &args, "git ls-files")?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(PathBuf::from)
        .collect())
}

fn merge_changed_files(files: Vec<ChangedRustFile>) -> Vec<ChangedRustFile> {
    let mut merged = Vec::new();
    for file in files {
        if let Some(existing) = merged
            .iter_mut()
            .find(|item: &&mut ChangedRustFile| item.path == file.path)
        {
            existing.status = file.status;
            if file.old_path.is_some() {
                existing.old_path = file.old_path;
            }
            continue;
        }
        merged.push(file);
    }
    merged
}

fn parse_unified_hunks(text: &str) -> HashMap<PathBuf, Vec<RustDiffHunk>> {
    let mut result: HashMap<PathBuf, Vec<RustDiffHunk>> = HashMap::new();
    let mut current_path: Option<PathBuf> = None;
    for line in text.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_path = Some(PathBuf::from(path));
            continue;
        }
        if !line.starts_with("@@") {
            continue;
        }
        let Some(hunk) = parse_hunk_header(line) else {
            continue;
        };
        if let Some(path) = current_path.clone() {
            result.entry(path).or_default().push(hunk);
        }
    }
    result
}

fn parse_hunk_header(line: &str) -> Option<RustDiffHunk> {
    let body = line.strip_prefix("@@")?.strip_prefix(' ')?;
    let end = body.find(" @@")?;
    let ranges = &body[..end];
    let mut parts = ranges.split_whitespace();
    let old = parts.next()?.strip_prefix('-')?;
    let new = parts.next()?.strip_prefix('+')?;
    let (old_start, old_count) = parse_diff_range(old)?;
    let (new_start, new_count) = parse_diff_range(new)?;
    let header = body[end + 3..].trim();
    Some(RustDiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        header: (!header.is_empty()).then(|| header.to_string()),
    })
}

fn parse_diff_range(value: &str) -> Option<(u32, u32)> {
    let mut parts = value.split(',');
    let start = parts.next()?.parse().ok()?;
    let count = parts.next().map_or(Some(1), |value| value.parse().ok())?;
    Some((start, count))
}

fn untracked_patch(root: &Path, path: &Path) -> String {
    let Ok(content) = std::fs::read_to_string(root.join(path)) else {
        return String::new();
    };
    let line_count = content.lines().count().max(1);
    let mut patch = format!(
        "diff --git a/{0} b/{0}\nnew file mode 100644\n--- /dev/null\n+++ b/{0}\n@@ -0,0 +1,{1} @@\n",
        path.display(),
        line_count
    );
    for line in content.lines() {
        patch.push('+');
        patch.push_str(line);
        patch.push('\n');
    }
    patch
}

fn is_rust_path(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("rs")
}

fn run_cargo_check_with_options(root: &Path, all_features: bool) -> Result<RustDiagnosticRun> {
    let mut args = vec![
        "check".to_string(),
        "--workspace".to_string(),
        "--all-targets".to_string(),
    ];
    if all_features {
        args.push("--all-features".to_string());
    }
    args.push("--message-format=json".to_string());
    let command = format!("cargo {}", args.join(" "));
    let output = Command::new("cargo")
        .current_dir(root)
        .args(&args)
        .output()
        .map_err(|error| NekocodeError::External(format!("failed to run cargo check: {error}")))?;

    Ok(RustDiagnosticRun {
        command: command.clone(),
        status: if output.status.success() {
            "success".to_string()
        } else {
            "failed".to_string()
        },
        messages: parse_cargo_diagnostics_with_root(
            &String::from_utf8_lossy(&output.stdout),
            Some(root),
        ),
        stderr: non_empty_text(&output.stderr),
        all_targets: true,
        all_features,
        provenance: ToolProvenance {
            tool: "cargo check".to_string(),
            command,
            cwd: root.to_path_buf(),
            version: command_version("cargo"),
            exit_code: output.status.code(),
        },
    })
}

#[cfg(test)]
fn parse_cargo_diagnostics(text: &str) -> Vec<RustDiagnostic> {
    parse_cargo_diagnostics_with_root(text, None)
}

fn parse_cargo_diagnostics_with_root(text: &str, root: Option<&Path>) -> Vec<RustDiagnostic> {
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
                .map(|file| normalize_reported_path(root, file));
            let line = span
                .and_then(|span| span.get("line_start"))
                .and_then(serde_json::Value::as_u64)
                .map(|line| line as u32);
            let column = span
                .and_then(|span| span.get("column_start"))
                .and_then(serde_json::Value::as_u64)
                .map(|column| column as u32);

            let spans = message
                .get("spans")
                .and_then(serde_json::Value::as_array)
                .map(|spans| {
                    spans
                        .iter()
                        .map(|span| RustDiagnosticSpan {
                            file: span
                                .get("file_name")
                                .and_then(serde_json::Value::as_str)
                                .map(|file| normalize_reported_path(root, file)),
                            line_start: span
                                .get("line_start")
                                .and_then(serde_json::Value::as_u64)
                                .map(|value| value as u32),
                            column_start: span
                                .get("column_start")
                                .and_then(serde_json::Value::as_u64)
                                .map(|value| value as u32),
                            line_end: span
                                .get("line_end")
                                .and_then(serde_json::Value::as_u64)
                                .map(|value| value as u32),
                            column_end: span
                                .get("column_end")
                                .and_then(serde_json::Value::as_u64)
                                .map(|value| value as u32),
                            is_primary: span
                                .get("is_primary")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(false),
                            label: span
                                .get("label")
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_string),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            Some(diagnostic_with_fingerprint(RustDiagnostic {
                level,
                message: text,
                code,
                file,
                line,
                column,
                rendered: message
                    .get("rendered")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                package_id: value
                    .get("package_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                target: value
                    .get("target")
                    .and_then(|target| target.get("name"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                spans,
                fingerprint: String::new(),
            }))
        })
        .collect()
}

fn normalize_reported_path(root: Option<&Path>, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if let Some(root) = root {
        if let Ok(relative) = path.strip_prefix(root) {
            return relative.to_path_buf();
        }
    }
    path
}

fn non_empty_text(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes).trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn format_command_failure(command: &str, stderr: &[u8]) -> String {
    let detail = String::from_utf8_lossy(stderr).trim().to_string();
    if detail.is_empty() {
        format!("{command} failed")
    } else {
        format!("{command} failed: {detail}")
    }
}

fn serialized_size(pack: &RustContextPack) -> Result<usize> {
    Ok(serde_json::to_vec(pack)?.len())
}

fn truncate_utf8(value: &mut String, max_bytes: usize) -> usize {
    if value.len() <= max_bytes {
        return 0;
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    let omitted = value.len() - end;
    value.truncate(end);
    omitted
}

fn limitations(
    include_diagnostics: bool,
    include_working_tree: bool,
    extra: &[String],
    budget_truncated: bool,
) -> Vec<String> {
    let mut limitations = Vec::new();
    if !include_diagnostics {
        limitations.push("Compiler diagnostics were not requested; use --diagnostics.".to_string());
    }
    if !include_working_tree {
        limitations.push(
            "Uncommitted working-tree and untracked files were not requested; use --working-tree."
                .to_string(),
        );
    }
    limitations
        .push("Symbol references and public API impact require a semantic backend.".to_string());
    limitations
        .push("No breaking-change conclusion is emitted from this snapshot alone.".to_string());
    limitations.extend(extra.iter().cloned());
    if budget_truncated {
        limitations.push(
            "Some diff, excerpts, diagnostics, delta items, or changed files were omitted to fit the budget."
                .to_string(),
        );
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
        assert_eq!(files[1].old_path, Some(PathBuf::from("src/old.rs")));
        assert!(files[1].is_rust);
    }

    #[test]
    fn parses_unified_diff_hunks() {
        let hunks = parse_unified_hunks(
            "diff --git a/src/lib.rs b/src/lib.rs\n+++ b/src/lib.rs\n@@ -12,2 +13,4 @@ fn demo\n",
        );
        let hunk = &hunks[&PathBuf::from("src/lib.rs")][0];
        assert_eq!(hunk.old_start, 12);
        assert_eq!(hunk.old_count, 2);
        assert_eq!(hunk.new_start, 13);
        assert_eq!(hunk.new_count, 4);
        assert_eq!(hunk.header.as_deref(), Some("fn demo"));
    }

    #[test]
    fn truncates_utf8_without_splitting_a_character() {
        let mut text = "変更されたRustコード".to_string();
        let omitted = truncate_utf8(&mut text, 5);
        assert!(text.is_char_boundary(text.len()));
        assert!(omitted > 0);
        assert!(text.len() <= 5);
    }

    #[test]
    fn parses_target_and_dependency_metadata() {
        let value = serde_json::json!({
            "id": "demo 0.1.0 (path+file:///tmp/demo)",
            "name": "demo",
            "version": "0.1.0",
            "manifest_path": "/tmp/demo/Cargo.toml",
            "targets": [{
                "name": "demo",
                "kind": ["lib"],
                "src_path": "/tmp/demo/src/lib.rs",
                "edition": "2021",
                "required-features": ["full"]
            }],
            "features": {"full": ["dep:full"]},
            "dependencies": [{"name": "serde"}, {"name": "serde"}]
        });

        let package = parse_package(&value).expect("package should parse");
        assert_eq!(package.id, "demo 0.1.0 (path+file:///tmp/demo)");
        assert_eq!(package.edition.as_deref(), Some("2021"));
        assert_eq!(package.dependencies, vec!["serde"]);
        assert_eq!(package.target_details[0].kind, vec!["lib"]);
        assert_eq!(package.target_details[0].required_features, vec!["full"]);
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
        assert_eq!(diagnostics[0].spans.len(), 1);
        assert!(diagnostics[0].spans[0].is_primary);
    }
}
