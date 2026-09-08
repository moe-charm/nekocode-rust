//! Bounded Git change scopes, line metrics, hunks, and path handling.

use super::execution;
use super::workspace;
use super::*;
use execution::*;
use workspace::*;

pub(crate) fn parse_name_status_z(
    bytes: &[u8],
    scope: GitChangeScope,
) -> Result<Vec<ChangedRustFile>> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if !bytes.ends_with(&[0]) {
        return Err(NekocodeError::External(
            "malformed NUL-delimited git name-status output".to_string(),
        ));
    }

    let mut fields = bytes[..bytes.len() - 1].split(|byte| *byte == 0);
    let mut files = Vec::new();
    while let Some(status_bytes) = fields.next() {
        let status = std::str::from_utf8(status_bytes).map_err(|_| {
            NekocodeError::External("git returned a non-UTF-8 change status".to_string())
        })?;
        if status.is_empty() {
            return Err(NekocodeError::External(
                "git returned an empty change status".to_string(),
            ));
        }

        let renamed = status.starts_with('R') || status.starts_with('C');
        let first_path = fields
            .next()
            .ok_or_else(|| NekocodeError::External("git change has no path".to_string()))?;
        let (old_path, path) = if renamed {
            let new_path = fields.next().ok_or_else(|| {
                NekocodeError::External("git rename or copy has no destination path".to_string())
            })?;
            (
                Some(git_path_from_bytes(first_path)?),
                git_path_from_bytes(new_path)?,
            )
        } else {
            (None, git_path_from_bytes(first_path)?)
        };

        files.push(ChangedRustFile {
            status: status.to_string(),
            is_rust: is_rust_path(&path),
            path,
            old_path,
            package: None,
            hunks: Vec::new(),
            scope_changes: vec![RustFileScopeChange {
                scope,
                status: status.to_string(),
                additions: None,
                deletions: None,
                line_count_status: LineCountStatus::NotRead,
            }],
        });
    }
    Ok(files)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitNumstatRecord {
    pub(crate) path: PathBuf,
    pub(crate) old_path: Option<PathBuf>,
    pub(crate) additions: Option<usize>,
    pub(crate) deletions: Option<usize>,
    pub(crate) line_count_status: LineCountStatus,
}

pub(crate) fn parse_numstat_z(bytes: &[u8]) -> Result<Vec<GitNumstatRecord>> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if !bytes.ends_with(&[0]) {
        return Err(NekocodeError::External(
            "malformed NUL-delimited git numstat output".to_string(),
        ));
    }

    let mut fields = bytes[..bytes.len() - 1].split(|byte| *byte == 0);
    let mut records = Vec::new();
    while let Some(header) = fields.next() {
        let mut columns = header.splitn(3, |byte| *byte == b'\t');
        let additions = columns.next().ok_or_else(|| {
            NekocodeError::External("git numstat entry has no additions column".to_string())
        })?;
        let deletions = columns.next().ok_or_else(|| {
            NekocodeError::External("git numstat entry has no deletions column".to_string())
        })?;
        let path_field = columns.next().ok_or_else(|| {
            NekocodeError::External("git numstat entry has no path column".to_string())
        })?;

        let (additions, deletions, line_count_status) = match (additions, deletions) {
            (b"-", b"-") => (None, None, LineCountStatus::Binary),
            (additions, deletions) => {
                let additions = parse_numstat_count(additions, "additions")?;
                let deletions = parse_numstat_count(deletions, "deletions")?;
                (Some(additions), Some(deletions), LineCountStatus::Counted)
            }
        };

        let (old_path, path) = if path_field.is_empty() {
            let old_path = fields.next().ok_or_else(|| {
                NekocodeError::External("git numstat rename or copy has no source path".to_string())
            })?;
            let path = fields.next().ok_or_else(|| {
                NekocodeError::External(
                    "git numstat rename or copy has no destination path".to_string(),
                )
            })?;
            (
                Some(git_path_from_bytes(old_path)?),
                git_path_from_bytes(path)?,
            )
        } else {
            (None, git_path_from_bytes(path_field)?)
        };

        records.push(GitNumstatRecord {
            path,
            old_path,
            additions,
            deletions,
            line_count_status,
        });
    }
    Ok(records)
}

