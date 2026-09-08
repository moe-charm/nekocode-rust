use super::capture::digest;
use super::delta_model::*;
use super::model::*;
use super::packet::{read, SymbolPacket};
use crate::{NekocodeError, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

const BASIS: &str = "unique-source-anchor-v1";

fn references(packet: &SymbolPacket) -> Vec<&SymbolItem> {
    packet
        .response
        .items
        .iter()
        .filter(|item| item.relation == "reference")
        .collect()
}

fn observation(packet: &SymbolPacket, path: &Path) -> DeltaObservation {
    DeltaObservation {
        packet: path.to_path_buf(),
        packet_id: packet.packet_id.clone(),
        status_at_capture: packet.response.status.clone(),
        scope: packet.response.scope.clone(),
        backend: packet.response.backend.clone(),
        freshness_at_capture: packet.response.freshness.clone(),
        reference_query: packet
            .response
            .queries
            .iter()
            .find(|q| q.method == "textDocument/references")
            .cloned(),
        captured_references: references(packet).len(),
    }
}

fn capture_reasons(packet: &SymbolPacket, side: &str, reasons: &mut Vec<String>) {
    let response = &packet.response;
    if response.status != "completed" || !response.omissions.is_empty() {
        reasons.push(format!("{side}_capture_incomplete"));
    }
    if response.freshness.state != "source_stable" || !response.freshness.input_scan_complete {
        reasons.push(format!("{side}_capture_not_stable"));
    }
    if response.backend.health.as_deref() != Some("ok")
        || !response.backend.readiness_observed
        || !response.backend.quiescent
    {
        reasons.push(format!("{side}_backend_not_ready"));
    }
    let queries: Vec<_> = response
        .queries
        .iter()
        .filter(|q| q.method == "textDocument/references")
        .collect();
    if queries.len() != 1
        || queries[0].status != "completed"
        || queries[0].result_count != Some(references(packet).len())
    {
        reasons.push(format!("{side}_references_not_fully_captured"));
    }
}

fn line<'a>(packet: &'a SymbolPacket, path: &Path, number: u32) -> Option<&'a str> {
    packet
        .sources
        .get(path)?
        .text
        .split_inclusive('\n')
        .nth(number.checked_sub(1)? as usize)
}

fn unique_line(packet: &SymbolPacket, path: &Path, number: u32) -> Option<String> {
    let value = line(packet, path, number)?.trim_end_matches(['\r', '\n']);
    if value.is_empty() {
        return None;
    }
    let text = &packet.sources.get(path)?.text;
    (text
        .lines()
        .filter(|candidate| *candidate == value)
        .take(2)
        .count()
        == 1)
        .then(|| value.to_string())
}

fn target_identity(packet: &SymbolPacket) -> Option<String> {
    if packet.response.target.resolution != "selected" {
        return None;
    }
    let definitions: Vec<_> = packet
        .response
        .items
        .iter()
        .filter(|item| item.relation == "definition")
        .collect();
    if definitions.len() != 1 {
        return None;
    }
    let item = definitions[0];
    let scope = item.containing_symbol.as_ref()?;
    let declaration = unique_line(packet, &item.location.path, item.location.start.line)?;
    serde_json::to_string(&(
        &item.location.path,
        &scope.name,
        scope.kind,
        digest(declaration.as_bytes()),
    ))
    .ok()
    .map(|key| digest(key.as_bytes()))
}

fn configuration_inputs(packet: &SymbolPacket) -> BTreeMap<&PathBuf, &String> {
    packet
        .inputs
        .files
        .iter()
        .filter(|(path, _)| {
            matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("Cargo.toml" | "Cargo.lock" | "rust-toolchain" | "rust-toolchain.toml")
            ) || (path
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name == ".cargo")
                && matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some("config" | "config.toml")
                ))
        })
        .collect()
}

fn comparison_reasons(before: &SymbolPacket, after: &SymbolPacket) -> Result<Vec<String>> {
    let mut reasons = Vec::new();
    capture_reasons(before, "before", &mut reasons);
    capture_reasons(after, "after", &mut reasons);
    if before.root != after.root {
        reasons.push("workspace_mismatch".to_string());
    }
    if serde_json::to_value(&before.response.scope)? != serde_json::to_value(&after.response.scope)?
    {
        reasons.push("scope_mismatch".to_string());
    }
    if before.response.backend.version.is_none()
        || before.response.backend.version != after.response.backend.version
        || before.response.backend.name != after.response.backend.name
    {
        reasons.push("backend_version_mismatch_or_unknown".to_string());
    }
    if configuration_inputs(before) != configuration_inputs(after) {
        reasons.push("configuration_inputs_changed".to_string());
    }
    match (target_identity(before), target_identity(after)) {
        (Some(left), Some(right)) if left == right => {}
        _ => reasons.push("target_identity_changed_or_unresolved".to_string()),
    }
    Ok(reasons)
}

