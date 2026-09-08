//! Bounded, single-observation rust-analyzer LSP transport.
//!
//! Backend readiness is recorded separately from source freshness. LSP does
//! not give these requests a shared analysis-generation identifier.

use serde::Serialize;
use serde_json::{json, Value};
use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const MAX_HEADER_BYTES: usize = 8192;
const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;
const MAX_STDOUT_BYTES: usize = 32 * 1024 * 1024;
const MAX_STDERR_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct RaOptions {
    pub timeout: Duration,
    pub all_features: bool,
    pub allow_build_scripts: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct RaState {
    pub version: Option<String>,
    pub health: Option<String>,
    pub message: Option<String>,
    pub quiescent: bool,
    pub readiness_observed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RaError {
    Unavailable(String),
    Unsupported(String),
    Timeout(String),
    Failed(String),
    OutputLimited(String),
}

impl fmt::Display for RaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Unavailable(message)
            | Self::Unsupported(message)
            | Self::Timeout(message)
            | Self::Failed(message)
            | Self::OutputLimited(message) => message,
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RaError {}

struct WriteJob {
    bytes: Vec<u8>,
    acknowledgement: mpsc::SyncSender<Result<(), String>>,
}

enum ReadEvent {
    Message(Value),
    Error(RaError),
}

pub(crate) struct RaClient {
    child: Child,
    writer: Option<mpsc::SyncSender<WriteJob>>,
    reader: Option<mpsc::Receiver<ReadEvent>>,
    workers: Vec<JoinHandle<()>>,
    stderr_limited: Arc<AtomicBool>,
    deadline: Instant,
    next_id: u64,
    state: RaState,
    observation_failure: Option<String>,
    root_uri: String,
    configuration: Value,
    opened_documents: std::collections::BTreeMap<String, i32>,
}

impl RaClient {
    pub(crate) fn start(root: &Path, options: &RaOptions) -> Result<Self, RaError> {
        let binary = std::env::var_os("NEKOCODE_RUST_ANALYZER_PATH")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "rust-analyzer".into());
        Self::start_with_binary(root, options, Path::new(&binary))
    }

    fn start_with_binary(root: &Path, options: &RaOptions, binary: &Path) -> Result<Self, RaError> {
        let deadline = Instant::now()
            .checked_add(options.timeout)
            .ok_or_else(|| RaError::Failed("backend timeout is too large".to_string()))?;
        let root = root
            .canonicalize()
            .map_err(|error| RaError::Failed(format!("workspace is unavailable: {error}")))?;
        let root_uri = path_to_uri(&root)?;
        let mut command = Command::new(binary);
        command
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_environment(&mut command);
        configure_process_group(&mut command);
        let mut child = command.spawn().map_err(|error| {
            RaError::Unavailable(format!(
                "rust-analyzer could not start ({error}); install the rust-analyzer and rust-src \
                 components or set NEKOCODE_RUST_ANALYZER_PATH"
            ))
        })?;
        let (Some(stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            terminate_process_tree(&mut child);
            return Err(RaError::Unavailable(
                "rust-analyzer protocol streams are unavailable".to_string(),
            ));
        };
        // Bounded channels prevent a noisy server from retaining unbounded
        // parsed frames, and a writer thread keeps pipe writes time-bounded.
        let (write_tx, write_rx) = mpsc::sync_channel::<WriteJob>(1);
        let writer_worker = thread::spawn(move || {
            let mut stdin = stdin;
            while let Ok(job) = write_rx.recv() {
                let result = stdin
                    .write_all(&job.bytes)
                    .and_then(|_| stdin.flush())
                    .map_err(|error| error.to_string());
                let failed = result.is_err();
                let _ = job.acknowledgement.send(result);
                if failed {
                    break;
                }
            }
        });
        let (read_tx, read_rx) = mpsc::sync_channel(2);
        let reader_worker = thread::spawn(move || {
            let mut stdout = BufReader::new(stdout);
            let mut total = 0;
            loop {
                match read_frame(&mut stdout, &mut total) {
                    Ok(Some(message)) => {
                        if read_tx.send(ReadEvent::Message(message)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        let _ = read_tx.send(ReadEvent::Error(RaError::Failed(
                            "rust-analyzer closed its protocol stream".to_string(),
                        )));
                        break;
                    }
                    Err(error) => {
                        let _ = read_tx.send(ReadEvent::Error(error));
                        break;
                    }
                }
            }
        });
        let stderr_limited = Arc::new(AtomicBool::new(false));
        let stderr_flag = Arc::clone(&stderr_limited);
        let stderr_worker = thread::spawn(move || {
            let mut stderr = stderr;
            let mut buffer = [0_u8; 8192];
            let mut total = 0_usize;
            while let Ok(count) = stderr.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                total = total.saturating_add(count);
                if total > MAX_STDERR_BYTES {
                    stderr_flag.store(true, Ordering::Relaxed);
                }
            }
        });
        // Keep Cargo project reloads enabled: rust-analyzer 1.89 otherwise
        // leaves even initial workspace loading pending after metadata changes.
        // Source stability is checked independently by the packet collector.
        let configuration = json!({
            "checkOnSave": false,
            "cargo": {
                "allTargets": true,
                "features": if options.all_features { json!("all") } else { json!([]) },
                "noDefaultFeatures": false,
                "extraArgs": ["--offline"],
                "buildScripts": { "enable": options.allow_build_scripts },
                "autoreload": true
            },
            "procMacro": { "enable": options.allow_build_scripts },
            "cfg": { "setTest": true },
            "cachePriming": { "enable": false }
        });
        let mut client = Self {
            child,
            writer: Some(write_tx),
            reader: Some(read_rx),
            workers: vec![writer_worker, reader_worker, stderr_worker],
            stderr_limited,
            deadline,
            next_id: 1,
            state: RaState::default(),
            observation_failure: None,
            root_uri,
            configuration,
            opened_documents: std::collections::BTreeMap::new(),
        };
        let initialized = client.request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "clientInfo": { "name": "nekocode", "version": crate::VERSION },
                "rootUri": client.root_uri,
                "workspaceFolders": [{ "uri": client.root_uri, "name": "workspace" }],
                "capabilities": {
                    "general": { "positionEncodings": ["utf-16"] },
                    "workspace": { "configuration": true, "workspaceFolders": true },
                    "textDocument": {
                        "documentSymbol": { "hierarchicalDocumentSymbolSupport": true },
                        "hover": { "contentFormat": ["plaintext"] }
                    },
                    "window": { "workDoneProgress": true },
                    "experimental": { "serverStatusNotification": true }
                },
                "initializationOptions": client.configuration
            }),
        );
        let initialized = match initialized {
            Err(RaError::Failed(message)) => {
                return Err(RaError::Unavailable(format!(
                    "rust-analyzer initialization failed ({message}); install the rust-analyzer \
                     and rust-src components or set NEKOCODE_RUST_ANALYZER_PATH"
                )));
            }
            result => result?,
        };
        client.state.version = initialized
            .pointer("/serverInfo/version")
            .and_then(Value::as_str)
            .map(str::to_string);
        if initialized
            .pointer("/capabilities/positionEncoding")
            .and_then(Value::as_str)
            .is_some_and(|encoding| encoding != "utf-16")
        {
            return Err(RaError::Unsupported(
                "rust-analyzer selected an unsupported position encoding".to_string(),
            ));
        }
        client.notify("initialized", json!({}))?;
        while !client.state.readiness_observed || !client.state.quiescent {
            let message = client.receive("workspace readiness")?;
            client.handle_server_message(&message)?;
        }
        Ok(client)
    }

    pub(crate) fn renew_deadline(&mut self, timeout: Duration) {
        self.deadline = Instant::now() + timeout;
    }

    pub(crate) fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None)) && !self.stderr_limited.load(Ordering::Relaxed)
    }

    pub(crate) fn state(&self) -> RaState {
        let mut state = self.state.clone();
        if let Some(message) = &self.observation_failure {
            state.health = None;
            state.readiness_observed = false;
            state.quiescent = false;
            state.message = Some(message.clone());
        }
        state
    }

    fn record_failure(&mut self, result: &Result<(), RaError>) {
        if let Err(error) = result {
            if !matches!(error, RaError::Unsupported(_)) {
                self.observation_failure = Some(bounded_detail(&error.to_string()));
            }
        }
    }

    pub(crate) fn document_notification_method(&self, path: &Path) -> &'static str {
        if path_to_uri(path)
            .ok()
            .is_some_and(|uri| self.opened_documents.contains_key(&uri))
        {
            "textDocument/didChange"
        } else {
            "textDocument/didOpen"
        }
    }

    pub(crate) fn open_document(&mut self, path: &Path, text: &str) -> Result<(), RaError> {
        let uri = path_to_uri(path)?;
        if let Some(previous) = self.opened_documents.get(&uri).copied() {
            let version = previous
                .checked_add(1)
                .ok_or_else(|| RaError::Failed("document version limit reached".to_string()))?;
            self.notify(
                "textDocument/didChange",
                json!({
                    "textDocument": { "uri": uri, "version": version },
                    "contentChanges": [{ "text": text }]
                }),
            )?;
            self.opened_documents.insert(uri, version);
        } else {
            self.notify(
                "textDocument/didOpen",
                json!({ "textDocument": {
                    "uri": uri, "languageId": "rust", "version": 1, "text": text
                }}),
            )?;
            self.opened_documents.insert(uri, 1);
        }
        Ok(())
    }

    pub(crate) fn request(&mut self, method: &str, params: Value) -> Result<Value, RaError> {
        let result = self.request_inner(method, params);
        self.record_failure(&result.as_ref().map(|_| ()).map_err(Clone::clone));
        result
    }

    fn request_inner(&mut self, method: &str, params: Value) -> Result<Value, RaError> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        loop {
            let message = self.receive(method)?;
            if message.get("method").is_some() {
                self.handle_server_message(&message)?;
                continue;
            }
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                let detail = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("backend request failed");
                let message = format!("{method}: {}", bounded_detail(detail));
                return Err(
                    if error.get("code").and_then(Value::as_i64) == Some(-32601) {
                        RaError::Unsupported(message)
                    } else {
                        RaError::Failed(message)
                    },
                );
            }
            return message.get("result").cloned().ok_or_else(|| {
                RaError::Failed(format!("{method}: response has neither result nor error"))
            });
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), RaError> {
        let result = self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
        self.record_failure(&result);
        result
    }

    fn remaining(&self, operation: &str) -> Result<Duration, RaError> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| RaError::Timeout(format!("{operation}: backend observation timed out")))
    }

    fn send(&mut self, message: Value) -> Result<(), RaError> {
        self.remaining("protocol write")?;
        let body = serde_json::to_vec(&message)
            .map_err(|error| RaError::Failed(format!("could not encode LSP request: {error}")))?;
        if body.len() > MAX_FRAME_BYTES {
            return Err(RaError::OutputLimited(
                "LSP request exceeded the frame limit".to_string(),
            ));
        }
        let mut bytes = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        bytes.extend_from_slice(&body);
        let (acknowledgement, response) = mpsc::sync_channel(1);
        let job = WriteJob {
            bytes,
            acknowledgement,
        };
        self.writer
            .as_ref()
            .ok_or_else(|| RaError::Failed("LSP writer is closed".to_string()))?
            .try_send(job)
            .map_err(|_| RaError::Failed("LSP writer is unavailable".to_string()))?;
        response
            .recv_timeout(self.remaining("protocol write")?)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => {
                    RaError::Timeout("protocol write: backend observation timed out".to_string())
                }
                mpsc::RecvTimeoutError::Disconnected => {
                    RaError::Failed("LSP writer stopped".to_string())
                }
            })?
            .map_err(|error| RaError::Failed(format!("could not write to rust-analyzer: {error}")))
    }

    fn receive(&self, operation: &str) -> Result<Value, RaError> {
        loop {
            if self.stderr_limited.load(Ordering::Relaxed) {
                return Err(RaError::OutputLimited(
                    "rust-analyzer stderr exceeded the output limit".to_string(),
                ));
            }
            let wait = self.remaining(operation)?.min(Duration::from_millis(50));
            match self
                .reader
                .as_ref()
                .ok_or_else(|| RaError::Failed("LSP reader is closed".to_string()))?
                .recv_timeout(wait)
            {
                Ok(ReadEvent::Message(message)) => return Ok(message),
                Ok(ReadEvent::Error(error)) => return Err(error),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(RaError::Failed("LSP reader stopped".to_string()));
                }
            }
        }
    }

    fn handle_server_message(&mut self, message: &Value) -> Result<(), RaError> {
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            return Ok(());
        };
        if method == "experimental/serverStatus" || method == "rust-analyzer/serverStatus" {
            let params = &message["params"];
            self.state.health = params
                .get("health")
                .and_then(Value::as_str)
                .map(str::to_string);
            self.state.message = params
                .get("message")
                .and_then(Value::as_str)
                .map(bounded_detail);
            if let Some(quiescent) = params.get("quiescent").and_then(Value::as_bool) {
                self.state.quiescent = quiescent;
                self.state.readiness_observed = true;
            }
        }
        let Some(id) = message.get("id") else {
            return Ok(());
        };
        let result = match method {
            "workspace/configuration" => {
                let items = message["params"]["items"].as_array().ok_or_else(|| {
                    RaError::Failed("invalid workspace/configuration request".to_string())
                })?;
                if items.len() > 256 {
                    return Err(RaError::OutputLimited(
                        "workspace/configuration exceeded its item limit".to_string(),
                    ));
                }
                Value::Array(
                    items
                        .iter()
                        .map(|item| self.configuration_for(item))
                        .collect(),
                )
            }
            "workspace/workspaceFolders" => {
                json!([{ "uri": self.root_uri, "name": "workspace" }])
            }
            "window/workDoneProgress/create"
            | "client/registerCapability"
            | "client/unregisterCapability"
            | "window/showMessageRequest"
            | "workspace/semanticTokens/refresh"
            | "workspace/codeLens/refresh"
            | "workspace/inlayHint/refresh"
            | "workspace/diagnostic/refresh" => Value::Null,
            _ => {
                return self.send(json!({
                    "jsonrpc": "2.0", "id": id,
                    "error": { "code": -32601, "message": "Client method not supported" }
                }));
            }
        };
        self.send(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
    }

    fn configuration_for(&self, item: &Value) -> Value {
        let section = item.get("section").and_then(Value::as_str).unwrap_or("");
        let section = section.strip_prefix("rust-analyzer").unwrap_or(section);
        let section = section.trim_start_matches('.');
        if section.is_empty() {
            return self.configuration.clone();
        }
        let mut value = &self.configuration;
        for component in section.split('.') {
            let Some(next) = value.get(component) else {
                return Value::Null;
            };
            value = next;
        }
        value.clone()
    }
}

