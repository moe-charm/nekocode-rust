//! Link policy uses the same Rust/Cargo input boundary as ordinary files.
use super::model::*;
use std::path::{Component, Path, PathBuf};

pub(super) fn is_input(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "rs")
        || matches!(
            path.file_name().and_then(|n| n.to_str()),
            Some("Cargo.toml" | "Cargo.lock" | "rust-toolchain" | "rust-toolchain.toml")
        )
        || (path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|n| n == ".cargo")
            && matches!(
                path.file_name().and_then(|n| n.to_str()),
                Some("config" | "config.toml")
            ))
}
fn lexical(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            _ => result.push(part.as_os_str()),
        }
    }
    result
}
fn excluded_tree(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).is_ok_and(|p| {
        p.components().any(|c| {
            matches!(
                c.as_os_str().to_str(),
                Some(".git" | "target" | ".nekocode")
            )
        })
    })
}
#[cfg(test)]
pub(super) fn inspect(root: &Path, path: &Path) -> LinkStamp {
    inspect_input(root, path, false)
}
pub(super) fn inspect_input(root: &Path, path: &Path, known_input: bool) -> LinkStamp {
    let target = std::fs::read_link(path).ok();
    let resolved = path.canonicalize().ok();
    let metadata = std::fs::metadata(path).ok();
    let status = match (&target, &resolved, &metadata) {
        (Some(_), Some(real), Some(m)) if m.is_dir() => {
            if real.starts_with(root) && !excluded_tree(root, real) {
                "verified_directory"
            } else {
                "unverified_directory_target"
            }
        }
        (Some(_), Some(real), Some(m)) if m.is_file() => {
            if known_input || is_input(path) || is_input(real) {
                if real.starts_with(root) && !excluded_tree(root, real) {
                    "verified_input_file"
                } else {
                    "unverified_input_target"
                }
            } else {
                "outside_input_file_scope"
            }
        }
        (Some(raw), None, None)
            if !known_input
                && !is_input(path)
                && matches!(
                    path.extension().and_then(|e| e.to_str()),
                    Some("so" | "dll" | "dylib" | "a" | "o")
                )
                && lexical(&path.parent().unwrap_or(root).join(raw))
                    .starts_with(root.join("target"))
                && std::fs::metadata(path)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            "outside_generated_output_scope"
        }
        _ => "unverified_link_target",
    };
    LinkStamp {
        target,
        resolved,
        status: status.into(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    #[test]
    fn missing_build_outputs_are_narrowly_scoped() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        symlink("target/release/libp.so", root.join("libp.so")).unwrap();
        assert_eq!(
            inspect(root, &root.join("libp.so")).status,
            "outside_generated_output_scope"
        );
        symlink("unknown/libp.so", root.join("other.so")).unwrap();
        assert_eq!(
            inspect(root, &root.join("other.so")).status,
            "unverified_link_target"
        );
        symlink("target/generated.rs", root.join("input.rs")).unwrap();
        assert_eq!(
            inspect(root, &root.join("input.rs")).status,
            "unverified_link_target"
        );
        symlink("cycle", root.join("cycle")).unwrap();
        assert_eq!(
            inspect(root, &root.join("cycle")).status,
            "unverified_link_target"
        );
    }
    #[test]
    fn external_directory_and_input_are_not_exempted() {
        let dir = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        std::fs::write(external.path().join("input.rs"), "// input").unwrap();
        symlink(external.path(), dir.path().join("alias")).unwrap();
        symlink(
            external.path().join("input.rs"),
            dir.path().join("input.rs"),
        )
        .unwrap();
        assert_eq!(
            inspect(dir.path(), &dir.path().join("alias")).status,
            "unverified_directory_target"
        );
        assert_eq!(
            inspect(dir.path(), &dir.path().join("input.rs")).status,
            "unverified_input_target"
        );
    }
    #[test]
    fn cargo_directory_alias_keeps_configuration_in_scope() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join("settings")).unwrap();
        std::fs::write(root.join("settings/config.toml"), "[build]\njobs=1\n").unwrap();
        std::fs::write(root.join("config-content"), "[net]\noffline=true\n").unwrap();
        symlink("../config-content", root.join("settings/config")).unwrap();
        symlink("settings", root.join(".cargo")).unwrap();
        symlink(".", root.join("directory-cycle")).unwrap();
        let before = crate::symbol_context::capture::inventory(root, "default");
        assert!(before.complete);
        assert!(before.files.contains_key(Path::new("settings/config.toml")));
        assert!(before.files.contains_key(Path::new("settings/config")));
        assert_eq!(
            before.links.as_ref().unwrap()[Path::new("settings/config")].status,
            "verified_input_file"
        );
        std::fs::write(root.join("settings/config.toml"), "[build]\njobs=2\n").unwrap();
        let after = crate::symbol_context::capture::inventory(root, "default");
        assert_ne!(before.files, after.files);
    }
    #[test]
    fn mappings_and_examples_stay_bounded() {
        let dir = tempfile::tempdir().unwrap();
        for n in 0..4097 {
            symlink(
                std::env::current_exe().unwrap(),
                dir.path().join(format!("tool{n}")),
            )
            .unwrap();
        }
        let inputs = crate::symbol_context::capture::inventory(dir.path(), "default");
        assert!(!inputs.complete);
        assert_eq!(inputs.links.unwrap().len(), 4096);
        let scan = inputs.scan.unwrap();
        assert!(scan
            .issues
            .iter()
            .any(|i| i.status == "symlink_record_limit"));
        let scope = scan.link_scope.unwrap();
        assert_eq!(scope.excluded, 4096);
        assert_eq!(scope.examples.len(), 16);
        assert_eq!(scope.examples_omitted, 4080);
    }
}
