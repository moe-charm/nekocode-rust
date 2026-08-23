"""Protocol smoke test for the standalone Rust-first MCP gateway."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SERVER = REPO_ROOT / "mcp-nekocode-server" / "mcp_server_rust_first.py"
sys.path.insert(0, str(SERVER.parent))


class RustFirstMCPProtocolTest(unittest.TestCase):
    @staticmethod
    def _stop(process: subprocess.Popen[str]) -> None:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)
        if process.stdin is not None and not process.stdin.closed:
            process.stdin.close()
        if process.stdout is not None and not process.stdout.closed:
            process.stdout.close()
        if process.stderr is not None and not process.stderr.closed:
            process.stderr.close()

    def test_initialize_list_and_snapshot_over_stdio(self) -> None:
        process = subprocess.Popen(
            [sys.executable, str(SERVER)],
            cwd=REPO_ROOT,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.addCleanup(self._stop, process)
        assert process.stdin is not None
        assert process.stdout is not None

        requests = [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
            {
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {"name": "nekocode_snapshot", "arguments": {"path": "."}},
            },
        ]
        for request in requests:
            process.stdin.write(json.dumps(request) + "\n")
            process.stdin.flush()

        responses = [json.loads(process.stdout.readline()) for _ in requests]
        self.assertEqual(responses[0]["result"]["serverInfo"]["name"], "nekocode-rust-first")
        self.assertEqual(
            [tool["name"] for tool in responses[1]["result"]["tools"]],
            ["nekocode_snapshot", "nekocode_context"],
        )
        snapshot_result = responses[2]["result"]
        self.assertFalse(snapshot_result["isError"])
        self.assertEqual(snapshot_result["structuredContent"]["evidence"], "tool-confirmed")
        self.assertEqual(snapshot_result["structuredContent"]["contract_version"], "snapshot-v1")
        self.assertEqual(snapshot_result["structuredContent"]["artifact_kind"], "snapshot")
        self.assertNotIn(str(REPO_ROOT), snapshot_result["content"][0]["text"])

        process.stdin.close()
        process.wait(timeout=10)

    def test_prebuilt_cli_mode_does_not_require_cargo_workspace(self) -> None:
        from mcp_server_rust_first import RustFirstMCPServer

        with tempfile.TemporaryDirectory() as temp_dir:
            temp = Path(temp_dir)
            fake_cli = temp / "nekocode"
            fake_cli.write_text(
                "#!/usr/bin/env python3\n"
                "import json\n"
                "print(json.dumps({'evidence': 'tool-confirmed', 'mode': 'prebuilt'}))\n",
                encoding="utf-8",
            )
            fake_cli.chmod(0o755)
            server = RustFirstMCPServer(workspace_dir=temp / "missing", binary_path=fake_cli)
            result = server.handle_tool_call(
                {"name": "nekocode_snapshot", "arguments": {"path": "."}}
            )

        self.assertFalse(result["isError"])
        self.assertEqual(result["structuredContent"]["mode"], "prebuilt")

    def test_context_forwards_working_tree_and_feature_options(self) -> None:
        from mcp_server_rust_first import RustFirstMCPServer

        with tempfile.TemporaryDirectory() as temp_dir:
            temp = Path(temp_dir)
            fake_cli = temp / "nekocode"
            fake_cli.write_text(
                "#!/usr/bin/env python3\n"
                "import json, sys\n"
                "print(json.dumps({'argv': sys.argv[1:]}))\n",
                encoding="utf-8",
            )
            fake_cli.chmod(0o755)
            server = RustFirstMCPServer(workspace_dir=temp / "missing", binary_path=fake_cli)
            result = server.handle_tool_call(
                {
                    "name": "nekocode_context",
                    "arguments": {
                        "path": ".",
                        "compare_ref": "HEAD~1",
                        "budget": 1200,
                        "working_tree": True,
                        "all_features": True,
                        "excerpt_lines": 12,
                        "baseline": "/tmp/baseline.json",
                    },
                }
            )

        self.assertFalse(result["isError"])
        argv = result["structuredContent"]["argv"]
        self.assertEqual(argv[:2], ["context", "."])
        self.assertIn("--compare-ref", argv)
        self.assertIn("HEAD~1", argv)
        self.assertIn("--budget", argv)
        self.assertIn("1200", argv)
        self.assertIn("--working-tree", argv)
        self.assertIn("--all-features", argv)
        self.assertIn("--excerpt-lines", argv)
        self.assertIn("12", argv)
        self.assertIn("--baseline", argv)
        self.assertIn("<path>", argv)

    def test_snapshot_forwards_explicit_options(self) -> None:
        from mcp_server_rust_first import RustFirstMCPServer

        with tempfile.TemporaryDirectory() as temp_dir:
            temp = Path(temp_dir)
            fake_cli = temp / "nekocode"
            fake_cli.write_text(
                "#!/usr/bin/env python3\n"
                "import json, sys\n"
                "print(json.dumps({'argv': sys.argv[1:]}))\n",
                encoding="utf-8",
            )
            fake_cli.chmod(0o755)
            server = RustFirstMCPServer(workspace_dir=temp / "missing", binary_path=fake_cli)
            result = server.handle_tool_call(
                {
                    "name": "nekocode_snapshot",
                    "arguments": {
                        "path": ".",
                        "analysis": "cargo-check",
                        "output": "/tmp/baseline.json",
                        "all_features": True,
                    },
                }
            )

        self.assertFalse(result["isError"])
        argv = result["structuredContent"]["argv"]
        self.assertEqual(argv[:2], ["snapshot", "."])
        self.assertIn("--analysis", argv)
        self.assertIn("cargo-check", argv)
        self.assertIn("--output", argv)
        self.assertIn("<path>", argv)
        self.assertIn("--all-features", argv)


if __name__ == "__main__":
    unittest.main()