impl Drop for RaClient {
    fn drop(&mut self) {
        // The transport owns a one-shot observation. Kill the entire process
        // group, including preparation subprocesses, on every exit path.
        self.writer.take();
        self.reader.take();
        terminate_process_tree(&mut self.child);
        let deadline = Instant::now() + Duration::from_millis(500);
        for worker in self.workers.drain(..) {
            while !worker.is_finished() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
    }
}

fn bounded_detail(message: &str) -> String {
    message.chars().take(1024).collect()
}

fn read_frame<R: BufRead>(reader: &mut R, total: &mut usize) -> Result<Option<Value>, RaError> {
    let mut header_bytes = 0;
    let mut length = None;
    loop {
        let mut line = Vec::new();
        loop {
            let available = reader
                .fill_buf()
                .map_err(|error| RaError::Failed(format!("LSP header read failed: {error}")))?;
            if available.is_empty() {
                if header_bytes == 0 && line.is_empty() {
                    return Ok(None);
                }
                return Err(RaError::Failed("incomplete LSP header".to_string()));
            }
            let count = available
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(available.len(), |position| position + 1);
            if header_bytes + line.len() + count > MAX_HEADER_BYTES {
                return Err(RaError::OutputLimited(
                    "LSP header exceeded its limit".to_string(),
                ));
            }
            line.extend_from_slice(&available[..count]);
            reader.consume(count);
            if line.last() == Some(&b'\n') {
                break;
            }
        }
        header_bytes += line.len();
        let line = std::str::from_utf8(&line)
            .map_err(|_| RaError::Failed("LSP header is not ASCII/UTF-8".to_string()))?
            .trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| RaError::Failed("invalid LSP header".to_string()))?;
        if name.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err(RaError::Failed("duplicate LSP Content-Length".to_string()));
            }
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| RaError::Failed("invalid LSP Content-Length".to_string()))?,
            );
        }
    }
    let length = length.ok_or_else(|| RaError::Failed("missing LSP Content-Length".to_string()))?;
    if length > MAX_FRAME_BYTES {
        return Err(RaError::OutputLimited(
            "LSP frame exceeded its limit".to_string(),
        ));
    }
    *total = total.saturating_add(header_bytes).saturating_add(length);
    if *total > MAX_STDOUT_BYTES {
        return Err(RaError::OutputLimited(
            "LSP output exceeded its total limit".to_string(),
        ));
    }
    let mut body = vec![0; length];
    reader
        .read_exact(&mut body)
        .map_err(|error| RaError::Failed(format!("LSP body read failed: {error}")))?;
    let message: Value = serde_json::from_slice(&body)
        .map_err(|error| RaError::Failed(format!("invalid LSP JSON: {error}")))?;
    if !message.is_object() || message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(RaError::Failed("invalid LSP JSON-RPC envelope".to_string()));
    }
    Ok(Some(message))
}

