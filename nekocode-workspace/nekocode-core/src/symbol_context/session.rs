use super::capture::{digest, InputInventory, Sources};
use super::lsp::{RaClient, RaError, RaOptions};
use super::{SymbolContextRequest, SymbolContextV1};
use crate::{NekocodeError, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Explicitly owned backend for sequential live investigations. Drop to stop it.
#[derive(Default)]
pub struct SymbolSession {
    cached: Option<Cached>,
    reused: bool,
    report: ReuseReport,
    previous_rejection: Vec<String>,
}
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct ReuseReport {
    pub acquisition: String,
    pub reasons: Vec<String>,
    pub retained: bool,
    pub retention_reasons: Vec<String>,
}
struct Cached {
    root: PathBuf,
    inputs: InputInventory,
    options: RaOptions,
    binary: Option<OsString>,
    client: RaClient,
}
impl SymbolSession {
    pub fn investigate(&mut self, request: &SymbolContextRequest) -> Result<SymbolContextV1> {
        self.reused = false;
        self.report = ReuseReport {
            acquisition: "not_observed".into(),
            ..Default::default()
        };
        if request.packet.is_some() {
            return Err(NekocodeError::Config(
                "session accepts live investigations only".into(),
            ));
        }
        // acquire takes ownership before backend work. Errors before acquisition
        // or after a healthy observation (e.g. packet saving) do not poison it.
        super::build_with_session(request, self)
    }
    pub fn reuse_report(&self) -> &ReuseReport {
        &self.report
    }
    pub(super) fn reject_retention(&mut self, reasons: Vec<String>) {
        self.previous_rejection = reasons.clone();
        self.report.retention_reasons = reasons;
        self.report.retained = false;
    }
    pub fn backend_reused(&self) -> bool {
        self.reused
    }
    pub(super) fn refresh_additional_inputs(
        &mut self,
        root: &Path,
        inputs: &mut InputInventory,
        sources: &mut Sources,
    ) {
        let Some(cached) = &self.cached else {
            return;
        };
        if cached.root != root {
            return;
        }
        let additional: Vec<_> = cached
            .inputs
            .files
            .keys()
            .filter(|path| !inputs.files.contains_key(*path))
            .cloned()
            .collect();
        for path in additional {
            match sources.text(&path) {
                Ok((relative, text)) => {
                    inputs.files.insert(relative, digest(text.as_bytes()));
                }
                Err(_) => {
                    self.cached = None;
                    self.previous_rejection = vec!["additional_input_unreadable".into()];
                    break;
                }
            }
        }
    }

    pub(super) fn acquire(
        &mut self,
        root: &Path,
        inputs: &InputInventory,
        options: &RaOptions,
    ) -> std::result::Result<RaClient, RaError> {
        self.report.acquisition = "fresh_backend".into();
        self.report.retention_reasons = vec!["observation_incomplete".into()];
        if let Some(mut cached) = self.cached.take() {
            let mut reasons = Vec::new();
            if cached.root != root {
                reasons.push("workspace_changed".into());
            }
            if !inputs.complete || !cached.inputs.complete {
                reasons.push("input_scan_incomplete".into());
            }
            if cached.inputs.files != inputs.files {
                reasons.push("inputs_changed".into());
            }
            if cached.inputs.profile() != inputs.profile() {
                reasons.push("scan_profile_changed".into());
            }
            if cached.options.all_features != options.all_features
                || cached.options.allow_build_scripts != options.allow_build_scripts
            {
                reasons.push("backend_options_changed".into());
            }
            if cached.binary != std::env::var_os("NEKOCODE_RUST_ANALYZER_PATH") {
                reasons.push("backend_selection_changed".into());
            }
            if !cached.client.is_alive() {
                reasons.push("backend_exited".into());
            }
            if reasons.is_empty() {
                cached.client.renew_deadline(options.timeout);
                self.reused = true;
                self.report.acquisition = "reused".into();
                return Ok(cached.client);
            }
            self.report.reasons = reasons;
        } else {
            self.report.reasons = vec!["no_cached_backend".into()];
            self.report.reasons.extend(self.previous_rejection.clone());
            if !inputs.complete
                && !self
                    .report
                    .reasons
                    .iter()
                    .any(|r| r == "input_scan_incomplete")
            {
                self.report.reasons.push("input_scan_incomplete".into());
            }
        }
        self.previous_rejection = vec!["observation_incomplete".into()];
        self.reused = false;
        RaClient::start(root, options)
    }
    pub(super) fn retain(
        &mut self,
        root: PathBuf,
        inputs: InputInventory,
        options: RaOptions,
        client: RaClient,
    ) {
        self.report.retained = true;
        self.report.retention_reasons.clear();
        self.previous_rejection.clear();
        self.cached = Some(Cached {
            root,
            inputs,
            options,
            binary: std::env::var_os("NEKOCODE_RUST_ANALYZER_PATH"),
            client,
        });
    }
}