fn container(packet: &SymbolPacket, item: &SymbolItem) -> Option<String> {
    let scope = item.containing_symbol.as_ref()?;
    if scope.location.path != item.location.path {
        return None;
    }
    let declaration = unique_line(packet, &scope.location.path, scope.location.start.line)?;
    serde_json::to_string(&(
        &scope.location.path,
        &scope.name,
        scope.kind,
        digest(declaration.as_bytes()),
    ))
    .ok()
    .map(|key| digest(key.as_bytes()))
}

fn anchor(packet: &SymbolPacket, item: &SymbolItem) -> Option<(String, String)> {
    let container = container(packet, item)?;
    if item.location.start.line != item.location.end.line {
        return None;
    }
    let source_line = line(packet, &item.location.path, item.location.start.line)?;
    let key = serde_json::to_string(&(
        &container,
        digest(source_line.as_bytes()),
        item.location.start.column,
        item.location.end.column,
    ))
    .ok()?;
    Some((container, key))
}

fn evidence(packet: &SymbolPacket, item: &SymbolItem) -> DeltaEvidence {
    let source_line = line(packet, &item.location.path, item.location.start.line).unwrap_or("");
    let mut boundary = source_line.len().min(1024);
    while !source_line.is_char_boundary(boundary) {
        boundary -= 1;
    }
    DeltaEvidence {
        item_id: item.id.clone(),
        location: item.location.clone(),
        containing_symbol: item.containing_symbol.clone(),
        source_line: source_line[..boundary].to_string(),
        truncated: boundary < source_line.len(),
    }
}

fn change(
    kind: &str,
    reason: &str,
    before: Option<(&SymbolPacket, &SymbolItem)>,
    after: Option<(&SymbolPacket, &SymbolItem)>,
) -> ReferenceChange {
    ReferenceChange {
        change: kind.to_string(),
        reason: reason.to_string(),
        before: before.map(|(packet, item)| evidence(packet, item)),
        after: after.map(|(packet, item)| evidence(packet, item)),
    }
}

