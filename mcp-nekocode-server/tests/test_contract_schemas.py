"""Small stdlib checks for the checked-in public artifact schemas."""

from __future__ import annotations

import json
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


class ContractSchemaTest(unittest.TestCase):
    def test_snapshot_and_context_versions_are_explicit(self) -> None:
        expected = {
            "snapshot-v1.schema.json": ("snapshot-v1", "snapshot"),
            "context-v1.schema.json": ("context-v1", "context"),
        }
        for filename, (version, artifact_kind) in expected.items():
            with self.subTest(filename=filename):
                schema = json.loads((REPO_ROOT / "schemas" / filename).read_text())
                self.assertEqual(schema["properties"]["contract_version"]["const"], version)
                self.assertEqual(schema["properties"]["artifact_kind"]["const"], artifact_kind)
                self.assertIn("contract_version", schema["required"])
                self.assertIn("status", schema["required"])
                self.assertIn("schema_version", schema["required"])
                self.assertIn("limitations", schema["required"])


if __name__ == "__main__":
    unittest.main()
