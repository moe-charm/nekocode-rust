use super::capture::*;
use super::lsp::{path_to_uri, unicode_position, uri_to_path, RaClient, RaError, RaOptions};
use super::model::*;
use super::packet::SymbolPacket;
use crate::{index_rust_workspace, NekocodeError, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn query(
    client: &mut RaClient,
    queries: &mut Vec<SymbolQuery>,
    method: &str,
    params: Value,
) -> Option<Value> {
    match client.request(method, params) {
        Ok(value) => {
            let count = if value.is_null() {
                0
            } else {
                value.as_array().map_or(1, Vec::len)
            };
            queries.push(SymbolQuery {
                method: method.to_string(),
                status: "completed".to_string(),
                result_count: Some(count),
                detail: None,
            });
            Some(value)
        }
        Err(error) => {
            queries.push(SymbolQuery {
                method: method.to_string(),
                status: error_status(&error).to_string(),
                result_count: None,
                detail: Some(error.to_string()),
            });
            None
        }
    }
}

fn error_status(error: &RaError) -> &'static str {
    match error {
        RaError::Unavailable(_) => "backend_unavailable",
        RaError::Unsupported(_) => "unsupported",
        RaError::Timeout(_) => "timed_out",
        RaError::Failed(_) => "failed",
        RaError::OutputLimited(_) => "output_limited",
    }
}

fn parse_at(at: &str) -> Result<(PathBuf, u32, u32)> {
    let (prefix, last) = at.rsplit_once(':').ok_or_else(|| {
        NekocodeError::Config(
            "--at requires FILE:LINE or FILE:LINE:COLUMN (one-based Unicode columns)".to_string(),
        )
    })?;
    let last: u32 = last
        .parse()
        .map_err(|_| NekocodeError::Config("invalid --at position".to_string()))?;
    let (file, line, column) = match prefix.rsplit_once(':') {
        Some((file, line)) if line.parse::<u32>().is_ok() => {
            (file, line.parse::<u32>().unwrap_or(0), last)
        }
        _ => (prefix, last, 1),
    };
    if file.is_empty() || line == 0 || column == 0 {
        return Err(NekocodeError::Config(
            "--at requires a file and positive line/column".to_string(),
        ));
    }
    Ok((PathBuf::from(file), line, column))
}

fn read_location(
    value: &Value,
    sources: &mut Sources,
) -> std::result::Result<(SymbolLocation, String), String> {
    let uri = value
        .get("uri")
        .or_else(|| value.get("targetUri"))
        .and_then(Value::as_str)
        .ok_or_else(|| "backend location has no file URI".to_string())?;
    let path = uri_to_path(uri).map_err(|error| error.to_string())?;
    let (relative, text) = sources.text(&path)?;
    let range = value
        .get("range")
        .or_else(|| value.get("targetSelectionRange"))
        .or_else(|| value.get("targetRange"))
        .ok_or_else(|| "backend location has no range".to_string())?;
    let location = location(relative, &text, range)
        .ok_or_else(|| "backend range does not fit captured UTF-8 source".to_string())?;
    Ok((location, text))
}

fn hover_text(value: &Value) -> Option<String> {
    fn content(value: &Value) -> Option<String> {
        match value {
            Value::String(text) => Some(text.clone()),
            Value::Object(value) => value
                .get("value")
                .and_then(Value::as_str)
                .map(str::to_string),
            Value::Array(values) => Some(
                values
                    .iter()
                    .filter_map(content)
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            ),
            _ => None,
        }
    }
    content(value.get("contents")?).filter(|text| !text.is_empty())
}

struct ItemCollector<'a> {
    client: &'a mut RaClient,
    sources: &'a mut Sources,
    response: &'a mut SymbolContextV1,
    outlines: BTreeMap<PathBuf, Value>,
    seen: BTreeSet<String>,
}

