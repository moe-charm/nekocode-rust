use super::model::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub(super) const MAX_ITEMS: usize = 256;
const MAX_INPUTS: usize = 4096;
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_CAPTURE_BYTES: usize = 16 * 1024 * 1024;

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct InputInventory {
    pub files: BTreeMap<PathBuf, String>,
    pub complete: bool,
}

/// Inventory Rust and Cargo inputs, including additions/deletions on replay.
/// External build inputs and symlinked directories are explicitly not covered.
pub(super) fn inventory(root: &Path) -> InputInventory {
    let mut result = InputInventory {
        files: BTreeMap::new(),
        complete: true,
    };
    let mut pending = vec![root.to_path_buf()];
    let mut examined = 0usize;
    let mut bytes_left = MAX_INPUT_BYTES;
    while let Some(directory) = pending.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => {
                result.complete = false;
                continue;
            }
        };
        for entry in entries {
            examined += 1;
            if examined > 32_768 || result.files.len() >= MAX_INPUTS {
                result.complete = false;
                return result;
            }
            let Ok(entry) = entry else {
                result.complete = false;
                continue;
            };
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                result.complete = false;
                continue;
            };
            let name = entry.file_name();
            if kind.is_dir() {
                if !matches!(name.to_str(), Some(".git" | "target" | ".nekocode")) {
                    pending.push(path);
                }
                continue;
            }
            let tracked = path.extension().is_some_and(|extension| extension == "rs")
                || matches!(
                    name.to_str(),
                    Some("Cargo.toml" | "Cargo.lock" | "rust-toolchain" | "rust-toolchain.toml")
                )
                || (path
                    .parent()
                    .and_then(Path::file_name)
                    .is_some_and(|name| name == ".cargo")
                    && matches!(name.to_str(), Some("config" | "config.toml")));
            if kind.is_symlink() {
                result.complete = false;
                continue;
            }
            if !tracked || !kind.is_file() {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                result.complete = false;
                continue;
            };
            if metadata.len() > bytes_left || metadata.len() > 8 * 1024 * 1024 {
                result.complete = false;
                continue;
            }
            bytes_left = bytes_left.saturating_sub(metadata.len());
            match read_limited(&path, 8 * 1024 * 1024) {
                Ok(bytes) => {
                    if let Ok(relative) = path.strip_prefix(root) {
                        result.files.insert(relative.to_path_buf(), digest(&bytes));
                    }
                }
                Err(_) => result.complete = false,
            }
        }
    }
    result
}

pub(super) fn changed_inputs(before: &InputInventory, after: &InputInventory) -> Vec<PathBuf> {
    let mut changes = before
        .files
        .keys()
        .chain(after.files.keys())
        .filter(|path| before.files.get(*path) != after.files.get(*path))
        .cloned()
        .collect::<Vec<_>>();
    changes.sort();
    changes.dedup();
    changes.truncate(MAX_INPUTS);
    changes
}

