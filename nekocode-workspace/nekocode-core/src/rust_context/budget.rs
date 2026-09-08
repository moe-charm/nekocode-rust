//! Hard byte budgets, omissions, and source excerpts.

use super::*;

pub(crate) fn omissions_for_pack(pack: &RustContextPack) -> Vec<Omission> {
    let mut omissions = Vec::new();
    let mut push = |kind: &str, count: usize, priority: &str| {
        if count > 0 {
            omissions.push(Omission {
                kind: kind.to_string(),
                reason: "item_limit".to_string(),
                omitted_count: count,
                priority: priority.to_string(),
            });
        }
    };
    push("changed_files", pack.omitted_changed_files, "context");
    push("source_excerpts", pack.omitted_excerpts, "context");
    push("diagnostics", pack.omitted_diagnostics, "warning");
    push("diagnostic_delta", pack.omitted_delta_items, "warning");
    push("diff_bytes", pack.omitted_diff_bytes, "context");
    if pack.budget_exceeded {
        omissions.push(Omission {
            kind: "context".to_string(),
            reason: "byte_budget".to_string(),
            omitted_count: 1,
            priority: "envelope".to_string(),
        });
    }
    omissions
}

pub(crate) fn safe_workspace_file(root: &Path, relative: &Path) -> Option<PathBuf> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return None;
    }
    let candidate = root.join(relative).canonicalize().ok()?;
    candidate.starts_with(root).then_some(candidate)
}

pub(crate) fn build_source_excerpts(
    root: &Path,
    changed_files: &[ChangedRustFile],
    context_lines: usize,
    resolved_head: Option<&str>,
) -> Vec<RustSourceExcerpt> {
    let mut groups: BTreeMap<(PathBuf, Option<GitChangeScope>), Vec<&RustDiffHunk>> =
        BTreeMap::new();
    for file in changed_files.iter().filter(|file| file.is_rust) {
        for hunk in &file.hunks {
            groups
                .entry((file.path.clone(), hunk.scope))
                .or_default()
                .push(hunk);
        }
    }
    let mut excerpts = Vec::new();
    for ((path, scope), hunks) in groups {
        // Revision and index coordinates must never be applied to dirty disk
        // content. Read the corresponding Git blob, including for nested roots.
        let content = match scope {
            Some(GitChangeScope::Revision) | Some(GitChangeScope::Staged) => {
                let revision = if scope == Some(GitChangeScope::Staged) {
                    ""
                } else {
                    let Some(head) = resolved_head else {
                        continue;
                    };
                    head
                };
                let spec = format!("{revision}:./{}", path.to_string_lossy());
                super::git::run_git(
                    root,
                    &[
                        "show".into(),
                        "--no-ext-diff".into(),
                        "--no-textconv".into(),
                        spec,
                    ],
                    "git show excerpt",
                )
                .ok()
            }
            _ => {
                safe_workspace_file(root, &path).and_then(|path| std::fs::read_to_string(path).ok())
            }
        };
        let Some(content) = content else {
            continue;
        };
        let lines = content.lines().collect::<Vec<_>>();
        let line_count = lines.len() as u32;
        let mut ranges = Vec::new();
        for hunk in hunks {
            let changed_start = hunk.new_start.max(1);
            let changed_end = changed_start.saturating_add(hunk.new_count.saturating_sub(1));
            let start = changed_start.saturating_sub(context_lines as u32).max(1);
            let end = changed_end
                .saturating_add(context_lines as u32)
                .min(line_count);
            if start <= end {
                ranges.push((start, end));
            }
        }
        ranges.sort_unstable();
        let mut merged: Vec<(u32, u32)> = Vec::new();
        for (start, end) in ranges {
            if let Some(last) = merged.last_mut() {
                if start <= last.1.saturating_add(1) {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            merged.push((start, end));
        }
        for (start, end) in merged {
            let mut excerpt = lines[(start as usize - 1)..(end as usize)].join("\n");
            let truncated = truncate_utf8(&mut excerpt, MAX_SOURCE_EXCERPT_BYTES) > 0;
            if truncated {
                excerpt.push_str("\n... [source excerpt truncated]");
            }
            excerpts.push(RustSourceExcerpt {
                scope,
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

pub(crate) fn serialized_size(pack: &RustContextPack) -> Result<usize> {
    Ok(serde_json::to_vec(pack)?.len())
}

pub(crate) fn truncate_utf8(value: &mut String, max_bytes: usize) -> usize {
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

pub(crate) fn limitations(
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