impl ItemCollector<'_> {
    fn add_locations(&mut self, relation: &str, value: &Value) {
        match value {
            Value::Array(values) => {
                for location in values.iter().take(MAX_ITEMS + 1) {
                    self.add(relation, location, None);
                }
                let extra = values.len().saturating_sub(MAX_ITEMS + 1);
                if extra > 0 {
                    self.response.totals.observed += extra;
                    self.omit_count("items", "collection_limit", extra);
                }
            }
            Value::Object(_) => self.add(relation, value, None),
            _ => {}
        }
    }

    fn add(&mut self, relation: &str, raw_location: &Value, detail: Option<String>) {
        self.response.totals.observed += 1;
        if self.response.items.len() >= MAX_ITEMS {
            self.omit("items", "collection_limit");
            return;
        }
        let (location, text) = match read_location(raw_location, self.sources) {
            Ok(result) => result,
            Err(reason) => {
                self.omit("source", &reason);
                return;
            }
        };
        let key = format!(
            "{relation}:{}:{}:{}:{}:{}",
            location.path.display(),
            location.start.line,
            location.start.column,
            location.end.line,
            location.end.column
        );
        if !self.seen.insert(key) {
            return;
        }
        if !self.outlines.contains_key(&location.path) && self.outlines.len() < 24 {
            let absolute = self.sources.root.join(&location.path);
            if let Ok(uri) = path_to_uri(&absolute) {
                let method = self.client.document_notification_method(&absolute);
                if let Err(error) = self.client.open_document(&absolute, &text) {
                    self.response.queries.push(SymbolQuery {
                        method: method.to_string(),
                        status: error_status(&error).to_string(),
                        result_count: None,
                        detail: Some(error.to_string()),
                    });
                }
                let outline = query(
                    self.client,
                    &mut self.response.queries,
                    "textDocument/documentSymbol",
                    json!({"textDocument":{"uri":uri}}),
                )
                .unwrap_or(Value::Null);
                self.outlines.insert(location.path.clone(), outline);
            }
        }
        let scope = self
            .outlines
            .get(&location.path)
            .and_then(|outline| containing_symbol(outline, &location.path, &text, &location));
        let (code, start, end, truncated) = excerpt(&text, &location, 4, 8192);
        self.response.items.push(SymbolItem {
            id: String::new(),
            relation: relation.to_string(),
            backend: "rust-analyzer".to_string(),
            source: match relation {
                "definition" => "textDocument/definition",
                "reference" => "textDocument/references",
                "type_definition" => "textDocument/typeDefinition",
                "test_candidate" => "rust-analyzer/relatedTests",
                "contract" => "textDocument/hover",
                _ => "requested_position",
            }
            .to_string(),
            location,
            reason: match relation {
                "reference" => "Backend-observed reference; not proof of an executed call.",
                "test_candidate" => "Backend-related test candidate; the test was not executed.",
                "contract" => "Type/documentation information returned by the semantic backend.",
                "type_definition" => "Type definition returned by the semantic backend.",
                "selected_source" => {
                    "Requested source position; definition query did not identify a definition."
                }
                _ => "Definition returned by the semantic backend.",
            }
            .to_string(),
            code,
            excerpt_start_line: start,
            excerpt_end_line: end,
            containing_symbol: scope,
            detail,
            truncated,
        });
    }

    fn omit(&mut self, kind: &str, reason: &str) {
        self.omit_count(kind, reason, 1);
    }

    fn omit_count(&mut self, kind: &str, reason: &str, count: usize) {
        if let Some(omission) = self
            .response
            .omissions
            .iter_mut()
            .find(|item| item.kind == kind && item.reason == reason)
        {
            omission.count += count;
        } else if self.response.omissions.len() < 32 {
            self.response.omissions.push(SymbolOmission {
                kind: kind.to_string(),
                reason: reason.to_string(),
                count,
            });
        }
    }
}

