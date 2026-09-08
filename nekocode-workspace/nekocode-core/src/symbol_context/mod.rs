//! Rust-analyzer-backed investigation and bounded, replayable evidence packets.
//! Semantic relationships come only from the backend; source text stays exact.

mod explanation;
mod links;
mod session;
mod text_candidates;
pub use session::{ReuseReport, SymbolSession};
mod capture;
mod delta;
mod delta_model;
pub use delta::{build_symbol_delta, format_symbol_delta_summary};
pub use delta_model::*;
mod collect;
mod lsp;
mod model;
mod packet;

use crate::{NekocodeError, Result};
pub use model::*;
use std::fmt::Write;

pub fn build_symbol_context(request: &SymbolContextRequest) -> Result<SymbolContextV1> {
    build_with_session(request, &mut SymbolSession::default())
}

fn build_with_session(
    request: &SymbolContextRequest,
    session: &mut SymbolSession,
) -> Result<SymbolContextV1> {
    if request
        .scan_profile
        .as_deref()
        .is_some_and(|p| !matches!(p, "default" | "large"))
    {
        return Err(NekocodeError::Config(
            "scan-profile must be default or large".into(),
        ));
    }
    if request.budget == 0 || request.budget > 1_000_000 {
        return Err(NekocodeError::Config(
            "symbol context budget must be between 1 and 1000000".to_string(),
        ));
    }
    if !(1..=100).contains(&request.max_items) || !(1..=600).contains(&request.timeout_seconds) {
        return Err(NekocodeError::Config(
            "max-items must be 1..100 and timeout-seconds must be 1..600".to_string(),
        ));
    }
    let selectors = usize::from(request.at.is_some())
        + usize::from(request.symbol.is_some())
        + usize::from(request.packet.is_some());
    if selectors != 1 {
        return Err(NekocodeError::Config(
            "choose exactly one of --at, --symbol, or --packet".to_string(),
        ));
    }
    if request.item.is_some() && request.cursor.is_some() {
        return Err(NekocodeError::Config(
            "--item and --cursor are mutually exclusive".to_string(),
        ));
    }
    if let Some(path) = request.packet.as_deref() {
        if request.save_packet.is_some()
            || request.all_features
            || request.allow_build_scripts
            || request.text_candidates
            || request.scan_profile.is_some()
        {
            return Err(NekocodeError::Config(
                "saved packet replay cannot change backend options or save another packet"
                    .to_string(),
            ));
        }
        let captured = packet::read(path)?;
        if let Some(requested_root) = request.path.as_deref() {
            let requested_root = requested_root.canonicalize().map_err(|error| {
                NekocodeError::Config(format!("explicit replay workspace cannot be read: {error}"))
            })?;
            if requested_root != captured.root {
                return Err(NekocodeError::Config(
                    "explicit replay workspace does not match the saved packet workspace"
                        .to_string(),
                ));
            }
        }
        return packet::page(&captured, request, Some(path), true);
    }
    if request.item.is_some() || request.cursor.is_some() {
        return Err(NekocodeError::Config(
            "--item and --cursor require --packet".to_string(),
        ));
    }
    if request
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.trim().is_empty() || symbol.len() > 1024)
    {
        return Err(NekocodeError::Config(
            "--symbol must be a nonempty name of at most 1024 bytes".to_string(),
        ));
    }
    let mut captured = collect::collect(request, session)?;
    if let Some(path) = request.save_packet.as_deref() {
        packet::save(path, &mut captured)?;
    }
    packet::page(&captured, request, request.save_packet.as_deref(), false)
}

fn terminal(text: &str) -> String {
    text.chars()
        .flat_map(|character| {
            if character.is_control() && character != '\n' && character != '\t' {
                character.escape_default().collect::<Vec<_>>()
            } else {
                vec![character]
            }
        })
        .collect()
}