fn configure_environment(command: &mut Command) {
    command.env_clear();
    for key in [
        "PATH",
        "HOME",
        "USER",
        "LOGNAME",
        "USERPROFILE",
        "SystemRoot",
        "WINDIR",
        "TEMP",
        "TMP",
        "TMPDIR",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "RUSTUP_TOOLCHAIN",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
        .env("CARGO_TERM_COLOR", "never")
        .env("CARGO_NET_OFFLINE", "true");
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(windows)]
fn configure_process_group(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0000_0200);
}

#[cfg(not(any(unix, windows)))]
fn configure_process_group(_command: &mut Command) {}

fn terminate_process_tree(child: &mut Child) {
    #[cfg(unix)]
    // SAFETY: the owned child was started as leader of this process group.
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
    #[cfg(windows)]
    let _ = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    let _ = child.wait();
}

pub(crate) fn path_to_uri(path: &Path) -> Result<String, RaError> {
    if !path.is_absolute() {
        return Err(RaError::Failed("LSP paths must be absolute".to_string()));
    }
    let path = path
        .to_str()
        .ok_or_else(|| RaError::Failed("LSP paths must be valid UTF-8".to_string()))?;
    #[cfg(windows)]
    let path = {
        let path = path.strip_prefix(r"\\?\").unwrap_or(path);
        if path.starts_with(r"\\") || path.starts_with(r"UNC\") {
            return Err(RaError::Unsupported(
                "UNC paths are outside the local file URI boundary".to_string(),
            ));
        }
        path.replace('\\', "/")
    };
    let mut uri = String::from("file://");
    if !path.starts_with('/') {
        uri.push('/');
    }
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/:".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            use std::fmt::Write;
            let _ = write!(uri, "%{byte:02X}");
        }
    }
    Ok(uri)
}

