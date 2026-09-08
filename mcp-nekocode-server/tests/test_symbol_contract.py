"""Validate real CLI and final MCP symbol packets with the standard validator."""

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from test_contract_schemas import assert_standard_json_schema, jsonschema

ROOT = Path(__file__).resolve().parents[2]
WORKSPACE = ROOT / "nekocode-workspace"
FAKE = WORKSPACE / "nekocode-core/src/symbol_context/fixtures/fake-ra.py"


@unittest.skipIf(jsonschema is None, "install requirements-dev.txt for schema validation")
class SymbolContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(
            ["cargo", "build", "--locked", "--offline", "-q", "-p", "nekocode"],
            cwd=WORKSPACE, check=True, capture_output=True,
        )
        cls.binary = WORKSPACE / "target/debug/nekocode"
        cls.schema = json.loads((ROOT / "schemas/symbol-context-v1.schema.json").read_text())
        jsonschema.Draft202012Validator.check_schema(cls.schema)

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="symbol-schema-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "src").mkdir()
        (self.root / "Cargo.toml").write_text(
            '[package]\nname="symbol-schema"\nversion="0.1.0"\nedition="2021"\n'
        )
        self.source = (
            'pub fn parse(value: i32) -> i32 { value /2 }\n'
            'pub fn caller() -> i32 { parse(42) }\n'
            '#[test]\nfn test_parse() { assert_eq!(parse(4), 2); }\n'
            'pub fn url() -> &\'static str { "https://example.com/api" }\n'
        )
        (self.root / "src/lib.rs").write_text(self.source)
        self.backend = self.root / "fake-ra"
        self.backend.write_bytes(FAKE.read_bytes())
        self.backend.chmod(0o755)
        self.packet = self.root / "packet.json"
        self.env = dict(os.environ, NEKOCODE_RUST_ANALYZER_PATH=str(self.backend))

    def cli(self, *args, env=None):
        result = subprocess.run(
            [str(self.binary), "context", str(self.root), *args],
            env=self.env if env is None else env, capture_output=True, text=True,
            check=True,
        )
        data = json.loads(result.stdout)
        assert_standard_json_schema(data, self.schema)
        return data

    def test_selection_paging_expansion_stale_and_small_budget(self):
        first = self.cli("--symbol", "parse", "--max-items", "1",
                         "--save-packet", str(self.packet))
        self.assertEqual(first["target"]["resolution"], "selected")
        self.assertEqual(len(first["items"]), 1)
        self.assertIn("value /2", first["items"][0]["code"])
        cursor = first["continuation"]["next_cursor"]
        self.assertIsNotNone(cursor)
        # Replays use captured sources even when the backend cannot start.
        missing = dict(self.env, NEKOCODE_RUST_ANALYZER_PATH=str(self.root / "absent"))
        following = self.cli("--packet", str(self.packet), "--cursor", cursor, env=missing)
        self.assertNotEqual(following["items"][0]["id"], first["items"][0]["id"])
        expanded = self.cli("--packet", str(self.packet), "--item",
                            first["items"][0]["id"], env=missing)
        self.assertIn("value /2", expanded["items"][0]["code"])
        tiny = self.cli("--packet", str(self.packet), "--budget", "1", env=missing)
        self.assertEqual(tiny["status"], "output_limited")
        self.assertTrue(tiny["budget"]["exceeded"])
        (self.root / "src/lib.rs").write_text(self.source.replace("value /2", "value /3"))
        stale = self.cli("--packet", str(self.packet), env=missing)
        self.assertEqual(stale["status"], "stale")
        self.assertIn("src/lib.rs", stale["freshness"]["changed_inputs"])

    def test_backend_failure_and_ambiguity_are_not_empty_success(self):
        missing = dict(self.env, NEKOCODE_RUST_ANALYZER_PATH=str(self.root / "absent"))
        unavailable = self.cli("--symbol", "parse", env=missing)
        self.assertEqual(unavailable["status"], "backend_unavailable")
        (self.root / "src/other.rs").write_text("pub fn parse() {}\n")
        ambiguous = self.cli("--symbol", "parse")
        self.assertEqual(ambiguous["status"], "ambiguous")
        self.assertGreaterEqual(len(ambiguous["target"]["candidates"]), 2)
        ambiguous["contract_version"] = "context-v1"
        with self.assertRaises(AssertionError):
            assert_standard_json_schema(ambiguous, self.schema)

    def test_final_mcp_payload_matches_symbol_contract(self):
        spec = importlib.util.spec_from_file_location(
            "symbol_contract_gateway", ROOT / "mcp-nekocode-server/mcp_server_rust_first.py"
        )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        server = module.RustFirstMCPServer(binary_path=self.binary)
        with patch.dict(os.environ, self.env):
            result = server.handle_tool_call({
                "name": "nekocode_context",
                "arguments": {"path": str(self.root), "symbol": "url"},
            })
        self.assertFalse(result["isError"])
        artifact = result["structuredContent"]
        assert_standard_json_schema(artifact, self.schema)
        self.assertEqual(json.loads(result["content"][0]["text"]), artifact)
        self.assertTrue(any('"https://example.com/api"' in item["code"]
                            for item in artifact["items"]))


if __name__ == "__main__":
    unittest.main()