pub(super) fn collect(
    request: &SymbolContextRequest,
    session: &mut super::SymbolSession,
) -> Result<SymbolPacket> {
    let workspace = index_rust_workspace(request.path.as_deref().unwrap_or(Path::new(".")))?;
    let root = workspace.workspace_root;
    let mut before = inventory(&root);
    let mut sources = Sources::new(&root);
    if let Some(at) = &request.at {
        let (path, line, column) = parse_at(at)?;
        let (relative, text) = sources.text(&path).map_err(NekocodeError::Config)?;
        unicode_position(&text, line, column)
            .map_err(|error| NekocodeError::Config(error.to_string()))?;
        before
            .files
            .entry(relative)
            .or_insert_with(|| digest(text.as_bytes()));
    }
    session.refresh_additional_inputs(&root, &mut before, &mut sources);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let id = digest(format!("{}:{nonce}:{}", root.display(), std::process::id()).as_bytes());
    let packet_id = format!("sp_{}", &id[7..31]);
    let mut response = SymbolContextV1 {
        coverage: None,
        contract_version: "symbol-context-v1".to_string(), artifact_kind: "symbol_context".to_string(),
        packet_id, status: "completed".to_string(),
        target: SymbolTarget { requested_at: request.at.clone(), requested_symbol: request.symbol.clone(),
            resolution: "unresolved".to_string(), location: None, candidates: Vec::new() },
        scope: SymbolScope { workspace_root: root.clone(), features: if request.all_features { "all_features" } else { "default_features" }.to_string(),
            cfg_test: "requested_true_unverified".to_string(), build_scripts: request.allow_build_scripts,
            proc_macros: request.allow_build_scripts, tests_executed: false, hop_limit: 1,
            external_configuration: "unobserved".to_string() },
        backend: SymbolBackend { name: "rust-analyzer".to_string(), version: None, health: None, message: None, quiescent: false,
            readiness_observed: false, startup_ms: 0, observation_ms: 0 },
        queries: Vec::new(), freshness: SymbolFreshness { verification: None, state: "unknown".to_string(), source_state: "unknown".to_string(),
            backend_synchronization: "unverified".to_string(), checked_inputs: before.files.len(), changed_inputs: Vec::new(),
            input_scan_complete: before.complete, limitations: vec![
                "Stable files and backend quiescence do not prove a common semantic analysis generation.".to_string(),
                "External dependencies, configuration, generated inputs and environment are outside the source freshness guarantee.".to_string(),
            ] },
        items: Vec::new(), totals: SymbolTotals { observed: 0, retained: 0, displayed: 0 }, omissions: Vec::new(),
        continuation: SymbolContinuation { packet: None, next_cursor: None, expandable: false },
        budget: SymbolBudget { requested_tokens: request.budget, max_bytes: request.budget.saturating_mul(4), serialized_bytes: 0, exceeded: false },
        limitations: vec!["One-hop backend observations are navigation evidence, not a complete impact or call graph.".to_string(),
            "A completed query means protocol completion, not coverage proof: backend references/test candidates may include inactive cfg branches and miss macro or indirect uses.".to_string(),
            "Scope options were requested through LSP initialization; external backend configuration was not independently audited.".to_string(),
            "Build scripts and procedural macros require explicit --allow-build-scripts; this option is not an OS sandbox.".to_string()],
    };
    let start = Instant::now();
    let options = RaOptions {
        timeout: Duration::from_secs(request.timeout_seconds),
        all_features: request.all_features,
        allow_build_scripts: request.allow_build_scripts,
    };
    let mut client = match session.acquire(&root, &before, &options) {
        Ok(client) => client,
        Err(error) => {
            response.status = match error_status(&error) {
                "failed" | "unsupported" => "partial",
                status => status,
            }
            .to_string();
            response.queries.push(SymbolQuery {
                method: "initialize".to_string(),
                status: error_status(&error).to_string(),
                result_count: None,
                detail: Some(error.to_string()),
            });
            response.backend.startup_ms = start.elapsed().as_millis() as u64;
            return finish(response, root, before, sources);
        }
    };
    response.backend.startup_ms = start.elapsed().as_millis() as u64;
    let observation_start = Instant::now();
    let selected = if let Some(at) = request.at.as_deref() {
        let (file, line, column) = parse_at(at)?;
        let (relative, text) = sources.text(&file).map_err(NekocodeError::Config)?;
        let position = unicode_position(&text, line, column)
            .map_err(|error| NekocodeError::Config(error.to_string()))?;
        let column_was_supplied = at
            .rsplit_once(':')
            .and_then(|(prefix, _)| prefix.rsplit_once(':'))
            .is_some_and(|(_, line)| line.parse::<u32>().is_ok());
        if column_was_supplied {
            Some((relative, text, position))
        } else {
            let absolute = root.join(&relative);
            let uri =
                path_to_uri(&absolute).map_err(|error| NekocodeError::Config(error.to_string()))?;
            let method = client.document_notification_method(&absolute);
            if let Err(error) = client.open_document(&absolute, &text) {
                response.queries.push(SymbolQuery {
                    method: method.to_string(),
                    status: error_status(&error).to_string(),
                    result_count: None,
                    detail: Some(error.to_string()),
                });
            }
            let outline = query(
                &mut client,
                &mut response.queries,
                "textDocument/documentSymbol",
                json!({"textDocument":{"uri":uri}}),
            );
            let mut symbols = outline
                .as_ref()
                .map(|outline| symbols_at_line(outline, &relative, &text, line))
                .unwrap_or_default();
            match symbols.len() {
                0 => Some((relative, text, position)),
                1 => {
                    let (_, selected_position) = symbols.remove(0);
                    Some((relative, text, selected_position))
                }
                _ => {
                    response.target.resolution = "ambiguous".to_string();
                    response.status = "ambiguous".to_string();
                    response.target.candidates = symbols
                        .into_iter()
                        .map(|(candidate, _)| candidate)
                        .collect();
                    None
                }
            }
        }
    } else {
        let name = request.symbol.as_deref().unwrap_or_default();
        let matches = query(
            &mut client,
            &mut response.queries,
            "workspace/symbol",
            json!({"query":name}),
        );
        let mut exact = Vec::new();
        let query_succeeded = matches.is_some();
        if let Some(Value::Array(symbols)) = matches {
            if symbols.len() > MAX_ITEMS {
                response.omissions.push(SymbolOmission {
                    kind: "symbol_candidates".to_string(),
                    reason: "collection_limit".to_string(),
                    count: symbols.len() - MAX_ITEMS,
                });
            }
            for symbol in symbols.iter().take(MAX_ITEMS) {
                if symbol.get("name").and_then(Value::as_str) != Some(name) {
                    continue;
                }
                if let Some(raw) = symbol.get("location") {
                    match read_location(raw, &mut sources) {
                        Ok((loc, text)) => {
                            if exact.iter().any(
                                |(candidate, _, _): &(SymbolCandidate, String, Value)| {
                                    candidate.location == loc
                                },
                            ) {
                                continue;
                            }
                            let position = raw
                                .get("range")
                                .and_then(|range| range.get("start"))
                                .cloned()
                                .unwrap_or(Value::Null);
                            exact.push((
                                SymbolCandidate {
                                    name: name.to_string(),
                                    kind: symbol
                                        .get("kind")
                                        .and_then(Value::as_u64)
                                        .and_then(|kind| u32::try_from(kind).ok()),
                                    location: loc,
                                },
                                text,
                                position,
                            ));
                        }
                        Err(reason) => response.omissions.push(SymbolOmission {
                            kind: "symbol_candidates".to_string(),
                            reason,
                            count: 1,
                        }),
                    }
                } else {
                    response.omissions.push(SymbolOmission {
                        kind: "symbol_candidates".to_string(),
                        reason: "backend symbol location was not acquired".to_string(),
                        count: 1,
                    });
                }
            }
        }
        if exact.len() == 1 && response.omissions.is_empty() {
            let (candidate, text, position) = exact.remove(0);
            Some((candidate.location.path, text, position))
        } else {
            response.target.resolution =
                if !query_succeeded || (exact.is_empty() && !response.omissions.is_empty()) {
                    "unresolved"
                } else if exact.is_empty() {
                    "not_found"
                } else {
                    "ambiguous"
                }
                .to_string();
            response.status = if response.target.resolution == "unresolved" {
                "partial".to_string()
            } else {
                response.target.resolution.clone()
            };
            response.target.candidates = exact
                .into_iter()
                .map(|(candidate, _, _)| candidate)
                .collect();
            None
        }
    };
    if let Some((path, text, position)) = selected {
        let absolute = root.join(&path);
        let uri =
            path_to_uri(&absolute).map_err(|error| NekocodeError::Config(error.to_string()))?;
        let raw_selected = json!({"uri":uri, "range":{"start":position,"end":position}});
        response.target.location = read_location(&raw_selected, &mut sources)
            .ok()
            .map(|(location, _)| location);
        response.target.resolution = "selected".to_string();
        let method = client.document_notification_method(&absolute);
        if let Err(error) = client.open_document(&absolute, &text) {
            response.queries.push(SymbolQuery {
                method: method.to_string(),
                status: error_status(&error).to_string(),
                result_count: None,
                detail: Some(error.to_string()),
            });
        }
        let params = json!({"textDocument":{"uri":uri},"position":position});
        let definition = query(
            &mut client,
            &mut response.queries,
            "textDocument/definition",
            params.clone(),
        );
        let hover = query(
            &mut client,
            &mut response.queries,
            "textDocument/hover",
            params.clone(),
        );
        let type_definition = query(
            &mut client,
            &mut response.queries,
            "textDocument/typeDefinition",
            params.clone(),
        );
        let references = query(
            &mut client,
            &mut response.queries,
            "textDocument/references",
            json!({"textDocument":{"uri":uri},"position":position,"context":{"includeDeclaration":false}}),
        );
        let tests = query(
            &mut client,
            &mut response.queries,
            "rust-analyzer/relatedTests",
            params,
        );
        let mut collector = ItemCollector {
            client: &mut client,
            sources: &mut sources,
            response: &mut response,
            outlines: BTreeMap::new(),
            seen: BTreeSet::new(),
        };
        if let Some(value) = &definition {
            collector.add_locations("definition", value);
        }
        if collector.response.items.is_empty() {
            collector.add("selected_source", &raw_selected, None);
        }
        if let Some(detail) = hover.as_ref().and_then(hover_text) {
            collector.add("contract", &raw_selected, Some(detail));
        }
        if let Some(value) = &type_definition {
            collector.add_locations("type_definition", value);
        }
        if let Some(value) = &references {
            collector.add_locations("reference", value);
        }
        if let Some(Value::Array(tests)) = tests {
            for test in tests.iter().take(MAX_ITEMS + 1) {
                // rust-analyzer returns Runnable { location: { targetUri,
                // targetRange, targetSelectionRange }, label, ... }.
                let runnable = test.get("runnable").unwrap_or(test);
                if let Some(location) = runnable.get("location") {
                    collector.add(
                        "test_candidate",
                        location,
                        runnable
                            .get("label")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    );
                } else {
                    collector.omit("test_candidates", "backend test has no source location");
                }
            }
            let extra = tests.len().saturating_sub(MAX_ITEMS + 1);
            if extra > 0 {
                collector.response.totals.observed += extra;
                collector.omit_count("test_candidates", "collection_limit", extra);
            }
        }
    }
    let semantic_reusable = response.status == "completed"
        && response.omissions.is_empty()
        && response
            .queries
            .iter()
            .all(|q| matches!(q.status.as_str(), "completed" | "unsupported"));
    if request.text_candidates {
        super::text_candidates::collect(&root, &mut sources, &mut response);
    }
    let state = client.state();
    response.backend.version = state.version;
    response.backend.health = state.health;
    response.backend.message = state.message;
    response.backend.quiescent = state.quiescent;
    response.backend.readiness_observed = state.readiness_observed;
    response.backend.observation_ms = observation_start.elapsed().as_millis() as u64;
    if response
        .queries
        .iter()
        .any(|query| query.status == "timed_out")
    {
        response.status = "timed_out".to_string();
    } else if response
        .queries
        .iter()
        .any(|query| query.status != "completed" && query.status != "unsupported")
        || !response.omissions.is_empty()
    {
        response.status = "partial".to_string();
    }
    if !response.backend.readiness_observed
        || response
            .backend
            .health
            .as_deref()
            .is_some_and(|health| health != "ok")
    {
        if !matches!(
            response.status.as_str(),
            "timed_out" | "backend_unavailable"
        ) {
            response.status = "partial".to_string();
        }
        if response.target.resolution == "not_found" {
            response.target.resolution = "unresolved".to_string();
        }
        response.limitations.push("The backend did not report healthy readiness; empty results are not a complete absence conclusion.".to_string());
    }
    let captured = finish(response, root.clone(), before.clone(), sources)?;
    if semantic_reusable
        && captured.response.backend.health.as_deref() == Some("ok")
        && captured.response.freshness.input_scan_complete
        && captured.response.freshness.source_state == "stable"
    {
        session.retain(root, before, options, client);
    }
    Ok(captured)
}