fn compare_references(before: &SymbolPacket, after: &SymbolPacket) -> Vec<ReferenceChange> {
    let left = references(before);
    let right = references(after);
    let mut left_keys: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut right_keys: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let left_anchors: Vec<_> = left.iter().map(|item| anchor(before, item)).collect();
    let right_anchors: Vec<_> = right.iter().map(|item| anchor(after, item)).collect();
    for (i, a) in left_anchors.iter().enumerate() {
        if let Some((_, key)) = a {
            left_keys.entry(key.clone()).or_default().push(i);
        }
    }
    for (i, a) in right_anchors.iter().enumerate() {
        if let Some((_, key)) = a {
            right_keys.entry(key.clone()).or_default().push(i);
        }
    }
    let mut used_left = BTreeSet::new();
    let mut used_right = BTreeSet::new();
    let mut changes = Vec::new();
    let keys: BTreeSet<_> = left_keys.keys().chain(right_keys.keys()).collect();
    for key in keys {
        let l = left_keys.get(key).map(Vec::as_slice).unwrap_or_default();
        let r = right_keys.get(key).map(Vec::as_slice).unwrap_or_default();
        if l.len() > 1 || r.len() > 1 {
            for i in l {
                used_left.insert(*i);
                changes.push(change(
                    "unresolved",
                    "duplicate_source_anchor",
                    Some((before, left[*i])),
                    None,
                ));
            }
            for i in r {
                used_right.insert(*i);
                changes.push(change(
                    "unresolved",
                    "duplicate_source_anchor",
                    None,
                    Some((after, right[*i])),
                ));
            }
        } else if let ([l], [r]) = (l, r) {
            used_left.insert(*l);
            used_right.insert(*r);
            changes.push(change(
                "matched",
                "same_unique_source_anchor",
                Some((before, left[*l])),
                Some((after, right[*r])),
            ));
        }
    }
    let left_uncertain =
        left_anchors.iter().any(Option::is_none) || left_keys.values().any(|v| v.len() > 1);
    let right_uncertain =
        right_anchors.iter().any(Option::is_none) || right_keys.values().any(|v| v.len() > 1);
    let identity = |item: &SymbolItem| {
        item.containing_symbol.as_ref().and_then(|scope| {
            serde_json::to_string(&(&scope.location.path, &scope.name, scope.kind)).ok()
        })
    };
    let left_containers: BTreeSet<_> = left
        .iter()
        .enumerate()
        .filter(|(i, _)| !used_left.contains(i))
        .filter_map(|(_, item)| identity(item))
        .collect();
    let right_containers: BTreeSet<_> = right
        .iter()
        .enumerate()
        .filter(|(i, _)| !used_right.contains(i))
        .filter_map(|(_, item)| identity(item))
        .collect();
    for (i, item) in left.iter().enumerate() {
        if used_left.contains(&i) {
            continue;
        }
        let (kind, reason) = match &left_anchors[i] {
            None => ("unresolved", "missing_or_ambiguous_source_anchor"),
            Some(_) if right_uncertain => ("unresolved", "opposite_anchor_unresolved"),
            Some(_) if identity(item).is_some_and(|c| right_containers.contains(&c)) => {
                ("unresolved", "edited_container_requires_review")
            }
            _ => ("removed", "before_only_reference_observation"),
        };
        changes.push(change(kind, reason, Some((before, item)), None));
    }
    for (i, item) in right.iter().enumerate() {
        if used_right.contains(&i) {
            continue;
        }
        let (kind, reason) = match &right_anchors[i] {
            None => ("unresolved", "missing_or_ambiguous_source_anchor"),
            Some(_) if left_uncertain => ("unresolved", "opposite_anchor_unresolved"),
            Some(_) if identity(item).is_some_and(|c| left_containers.contains(&c)) => {
                ("unresolved", "edited_container_requires_review")
            }
            _ => ("added", "after_only_reference_observation"),
        };
        changes.push(change(kind, reason, None, Some((after, item))));
    }
    changes.sort_by_key(|c| match c.change.as_str() {
        "added" => 0,
        "removed" => 1,
        "unresolved" => 2,
        _ => 3,
    });
    changes
}

fn settle_size(delta: &mut SymbolDeltaV1) -> Result<usize> {
    loop {
        let size = serde_json::to_vec(delta)?.len();
        if size == delta.budget.serialized_bytes {
            return Ok(size);
        }
        delta.budget.serialized_bytes = size;
    }
}

