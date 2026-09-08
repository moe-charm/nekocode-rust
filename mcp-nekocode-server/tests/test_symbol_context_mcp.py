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
            {"symbol": "value", "timeout_seconds": 121},
            {"symbol": "value", "save_packet": None},
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
                ({"path": "project", "at": "src/lib.rs:2:4", "save_packet": "capture.json", "output": "response.json", "all_features": True, "allow_build_scripts": True},
                 {"--save-packet": "capture.json", "--output": "response.json"}),
                ({"packet": "capture.json", "cursor": "packet:8", "max_items": 3},
                 {"--packet": "capture.json"}),
                ({"packet": "capture.json", "item": "item-1"},
                 {"--packet": "capture.json"}),
            ]
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