pub(crate) fn parse_numstat_count(bytes: &[u8], label: &str) -> Result<usize> {
    let value = std::str::from_utf8(bytes)
        .map_err(|_| NekocodeError::External(format!("git numstat returned non-UTF-8 {label}")))?;
    value.parse::<usize>().map_err(|_| {
        NekocodeError::External(format!("git numstat returned invalid {label}: {value}"))
    })
}

pub(crate) fn apply_numstat(
    files: &mut [ChangedRustFile],
    records: Vec<GitNumstatRecord>,
    scope: GitChangeScope,
) -> Result<()> {
    for record in records {
        let file = files
            .iter_mut()
            .find(|file| file.path == record.path)
            .ok_or_else(|| {
                NekocodeError::External(format!(
                    "git numstat path was absent from name-status output: {}",
                    record.path.display()
                ))
            })?;
        if let (Some(name_status_old_path), Some(numstat_old_path)) =
            (file.old_path.as_ref(), record.old_path.as_ref())
        {
            if name_status_old_path != numstat_old_path {
                return Err(NekocodeError::External(format!(
                    "git numstat rename source disagreed with name-status for {}",
                    record.path.display()
                )));
            }
        }
        if file.old_path.is_none() && record.old_path.is_some() {
            file.old_path = record.old_path;
        }
        let change = file
            .scope_changes
            .iter_mut()
            .find(|change| change.scope == scope)
            .ok_or_else(|| {
                NekocodeError::External(format!(
                    "git numstat scope was absent from name-status output: {}",
                    record.path.display()
                ))
            })?;
        if change.line_count_status != LineCountStatus::NotRead {
            return Err(NekocodeError::External(format!(
                "git numstat returned a duplicate path: {}",
                record.path.display()
            )));
        }
        change.additions = record.additions;
        change.deletions = record.deletions;
        change.line_count_status = record.line_count_status;
    }
    if let Some(file) = files.iter().find(|file| {
        file.scope_changes.iter().any(|change| {
            change.scope == scope && change.line_count_status == LineCountStatus::NotRead
        })
    }) {
        return Err(NekocodeError::External(format!(
            "git numstat omitted a path from name-status output: {}",
            file.path.display()
        )));
    }
    Ok(())
}

pub(crate) fn git_path_from_bytes(bytes: &[u8]) -> Result<PathBuf> {
    let path = std::str::from_utf8(bytes)
        .map_err(|_| NekocodeError::External("git returned a non-UTF-8 path".to_string()))?;
    if path.is_empty() {
        return Err(NekocodeError::External(
            "git returned an empty path".to_string(),
        ));
    }
    Ok(PathBuf::from(path))
}

