//! Bounded external process execution and compiler diagnostic collection.

use super::*;
use super::{diagnostics, workspace};
use diagnostics::*;
use workspace::*;

pub(crate) fn is_rust_path(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("rs")
}

#[derive(Debug)]
pub(crate) struct CappedBytes {
    pub(crate) bytes: Vec<u8>,
    pub(crate) truncated: bool,
}

#[derive(Debug)]
pub(crate) struct BoundedCommandOutput {
    pub(crate) status: Option<std::process::ExitStatus>,
    pub(crate) stdout: CappedBytes,
    pub(crate) stderr: CappedBytes,
    pub(crate) timed_out: bool,
    pub(crate) output_limited: bool,
}

pub(crate) fn read_capped<R: Read>(mut reader: R, limit: usize) -> io::Result<CappedBytes> {
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    let mut buffer = [0_u8; 16 * 1024];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(bytes.len());
        let keep = read.min(remaining);
        bytes.extend_from_slice(&buffer[..keep]);
        if keep < read {
            truncated = true;
        }
    }
    Ok(CappedBytes { bytes, truncated })
}

pub(crate) fn join_reader(
    reader: thread::JoinHandle<io::Result<CappedBytes>>,
    timeout: Duration,
    stream_name: &str,
) -> Result<CappedBytes> {
    let deadline = Instant::now() + timeout;
    while !reader.is_finished() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    if !reader.is_finished() {
        return Err(NekocodeError::External(format!(
            "{stream_name} reader did not finish within the safety deadline"
        )));
    }
    reader
        .join()
        .map_err(|_| NekocodeError::External(format!("{stream_name} reader thread panicked")))?
        .map_err(|error| NekocodeError::External(format!("{stream_name} reader failed: {error}")))
}

pub(crate) fn run_bounded_command(
    mut command: Command,
    timeout: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
) -> Result<BoundedCommandOutput> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_group(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| NekocodeError::External(format!("failed to start command: {error}")))?;
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            terminate_process_tree(&mut child);
            return Err(NekocodeError::External(
                "command stdout was not piped".to_string(),
            ));
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            terminate_process_tree(&mut child);
            return Err(NekocodeError::External(
                "command stderr was not piped".to_string(),
            ));
        }
    };
    let stdout_thread = thread::spawn(move || read_capped(stdout, stdout_limit));
    let stderr_thread = thread::spawn(move || read_capped(stderr, stderr_limit));

    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => {
                timed_out = true;
                terminate_process_tree(&mut child);
                break child.try_wait().ok().flatten();
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                terminate_process_tree(&mut child);
                return Err(NekocodeError::External(format!(
                    "failed to wait for command: {error}"
                )));
            }
        }
    };

    let reader_timeout = timeout.min(Duration::from_secs(5));
    let stdout = match join_reader(stdout_thread, reader_timeout, "stdout") {
        Ok(stdout) => stdout,
        Err(error) => {
            terminate_process_tree(&mut child);
            return Err(error);
        }
    };
    let stderr = match join_reader(stderr_thread, reader_timeout, "stderr") {
        Ok(stderr) => stderr,
        Err(error) => {
            terminate_process_tree(&mut child);
            return Err(error);
        }
    };
    Ok(BoundedCommandOutput {
        output_limited: stdout.truncated || stderr.truncated,
        stdout,
        stderr,
        timed_out,
        status,
    })
}

