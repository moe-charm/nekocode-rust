use super::capture::{CapturedSource, InputInventory};
use super::model::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(super) fn coverage(response: &SymbolContextV1) -> Vec<CoverageExplanation> {
    let scope = &response.scope;
    let backend = format!(
        "backend health: {}; readiness observed: {}",
        response.backend.health.as_deref().unwrap_or("unknown"),
        response.backend.readiness_observed
    );
    let query = |method: &str| {
        response
            .queries
            .iter()
            .find(|q| q.method == method)
            .map(|q| {
                format!(
                    "{}; count: {}",
                    q.status,
                    q.result_count
                        .map_or_else(|| "unknown".into(), |n| n.to_string())
                )
            })
            .unwrap_or_else(|| "not observed".into())
    };
    let entry =
        |area: &str, requested: String, observed: String, verification: &str, limitation: &str| {
            CoverageExplanation {
                area: area.into(),
                requested,
                observed,
                verification: verification.into(),
                limitation: limitation.into(),
            }
        };
    vec![
        entry("features", scope.features.clone(), backend.clone(), "unverified", "Requested features are not an audited effective cfg/target matrix."),
        entry("cfg_test", scope.cfg_test.clone(), query("rust-analyzer/relatedTests"), "unverified", "Test-code inclusion is not independently verified; related-test results do not establish coverage."),
        entry("macros", format!("proc_macros={}; build_scripts={}", scope.proc_macros, scope.build_scripts), backend, "unverified", "Enablement does not prove expansion or reference coverage for builtin, declarative or procedural macros. assert! calls may be missed."),
        entry("tests", "candidate discovery only".into(), format!("{}; tests_executed={}", query("rust-analyzer/relatedTests"), scope.tests_executed), if scope.tests_executed { "unverified" } else { "not_executed" }, "Test candidates have not been validated by running them in this investigation."),
        entry("text_search", if response.queries.iter().any(|q| q.method == "rg/text_candidates") { "requested" } else { "not recorded" }.into(), query("rg/text_candidates"), "text_only", "Whole-word workspace Rust matches are unconfirmed candidates, not semantic references; indirect and external calls remain outside this scan."),
    ]
}