pub(crate) fn uri_to_path(uri: &str) -> Result<PathBuf, RaError> {
    let remainder = uri
        .strip_prefix("file://")
        .ok_or_else(|| RaError::Unsupported("only local file URIs are supported".to_string()))?;
    let path = remainder
        .strip_prefix("localhost/")
        .map_or(remainder, |local| {
            // Keep the slash by selecting the same suffix from the original URI.
            &remainder[remainder.len() - local.len() - 1..]
        });
    if !path.starts_with('/') || path.contains(['?', '#']) {
        return Err(RaError::Unsupported(
            "only local file URIs are supported".to_string(),
        ));
    }
    let mut decoded = Vec::with_capacity(path.len());
    let bytes = path.as_bytes();
    let mut offset = 0;
    while offset < bytes.len() {
        let byte = if bytes[offset] == b'%' {
            let pair = bytes
                .get(offset + 1..offset + 3)
                .ok_or_else(|| RaError::Failed("invalid URI percent encoding".to_string()))?;
            let pair = std::str::from_utf8(pair)
                .map_err(|_| RaError::Failed("invalid URI percent encoding".to_string()))?;
            offset += 3;
            u8::from_str_radix(pair, 16)
                .map_err(|_| RaError::Failed("invalid URI percent encoding".to_string()))?
        } else {
            offset += 1;
            bytes[offset - 1]
        };
        if byte == 0 {
            return Err(RaError::Failed("file URI contains a NUL byte".to_string()));
        }
        decoded.push(byte);
    }
    let path = String::from_utf8(decoded)
        .map_err(|_| RaError::Failed("file URI is not valid UTF-8".to_string()))?;
    #[cfg(windows)]
    let path = if path.as_bytes().get(2) == Some(&b':') {
        &path[1..]
    } else {
        &path
    };
    Ok(PathBuf::from(path))
}

