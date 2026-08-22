"""Protocol smoke test for the standalone Rust-first MCP gateway."""

from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SERVER = REPO_ROOT / "mcp-nekocode-server" / "mcp_server_rust_first.py"


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

    def test_initialize_list_and_index_over_stdio(self) -> None:
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
                "params": {"name": "index", "arguments": {"path": "."}},
            },
        ]
        for request in requests:
            process.stdin.write(json.dumps(request) + "\n")
            process.stdin.flush()

        responses = [json.loads(process.stdout.readline()) for _ in requests]
        self.assertEqual(responses[0]["result"]["serverInfo"]["name"], "nekocode-rust-first")
        self.assertEqual(
            [tool["name"] for tool in responses[1]["result"]["tools"]], ["index", "context"]
        )
        index_result = responses[2]["result"]
        self.assertFalse(index_result["isError"])
        self.assertEqual(index_result["structuredContent"]["evidence"], "tool-confirmed")
        self.assertNotIn(str(REPO_ROOT), index_result["content"][0]["text"])

        process.stdin.close()
        process.wait(timeout=10)


if __name__ == "__main__":
    unittest.main()