pub(super) fn verify_inputs(
    root: &Path,
    before: &InputInventory,
    after: &InputInventory,
    sources: &BTreeMap<PathBuf, CapturedSource>,
    basis: &str,
) -> (InputVerification, Vec<PathBuf>) {
    let mut result = InputVerification {
        basis: basis.into(),
        verdict: "unknown".into(),
        matched: 0,
        modified: 0,
        missing: 0,
        unreadable: 0,
        unobserved: 0,
        newly_observed: 0,
        captured_mismatches: 0,
        baseline_scan_complete: before.complete,
        current_scan_complete: after.complete,
        issues: Vec::new(),
        issues_omitted: 0,
    };
    let mut changed = BTreeSet::new();
    let mut issues = BTreeMap::new();
    for (path, hash) in &before.files {
        match after.files.get(path) {
            Some(current) if current == hash => result.matched += 1,
            Some(_) => {
                result.modified += 1;
                changed.insert(path.clone());
                issues.insert(path.clone(), "modified");
            }
            None => {
                let status = match std::fs::symlink_metadata(root.join(path)) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        result.missing += 1;
                        changed.insert(path.clone());
                        "missing"
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        result.unreadable += 1;
                        "unreadable"
                    }
                    Ok(metadata) if !metadata.is_file() && !metadata.is_symlink() => {
                        result.modified += 1;
                        changed.insert(path.clone());
                        "modified"
                    }
                    Ok(metadata)
                        if metadata.is_file() && std::fs::File::open(root.join(path)).is_err() =>
                    {
                        result.unreadable += 1;
                        "unreadable"
                    }
                    _ => {
                        result.unobserved += 1;
                        "unobserved"
                    }
                };
                issues.insert(path.clone(), status);
            }
        }
    }
    for path in after
        .files
        .keys()
        .filter(|p| !before.files.contains_key(*p))
    {
        result.newly_observed += 1;
        if before.complete {
            changed.insert(path.clone());
        }
        issues.insert(path.clone(), "newly_observed");
    }
    for (path, source) in sources {
        if after
            .files
            .get(path)
            .is_some_and(|current| current != &source.sha256)
        {
            result.captured_mismatches += 1;
            changed.insert(path.clone());
            issues.insert(path.clone(), "captured_mismatch");
        }
    }
    result.verdict = if !changed.is_empty() {
        "changed"
    } else if before.complete && after.complete && result.unreadable == 0 && result.unobserved == 0
    {
        "match"
    } else {
        "unknown"
    }
    .into();
    result.issues_omitted = issues.len().saturating_sub(16);
    result.issues = issues
        .into_iter()
        .take(16)
        .map(|(path, status)| InputIssue {
            path,
            status: status.into(),
        })
        .collect();
    (result, changed.into_iter().take(4096).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinguishes_hash_changes_missing_files_and_unobserved_inputs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("unobserved.rs"), "exists").unwrap();
        let before = InputInventory {
            complete: true,
            files: [
                ("same.rs".into(), "a".into()),
                ("changed.rs".into(), "b".into()),
                ("missing.rs".into(), "c".into()),
                ("unobserved.rs".into(), "d".into()),
            ]
            .into(),
        };
        let after = InputInventory {
            complete: false,
            files: [
                ("same.rs".into(), "a".into()),
                ("changed.rs".into(), "new".into()),
                ("new.rs".into(), "e".into()),
            ]
            .into(),
        };
        let (v, changes) = verify_inputs(dir.path(), &before, &after, &BTreeMap::new(), "test");
        assert_eq!(
            (
                v.matched,
                v.modified,
                v.missing,
                v.unobserved,
                v.newly_observed
            ),
            (1, 1, 1, 1, 1)
        );
        assert_eq!(v.verdict, "changed");
        assert!(!changes.contains(&PathBuf::from("unobserved.rs")));
    }
    #[test]
    fn unobserved_alone_is_unknown_and_issue_examples_are_bounded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "exists").unwrap();
        let before = InputInventory {
            complete: true,
            files: [("a.rs".into(), "hash".into())].into(),
        };
        let after = InputInventory {
            complete: false,
            files: BTreeMap::new(),
        };
        let (v, changes) = verify_inputs(dir.path(), &before, &after, &BTreeMap::new(), "test");
        assert_eq!(v.verdict, "unknown");
        assert_eq!(v.unobserved, 1);
        assert!(changes.is_empty());
        let many = InputInventory {
            complete: true,
            files: (0..18)
                .map(|i| (PathBuf::from(format!("missing{i}.rs")), "hash".into()))
                .collect(),
        };
        let (v, _) = verify_inputs(dir.path(), &many, &after, &BTreeMap::new(), "test");
        assert_eq!(v.missing, 18);
        assert_eq!(v.issues.len(), 16);
        assert_eq!(v.issues_omitted, 2);
    }

    #[test]
    fn incomplete_scan_is_not_a_match_and_captured_content_is_checked_separately() {
        let dir = tempfile::tempdir().unwrap();
        let before = InputInventory {
            complete: false,
            files: [("a.rs".into(), "same".into())].into(),
        };
        let (v, changes) = verify_inputs(dir.path(), &before, &before, &BTreeMap::new(), "test");
        assert_eq!(v.matched, 1);
        assert_eq!(v.verdict, "unknown");
        assert!(changes.is_empty());
        let sources = [(
            "a.rs".into(),
            CapturedSource {
                sha256: "different".into(),
                text: String::new(),
            },
        )]
        .into();
        let (v, changes) = verify_inputs(dir.path(), &before, &before, &sources, "test");
        assert_eq!(v.captured_mismatches, 1);
        assert_eq!(v.verdict, "changed");
        assert_eq!(changes.len(), 1);
    }
}