pub(super) fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() > limit {
        return Err(format!(
            "file exceeds {limit} byte capture limit: {}",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("file grew beyond capture limit".to_string());
    }
    Ok(bytes)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct CapturedSource {
    pub sha256: String,
    pub text: String,
}

/// Captured backend locations can be include! files with any extension. They
/// remain part of freshness even when absent from the Rust/Cargo inventory.
pub(super) fn refresh_captured_inputs(
    root: &Path,
    sources: &BTreeMap<PathBuf, CapturedSource>,
    inputs: &mut InputInventory,
) {
    for path in sources.keys() {
        let absolute = root.join(path);
        let current = absolute
            .canonicalize()
            .ok()
            .filter(|path| path.starts_with(root))
            .and_then(|path| read_limited(&path, MAX_SOURCE_BYTES).ok());
        match current {
            Some(bytes) => {
                inputs.files.insert(path.clone(), digest(&bytes));
            }
            None => {
                inputs.files.remove(path);
                inputs.complete = false;
            }
        }
    }
}

pub(super) struct Sources {
    pub root: PathBuf,
    pub files: BTreeMap<PathBuf, CapturedSource>,
    bytes: usize,
}

impl Sources {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            files: BTreeMap::new(),
            bytes: 0,
        }
    }

    pub fn relative(&self, path: &Path) -> Result<PathBuf, String> {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        let absolute = absolute.canonicalize().map_err(|error| error.to_string())?;
        absolute
            .strip_prefix(&self.root)
            .map(Path::to_path_buf)
            .map_err(|_| "backend location is outside the workspace capture boundary".to_string())
    }

    pub fn text(&mut self, path: &Path) -> Result<(PathBuf, String), String> {
        let relative = self.relative(path)?;
        if let Some(source) = self.files.get(&relative) {
            return Ok((relative, source.text.clone()));
        }
        let bytes = read_limited(&self.root.join(&relative), MAX_SOURCE_BYTES)?;
        if self.bytes.saturating_add(bytes.len()) > MAX_CAPTURE_BYTES {
            return Err("total source capture byte limit reached".to_string());
        }
        let sha256 = digest(&bytes);
        let text = String::from_utf8(bytes).map_err(|_| "source is not UTF-8".to_string())?;
        self.bytes += text.len();
        self.files.insert(
            relative.clone(),
            CapturedSource {
                sha256,
                text: text.clone(),
            },
        );
        Ok((relative, text))
    }
}

fn from_utf16(text: &str, line: u32, column: u32) -> Option<SymbolPosition> {
    let text_line = text.split('\n').nth(line as usize)?;
    let mut utf16 = 0u32;
    let mut scalar = 0u32;
    for character in text_line.chars() {
        if utf16 == column {
            break;
        }
        utf16 += character.len_utf16() as u32;
        scalar += 1;
        if utf16 > column {
            return None;
        }
    }
    (utf16 == column).then_some(SymbolPosition {
        line: line + 1,
        column: scalar + 1,
    })
}

pub(super) fn location(path: PathBuf, text: &str, range: &Value) -> Option<SymbolLocation> {
    let start = range.get("start")?;
    let end = range.get("end")?;
    let start = from_utf16(
        text,
        u32::try_from(start.get("line")?.as_u64()?).ok()?,
        u32::try_from(start.get("character")?.as_u64()?).ok()?,
    )?;
    let end = from_utf16(
        text,
        u32::try_from(end.get("line")?.as_u64()?).ok()?,
        u32::try_from(end.get("character")?.as_u64()?).ok()?,
    )?;
    if (start.line, start.column) > (end.line, end.column) {
        return None;
    }
    Some(SymbolLocation { path, start, end })
}

pub(super) fn excerpt(
    text: &str,
    location: &SymbolLocation,
    context: usize,
    limit: usize,
) -> (String, u32, u32, bool) {
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    if lines.is_empty() {
        return (String::new(), 1, 1, false);
    }
    let start = (location.start.line.saturating_sub(1) as usize)
        .saturating_sub(context)
        .min(lines.len());
    let range_end = if location.end.column == 1 && location.end.line > location.start.line {
        location.end.line.saturating_sub(1)
    } else {
        location.end.line
    };
    let end = (range_end as usize)
        .saturating_add(context)
        .min(lines.len())
        .max(start);
    let mut code = lines[start..end].concat();
    let truncated = code.len() > limit;
    if truncated {
        let mut boundary = limit;
        while !code.is_char_boundary(boundary) {
            boundary -= 1;
        }
        code.truncate(boundary);
    }
    let count = code.split_inclusive('\n').count().max(1);
    (code, start as u32 + 1, (start + count) as u32, truncated)
}

