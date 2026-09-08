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
        if request.packet.is_some() {
            return Err(NekocodeError::Config(
                "session accepts live investigations only".into(),
            ));
        }
        // acquire takes ownership before backend work. Errors before acquisition
        // or after a healthy observation (e.g. packet saving) do not poison it.
        super::build_with_session(request, self)
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
        if let Some(mut cached) = self.cached.take() {
            if cached.root == root
                && inputs.complete
                && cached.inputs.complete
                && cached.inputs.files == inputs.files
                && cached.options.all_features == options.all_features
                && cached.options.allow_build_scripts == options.allow_build_scripts
                && cached.binary == std::env::var_os("NEKOCODE_RUST_ANALYZER_PATH")
                && cached.client.is_alive()
            {
                cached.client.renew_deadline(options.timeout);
                self.reused = true;
                return Ok(cached.client);
            }
        }
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
        self.cached = Some(Cached {
            root,
            inputs,
            options,
            binary: std::env::var_os("NEKOCODE_RUST_ANALYZER_PATH"),
            client,
        });
    }
}
