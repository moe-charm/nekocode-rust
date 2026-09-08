use super::capture::*;
use super::model::*;
use crate::{NekocodeError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_PACKET_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SymbolPacket {
    pub contract_version: String,
    pub artifact_kind: String,
    pub packet_id: String,
    pub integrity_sha256: String,
    pub root: PathBuf,
    pub inputs: InputInventory,
    pub sources: BTreeMap<PathBuf, CapturedSource>,
    pub response: SymbolContextV1,
}

fn integrity(packet: &SymbolPacket) -> Result<String> {
    let mut value = serde_json::to_value(packet)?;
    value["integrity_sha256"] = serde_json::Value::String(String::new());
    Ok(digest(&serde_json::to_vec(&value)?))
}

pub(super) fn save(path: &Path, packet: &mut SymbolPacket) -> Result<()> {
    // An explicitly named packet may replace a prior packet, but never source
    // captured by the investigation (including its Cargo inputs).
    if let Ok(existing) = path.canonicalize() {
        if let Ok(relative) = existing.strip_prefix(&packet.root) {
            if packet.inputs.files.contains_key(relative) || packet.sources.contains_key(relative) {
                return Err(NekocodeError::Config(
                    "--save-packet cannot overwrite a captured source or Cargo input".to_string(),
                ));
            }
        }
    }
    packet.integrity_sha256 = integrity(packet)?;
    let bytes = serde_json::to_vec(packet)?;
    if bytes.len() as u64 > MAX_PACKET_BYTES {
        return Err(NekocodeError::Config(
            "captured packet exceeds the 32 MiB storage limit".to_string(),
        ));
    }
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| NekocodeError::Config("packet path must name a file".to_string()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(
        ".{}.{}.{nonce}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub(super) fn read(path: &Path) -> Result<SymbolPacket> {
    let bytes = read_limited(path, MAX_PACKET_BYTES).map_err(NekocodeError::Config)?;
    let packet: SymbolPacket = serde_json::from_slice(&bytes)?;
    if packet.contract_version != "symbol-packet-v1"
        || packet.artifact_kind != "symbol_packet"
        || packet.response.contract_version != "symbol-context-v1"
        || packet.response.artifact_kind != "symbol_context"
    {
        return Err(NekocodeError::Config(
            "unsupported symbol packet contract or artifact identity".to_string(),
        ));
    }
    if packet.integrity_sha256.is_empty() || integrity(&packet)? != packet.integrity_sha256 {
        return Err(NekocodeError::Config(
            "symbol packet integrity mismatch".to_string(),
        ));
    }
    if packet.packet_id != packet.response.packet_id
        || !packet.root.is_absolute()
        || packet.response.items.len() > MAX_ITEMS
        || packet.inputs.files.len() > 20480
        || packet.sources.len() > 4096
        || packet.response.queries.len() > 1024
        || packet.response.target.candidates.len() > MAX_ITEMS
    {
        return Err(NekocodeError::Config(
            "symbol packet identity or collection bounds are invalid".to_string(),
        ));
    }
    let mut source_bytes = 0usize;
    for (path, source) in &packet.sources {
        source_bytes = source_bytes.saturating_add(source.text.len());
        if !safe_relative(path)
            || source.text.len() > 2 * 1024 * 1024
            || digest(source.text.as_bytes()) != source.sha256
        {
            return Err(NekocodeError::Config(
                "symbol packet source integrity or path is invalid".to_string(),
            ));
        }
    }
    if source_bytes > 16 * 1024 * 1024
        || packet.inputs.files.keys().any(|path| !safe_relative(path))
    {
        return Err(NekocodeError::Config(
            "symbol packet input bounds are invalid".to_string(),
        ));
    }
    Ok(packet)
}

fn safe_relative(path: &Path) -> bool {
    !path.is_absolute()
        && !path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
}

fn replay_freshness(packet: &SymbolPacket, response: &mut SymbolContextV1) {
    let mut now = inventory(&packet.root, packet.inputs.profile());
    refresh_captured_inputs(&packet.root, &packet.sources, &mut now);
    let (verification, changes) = super::explanation::verify_inputs(
        &packet.root,
        &packet.inputs,
        &now,
        &packet.sources,
        "packet_inputs_vs_current",
    );
    let verdict = verification.verdict.clone();
    response.freshness.scans = Some(ScanComparison {
        baseline: packet.inputs.scan.clone(),
        current: now.scan.clone(),
    });
    response.freshness.verification = Some(verification);
    response.freshness.checked_inputs = now.files.len();
    response.freshness.input_scan_complete = packet.inputs.complete && now.complete;
    // Preserve evidence of capture-time changes even when current inputs match.
    let mut known_changes: std::collections::BTreeSet<_> =
        response.freshness.changed_inputs.iter().cloned().collect();
    known_changes.extend(changes);
    response.freshness.changed_inputs = known_changes.into_iter().take(4096).collect();
    if verdict == "changed" {
        response.status = "stale".to_string();
        response.freshness.state = "stale".to_string();
        response.freshness.source_state = "changed".to_string();
        response.freshness.limitations.push(
            "Items are retained captured evidence; live inputs changed after capture.".to_string(),
        );
    } else if verdict == "unknown" {
        response.freshness.state = "unknown".to_string();
        response.freshness.source_state = "unknown".to_string();
    }
}

pub(super) fn page(
    packet: &SymbolPacket,
    request: &SymbolContextRequest,
    saved_path: Option<&Path>,
    replay: bool,
) -> Result<SymbolContextV1> {
    let mut response = packet.response.clone();
    if replay {
        replay_freshness(packet, &mut response);
    }
    response.coverage = Some(super::explanation::coverage(&response));
    let mut offset = 0usize;
    if let Some(cursor) = &request.cursor {
        let (id, start) = cursor
            .rsplit_once(':')
            .ok_or_else(|| NekocodeError::Config("invalid packet cursor".to_string()))?;
        offset = start
            .parse()
            .map_err(|_| NekocodeError::Config("invalid packet cursor offset".to_string()))?;
        if id != packet.packet_id || offset > packet.response.items.len() {
            return Err(NekocodeError::Config(
                "cursor belongs to another packet or exceeds captured items".to_string(),
            ));
        }
    }
    response.continuation = SymbolContinuation {
        packet: saved_path.map(Path::to_path_buf),
        next_cursor: None,
        expandable: saved_path.is_some(),
    };
    let expanded = request.item.is_some();
    if let Some(id) = request.item.as_ref() {
        let mut item = packet
            .response
            .items
            .iter()
            .find(|item| &item.id == id)
            .cloned()
            .ok_or_else(|| {
                NekocodeError::Config("item was not captured in this packet".to_string())
            })?;
        let source = packet
            .sources
            .get(&item.location.path)
            .ok_or_else(|| NekocodeError::Config("captured item source is missing".to_string()))?;
        let location = item
            .containing_symbol
            .as_ref()
            .map_or(&item.location, |scope| &scope.location);
        let context = if item.containing_symbol.is_some() {
            0
        } else {
            20
        };
        let (code, start, end, truncated) =
            excerpt(&source.text, location, context, 2 * 1024 * 1024);
        item.code = code;
        item.excerpt_start_line = start;
        item.excerpt_end_line = end;
        item.truncated = truncated;
        response.items = vec![item];
    } else {
        response.items = packet
            .response
            .items
            .iter()
            .skip(offset)
            .take(request.max_items)
            .cloned()
            .collect();
    }
    response.totals.displayed = response.items.len();
    response.budget = SymbolBudget {
        requested_tokens: request.budget,
        max_bytes: request.budget.saturating_mul(4),
        serialized_bytes: 0,
        exceeded: false,
    };
    refresh_navigation(&mut response, packet.response.items.len(), offset, expanded);
    fit_budget(&mut response, packet.response.items.len(), offset, expanded)?;
    Ok(response)
}

fn refresh_navigation(response: &mut SymbolContextV1, total: usize, offset: usize, expanded: bool) {
    response
        .omissions
        .retain(|item| item.kind != "display_items");
    response.totals.displayed = response.items.len();
    if !expanded {
        let next = offset + response.items.len();
        let remaining = total.saturating_sub(next);
        response.continuation.next_cursor =
            if remaining > 0 && response.continuation.packet.is_some() {
                Some(format!("{}:{next}", response.packet_id))
            } else {
                None
            };
        if remaining > 0 {
            response.omissions.push(SymbolOmission {
                kind: "display_items".to_string(),
                reason: if response.continuation.packet.is_some() {
                    "captured_in_saved_packet"
                } else {
                    "item_or_byte_limit_without_saved_packet"
                }
                .to_string(),
                count: remaining,
            });
        }
    }
}

fn settle_size(response: &mut SymbolContextV1) -> Result<usize> {
    // Serialized byte count includes its own decimal representation.
    for _ in 0..8 {
        let size = serde_json::to_vec(response)?.len();
        if response.budget.serialized_bytes == size {
            return Ok(size);
        }
        response.budget.serialized_bytes = size;
    }
    Ok(serde_json::to_vec(response)?.len())
}

fn fit_budget(
    response: &mut SymbolContextV1,
    total: usize,
    offset: usize,
    expanded: bool,
) -> Result<()> {
    loop {
        if settle_size(response)? <= response.budget.max_bytes {
            break;
        }
        // Derived prose and example paths yield before the actual code evidence.
        if let Some(coverage) = response.coverage.take() {
            response.omissions.push(SymbolOmission {
                kind: "coverage".into(),
                reason: "byte_budget".into(),
                count: coverage.len(),
            });
            continue;
        }
        if response.freshness.scans.take().is_some() {
            response.omissions.push(SymbolOmission {
                kind: "input_scans".into(),
                reason: "byte_budget".into(),
                count: 1,
            });
            continue;
        }
        if let Some(verification) = &mut response.freshness.verification {
            if !verification.issues.is_empty() {
                verification.issues_omitted += verification.issues.len();
                verification.issues.clear();
                continue;
            }
        }
        if response.items.len() > 1 {
            response.items.pop();
            refresh_navigation(response, total, offset, expanded);
            continue;
        }
        if let Some(item) = response.items.first_mut() {
            if item.code.len() > 128 {
                let mut end = item.code.len() / 2;
                while !item.code.is_char_boundary(end) {
                    end -= 1;
                }
                item.code.truncate(end);
                item.excerpt_end_line = item.excerpt_start_line
                    + item.code.split_inclusive('\n').count().saturating_sub(1) as u32;
                item.truncated = true;
                if !response.omissions.iter().any(|item| item.kind == "code") {
                    response.omissions.push(SymbolOmission {
                        kind: "code".to_string(),
                        reason: "byte_budget".to_string(),
                        count: 1,
                    });
                }
                continue;
            }
            if item.detail.take().is_some() {
                response.omissions.push(SymbolOmission {
                    kind: "contract_detail".to_string(),
                    reason: "byte_budget".to_string(),
                    count: 1,
                });
                continue;
            }
            response.items.clear();
            refresh_navigation(response, total, offset, expanded);
            continue;
        }
        if response.target.candidates.len() > 1 {
            response.target.candidates.pop();
            if let Some(omission) = response
                .omissions
                .iter_mut()
                .find(|item| item.kind == "display_candidates")
            {
                omission.count += 1;
            } else {
                response.omissions.push(SymbolOmission {
                    kind: "display_candidates".to_string(),
                    reason: "byte_budget".to_string(),
                    count: 1,
                });
            }
            continue;
        }
        // Failure/status/freshness information is mandatory. Its minimum
        // envelope may exceed a tiny requested budget, which is explicit.
        response.status = "output_limited".to_string();
        response.budget.exceeded = true;
        settle_size(response)?;
        break;
    }
    response.totals.displayed = response.items.len();
    if response.items.is_empty() && total > offset {
        response.continuation.next_cursor = None;
        response.status = "output_limited".to_string();
        response.omissions.push(SymbolOmission {
            kind: if expanded {
                "expanded_item"
            } else {
                "display_items_retry"
            }
            .to_string(),
            reason: "byte_budget_no_progress_increase_budget".to_string(),
            count: if expanded { 1 } else { total - offset },
        });
    }
    settle_size(response)?;
    if response.budget.serialized_bytes > response.budget.max_bytes {
        response.budget.exceeded = true;
        response.status = "output_limited".to_string();
        settle_size(response)?;
    }
    Ok(())
}