/// A terminal-safe projection. JSON remains the exact source representation.
pub fn format_symbol_context_summary(response: &SymbolContextV1) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "NekoCode symbol context");
    let _ = writeln!(
        output,
        "Status: {}; target: {}",
        response.status, response.target.resolution
    );
    let _ = writeln!(
        output,
        "Freshness: {}; backend synchronization: {}",
        response.freshness.state, response.freshness.backend_synchronization
    );
    if let Some(scans) = &response.freshness.scans {
        for (phase, scan) in [("baseline", &scans.baseline), ("current", &scans.current)] {
            if let Some(s) = scan {
                let _ = writeln!(output, "Input scan {phase} ({}): complete={}; entries {}/{}; files {}/{}; bytes {}/{}; per-file limit {}", s.profile, s.complete, s.examined_entries, s.max_entries, s.hashed_files, s.max_files, s.hashed_bytes, s.max_bytes, s.max_file_bytes);
                if let Some(links) = &s.link_scope {
                    let _ = writeln!(output, "Symlink scope: {} verified, {} excluded, {} unverified; {} examples omitted", links.verified, links.excluded, links.unverified, links.examples_omitted);
                    for example in &links.examples {
                        let _ = writeln!(
                            output,
                            "  {}: {} -> {:?}; resolved {:?}",
                            example.mapping.status,
                            example.path.display(),
                            example.mapping.target,
                            example.mapping.resolved
                        );
                    }
                }
                for issue in &s.issues {
                    let _ = writeln!(
                        output,
                        "Scan issue: {} {}",
                        issue.status,
                        issue.path.display()
                    );
                }
                if s.issues_omitted > 0 {
                    let _ = writeln!(output, "Scan issue examples omitted: {}", s.issues_omitted);
                }
            }
        }
    }
    if let Some(v) = &response.freshness.verification {
        let _ = writeln!(output, "Input verification ({}): {}; matched {}, modified {}, missing {}, unreadable {}, unobserved {}, newly observed {}, captured mismatches {}; scans complete: {}/{}",
            v.basis, v.verdict, v.matched, v.modified, v.missing, v.unreadable, v.unobserved, v.newly_observed, v.captured_mismatches, v.baseline_scan_complete, v.current_scan_complete);
        if let Some(count) = v.link_changes {
            let _ = writeln!(output, "Confirmed symlink mapping changes: {count}");
        }
        for issue in &v.issues {
            let _ = writeln!(output, "  {}: {}", issue.status, issue.path.display());
        }
        if v.issues_omitted > 0 {
            let _ = writeln!(
                output,
                "  {} additional input issues omitted",
                v.issues_omitted
            );
        }
    }
    if let Some(coverage) = &response.coverage {
        for entry in coverage {
            let _ = writeln!(
                output,
                "Coverage {}: requested {}; observed {}; verification {}. {}",
                entry.area, entry.requested, entry.observed, entry.verification, entry.limitation
            );
        }
    }
    let _ = writeln!(
        output,
        "Backend: {}; startup {} ms; observation {} ms",
        response.backend.name, response.backend.startup_ms, response.backend.observation_ms
    );
    if let Some(message) = &response.backend.message {
        let _ = writeln!(output, "Backend detail: {message}");
    }
    let _ = writeln!(
        output,
        "Items: {} displayed / {} retained / {} observed",
        response.totals.displayed, response.totals.retained, response.totals.observed
    );
    for candidate in &response.target.candidates {
        let _ = writeln!(
            output,
            "Candidate: {} at {}:{}:{}",
            candidate.name,
            candidate.location.path.display(),
            candidate.location.start.line,
            candidate.location.start.column
        );
    }
    for item in &response.items {
        let _ = writeln!(
            output,
            "\n{} [{}] {}:{}:{}",
            item.id,
            item.relation,
            item.location.path.display(),
            item.location.start.line,
            item.location.start.column
        );
        if let Some(scope) = &item.containing_symbol {
            let _ = writeln!(output, "Containing symbol: {}", scope.name);
        }
        let _ = writeln!(output, "{}", item.reason);
        if let Some(detail) = &item.detail {
            let _ = writeln!(output, "{detail}");
        }
        let _ = writeln!(output, "{}", item.code);
        if item.truncated {
            let _ = writeln!(output, "[excerpt truncated]");
        }
    }
    for query in &response.queries {
        let _ = writeln!(
            output,
            "Query {}: {}{}",
            query.method,
            query.status,
            query
                .result_count
                .map(|count| format!(" ({count} results)"))
                .unwrap_or_default()
        );
        if let Some(detail) = &query.detail {
            let _ = writeln!(output, "{detail}");
        }
    }
    for omission in &response.omissions {
        let _ = writeln!(
            output,
            "Omitted {}: {} ({})",
            omission.kind, omission.count, omission.reason
        );
    }
    let _ = writeln!(
        output,
        "Budget: {}/{} bytes{}",
        response.budget.serialized_bytes,
        response.budget.max_bytes,
        if response.budget.exceeded {
            " (envelope exceeds limit)"
        } else {
            ""
        }
    );
    if let Some(cursor) = &response.continuation.next_cursor {
        let _ = writeln!(output, "Next cursor: {cursor}");
    }
    if let Some(packet) = &response.continuation.packet {
        let _ = writeln!(output, "Packet: {}", packet.display());
    }
    for limitation in response
        .limitations
        .iter()
        .chain(&response.freshness.limitations)
    {
        let _ = writeln!(output, "Limit: {limitation}");
    }
    terminal(&output)
}

#[cfg(test)]
mod timeout_validation_tests {
    use super::*;

    #[test]
    fn timeout_bounds_are_checked_before_backend_start() {
        for seconds in [1, 120, 300, 600] {
            let request = SymbolContextRequest {
                timeout_seconds: seconds,
                ..Default::default()
            };
            let error = build_symbol_context(&request).unwrap_err().to_string();
            assert!(error.contains("choose exactly one"), "{seconds}: {error}");
        }
        for seconds in [0, 601, u64::MAX] {
            let request = SymbolContextRequest {
                timeout_seconds: seconds,
                ..Default::default()
            };
            let error = build_symbol_context(&request).unwrap_err().to_string();
            assert!(
                error.contains("timeout-seconds must be 1..600"),
                "{seconds}: {error}"
            );
        }
    }
}
