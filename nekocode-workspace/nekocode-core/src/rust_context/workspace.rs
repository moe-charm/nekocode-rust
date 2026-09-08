//! Cargo workspace discovery and input provenance.

use super::execution;
use super::*;
use execution::*;

pub fn index_rust_workspace(path: impl AsRef<Path>) -> Result<RustWorkspaceSnapshot> {
    let manifest_root = discover_manifest_root(path.as_ref())?;
    let command = "cargo metadata --format-version=1 --no-deps --offline --config 'build.rustc-wrapper=\"\"' --config 'build.rustc-workspace-wrapper=\"\"'".to_string();
    let mut cargo = Command::new("cargo");
    cargo.current_dir(&manifest_root).args([
        "metadata",
        "--format-version=1",
        "--no-deps",
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

    if output.timed_out {
        return Err(NekocodeError::External(
            "cargo metadata timed out".to_string(),
        ));
    }
    if output.output_limited {
        return Err(NekocodeError::External(
            "cargo metadata output exceeded the safety limit".to_string(),
        ));
    }
    if !output.status.is_some_and(|status| status.success()) {
        return Err(NekocodeError::External(format_command_failure(
            "cargo metadata",
            &output.stderr.bytes,
        )));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout.bytes)?;
    let workspace_root = metadata
        .get("workspace_root")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| {
            NekocodeError::External("cargo metadata omitted workspace_root".to_string())
        })?
        .canonicalize()
        .map_err(|error| {
            NekocodeError::External(format!(
                "cargo metadata returned an invalid workspace_root: {error}"
            ))
        })?;

    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| NekocodeError::External("cargo metadata omitted packages".to_string()))?
        .iter()
        .map(parse_package)
        .collect::<Result<Vec<_>>>()?;
    let toolchain = detect_toolchain();
    let workspace_members = metadata
        .get("workspace_members")
        .and_then(serde_json::Value::as_array)
        .map(|members| {
            members
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let inputs = collect_input_digests(&workspace_root, &packages);

    Ok(RustWorkspaceSnapshot {
        schema_version: SCHEMA_VERSION,
        evidence: EvidenceLevel::ToolConfirmed,
        root: workspace_root.clone(),
        workspace_root,
        toolchain,
        packages,
        workspace_members,
        inputs,
        provenance: ToolProvenance {
            tool: "cargo metadata".to_string(),
            command,
            cwd: manifest_root,
            version: command_version("cargo"),
            exit_code: Some(0),
        },
    })
}

pub(crate) fn discover_manifest_root(path: &Path) -> Result<PathBuf> {
    let candidate = path.canonicalize()?;
    let start = if candidate.is_file() {
        candidate
            .parent()
            .ok_or_else(|| NekocodeError::Config("input file has no parent directory".to_string()))?
            .to_path_buf()
    } else {
        candidate
    };

    for ancestor in start.ancestors() {
        if ancestor.join("Cargo.toml").is_file() {
            return Ok(ancestor.to_path_buf());
        }
    }

    Err(NekocodeError::Config(format!(
        "Rust workspace not found at or above: {}",
        start.display()
    )))
}

pub(crate) fn collect_input_digests(root: &Path, packages: &[RustPackage]) -> Vec<RustInputDigest> {
    let mut paths = vec![root.join("Cargo.toml")];
    paths.extend(packages.iter().map(|package| package.manifest_path.clone()));
    push_existing_input(&mut paths, root.join("Cargo.lock"));

    // Cargo and rustup search configuration/toolchain files through the
    // current directory's ancestors. The workspace-local files alone are not
    // a closed provenance set when the repository itself is nested.
    for ancestor in root.ancestors() {
        for name in [
            "rust-toolchain",
            "rust-toolchain.toml",
            ".cargo/config.toml",
            ".cargo/config",
        ] {
            push_existing_input(&mut paths, ancestor.join(name));
        }
    }
    if let Some(cargo_home) = cargo_home_dir() {
        for name in ["config.toml", "config"] {
            push_existing_input(&mut paths, cargo_home.join(name));
        }
    }
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .filter_map(|path| {
            let bytes = std::fs::read(&path).ok()?;
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            let digest = format!("{:x}", hasher.finalize());
            Some(RustInputDigest {
                path,
                sha256: digest,
            })
        })
        .collect()
}

fn push_existing_input(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_file() {
        paths.push(path);
    }
}

fn cargo_home_dir() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")))
}

pub(crate) fn annotate_changed_files(
    files: &mut [ChangedRustFile],
    root: &Path,
    packages: &[RustPackage],
) {
    for file in files {
        file.is_rust = is_rust_path(&file.path);
        file.package = packages
            .iter()
            .filter_map(|package| {
                let package_root = Path::new(&package.manifest_path).parent()?;
                let candidate = root.join(&file.path).canonicalize().ok()?;
                let package_root = package_root.canonicalize().ok()?;
                candidate
                    .starts_with(&package_root)
                    .then(|| package.name.clone())
            })
            .next();
    }
}

pub(crate) fn parse_package(value: &serde_json::Value) -> Result<RustPackage> {
    let id = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| NekocodeError::External("cargo metadata package has no name".to_string()))?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| NekocodeError::External(format!("package {name} has no version")))?;
    let manifest_path = value
        .get("manifest_path")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| NekocodeError::External(format!("package {name} has no manifest_path")))?;
    let target_details: Vec<RustTarget> = value
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .map(|targets| {
            targets
                .iter()
                .filter_map(|target| {
                    let name = target.get("name")?.as_str()?.to_string();
                    let kind = target
                        .get("kind")
                        .and_then(serde_json::Value::as_array)
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let src_path = target
                        .get("src_path")
                        .and_then(serde_json::Value::as_str)
                        .map(PathBuf::from);
                    let edition = target
                        .get("edition")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string);
                    let required_features = target
                        .get("required-features")
                        .and_then(serde_json::Value::as_array)
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    Some(RustTarget {
                        name,
                        kind,
                        src_path,
                        edition,
                        required_features,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let targets = target_details
        .iter()
        .map(|target| target.name.clone())
        .collect::<Vec<_>>();
    let mut features = value
        .get("features")
        .and_then(serde_json::Value::as_object)
        .map(|features| features.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    features.sort();
    let mut feature_definitions = value
        .get("features")
        .and_then(serde_json::Value::as_object)
        .map(|features| {
            features
                .iter()
                .map(|(name, values)| RustFeatureDefinition {
                    name: name.clone(),
                    enables: values
                        .as_array()
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for definition in &mut feature_definitions {
        definition.enables.sort();
    }
    feature_definitions.sort_by(|left, right| left.name.cmp(&right.name));
    let mut dependencies = value
        .get("dependencies")
        .and_then(serde_json::Value::as_array)
        .map(|dependencies| {
            dependencies
                .iter()
                .filter_map(|dependency| dependency.get("name").and_then(serde_json::Value::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    dependencies.sort();
    dependencies.dedup();
    let edition = value
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .and_then(|targets| targets.first())
        .and_then(|target| target.get("edition"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);

    Ok(RustPackage {
        id,
        name: name.to_string(),
        version: version.to_string(),
        manifest_path,
        targets,
        target_details,
        features,
        feature_definitions,
        dependencies,
        edition,
    })
}

pub(crate) fn detect_toolchain() -> RustToolchainInfo {
    let rustc_output = Command::new("rustc").args(["-Vv"]).output().ok();
    let mut rustc_version = None;
    let mut host = None;
    if let Some(output) = rustc_output.filter(|output| output.status.success()) {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut lines = text.lines();
        rustc_version = lines.next().map(str::to_string);
        host = text
            .lines()
            .find_map(|line| line.strip_prefix("host: ").map(str::to_string));
    }

    RustToolchainInfo {
        rustc_version,
        cargo_version: command_version("cargo"),
        host,
    }
}

pub(crate) fn command_version(command: &str) -> Option<String> {
    command_version_with_args(command, &["--version"])
}

pub(crate) fn command_version_with_args(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::to_string)
}

pub(crate) fn diagnostic_producer_version(producer: DiagnosticProducer) -> Option<String> {
    match producer {
        DiagnosticProducer::CargoCheck => command_version("cargo"),
        DiagnosticProducer::Clippy => command_version_with_args("cargo", &["clippy", "--version"]),
    }
}