/// Convert the CLI's one-based Unicode scalar column to LSP UTF-16 units.
pub(crate) fn unicode_position(text: &str, line: u32, column: u32) -> Result<Value, RaError> {
    if line == 0 || column == 0 {
        return Err(RaError::Failed(
            "source positions are one-based".to_string(),
        ));
    }
    let source_line = text
        .split('\n')
        .nth((line - 1) as usize)
        .ok_or_else(|| RaError::Failed("source line is out of range".to_string()))?
        .trim_end_matches('\r');
    let count = (column - 1) as usize;
    if source_line.chars().count() < count {
        return Err(RaError::Failed("source column is out of range".to_string()));
    }
    let utf16: usize = source_line.chars().take(count).map(char::len_utf16).sum();
    Ok(json!({ "line": line - 1, "character": utf16 }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn protocol_frames_enforce_lengths_and_preserve_utf8() {
        let value = json!({ "jsonrpc": "2.0", "id": 1, "result": "猫😺" });
        let body = serde_json::to_vec(&value).unwrap();
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(&body);
        assert_eq!(
            read_frame(&mut Cursor::new(frame), &mut 0).unwrap(),
            Some(value)
        );
        assert!(matches!(
            read_frame(
                &mut Cursor::new(b"Content-Length: 999999999\r\n\r\n"),
                &mut 0
            ),
            Err(RaError::OutputLimited(_))
        ));
        assert!(matches!(
            read_frame(&mut Cursor::new(vec![b'x'; MAX_HEADER_BYTES + 1]), &mut 0),
            Err(RaError::OutputLimited(_))
        ));
        assert!(read_frame(
            &mut Cursor::new(b"Content-Length: 1\r\nContent-Length: 1\r\n\r\n0"),
            &mut 0
        )
        .is_err());
    }

    #[test]
    fn unicode_paths_and_positions_round_trip() {
        let path = std::env::temp_dir().join("猫 😺 #?%\\.rs");
        let uri = path_to_uri(&path).unwrap();
        assert_eq!(uri_to_path(&uri).unwrap(), path);
        assert!(uri.contains("%23%3F%25"));
        assert_eq!(
            unicode_position("a😺猫\r\nnext", 1, 3).unwrap(),
            json!({"line": 0, "character": 3})
        );
        assert_eq!(
            unicode_position("a😺猫\r\nnext", 2, 1).unwrap(),
            json!({"line": 1, "character": 0})
        );
        assert!(unicode_position("a", 1, 3).is_err());
        assert!(uri_to_path("file://remote/secret.rs").is_err());
        assert!(uri_to_path("file:///tmp/%00.rs").is_err());
    }

    #[cfg(unix)]
    fn fake_server() -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        // Use the immutable executable fixture. Writing executable copies while
        // sibling tests spawn children can leave an inherited writable fd and
        // cause a transient ETXTBSY on Linux.
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/symbol_context/fixtures/fake-ra.py");
        (directory, script)
    }

    #[cfg(unix)]
    #[test]
    fn dead_transport_clears_health_and_tracks_document_notification_kind() {
        let (directory, binary) = fake_server();
        let options = RaOptions {
            timeout: Duration::from_secs(5),
            all_features: false,
            allow_build_scripts: false,
        };
        let mut client = RaClient::start_with_binary(directory.path(), &options, &binary).unwrap();
        let path = directory.path().join("target.rs");
        assert_eq!(
            client.document_notification_method(&path),
            "textDocument/didOpen"
        );
        client.open_document(&path, "fn target() {}\n").unwrap();
        assert_eq!(
            client.document_notification_method(&path),
            "textDocument/didChange"
        );
        client.child.kill().unwrap();
        client.child.wait().unwrap();
        assert!(!client.is_alive());
        assert!(client.open_document(&path, "fn target() {}\n").is_err());
        assert!(client.state().health.is_none());
        assert!(!client.state().readiness_observed);
        assert!(!client.state().quiescent);
        assert!(client.request("fixture/state", Value::Null).is_err());
        assert!(client.state().health.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn client_handles_configuration_documents_and_unsupported_queries() {
        let (directory, binary) = fake_server();
        let options = RaOptions {
            timeout: Duration::from_secs(5),
            all_features: true,
            allow_build_scripts: false,
        };
        let mut client = RaClient::start_with_binary(directory.path(), &options, &binary).unwrap();
        assert_eq!(client.state().version.as_deref(), Some("fixture-1"));
        assert!(client.state().readiness_observed && client.state().quiescent);
        let path = directory.path().join("猫.rs");
        client.open_document(&path, "fn 猫() {}\n").unwrap();
        client
            .open_document(&path, "fn 猫() { let n = 10 /2; }\n")
            .unwrap();
        let result = client.request("fixture/state", Value::Null).unwrap();
        assert_eq!(result["document"]["version"], 2);
        assert_eq!(result["document"]["text"], "fn 猫() { let n = 10 /2; }\n");
        assert_eq!(result["configuration"]["checkOnSave"], false);
        assert_eq!(
            result["configuration"]["cargo"]["buildScripts"]["enable"],
            false
        );
        assert_eq!(result["configuration"]["procMacro"]["enable"], false);
        assert_eq!(result["configuration"]["cargo"]["features"], "all");
        assert_eq!(result["configuration"]["cargo"]["autoreload"], true);
        client.request("fixture/warning", Value::Null).unwrap();
        assert_eq!(client.state().health.as_deref(), Some("warning"));
        assert_eq!(
            client.state().message.as_deref(),
            Some("fixture workspace needs attention")
        );
        assert!(matches!(
            client.request("unsupported", Value::Null),
            Err(RaError::Unsupported(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn all_queries_share_deadline_and_timeout_cleanup_finishes() {
        let (directory, binary) = fake_server();
        let options = RaOptions {
            timeout: Duration::from_millis(500),
            all_features: false,
            allow_build_scripts: true,
        };
        let start = Instant::now();
        let mut client = RaClient::start_with_binary(directory.path(), &options, &binary).unwrap();
        assert!(matches!(
            client.request("fixture/slow", Value::Null),
            Err(RaError::Timeout(_))
        ));
        assert!(matches!(
            client.request("fixture/state", Value::Null),
            Err(RaError::Timeout(_))
        ));
        drop(client);
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[cfg(unix)]
    #[test]
    fn blocked_protocol_write_respects_shared_deadline() {
        let (directory, binary) = fake_server();
        let options = RaOptions {
            timeout: Duration::from_millis(500),
            all_features: false,
            allow_build_scripts: false,
        };
        let start = Instant::now();
        let mut client = RaClient::start_with_binary(directory.path(), &options, &binary).unwrap();
        client.request("fixture/no-read", Value::Null).unwrap();
        assert!(matches!(
            client.open_document(
                &directory.path().join("large.rs"),
                &"x".repeat(2 * 1024 * 1024)
            ),
            Err(RaError::Timeout(_))
        ));
        drop(client);
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn dropping_client_terminates_descendant_processes() {
        let (directory, binary) = fake_server();
        let options = RaOptions {
            timeout: Duration::from_secs(5),
            all_features: false,
            allow_build_scripts: false,
        };
        let mut client = RaClient::start_with_binary(directory.path(), &options, &binary).unwrap();
        let result = client.request("fixture/spawn-child", Value::Null).unwrap();
        let pid = result["pid"].as_u64().unwrap();
        drop(client);
        // An adopted zombie has terminated too; the OS init process owns its
        // eventual reap after the fake server's process group is stopped.
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"));
        assert!(
            stat.is_err()
                || stat
                    .unwrap()
                    .split(") ")
                    .nth(1)
                    .is_some_and(|state| state.starts_with('Z'))
        );
    }
}