pub(crate) fn git_context(
    root: &Path,
    compare_ref: Option<&str>,
    include_working_tree: bool,
    include_untracked_content: bool,
    include_patch: bool,
) -> Result<(Vec<ChangedRustFile>, RustDiffSummary)> {
    if let Some(reference) = compare_ref {
        validate_compare_ref(reference)?;
    }

    let mut changed_files = Vec::new();
    let mut patch_parts = Vec::new();
    let mut commands = Vec::new();
    let mut requested_scopes = Vec::new();

    if let Some(reference) = compare_ref {
        let spec = format!("{reference}...HEAD");
        let collected = collect_git_diff_scope(
            root,
            GitChangeScope::Revision,
            false,
            Some(&spec),
            include_patch,
        )?;
        requested_scopes.push(GitChangeScope::Revision);
        changed_files.extend(collected.files);
        if let Some(patch) = collected.patch {
            patch_parts.push(patch);
        }
        commands.extend(collected.commands);
    }

    if include_working_tree {
        let staged_base = git_rev_parse(root, "HEAD");
        let staged = collect_git_diff_scope(
            root,
            GitChangeScope::Staged,
            true,
            staged_base.as_deref(),
            include_patch,
        )?;
        requested_scopes.push(GitChangeScope::Staged);
        changed_files.extend(staged.files);
        if let Some(patch) = staged.patch {
            patch_parts.push(patch);
        }
        commands.extend(staged.commands);

        let unstaged =
            collect_git_diff_scope(root, GitChangeScope::Unstaged, false, None, include_patch)?;
        requested_scopes.push(GitChangeScope::Unstaged);
        changed_files.extend(unstaged.files);
        if let Some(patch) = unstaged.patch {
            patch_parts.push(patch);
        }
        commands.extend(unstaged.commands);

        requested_scopes.push(GitChangeScope::Untracked);
        let untracked = git_untracked_files(root)?;
        commands.push("git ls-files --others --exclude-standard -z".to_string());
        for path in &untracked {
            changed_files.push(ChangedRustFile {
                status: "??".to_string(),
                path: path.clone(),
                old_path: None,
                is_rust: is_rust_path(path),
                package: None,
                hunks: Vec::new(),
                scope_changes: vec![RustFileScopeChange {
                    scope: GitChangeScope::Untracked,
                    status: "??".to_string(),
                    additions: None,
                    deletions: None,
                    line_count_status: LineCountStatus::NotRead,
                }],
            });
            if include_patch && include_untracked_content {
                if let Some(patch) = untracked_patch(root, path) {
                    if let Some(file_hunks) = parse_unified_hunks(&patch).remove(path) {
                        if let Some(file) = changed_files.last_mut() {
                            file.hunks = file_hunks
                                .into_iter()
                                .map(|mut hunk| {
                                    hunk.scope = Some(GitChangeScope::Untracked);
                                    hunk
                                })
                                .collect();
                        }
                    }
                    patch_parts.push(patch);
                }
            }
        }
    }

    changed_files = merge_changed_files(changed_files);
    let change_scopes = summarize_change_scopes(&changed_files, &requested_scopes);
    let patch = patch_parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let summary = RustDiffSummary {
        compare_ref: compare_ref.map(str::to_string),
        resolved_base: compare_ref.and_then(|reference| git_rev_parse(root, reference)),
        resolved_head: git_rev_parse(root, "HEAD"),
        include_working_tree,
        include_untracked_content,
        patch,
        patch_truncated: false,
        omitted_patch_bytes: 0,
        change_scopes,
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

fn validate_compare_ref(reference: &str) -> Result<()> {
    let mut chars = reference.chars();
    let Some(first) = chars.next() else {
        return Err(NekocodeError::Config(
            "compare ref must be a non-empty simple Git revision".to_string(),
        ));
    };
    if !first.is_ascii_alphanumeric()
        || !chars.all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '.' | '_' | '/' | '@' | '+' | '~' | '^' | '-')
        })
    {
        return Err(NekocodeError::Config(
            "compare ref must be a simple Git revision".to_string(),
        ));
    }
    Ok(())
}

pub(crate) struct CollectedGitScope {
    pub(crate) files: Vec<ChangedRustFile>,
    pub(crate) patch: Option<String>,
    pub(crate) commands: Vec<String>,
}