#[cfg(unix)]
pub(crate) fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    // A Cargo invocation can spawn build scripts and compiler wrappers. Put
    // the child in its own process group so timeout cleanup reaches them.
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(windows)]
pub(crate) fn configure_process_group(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
pub(crate) fn terminate_process_tree(child: &mut Child) {
    let pid = child.id() as libc::pid_t;
    // Negative pid addresses the process group created above.
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(windows)]
pub(crate) fn terminate_process_tree(child: &mut Child) {
    let pid = child.id().to_string();
    let _ = Command::new("taskkill")
        .args(["/PID", pid.as_str(), "/T", "/F"])
        .status();
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn terminate_process_tree(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub(crate) fn dedicated_target_dir(root: &Path) -> Result<PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(root.to_string_lossy().as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    let target = std::env::temp_dir()
        .join(CARGO_TARGET_DIR_NAME)
        .join(&digest[..16]);
    std::fs::create_dir_all(&target)?;
    Ok(target)
}

pub(crate) fn configure_safe_environment(command: &mut Command) {
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
        if let Ok(value) = std::env::var(key) {
            command.env(key, value);
        }
    }
    command
        .env("CARGO_TERM_COLOR", "never")
        .env_remove("RUSTC")
        .env_remove("RUSTDOC")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("CARGO_BUILD_RUSTC")
        .env_remove("CARGO_BUILD_RUSTC_WRAPPER")
        .env_remove("LD_PRELOAD")
        .env_remove("DYLD_INSERT_LIBRARIES")
        .env_remove("BASH_ENV")
        .env_remove("ENV");
}

pub(crate) fn configure_cargo_environment(command: &mut Command, target_dir: &Path) {
    configure_safe_environment(command);
    command
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TARGET_DIR", target_dir);
}

pub(crate) fn run_diagnostic_with_options(
    root: &Path,
    producer: DiagnosticProducer,
    all_features: bool,
) -> Result<RustDiagnosticRun> {
    let subcommand = match producer {
        DiagnosticProducer::CargoCheck => "check",
        DiagnosticProducer::Clippy => "clippy",
    };
    let mut args = vec![
        subcommand.to_string(),
        "--workspace".to_string(),
        "--all-targets".to_string(),
        "--offline".to_string(),
        "--config".to_string(),
        "build.rustc-wrapper=\"\"".to_string(),
        "--config".to_string(),
        "build.rustc-workspace-wrapper=\"\"".to_string(),
    ];
    // Establish a stable locked profile when Cargo can resolve a first
    // lockfile offline. A dependency that is unavailable offline still gets
    // the original lockfile-less attempt, whose failure remains evidence.
    if !root.join("Cargo.lock").is_file() {
        let _ = generate_lockfile(root);
    }
    if root.join("Cargo.lock").is_file() {
        args.insert(4, "--locked".to_string());
    }
    if all_features {
        args.push("--all-features".to_string());
    }
    args.push("--message-format=json".to_string());
    let command = format!("cargo {}", args.join(" "));
    let target_dir = dedicated_target_dir(root)?;
    let mut cargo = Command::new("cargo");
    cargo.current_dir(root).args(&args);
    configure_cargo_environment(&mut cargo, &target_dir);
    let output = run_bounded_command(
        cargo,
        CARGO_CHECK_TIMEOUT,
        MAX_CARGO_STDOUT_BYTES,
        MAX_CARGO_STDERR_BYTES,
    )?;
    let status = if output.timed_out {
        "timed_out"
    } else if output.output_limited {
        "output_limited"
    } else if output.status.is_some_and(|status| status.success()) {
        "success"
    } else {
        "failed"
    };
    let mut stderr = non_empty_text(&output.stderr.bytes);
    if output.stderr.truncated {
        let marker = "[stderr truncated by safety limit]";
        stderr = Some(match stderr {
            Some(text) => format!("{text}\n{marker}"),
            None => marker.to_string(),
        });
    }

    let messages = parse_cargo_diagnostics_with_root(
        &String::from_utf8_lossy(&output.stdout.bytes),
        Some(root),
    )
    .into_iter()
    .map(|mut diagnostic| {
        diagnostic.fingerprint =
            diagnostic_fingerprint_with_context(&diagnostic, producer, producer.profile());
        diagnostic
    })
    .collect();

    Ok(RustDiagnosticRun {
        producer,
        profile: producer.profile(),
        producer_version: diagnostic_producer_version(producer),
        command: command.clone(),
        status: status.to_string(),
        messages,
        stderr,
        all_targets: true,
        all_features,
        provenance: ToolProvenance {
            tool: producer.command_name().to_string(),
            command,
            cwd: root.to_path_buf(),
            version: command_version("cargo"),
            exit_code: output.status.and_then(|status| status.code()),
        },
        comparison_basis: None,
    })
}

fn generate_lockfile(root: &Path) -> Result<()> {
    let mut cargo = Command::new("cargo");
    cargo.current_dir(root).args([
        "generate-lockfile",
        "--offline",
        "--config",
        "build.rustc-wrapper=\"\"",
        "--config",
        "build.rustc-workspace-wrapper=\"\"",
    ]);
    configure_safe_environment(&mut cargo);
    let output = run_bounded_command(
        cargo,
        Duration::from_secs(60),
        MAX_CARGO_STDOUT_BYTES,
        MAX_CARGO_STDERR_BYTES,
    )?;
    if output.timed_out || output.output_limited {
        return Err(NekocodeError::External(
            "cargo generate-lockfile did not complete within its safety limits".to_string(),
        ));
    }
    if output.status.is_some_and(|status| status.success()) {
        Ok(())
    } else {
        Err(NekocodeError::External(format_command_failure(
            "cargo generate-lockfile",
            &output.stderr.bytes,
        )))
    }
}

#[cfg(test)]
pub(crate) fn parse_cargo_diagnostics(text: &str) -> Vec<RustDiagnostic> {
    parse_cargo_diagnostics_with_root(text, None)
}

pub(crate) fn parse_cargo_diagnostics_with_root(
    text: &str,
    root: Option<&Path>,
) -> Vec<RustDiagnostic> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| {
            value.get("reason").and_then(serde_json::Value::as_str) == Some("compiler-message")
        })
        .filter_map(|value| {
            let message = value.get("message")?;
            let level = message.get("level")?.as_str()?.to_string();
            let text = message.get("message")?.as_str()?.to_string();
            let code = message
                .get("code")
                .and_then(|code| code.get("code"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let span = message
                .get("spans")
                .and_then(serde_json::Value::as_array)
                .and_then(|spans| {
                    spans
                        .iter()
                        .find(|span| {
                            span.get("is_primary").and_then(serde_json::Value::as_bool)
                                == Some(true)
                        })
                        .or_else(|| spans.first())
                });
            let file = span
                .and_then(|span| span.get("file_name"))
                .and_then(serde_json::Value::as_str)
                .map(|file| normalize_reported_path(root, file));
            let line = span
                .and_then(|span| span.get("line_start"))
                .and_then(serde_json::Value::as_u64)
                .map(|line| line as u32);
            let column = span
                .and_then(|span| span.get("column_start"))
                .and_then(serde_json::Value::as_u64)
                .map(|column| column as u32);

            let spans = message
                .get("spans")
                .and_then(serde_json::Value::as_array)
                .map(|spans| {
                    spans
                        .iter()
                        .map(|span| RustDiagnosticSpan {
                            file: span
                                .get("file_name")
                                .and_then(serde_json::Value::as_str)
                                .map(|file| normalize_reported_path(root, file)),
                            line_start: span
                                .get("line_start")
                                .and_then(serde_json::Value::as_u64)
                                .map(|value| value as u32),
                            column_start: span
                                .get("column_start")
                                .and_then(serde_json::Value::as_u64)
                                .map(|value| value as u32),
                            line_end: span
                                .get("line_end")
                                .and_then(serde_json::Value::as_u64)
                                .map(|value| value as u32),
                            column_end: span
                                .get("column_end")
                                .and_then(serde_json::Value::as_u64)
                                .map(|value| value as u32),
                            is_primary: span
                                .get("is_primary")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(false),
                            label: span
                                .get("label")
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_string),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            Some(diagnostic_with_fingerprint(RustDiagnostic {
                level,
                message: text,
                code,
                file,
                line,
                column,
                rendered: message
                    .get("rendered")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                package_id: value
                    .get("package_id")
                    .and_then(serde_json::Value::as_str)
                    .map(|id| match root {
                        Some(root) => {
                            id.replace(&root.to_string_lossy().replace('\\', "/"), "$WORKSPACE")
                        }
                        None => id.to_string(),
                    }),
                target: value
                    .get("target")
                    .and_then(|target| target.get("name"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                spans,
                fingerprint: String::new(),
            }))
        })
        .collect()
}

pub(crate) fn normalize_reported_path(root: Option<&Path>, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if let Some(root) = root {
        if let Ok(relative) = path.strip_prefix(root) {
            return relative.to_path_buf();
        }
    }
    path
}

pub(crate) fn non_empty_text(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes).trim().to_string();
    (!text.is_empty()).then_some(text)
}

pub(crate) fn format_command_failure(command: &str, stderr: &[u8]) -> String {
    let detail = String::from_utf8_lossy(stderr).trim().to_string();
    if detail.is_empty() {
        format!("{command} failed")
    } else {
        format!("{command} failed: {detail}")
    }
}