pub(super) fn containing_symbol(
    outline: &Value,
    path: &Path,
    text: &str,
    point: &SymbolLocation,
) -> Option<ContainingSymbol> {
    let mut pending = outline.as_array()?.iter().collect::<Vec<_>>();
    let mut best: Option<ContainingSymbol> = None;
    let mut visited = 0usize;
    while let Some(symbol) = pending.pop() {
        visited += 1;
        if visited > 4096 {
            break;
        }
        if let Some(children) = symbol.get("children").and_then(Value::as_array) {
            pending.extend(
                children
                    .iter()
                    .take(4096usize.saturating_sub(pending.len())),
            );
        }
        let range = symbol
            .get("range")
            .or_else(|| symbol.get("location")?.get("range"));
        let Some(loc) = range.and_then(|range| location(path.to_path_buf(), text, range)) else {
            continue;
        };
        let p = (point.start.line, point.start.column);
        if (loc.start.line, loc.start.column) <= p && p < (loc.end.line, loc.end.column) {
            let smaller = best.as_ref().is_none_or(|best| {
                (
                    loc.end.line - loc.start.line,
                    loc.end.column.saturating_sub(loc.start.column),
                ) < (
                    best.location.end.line - best.location.start.line,
                    best.location
                        .end
                        .column
                        .saturating_sub(best.location.start.column),
                )
            });
            if smaller {
                best = Some(ContainingSymbol {
                    name: symbol
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("<unnamed>")
                        .to_string(),
                    kind: symbol
                        .get("kind")
                        .and_then(Value::as_u64)
                        .and_then(|kind| u32::try_from(kind).ok()),
                    location: loc,
                });
            }
        }
    }
    best
}

/// Line-only requests choose a backend-reported innermost symbol, using its
/// selectionRange rather than guessing an identifier from source text.
pub(super) fn symbols_at_line(
    outline: &Value,
    path: &Path,
    text: &str,
    line: u32,
) -> Vec<(SymbolCandidate, Value)> {
    let Some(symbols) = outline.as_array() else {
        return Vec::new();
    };
    let mut pending = symbols.iter().take(4096).collect::<Vec<_>>();
    let mut candidates = Vec::new();
    let mut visited = 0usize;
    while let Some(symbol) = pending.pop() {
        visited += 1;
        if visited > 4096 {
            break;
        }
        if let Some(children) = symbol.get("children").and_then(Value::as_array) {
            pending.extend(
                children
                    .iter()
                    .take(4096usize.saturating_sub(pending.len())),
            );
        }
        let Some(range) = symbol.get("range") else {
            continue;
        };
        let Some(scope) = location(path.to_path_buf(), text, range) else {
            continue;
        };
        let end_line =
            scope.end.line - u32::from(scope.end.column == 1 && scope.end.line > scope.start.line);
        if scope.start.line > line || end_line < line {
            continue;
        }
        let selected = symbol.get("selectionRange").unwrap_or(range);
        let Some(selected_location) = location(path.to_path_buf(), text, selected) else {
            continue;
        };
        candidates.push((
            scope,
            SymbolCandidate {
                name: symbol
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("<unnamed>")
                    .to_string(),
                kind: symbol
                    .get("kind")
                    .and_then(Value::as_u64)
                    .and_then(|kind| u32::try_from(kind).ok()),
                location: selected_location,
            },
            selected.get("start").cloned().unwrap_or(Value::Null),
        ));
        if candidates.len() >= MAX_ITEMS {
            break;
        }
    }
    let mut selected = candidates
        .iter()
        .filter(|(scope, _, _)| {
            !candidates.iter().any(|(other, _, _)| {
                let start = (scope.start.line, scope.start.column);
                let end = (scope.end.line, scope.end.column);
                let other_start = (other.start.line, other.start.column);
                let other_end = (other.end.line, other.end.column);
                start <= other_start
                    && other_end <= end
                    && (start != other_start || end != other_end)
            })
        })
        .map(|(_, candidate, position)| (candidate.clone(), position.clone()))
        .collect::<Vec<_>>();
    selected.sort_by_key(|(candidate, _)| {
        (
            candidate.location.start.line,
            candidate.location.start.column,
        )
    });
    selected.dedup_by(|(left, _), (right, _)| left.location == right.location);
    selected
}