pub(crate) fn collect_git_diff_scope(
    root: &Path,
    scope: GitChangeScope,
    cached: bool,
    spec: Option<&str>,
    include_patch: bool,
) -> Result<CollectedGitScope> {
    let mut name_status_args = vec!["diff".to_string()];
    if cached {
        name_status_args.push("--cached".to_string());
    }
    name_status_args.extend([
        "--relative".to_string(),
        "--name-status".to_string(),
        "-z".to_string(),
        "--no-ext-diff".to_string(),
        "--no-textconv".to_string(),
    ]);
    if let Some(spec) = spec {
        name_status_args.push(spec.to_string());
    }
    let mut files = parse_name_status_z(
        &run_git_bytes(root, &name_status_args, "git diff --name-status")?,
        scope,
    )?;

    let mut numstat_args = vec!["diff".to_string()];
    if cached {
        numstat_args.push("--cached".to_string());
    }
    numstat_args.extend([
        "--relative".to_string(),
        "--numstat".to_string(),
        "-z".to_string(),
        "--no-ext-diff".to_string(),
        "--no-textconv".to_string(),
    ]);
    if let Some(spec) = spec {
        numstat_args.push(spec.to_string());
    }
    let numstat = parse_numstat_z(&run_git_bytes(root, &numstat_args, "git diff --numstat")?)?;
    apply_numstat(&mut files, numstat, scope)?;

    let mut commands = vec![
        format!("git {}", name_status_args.join(" ")),
        format!("git {}", numstat_args.join(" ")),
    ];
    let patch = if include_patch {
        let mut patch_args = vec![
            "-c".to_string(),
            "core.quotePath=false".to_string(),
            "diff".to_string(),
        ];
        if cached {
            patch_args.push("--cached".to_string());
        }
        patch_args.extend([
            "--relative".to_string(),
            "--no-ext-diff".to_string(),
            "--no-textconv".to_string(),
            "--unified=3".to_string(),
        ]);
        if let Some(spec) = spec {
            patch_args.push(spec.to_string());
        }
        commands.push(format!("git {}", patch_args.join(" ")));
        Some(run_git(root, &patch_args, "git diff")?)
    } else {
        None
    };

    if let Some(patch) = &patch {
        let hunks = parse_unified_hunks(patch);
        for file in &mut files {
            if let Some(file_hunks) = hunks.get(&file.path) {
                file.hunks = file_hunks
                    .iter()
                    .cloned()
                    .map(|mut hunk| {
                        hunk.scope = Some(scope);
                        hunk
                    })
                    .collect();
            }
        }
    }
    Ok(CollectedGitScope {
        files,
        patch,
        commands,
    })
}

pub(crate) fn run_git_bytes(root: &Path, args: &[String], label: &str) -> Result<Vec<u8>> {
    let mut command = Command::new("git");
    command.current_dir(root).args(args);
    let output = run_bounded_command(
        command,
        GIT_COMMAND_TIMEOUT,
        MAX_GIT_STDOUT_BYTES,
        MAX_GIT_STDERR_BYTES,
    )?;
    if output.timed_out {
        return Err(NekocodeError::External(format!(
            "{label} timed out after {} seconds",
            GIT_COMMAND_TIMEOUT.as_secs()
        )));
    }
    if output.output_limited {
        return Err(NekocodeError::External(format!(
            "{label} output exceeded the safety limit"
        )));
    }
    if !output.status.is_some_and(|status| status.success()) {
        return Err(NekocodeError::External(format_command_failure(
            label,
            &output.stderr.bytes,
        )));
    }
    Ok(output.stdout.bytes)
}

pub(crate) fn run_git(root: &Path, args: &[String], label: &str) -> Result<String> {
    Ok(String::from_utf8_lossy(&run_git_bytes(root, args, label)?).into_owned())
}