/// Compare saved observations. This path never starts a backend or reads live source.
pub fn build_symbol_delta(request: &SymbolDeltaRequest) -> Result<SymbolDeltaV1> {
    if !(1..=1_000_000).contains(&request.budget) || !(1..=100).contains(&request.max_items) {
        return Err(NekocodeError::Config(
            "delta budget must be 1..1000000 and max-items 1..100".into(),
        ));
    }
    let before = read(&request.before)?;
    let after = read(&request.after)?;
    if let Some(path) = &request.path {
        if path.canonicalize()? != after.root {
            return Err(NekocodeError::Config(
                "explicit delta workspace does not match after packet".into(),
            ));
        }
    }
    let reasons = comparison_reasons(&before, &after)?;
    let comparable = reasons.is_empty();
    let changes = if comparable {
        compare_references(&before, &after)
    } else {
        Vec::new()
    };
    let count = |kind| {
        comparable.then(|| {
            changes
                .iter()
                .filter(|change| change.change == kind)
                .count()
        })
    };
    let id = digest(
        format!(
            "{BASIS}:{}:{}",
            before.integrity_sha256, after.integrity_sha256
        )
        .as_bytes(),
    )
    .replace("sha256:", "sd_");
    let mut delta = SymbolDeltaV1 {
        contract_version: "symbol-delta-v1".into(), artifact_kind: "symbol_delta".into(), comparison_id: id.clone(),
        status: if !comparable { "not_comparable" } else if count("unresolved") != Some(0) { "partial" } else { "completed" }.into(),
        comparison_status: if comparable { "observed_comparable" } else { "not_comparable" }.into(),
        matching_basis: BASIS.into(), before: observation(&before, &request.before), after: observation(&after, &request.after), reasons,
        totals: DeltaTotals { added: count("added"), removed: count("removed"), matched: count("matched"), unresolved: count("unresolved"), displayed: 0, omitted: 0 },
        changes: Vec::new(), next_cursor: None,
        budget: SymbolBudget { requested_tokens: request.budget, max_bytes: request.budget * 4, serialized_bytes: 0, exceeded: false },
        limitations: vec!["Comparison uses captured observations, not current source or Git history; no backend is started.".into(),
            "Matching captured conditions does not verify external configuration or a common backend analysis generation.".into(),
            "Host/target and effective external compiler configuration are not recorded by symbol-packet-v1.".into(),
            "Reference queries can miss macro/indirect uses or include inactive cfg; absence is not safe deletion proof.".into(),
            "Source anchors are not semantic rename tracking. Unresolved counts count individual observations on either side.".into()],
    };
    let offset = if let Some(cursor) = &request.cursor {
        let (cursor_id, offset) = cursor
            .rsplit_once(':')
            .ok_or_else(|| NekocodeError::Config("invalid delta cursor".into()))?;
        let offset = offset
            .parse::<usize>()
            .map_err(|_| NekocodeError::Config("invalid delta cursor offset".into()))?;
        if cursor_id != id || offset > changes.len() {
            return Err(NekocodeError::Config(
                "delta cursor belongs to different packets or exceeds changes".into(),
            ));
        }
        offset
    } else {
        0
    };
    let total = changes.len();
    delta.changes = changes
        .into_iter()
        .skip(offset)
        .take(request.max_items)
        .collect();
    loop {
        delta.totals.displayed = delta.changes.len();
        delta.totals.omitted = total - delta.changes.len();
        let next = offset + delta.changes.len();
        delta.next_cursor =
            (next < total && !delta.changes.is_empty()).then(|| format!("{id}:{next}"));
        if delta.changes.is_empty() && offset < total {
            delta.status = "output_limited".into();
            if !delta
                .reasons
                .iter()
                .any(|reason| reason == "byte_budget_no_progress_increase_budget")
            {
                delta
                    .reasons
                    .push("byte_budget_no_progress_increase_budget".into());
            }
        }
        if settle_size(&mut delta)? <= delta.budget.max_bytes {
            break;
        }
        if delta.changes.pop().is_none() {
            delta.status = "output_limited".into();
            delta.budget.exceeded = true;
            settle_size(&mut delta)?;
            break;
        }
    }
    Ok(delta)
}

pub fn format_symbol_delta_summary(delta: &SymbolDeltaV1) -> String {
    let mut out = format!(
        "NekoCode reference comparison\nStatus: {}; comparison: {}\n",
        delta.status, delta.comparison_status
    );
    let _ = writeln!(
        out,
        "Before: {} ({})\nAfter: {} ({})",
        delta.before.packet.display(),
        delta.before.packet_id,
        delta.after.packet.display(),
        delta.after.packet_id
    );
    let show = |n: Option<usize>| n.map_or_else(|| "unknown".to_string(), |n| n.to_string());
    let _ = writeln!(
        out,
        "Added: {}; removed: {}; matched: {}; unresolved observations: {}",
        show(delta.totals.added),
        show(delta.totals.removed),
        show(delta.totals.matched),
        show(delta.totals.unresolved)
    );
    for reason in &delta.reasons {
        let _ = writeln!(out, "Reason: {reason}");
    }
    for change in &delta.changes {
        let _ = writeln!(out, "\n{}: {}", change.change, change.reason);
        for (side, item) in [("before", &change.before), ("after", &change.after)] {
            if let Some(item) = item {
                let _ = writeln!(
                    out,
                    "{side} {}:{}:{} [{}]\n{}",
                    item.location.path.display(),
                    item.location.start.line,
                    item.location.start.column,
                    item.item_id,
                    item.source_line
                );
                if item.truncated {
                    let _ = writeln!(
                        out,
                        "[source line truncated; expand the original packet item]"
                    );
                }
            }
        }
    }
    let _ = writeln!(
        out,
        "Displayed: {}; omitted: {}; budget: {}/{} bytes",
        delta.totals.displayed,
        delta.totals.omitted,
        delta.budget.serialized_bytes,
        delta.budget.max_bytes
    );
    if let Some(cursor) = &delta.next_cursor {
        let _ = writeln!(out, "Next cursor: {cursor}");
    }
    for limitation in &delta.limitations {
        let _ = writeln!(out, "Limit: {limitation}");
    }
    super::terminal(&out)
}
