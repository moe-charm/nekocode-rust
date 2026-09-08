//! Text evidence stays distinct from semantic references.
use super::capture::{Sources, MAX_ITEMS};
use super::model::*;
use crate::rust_context::execution::{configure_safe_environment, run_bounded_command};
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
type ScanError = (&'static str, String);
fn invalid(message: &str) -> ScanError {
    ("failed", message.into())
}

fn identifier(location: &SymbolLocation, sources: &mut Sources) -> Option<String> {
    if location.start.line != location.end.line {
        return None;
    }
    let (_, text) = sources.text(&location.path).ok()?;
    let line = text
        .split('\n')
        .nth(location.start.line.checked_sub(1)? as usize)?;
    let name: String = line
        .chars()
        .skip(location.start.column.checked_sub(1)? as usize)
        .take(location.end.column.checked_sub(location.start.column)? as usize)
        .collect();
    let name = name.strip_prefix("r#").unwrap_or(&name);
    if name.is_empty()
        || !name.chars().all(|c| c == '_' || c.is_alphanumeric())
        || !name
            .chars()
            .next()
            .is_some_and(|c| c == '_' || c.is_alphabetic())
    {
        return None;
    }
    Some(name.to_string())
}
fn overlaps(a: &SymbolLocation, b: &SymbolLocation) -> bool {
    a.path == b.path
        && (a.start.line, a.start.column) < (b.end.line, b.end.column)
        && (b.start.line, b.start.column) < (a.end.line, a.end.column)
}
pub(super) fn collect(root: &Path, sources: &mut Sources, response: &mut SymbolContextV1) {
    let (status, count, detail) = match scan(root, sources, response) {
        Ok(count) => ("completed", Some(count), "Unconfirmed whole-word matches in workspace *.rs (hidden/ignored included; .git/target/.nekocode excluded). Not verified references.".into()),
        Err((status, detail)) => (status, None, detail),
    };
    response.queries.push(SymbolQuery {
        method: "rg/text_candidates".into(),
        status: status.into(),
        result_count: count,
        detail: Some(detail),
    });
}
fn scan(
    root: &Path,
    sources: &mut Sources,
    response: &mut SymbolContextV1,
) -> Result<usize, ScanError> {
    let definitions: Vec<_> = response
        .items
        .iter()
        .filter(|i| i.relation == "definition")
        .map(|i| i.location.clone())
        .collect();
    let references = response
        .items
        .iter()
        .filter(|i| i.relation == "reference")
        .count();
    if definitions.len() != 1
        || !response.queries.iter().any(|q| {
            q.method == "textDocument/references"
                && q.status == "completed"
                && q.result_count == Some(references)
        })
        || !response.omissions.is_empty()
    {
        return Err(("unsupported", "not_applicable: requires one definition and complete retained semantic reference observations".into()));
    }
    let name = identifier(&definitions[0], sources).ok_or((
        "unsupported",
        "not_applicable: definition identifier cannot be extracted".into(),
    ))?;
    let mut cmd = Command::new("rg");
    configure_safe_environment(&mut cmd);
    cmd.current_dir(root).args([
        "--json",
        "--no-config",
        "--no-ignore",
        "--hidden",
        "--fixed-strings",
        "--word-regexp",
        "--glob",
        "*.rs",
        "--glob",
        "!**/.git/**",
        "--glob",
        "!**/target/**",
        "--glob",
        "!**/.nekocode/**",
        "--",
        &name,
        ".",
    ]);
    let output = run_bounded_command(cmd, Duration::from_secs(10), 1024 * 1024, 64 * 1024)
        .map_err(|e| invalid(&format!("rg could not run: {e}")))?;
    if output.timed_out {
        return Err((
            "timed_out",
            "rg exceeded its 10-second scan deadline".into(),
        ));
    }
    if output.output_limited || output.stdout.truncated || output.stderr.truncated {
        return Err((
            "output_limited",
            "rg output exceeded capture bounds; no complete text comparison".into(),
        ));
    }
    if !output
        .status
        .is_some_and(|s| matches!(s.code(), Some(0 | 1)))
    {
        return Err(invalid(&format!(
            "rg scan failed: {}",
            String::from_utf8_lossy(&output.stderr.bytes)
        )));
    }
    let mut candidates = Vec::new();
    let mut count = 0;
    for record in output
        .stdout
        .bytes
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
    {
        let value: Value =
            serde_json::from_slice(record).map_err(|_| invalid("rg emitted invalid JSON"))?;
        if value["type"] != "match" {
            continue;
        }
        let data = &value["data"];
        let path = data["path"]["text"]
            .as_str()
            .ok_or(invalid("rg path is not UTF-8"))?;
        let matched_line = data["lines"]["text"]
            .as_str()
            .ok_or(invalid("rg source is not UTF-8"))?;
        let line_number = data["line_number"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .filter(|n| *n > 0)
            .ok_or(invalid("rg line number is invalid"))?;
        let (relative, text) = sources
            .text(Path::new(path))
            .map_err(|e| invalid(&format!("rg candidate source unavailable: {e}")))?;
        let captured_line = text
            .split_inclusive('\n')
            .nth(line_number as usize - 1)
            .ok_or(invalid("rg source line disappeared"))?;
        if captured_line != matched_line {
            return Err(invalid(
                "rg source changed relative to captured evidence; retry investigation",
            ));
        }
        for m in data["submatches"]
            .as_array()
            .ok_or(invalid("rg submatches missing"))?
        {
            let start = m["start"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or(invalid("rg start invalid"))?;
            let end = m["end"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or(invalid("rg end invalid"))?;
            let prefix = captured_line
                .get(..start)
                .ok_or(invalid("rg offset invalid"))?;
            let matched = captured_line
                .get(start..end)
                .filter(|s| *s == name)
                .ok_or(invalid("rg match differs from identifier"))?;
            let location = SymbolLocation {
                path: relative.clone(),
                start: SymbolPosition {
                    line: line_number,
                    column: prefix.chars().count() as u32 + 1,
                },
                end: SymbolPosition {
                    line: line_number,
                    column: (prefix.chars().count() + matched.chars().count()) as u32 + 1,
                },
            };
            if response.items.iter().any(|i| {
                matches!(i.relation.as_str(), "definition" | "reference")
                    && overlaps(&location, &i.location)
            }) {
                continue;
            }
            count += 1;
            if candidates.len() >= 64 || response.items.len() + candidates.len() >= MAX_ITEMS {
                continue;
            }
            let mut excerpt_end = captured_line.len().min(4096);
            while !captured_line.is_char_boundary(excerpt_end) {
                excerpt_end -= 1;
            }
            let truncated = excerpt_end < captured_line.len();
            candidates.push(SymbolItem { id: String::new(), relation: "unconfirmed_text_candidate".into(), backend: "ripgrep".into(), source: "rg/text_candidates".into(), location, reason: "Text match absent from captured semantic references; may be a macro call, comment, string, inactive code or different symbol.".into(), code: captured_line[..excerpt_end].into(), excerpt_start_line: line_number, excerpt_end_line: line_number, containing_symbol: None, detail: Some(format!("Whole-word literal search: {name}. This is not a verified caller.")), truncated });
        }
    }
    let omitted = count - candidates.len();
    response.totals.observed += count;
    response.items.extend(candidates);
    if omitted > 0 {
        response.omissions.push(SymbolOmission {
            kind: "unconfirmed_text_candidates".into(),
            reason: "collection_limit".into(),
            count: omitted,
        });
        return Err((
            "output_limited",
            "Text candidate capture limit reached; retained candidates are partial".into(),
        ));
    }
    Ok(count)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_line_different_calls_do_not_overlap() {
        let a = SymbolLocation {
            path: "src/lib.rs".into(),
            start: SymbolPosition { line: 1, column: 4 },
            end: SymbolPosition {
                line: 1,
                column: 10,
            },
        };
        let mut b = a.clone();
        b.start.column = 14;
        b.end.column = 20;
        assert!(!overlaps(&a, &b));
        assert!(overlaps(&a, &a));
    }
}