fn finish(
    mut response: SymbolContextV1,
    root: PathBuf,
    mut before: InputInventory,
    sources: Sources,
) -> Result<SymbolPacket> {
    let mut after = inventory(&root);
    refresh_captured_inputs(&root, &sources.files, &mut after);
    let mut late_sources = 0usize;
    for (path, source) in &sources.files {
        if !before.files.contains_key(path) {
            before.files.insert(path.clone(), source.sha256.clone());
            before.complete = false;
            late_sources += 1;
        }
    }
    if late_sources > 0 {
        response.freshness.limitations.push(format!("{late_sources} captured source files were first observed after backend startup; their earlier state is unknown."));
    }
    let (verification, changes) = super::explanation::verify_inputs(
        &root,
        &before,
        &after,
        &sources.files,
        "capture_start_vs_end",
    );
    let verdict = verification.verdict.clone();
    response.freshness.verification = Some(verification);
    response.freshness.checked_inputs = before.files.len();
    response.freshness.changed_inputs = changes;
    response.freshness.input_scan_complete = before.complete && after.complete;
    if verdict == "changed" {
        response.freshness.state = "changed_during_observation".to_string();
        response.freshness.source_state = "changed".to_string();
        if response.status == "completed" {
            response.status = "partial".to_string();
        }
    } else if verdict == "match" {
        response.freshness.state = "source_stable".to_string();
        response.freshness.source_state = "stable".to_string();
    }
    for (index, item) in response.items.iter_mut().enumerate() {
        item.id = format!("{}:item:{}", response.packet_id, index + 1);
    }
    response.totals.retained = response.items.len();
    response.totals.displayed = response.items.len();
    Ok(SymbolPacket {
        contract_version: "symbol-packet-v1".to_string(),
        artifact_kind: "symbol_packet".to_string(),
        packet_id: response.packet_id.clone(),
        integrity_sha256: String::new(),
        root,
        inputs: before,
        sources: sources.files,
        response,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_positions_from_the_right() {
        assert_eq!(
            parse_at("src/日本.rs:7").unwrap(),
            (PathBuf::from("src/日本.rs"), 7, 1)
        );
        assert_eq!(
            parse_at("C:/src/lib.rs:7:9").unwrap(),
            (PathBuf::from("C:/src/lib.rs"), 7, 9)
        );
        assert!(parse_at("src/lib.rs:0").is_err());
    }
}
