"""Final MCP response and path/selector contracts for symbol investigations."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import mcp_server_rust_first as gateway

REPO_ROOT = Path(__file__).resolve().parents[2]


class SymbolContextMCPTest(unittest.TestCase):
    def call(self, server, arguments, name=gateway.CONTEXT_TOOL):
        _, response = server.dispatch({
            "jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": {"name": name, "arguments": arguments},
        })
        return response["result"]

    def test_final_tools_call_preserves_source_and_query_states(self):
        source = 'let ratio = value / divisor; // https://example.test/a/b\nlet path = r"C:\\tmp\\code";'
        server = gateway.RustFirstMCPServer()
        for status in ("complete", "unsupported", "timed_out", "failed"):
            payload = {
                "contract_version": "symbol-context-v1",
                "items": [{"id": "item-1", "code": source}],
                "queries": [{"kind": "references", "status": status, "count": 0}],
                "continuation": {"cursor": "packet-1:8"},
            }
            with self.subTest(status=status), mock.patch.object(server, "_run_cli", return_value=payload):
                result = self.call(server, {"at": "src/lib.rs:1"})
                self.assertFalse(result["isError"])
                self.assertEqual(result["structuredContent"], payload)
                self.assertEqual(json.loads(result["content"][0]["text"]), payload)
                self.assertEqual(result["structuredContent"]["items"][0]["code"], source)

    def test_invalid_tool_names_return_errors_instead_of_crashing(self):
        server = gateway.RustFirstMCPServer()
        for name in ([], {}, 1, None):
            with self.subTest(name=name):
                self.assertTrue(self.call(server, {}, name)["isError"])

    def test_symbol_option_conflicts_are_rejected_before_execution(self):
        server = gateway.RustFirstMCPServer()
        cases = [
            {"at": "src/lib.rs:1", "symbol": "value"},
            {"at": "src/lib.rs:1", "compare_ref": "HEAD"},
            {"symbol": "value", "diagnostics": False},
            {"symbol": "value", "diagnostic_producer": "cargo-check"},
            {"symbol": "value", "excerpt_lines": 8},
            {"cursor": "packet:8"},
            {"item": "item-1"},
            {"max_items": 8},
            {"packet": "saved.json", "item": "item-1", "cursor": "packet:8"},
            {"packet": "saved.json", "allow_build_scripts": True},
            {"packet": "saved.json", "timeout_seconds": 60},
            {"at": []},
            {"symbol": "value", "max_items": True},
            {"symbol": "value", "timeout_seconds": 601},
            {"symbol": "value", "save_packet": None},
            {"packet": "saved.json", "text_candidates": True},
            {"symbol": "value", "text_candidates": "yes"},
        ]
        with mock.patch.object(gateway.subprocess, "Popen") as launch:
            for arguments in cases:
                with self.subTest(arguments=arguments):
                    self.assertTrue(self.call(server, arguments)["isError"])
            launch.assert_not_called()

    def test_caller_paths_and_replay_arguments_match_in_binary_and_cargo_modes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            caller = root / "caller"
            caller.mkdir()
            tool_workspace = root / "tool-workspace"
            tool_workspace.mkdir()
            fake_cli = root / "echo-cli"
            fake_cli.write_text(
                f"#!{sys.executable}\n"
                "import json, os, sys\n"
                "args = sys.argv[1:]\n"
                "if '--' in args: args = args[args.index('--') + 1:]\n"
                "print(json.dumps({'argv': args, 'analyzer': os.environ.get('NEKOCODE_RUST_ANALYZER_PATH')}))\n",
                encoding="utf-8",
            )
            fake_cli.chmod(0o755)
            cases = [
                ({"path": "project", "compare_ref": "HEAD", "baseline": "base.json", "output": "response.json"},
                 {"--baseline": "base.json", "--output": "response.json"}),
                ({"path": "project", "at": "src/lib.rs:2:4", "save_packet": "capture.json", "output": "response.json", "all_features": True, "allow_build_scripts": True, "text_candidates": True},
                 {"--save-packet": "capture.json", "--output": "response.json"}),
                ({"packet": "capture.json", "cursor": "packet:8", "max_items": 3},
                 {"--packet": "capture.json"}),
                ({"packet": "capture.json", "item": "item-1"},
                 {"--packet": "capture.json"}),
            ]
            cases.extend(
                ({"path": "project", "symbol": "value", "timeout_seconds": seconds}, {})
                for seconds in (300, 600)
            )
            self.assertGreaterEqual(gateway.COMMAND_TIMEOUT_SECONDS, 660)
            with mock.patch.dict(os.environ, {"NEKOCODE_CLI_CWD": str(caller), "NEKOCODE_RUST_ANALYZER_PATH": "bin/analyzer"}), mock.patch.object(gateway.shutil, "which", return_value=str(fake_cli)):
                for arguments, paths in cases:
                    results = []
                    for binary in (fake_cli, None):
                        with self.subTest(arguments=arguments, binary=binary):
                            server = gateway.RustFirstMCPServer(workspace_dir=tool_workspace, binary_path=binary)
                            # An inherited binary override must not hide the Cargo case.
                            server.binary_path = binary
                            result = self.call(server, arguments)
                            self.assertFalse(result["isError"], result)
                            payload = result["structuredContent"]
                            self.assertEqual(json.loads(result["content"][0]["text"]), payload)
                            argv = payload["argv"]
                            for flag, relative in paths.items():
                                self.assertEqual(argv[argv.index(flag) + 1], str(caller / relative))
                            if arguments.get("text_candidates"):
                                self.assertIn("--text-candidates", argv)
                            if "timeout_seconds" in arguments:
                                self.assertEqual(argv[argv.index("--timeout-seconds") + 1], str(arguments["timeout_seconds"]))
                            if "path" in arguments:
                                self.assertEqual(argv[:2], ["context", str(caller / "project")])
                            else:
                                self.assertEqual(argv[:2], ["context", "--packet"])
                                self.assertNotIn("--timeout-seconds", argv)
                            self.assertEqual(payload["analyzer"], str(caller / "bin/analyzer"))
                            results.append(payload)
                    self.assertEqual(*results)


@unittest.skipUnless(os.name == "posix", "executable fake LSP fixture uses a POSIX shebang")
class SavedSymbolContextMCPTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        workspace = REPO_ROOT / "nekocode-workspace"
        subprocess.run(
            ["cargo", "build", "--locked", "--offline", "-p", "nekocode"],
            cwd=workspace, capture_output=True, text=True, check=True, timeout=120,
        )
        metadata = subprocess.run(
            ["cargo", "metadata", "--locked", "--offline", "--no-deps", "--format-version", "1"],
            cwd=workspace, capture_output=True, text=True, check=True, timeout=30,
        )
        cls.binary = Path(json.loads(metadata.stdout)["target_directory"]) / "debug" / "nekocode"

    @unittest.skipUnless(shutil.which("rg"), "text candidate integration requires ripgrep")
    def test_text_candidates_keep_macro_and_comment_matches_separate_and_replayable(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            root = base / "project"
            (root / "src").mkdir(parents=True)
            (root / "Cargo.toml").write_text('[package]\nname="text-candidates"\nversion="0.1.0"\nedition="2021"\n')
            (root / "src/lib.rs").write_text('pub fn target() -> bool { true }\npub fn caller() { let _ = "猫"; target(); assert!(target()); } // target\n')
            (root / "target").mkdir()
            (root / "target/ignored.rs").write_text('// target\n')
            fixture = (REPO_ROOT / "nekocode-workspace/nekocode-core/src/symbol_context/fixtures/fake-ra.py").read_text()
            fixture = fixture.replace('                    result.append({"uri": uri, "range": span})', '                    if not text[max(0, word.start()-8):word.start()].endswith(("assert!(", "// ")):\n                        result.append({"uri": uri, "range": span})')
            backend = base / "backend"
            backend.write_text(fixture)
            backend.chmod(0o755)
            env = dict(os.environ, NEKOCODE_RUST_ANALYZER_PATH=str(backend))
            packet = base / "packet.json"
            result = subprocess.run([str(self.binary), "context", str(root), "--at", "src/lib.rs:1", "--text-candidates", "--max-items", "100", "--save-packet", str(packet)], env=env, capture_output=True, text=True, timeout=20, check=True)
            artifact = json.loads(result.stdout)
            coverage = {row["area"]: row for row in artifact["coverage"]}
            self.assertEqual(coverage["features"]["verification"], "unverified")
            self.assertEqual(coverage["cfg_test"]["requested"], "requested_true_unverified")
            self.assertEqual(coverage["macros"]["verification"], "unverified")
            self.assertEqual(coverage["tests"]["verification"], "not_executed")
            verification = artifact["freshness"]["verification"]
            self.assertEqual(verification["basis"], "capture_start_vs_end")
            self.assertEqual(verification["verdict"], "match")
            self.assertGreater(verification["matched"], 0)
            # Simulate an old packet without either additive optional field.
            import hashlib
            old = json.loads(packet.read_text())
            old["response"].pop("coverage", None)
            old["response"]["freshness"].pop("verification", None)
            old["integrity_sha256"] = ""
            old["integrity_sha256"] = "sha256:" + hashlib.sha256(json.dumps(old, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
            old_packet = base / "old.packet.json"
            old_packet.write_text(json.dumps(old, ensure_ascii=False))
            old_replay = subprocess.run([str(self.binary), "context", "--packet", str(old_packet)], capture_output=True, text=True, timeout=10, check=True)
            old_response = json.loads(old_replay.stdout)
            self.assertEqual(old_response["freshness"]["verification"]["basis"], "packet_inputs_vs_current")
            self.assertEqual(old_response["freshness"]["verification"]["verdict"], "match")
            self.assertIn("coverage", old_response)
            query = next(q for q in artifact["queries"] if q["method"] == "rg/text_candidates")
            self.assertEqual(query["status"], "completed", artifact)
            self.assertEqual(query["result_count"], 2, artifact)
            refs = next(q for q in artifact["queries"] if q["method"] == "textDocument/references")
            self.assertEqual(refs["result_count"], 1)
            items = [i for i in artifact["items"] if i["relation"] == "unconfirmed_text_candidate"]
            self.assertEqual(len(items), 2)
            self.assertTrue(all(i["backend"] == "ripgrep" and i["location"]["path"] == "src/lib.rs" for i in items))
            self.assertNotEqual(items[0]["location"]["start"]["column"], items[1]["location"]["start"]["column"])
            replay = subprocess.run([str(self.binary), "context", "--packet", str(packet), "--item", items[0]["id"]], env=dict(env, PATH="/no-programs"), capture_output=True, text=True, timeout=10, check=True)
            self.assertIn('assert!(target())', json.loads(replay.stdout)["items"][0]["code"])
            delta = subprocess.run([str(self.binary), "context", "--packet", str(packet), "--compare-packet", str(packet)], capture_output=True, text=True, timeout=10, check=True)
            self.assertEqual(json.loads(delta.stdout)["before"]["captured_references"], 1)
            # Replay must preserve capture history independently of current hashes.
            def write_variant(name, edit):
                value = json.loads(packet.read_text())
                edit(value["response"])
                value["integrity_sha256"] = ""
                value["integrity_sha256"] = "sha256:" + hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
                path = base / name
                path.write_text(json.dumps(value))
                return path
            def replay(path, *args):
                result = subprocess.run([str(self.binary), "context", "--packet", str(path), *args], capture_output=True, text=True, timeout=10, check=True)
                return json.loads(result.stdout)
            def changed_capture(response):
                response["status"] = "partial"
                response["freshness"].update(state="changed_during_observation", source_state="changed", changed_inputs=["src/lib.rs"])
            history = replay(write_variant("history.json", changed_capture))
            self.assertEqual(history["freshness"]["verification"]["verdict"], "match")
            self.assertEqual(history["freshness"]["changed_inputs"], ["src/lib.rs"])
            self.assertEqual(history["status"], "partial")
            small = replay(packet, "--budget", "1200")
            self.assertNotIn("coverage", small)
            self.assertTrue(any(o["kind"] == "coverage" for o in small["omissions"]))
            self.assertTrue(small["items"], small)
            self.assertFalse(small["budget"]["exceeded"], small)
            # Text failure alone is comparable; semantic failures still fail closed.
            for failure in ("failed", "timed_out", "output_limited"):
                def text_failure(response):
                    response["status"] = "timed_out" if failure == "timed_out" else "partial"
                    q = next(q for q in response["queries"] if q["method"] == "rg/text_candidates")
                    q.update(status=failure, result_count=None)
                variant = write_variant("text-failure.json", text_failure)
                comparison = replay(variant, "--compare-packet", str(packet))
                self.assertNotEqual(comparison["comparison_status"], "not_comparable", comparison)
                def semantic_failure(response):
                    text_failure(response)
                    next(q for q in response["queries"] if q["method"] == "textDocument/references")["status"] = "failed"
                variant = write_variant("semantic-failure.json", semantic_failure)
                self.assertEqual(replay(variant, "--compare-packet", str(packet))["comparison_status"], "not_comparable")
            def semantic_omission(response):
                text_failure(response)
                response["omissions"].append({"kind": "reference", "reason": "collection_limit", "count": 1})
            variant = write_variant("semantic-omission.json", semantic_omission)
            self.assertEqual(replay(variant, "--compare-packet", str(packet))["comparison_status"], "not_comparable")
            tools_dir = base / "tools"
            tools_dir.mkdir()
            (tools_dir / "rg").write_text('#!/bin/sh\nexit 2\n')
            (tools_dir / "rg").chmod(0o755)
            failed_env = dict(env, PATH=str(tools_dir) + os.pathsep + env["PATH"])
            requests = [{"at": "src/lib.rs:1", "text_candidates": True}, {"at": "src/lib.rs:1"}]
            session_result = subprocess.run([str(self.binary), "context", str(root), "--session"], input=''.join(json.dumps(r) + "\n" for r in requests), env=failed_env, capture_output=True, text=True, timeout=20, check=True)
            failed, next_query = [json.loads(line) for line in session_result.stdout.splitlines()]
            failure = next(q for q in failed["context"]["queries"] if q["method"] == "rg/text_candidates")
            self.assertEqual(failure["status"], "failed")
            self.assertIsNone(failure["result_count"])
            self.assertTrue(next_query["backend_reused"], next_query)
            source_path = root / "src/lib.rs"
            source_path.write_text(source_path.read_text() + '// target\n' * 70)
            stale_replay = subprocess.run([str(self.binary), "context", "--packet", str(old_packet)], capture_output=True, text=True, timeout=10, check=True)
            stale = json.loads(stale_replay.stdout)
            self.assertEqual(stale["freshness"]["verification"]["verdict"], "changed")
            self.assertEqual(stale["freshness"]["verification"]["modified"], 1)
            self.assertEqual(stale["freshness"]["backend_synchronization"], "unverified")

            limited = subprocess.run([str(self.binary), "context", str(root), "--at", "src/lib.rs:1", "--text-candidates", "--max-items", "100", "--budget", "100000", "--save-packet", str(base / "limited.json")], env=env, capture_output=True, text=True, timeout=20, check=True)
            partial = json.loads(limited.stdout)
            self.assertEqual(len([i for i in partial["items"] if i["relation"] == "unconfirmed_text_candidate"]), 64)
            self.assertTrue(any(o["kind"] == "unconfirmed_text_candidates" for o in partial["omissions"]))
            comparison = replay(base / "limited.json", "--compare-packet", str(base / "limited.json"))
            self.assertNotEqual(comparison["comparison_status"], "not_comparable", comparison)
            source_path.unlink()
            source_path.mkdir()
            replaced = replay(old_packet)
            self.assertEqual(replaced["status"], "stale")
            self.assertEqual(replaced["freshness"]["verification"]["modified"], 1)
            self.assertIn("src/lib.rs", replaced["freshness"]["changed_inputs"])



    def test_session_rejects_oversized_input_and_conflicting_cli_options(self):
        with tempfile.TemporaryDirectory() as temporary:
            oversized = subprocess.run([str(self.binary), "context", temporary, "--session"], input="x" * 65537, capture_output=True, text=True, timeout=10)
            self.assertNotEqual(oversized.returncode, 0)
            self.assertIn("64 KiB", oversized.stderr)
            for extra in (["--at", "src/lib.rs:1"], ["--budget", "10"], ["--all-features"], ["--format", "summary"]):
                conflict = subprocess.run([str(self.binary), "context", temporary, "--session", *extra], input="", capture_output=True, text=True, timeout=10)
                self.assertNotEqual(conflict.returncode, 0, extra)

    def test_session_reuses_process_and_restarts_on_source_and_feature_changes(self):
        import select
        import time
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            root = base / "project"
            (root / "src").mkdir(parents=True)
            (root / "Cargo.toml").write_text('[package]\nname="session-check"\nversion="0.1.0"\nedition="2021"\n[features]\nextra=[]\n')
            source = root / "src/lib.rs"
            source.write_text('pub fn target() {}\npub fn caller() { target(); }\n')
            subprocess.run(["cargo", "generate-lockfile", "--offline"], cwd=root, check=True, capture_output=True)
            starts = base / "starts"
            fixture = REPO_ROOT / "nekocode-workspace/nekocode-core/src/symbol_context/fixtures/fake-ra.py"
            wrapper = base / "backend"
            wrapper.write_text(f"#!{sys.executable}\nimport os, pathlib\nwith pathlib.Path({str(starts)!r}).open('a') as f: f.write(str(os.getpid())+'\\n')\nos.execv({sys.executable!r}, [{sys.executable!r}, {str(fixture)!r}])\n")
            wrapper.chmod(0o755)
            env = dict(os.environ, NEKOCODE_RUST_ANALYZER_PATH=str(wrapper))
            proc = subprocess.Popen([str(self.binary), "context", str(root), "--session"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
            def ask(request):
                proc.stdin.write(json.dumps(request) + "\n")
                proc.stdin.flush()
                self.assertTrue(select.select([proc.stdout], [], [], 15)[0], "session response timed out")
                response = json.loads(proc.stdout.readline())
                import jsonschema
                from referencing import Registry, Resource
                context_schema = json.loads((REPO_ROOT / "schemas/symbol-context-v1.schema.json").read_text())
                registry = Registry().with_resource("https://nekocode.dev/schemas/symbol-context-v1.schema.json", Resource.from_contents(context_schema))
                schema = json.loads((REPO_ROOT / "schemas/symbol-session-v1.schema.json").read_text())
                jsonschema.Draft202012Validator(schema, registry=registry).validate(response)
                return response
            try:
                first = ask({"at": "src/lib.rs:1", "timeout_seconds": 1})
                self.assertIsNone(first["error"], first)
                self.assertEqual(first["context"]["status"], "completed", first)
                self.assertFalse(first["backend_reused"])
                time.sleep(1.1)  # The next observation must get a new deadline.
                second = ask({"at": "src/lib.rs:2", "timeout_seconds": 10})
                self.assertTrue(second["backend_reused"], second)
                self.assertEqual(len(starts.read_text().splitlines()), 1)
                source.write_text(source.read_text() + 'pub fn added() { target(); }\n')
                changed = ask({"at": "src/lib.rs:1"})
                self.assertFalse(changed["backend_reused"], changed)
                self.assertEqual(len(starts.read_text().splitlines()), 2)
                configured = ask({"at": "src/lib.rs:1", "all_features": True})
                self.assertFalse(configured["backend_reused"], configured)
                self.assertEqual(len(starts.read_text().splitlines()), 3)
                self.assertIsNotNone(ask({"path": str(root), "at": "src/lib.rs:1"})["error"])
                self.assertIsNotNone(ask({"at": "src/lib.rs:1", "unexpected": True})["error"])
                (root / "unobserved-link").symlink_to(source)
                for expected_starts in (4, 5):
                    incomplete = ask({"at": "src/lib.rs:1", "all_features": True})
                    self.assertFalse(incomplete["backend_reused"], incomplete)
                    self.assertFalse(incomplete["context"]["freshness"]["input_scan_complete"])
                    self.assertEqual(len(starts.read_text().splitlines()), expected_starts)
                (root / "unobserved-link").unlink()
                stable_request = {"at": "src/lib.rs:1", "all_features": True}
                self.assertIsNone(ask(stable_request)["error"])
                count = len(starts.read_text().splitlines())
                for invalid in ({"budget": 0}, {"timeout_seconds": 0}, {"at": "src/lib.rs:9999"}, {"save_packet": str(base / "missing/packet.json")}):
                    rejected = ask({**stable_request, **invalid})
                    self.assertIsNotNone(rejected["error"])
                    self.assertEqual(rejected["backend_reused"], "save_packet" in invalid, rejected)
                    recovered = ask(stable_request)
                    self.assertTrue(recovered["backend_reused"], recovered)
                    self.assertEqual(len(starts.read_text().splitlines()), count)
                for invalid in ({"path": str(root)}, {"unexpected": True}):
                    rejected = ask(invalid)
                    self.assertIsNotNone(rejected["error"])
                    self.assertFalse(rejected["backend_reused"], rejected)
                proc.stdin.write("{invalid json\n")
                proc.stdin.flush()
                self.assertTrue(select.select([proc.stdout], [], [], 15)[0])
                self.assertFalse(json.loads(proc.stdout.readline())["backend_reused"])
                extra = root / "src/extra.inc"
                extra.write_text("pub fn extra_target() {}\n")
                self.assertIsNone(ask({"at": "src/extra.inc:1", "all_features": True})["error"])
                count += 1
                returned = ask(stable_request)
                self.assertTrue(returned["backend_reused"], returned)
                self.assertEqual(len(starts.read_text().splitlines()), count)
                extra.write_text("pub fn edited_target() {}\n")
                self.assertFalse(ask(stable_request)["backend_reused"])
                count += 1
                extra.unlink()
                self.assertFalse(ask(stable_request)["backend_reused"])
                count += 1
                self.assertEqual(len(starts.read_text().splitlines()), count)
                import signal
                dead_pid = int(starts.read_text().splitlines()[-1])
                os.kill(dead_pid, signal.SIGKILL)
                # Observe actual process exit before testing acquire's liveness probe.
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    try:
                        state = Path(f"/proc/{dead_pid}/stat").read_text().split(") ", 1)[1][0]
                        if state == "Z": break
                    except FileNotFoundError: break
                    time.sleep(0.01)
                recovered = ask(stable_request)
                self.assertFalse(recovered["backend_reused"], recovered)
                self.assertEqual(recovered["context"]["status"], "completed", recovered)
                self.assertEqual(recovered["context"]["backend"]["health"], "ok")
                self.assertEqual(len(starts.read_text().splitlines()), count + 1)
                proc.stdin.close()
                self.assertEqual(proc.wait(timeout=10), 0, proc.stderr.read())
                for pid in starts.read_text().splitlines():
                    with self.assertRaises(ProcessLookupError):
                        os.kill(int(pid), 0)
            finally:
                if proc.poll() is None:
                    proc.kill()
                    proc.wait(timeout=10)
                for stream in (proc.stdin, proc.stdout, proc.stderr):
                    stream.close()

    def test_final_tools_call_pages_and_expands_real_saved_core_packet(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "src").mkdir()
            (root / "Cargo.toml").write_text(
                '[package]\nname="mcp-symbol-paging"\nversion="0.1.0"\nedition="2021"\n',
                encoding="utf-8",
            )
            (root / "src/lib.rs").write_text(
                'pub fn target(value: u32) -> u32 {\n'
                '    let _url = "https://example.test/a/b";\n'
                '    let _path = r"C:\\work\\item";\n'
                '    value / 2\n'
                '}\n'
                'pub fn caller() -> u32 { target(8) }\n'
                '#[test]\nfn target_test() { assert_eq!(target(4), 2); }\n',
                encoding="utf-8",
            )
            analyzer = root / "fake-ra"
            shutil.copyfile(
                REPO_ROOT / "nekocode-workspace/nekocode-core/src/symbol_context/fixtures/fake-ra.py",
                analyzer,
            )
            analyzer.chmod(0o755)
            server = gateway.RustFirstMCPServer(binary_path=self.binary)

            def call(arguments):
                _, response = server.dispatch({
                    "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                    "params": {"name": gateway.CONTEXT_TOOL, "arguments": arguments},
                })
                result = response["result"]
                self.assertFalse(result["isError"], result)
                payload = result["structuredContent"]
                self.assertEqual(json.loads(result["content"][0]["text"]), payload)
                self.assertEqual(payload["contract_version"], "symbol-context-v1")
                return payload

            with mock.patch.dict(os.environ, {"NEKOCODE_CLI_CWD": str(root), "NEKOCODE_RUST_ANALYZER_PATH": str(analyzer)}):
                initial = call({"at": "src/lib.rs:1:8", "save_packet": "saved.json", "max_items": 1})
                self.assertEqual(len(initial["items"]), 1)
                cursor = initial["continuation"]["next_cursor"]
                self.assertIsInstance(cursor, str)
                item_id = initial["items"][0]["id"]
                analyzer.unlink()
                with mock.patch.dict(os.environ, {"PATH": str(root / "no-executables")}):
                    page = call({"packet": "saved.json", "cursor": cursor, "max_items": 1})
                    self.assertEqual(page["packet_id"], initial["packet_id"])
                    self.assertEqual(len(page["items"]), 1)
                    self.assertNotEqual(page["items"][0]["id"], item_id)
                    expanded = call({"packet": "saved.json", "item": item_id})
                self.assertEqual(expanded["packet_id"], initial["packet_id"])
                code = expanded["items"][0]["code"]
                self.assertIn("https://example.test/a/b", code)
                self.assertIn("value / 2", code)
                self.assertIn(r"C:\work\item", code)


if __name__ == "__main__":
    unittest.main()
