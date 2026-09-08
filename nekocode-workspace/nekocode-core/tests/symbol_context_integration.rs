#![cfg(unix)]

use nekocode_core::{build_symbol_context, format_symbol_context_summary, SymbolContextRequest};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tempfile::tempdir;

struct RestoreEnv(&'static str, Option<std::ffi::OsString>);
impl RestoreEnv {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self(key, previous)
    }
}
impl Drop for RestoreEnv {
    fn drop(&mut self) {
        if let Some(value) = &self.1 {
            std::env::set_var(self.0, value);
        } else {
            std::env::remove_var(self.0);
        }
    }
}

#[test]
fn symbol_investigation_captures_exact_evidence_and_replays_without_tools() {
    let workspace = tempdir().unwrap();
    let storage = tempdir().unwrap();
    let root = workspace.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname='symbol_fixture'\nversion='0.1.0'\nedition='2021'\n[lib]\npath='src/日本 file.rs'\n").unwrap();
    let source = "pub fn target(x: i32) -> i32 {\n    let _url = \"https://example.invalid/a/b\";\n    let _path = r\"C:\\work\\file.rs\";\n    x / 2\n}\n\npub fn caller() -> i32 { target(4) }\n#[test]\nfn test_target() { assert_eq!(target(4), 2); }\npub fn unused() {}\npub fn duplicated() {}\nmod nested { pub fn duplicated() {} }\n";
    fs::write(root.join("src/日本 file.rs"), source).unwrap();
    let fake = storage.path().join("fake-ra");
    fs::write(
        &fake,
        include_str!("../src/symbol_context/fixtures/fake-ra.py"),
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let _ra = RestoreEnv::set("NEKOCODE_RUST_ANALYZER_PATH", &fake);
    let packet = storage.path().join("packet.json");
    let request = SymbolContextRequest {
        path: Some(root.to_path_buf()),
        symbol: Some("target".to_string()),
        save_packet: Some(packet.clone()),
        max_items: 1,
        ..Default::default()
    };
    let first = build_symbol_context(&request).unwrap();
    assert_eq!(first.status, "completed");
    assert_eq!(first.freshness.state, "source_stable");
    assert_eq!(first.freshness.backend_synchronization, "unverified");
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].relation, "definition");
    assert_eq!(
        first.items[0].containing_symbol.as_ref().unwrap().name,
        "target"
    );
    assert!(source.contains(&first.items[0].code));
    assert!(first.items[0].code.contains("https://example.invalid/a/b"));
    assert!(first.items[0].code.contains(r"C:\work\file.rs"));
    assert!(first.items[0].code.contains("x / 2"));
    assert!(first.totals.retained > first.totals.displayed);
    assert_eq!(
        first
            .queries
            .iter()
            .find(|query| query.method == "textDocument/references")
            .unwrap()
            .result_count,
        Some(2)
    );
    assert_eq!(
        first
            .queries
            .iter()
            .find(|query| query.method == "rust-analyzer/relatedTests")
            .unwrap()
            .status,
        "completed"
    );
    assert_eq!(
        serde_json::to_vec(&first).unwrap().len(),
        first.budget.serialized_bytes
    );
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&packet).unwrap()).unwrap();
    assert_eq!(
        saved["response"]["items"].as_array().unwrap().len(),
        first.totals.retained
    );
    assert!(saved["response"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["relation"] == "test_candidate"));

    // Replay cannot invoke Cargo or RA: neither executable is available.
    {
        let _path = RestoreEnv::set("PATH", "/nekocode-test-no-executables");
        let _ra_missing = RestoreEnv::set("NEKOCODE_RUST_ANALYZER_PATH", "/nekocode-test-no-ra");
        let replay = SymbolContextRequest {
            packet: Some(packet.clone()),
            cursor: first.continuation.next_cursor.clone(),
            max_items: 1,
            ..Default::default()
        };
        let next = build_symbol_context(&replay).unwrap();
        assert_eq!(next.items[0].relation, "contract");
        assert_ne!(next.items[0].id, first.items[0].id);
        let expand = SymbolContextRequest {
            packet: Some(packet.clone()),
            item: Some(first.items[0].id.clone()),
            ..Default::default()
        };
        let expanded = build_symbol_context(&expand).unwrap();
        assert!(source.contains(&expanded.items[0].code));
        assert!(expanded.items[0].code.ends_with("}\n"));
        assert_eq!(expanded.freshness.state, "source_stable");
        let tiny_expansion = build_symbol_context(&SymbolContextRequest {
            budget: 1,
            ..expand.clone()
        })
        .unwrap();
        assert_eq!(tiny_expansion.status, "output_limited");
        assert!(tiny_expansion.items.is_empty());
        assert!(tiny_expansion.continuation.next_cursor.is_none());
        assert!(tiny_expansion
            .omissions
            .iter()
            .any(|omission| omission.kind == "expanded_item"));
        let wrong = SymbolContextRequest {
            packet: Some(packet.clone()),
            cursor: Some("another_packet:1".to_string()),
            ..Default::default()
        };
        assert!(build_symbol_context(&wrong)
            .unwrap_err()
            .to_string()
            .contains("another packet"));
        for budget in [1, 300, 1000, 8000] {
            let bounded = build_symbol_context(&SymbolContextRequest {
                packet: Some(packet.clone()),
                budget,
                ..Default::default()
            })
            .unwrap();
            let size = serde_json::to_vec(&bounded).unwrap().len();
            assert_eq!(bounded.budget.serialized_bytes, size);
            assert!(size <= bounded.budget.max_bytes || bounded.budget.exceeded);
            assert!(bounded
                .queries
                .iter()
                .any(|query| query.method == "textDocument/references"));
            if bounded.items.is_empty() {
                assert_eq!(bounded.status, "output_limited");
                assert!(bounded.continuation.next_cursor.is_none());
                assert!(bounded
                    .omissions
                    .iter()
                    .any(|omission| omission.reason.contains("no_progress")));
            }
        }
        fs::write(root.join("src/new.rs"), "pub fn new_source() {}\n").unwrap();
        let stale = build_symbol_context(&expand).unwrap();
        assert_eq!(stale.status, "stale");
        assert!(stale
            .freshness
            .changed_inputs
            .contains(&PathBuf::from("src/new.rs")));
        assert_eq!(stale.items[0].code, expanded.items[0].code);
        fs::remove_file(root.join("src/new.rs")).unwrap();
    }

    let ambiguous = build_symbol_context(&SymbolContextRequest {
        path: Some(root.to_path_buf()),
        symbol: Some("duplicated".to_string()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(ambiguous.status, "ambiguous");
    assert_eq!(ambiguous.target.candidates.len(), 2);
    assert!(ambiguous.items.is_empty());
    let zero = build_symbol_context(&SymbolContextRequest {
        path: Some(root.to_path_buf()),
        symbol: Some("unused".to_string()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        zero.queries
            .iter()
            .find(|query| query.method == "textDocument/references")
            .unwrap()
            .result_count,
        Some(0)
    );
    assert!(zero.continuation.next_cursor.is_none());
    let at = build_symbol_context(&SymbolContextRequest {
        path: Some(root.to_path_buf()),
        at: Some("src/日本 file.rs:1:8".to_string()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(at.target.resolution, "selected");
    assert_eq!(at.items[0].location.path, PathBuf::from("src/日本 file.rs"));
    let line_only = build_symbol_context(&SymbolContextRequest {
        path: Some(root.to_path_buf()),
        at: Some("src/日本 file.rs:4".to_string()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(line_only.target.location.as_ref().unwrap().start.line, 1);
    assert_eq!(line_only.target.location.as_ref().unwrap().start.column, 8);
    assert_eq!(line_only.items[0].relation, "definition");
    let wrong_root = build_symbol_context(&SymbolContextRequest {
        path: Some(storage.path().to_path_buf()),
        packet: Some(packet.clone()),
        ..Default::default()
    })
    .unwrap_err();
    assert!(wrong_root.to_string().contains("does not match"));

    let mut tampered = saved;
    tampered["response"]["items"][0]["code"] = "tampered".into();
    fs::write(&packet, serde_json::to_vec(&tampered).unwrap()).unwrap();
    assert!(build_symbol_context(&SymbolContextRequest {
        packet: Some(packet),
        ..Default::default()
    })
    .unwrap_err()
    .to_string()
    .contains("integrity mismatch"));

    // Rust include! sources are not necessarily named *.rs, but their exact
    // captured bytes must still participate in capture and replay freshness.
    let included = root.join("src/body.inc");
    fs::write(&included, "pub fn included() -> u32 { 7 }\n").unwrap();
    let included_packet = storage.path().join("included.json");
    let included_capture = build_symbol_context(&SymbolContextRequest {
        path: Some(root.to_path_buf()),
        at: Some("src/body.inc:1:8".to_string()),
        save_packet: Some(included_packet.clone()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(included_capture.freshness.state, "source_stable");
    let old_code = included_capture.items[0].code.clone();
    fs::write(&included, "pub fn included() -> u32 { 9 }\n").unwrap();
    let included_replay = build_symbol_context(&SymbolContextRequest {
        packet: Some(included_packet),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(included_replay.status, "stale");
    assert!(included_replay
        .freshness
        .changed_inputs
        .contains(&PathBuf::from("src/body.inc")));
    assert_eq!(included_replay.items[0].code, old_code);

    let _missing = RestoreEnv::set("NEKOCODE_RUST_ANALYZER_PATH", "/nekocode-test-no-ra");
    let unavailable = build_symbol_context(&SymbolContextRequest {
        path: Some(root.to_path_buf()),
        symbol: Some("target".to_string()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(unavailable.status, "backend_unavailable");
    assert_eq!(unavailable.queries[0].result_count, None);
    let mut terminal = unavailable;
    terminal
        .limitations
        .push("untrusted\u{1b}[2J terminal".to_string());
    assert!(!format_symbol_context_summary(&terminal).contains('\u{1b}'));
}