pub(crate) fn git_rev_parse(root: &Path, reference: &str) -> Option<String> {
    let args = vec!["rev-parse".to_string(), reference.to_string()];
    run_git(root, &args, "git rev-parse")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn git_untracked_files(root: &Path) -> Result<Vec<PathBuf>> {
    let args = vec![
        "ls-files".to_string(),
        "--others".to_string(),
        "--exclude-standard".to_string(),
        "-z".to_string(),
    ];
    let output = run_git_bytes(root, &args, "git ls-files")?;
    if output.is_empty() {
        return Ok(Vec::new());
    }
    if !output.ends_with(&[0]) {
        return Err(NekocodeError::External(
            "malformed NUL-delimited git ls-files output".to_string(),
        ));
    }
    output[..output.len() - 1]
        .split(|byte| *byte == 0)
        .map(git_path_from_bytes)
        .collect()
}

pub(crate) fn merge_changed_files(files: Vec<ChangedRustFile>) -> Vec<ChangedRustFile> {
    let mut merged = Vec::new();
    for mut file in files {
        if let Some(existing) = merged
            .iter_mut()
            .find(|item: &&mut ChangedRustFile| item.path == file.path)
        {
            existing.status = file.status;
            if file.old_path.is_some() {
                existing.old_path = file.old_path;
            }
            existing.hunks.append(&mut file.hunks);
            for change in file.scope_changes.drain(..) {
                if let Some(existing_change) = existing
                    .scope_changes
                    .iter_mut()
                    .find(|candidate| candidate.scope == change.scope)
                {
                    *existing_change = change;
                } else {
                    existing.scope_changes.push(change);
                }
            }
            continue;
        }
        merged.push(file);
    }
    merged
}

pub(crate) fn summarize_change_scopes(
    files: &[ChangedRustFile],
    requested_scopes: &[GitChangeScope],
) -> Vec<RustChangeScopeSummary> {
    requested_scopes
        .iter()
        .copied()
        .map(|scope| {
            let mut summary = RustChangeScopeSummary {
                scope,
                file_count: 0,
                rust_file_count: 0,
                additions: 0,
                deletions: 0,
                counted_files: 0,
                binary_files: 0,
                not_read_files: 0,
            };
            for file in files {
                let Some(change) = file
                    .scope_changes
                    .iter()
                    .find(|change| change.scope == scope)
                else {
                    continue;
                };
                summary.file_count += 1;
                summary.rust_file_count += usize::from(file.is_rust);
                match change.line_count_status {
                    LineCountStatus::Counted => {
                        summary.counted_files += 1;
                        summary.additions += change.additions.unwrap_or(0);
                        summary.deletions += change.deletions.unwrap_or(0);
                    }
                    LineCountStatus::Binary => summary.binary_files += 1,
                    LineCountStatus::NotRead => summary.not_read_files += 1,
                }
            }
            summary
        })
        .collect()
}

pub(crate) fn parse_unified_hunks(text: &str) -> HashMap<PathBuf, Vec<RustDiffHunk>> {
    let mut result: HashMap<PathBuf, Vec<RustDiffHunk>> = HashMap::new();
    let mut current_path: Option<PathBuf> = None;
    let mut in_hunk = false;
    for line in text.lines() {
        if line.starts_with("diff --git ") {
            current_path = None;
            in_hunk = false;
            continue;
        }
        if line.starts_with("@@") {
            if let (Some(path), Some(hunk)) = (current_path.clone(), parse_hunk_header(line)) {
                result.entry(path).or_default().push(hunk);
            }
            in_hunk = true;
            continue;
        }
        if in_hunk {
            continue;
        }
        if let Some(path) = line.strip_prefix("--- a/") {
            current_path = Some(PathBuf::from(path.strip_suffix('\t').unwrap_or(path)));
            continue;
        }
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_path = Some(PathBuf::from(path.strip_suffix('\t').unwrap_or(path)));
            continue;
        }
        if line == "+++ /dev/null" {
            continue;
        }
    }
    result
}

pub(crate) fn parse_hunk_header(line: &str) -> Option<RustDiffHunk> {
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
        scope: None,
        old_start,
        old_count,
        new_start,
        new_count,
        header: (!header.is_empty()).then(|| header.to_string()),
    })
}

pub(crate) fn parse_diff_range(value: &str) -> Option<(u32, u32)> {
    let mut parts = value.split(',');
    let start = parts.next()?.parse().ok()?;
    let count = parts.next().map_or(Some(1), |value| value.parse().ok())?;
    Some((start, count))
}

pub(crate) fn untracked_patch(root: &Path, path: &Path) -> Option<String> {
    let safe_path = safe_workspace_file(root, path)?;
    let content = std::fs::read_to_string(safe_path).ok()?;
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
    Some(patch)
}
