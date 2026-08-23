"""Small stdlib checks for the checked-in public artifact schemas."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def _resolve_ref(root: dict, reference: str) -> dict:
    if not reference.startswith("#/"):
        raise AssertionError(f"unsupported external schema reference: {reference}")
    current: object = root
    for segment in reference[2:].split("/"):
        if not isinstance(current, dict) or segment not in current:
            raise AssertionError(f"unresolved schema reference: {reference}")
        current = current[segment]
    if not isinstance(current, dict):
        raise AssertionError(f"schema reference is not an object: {reference}")
    return current


def _matches_type(value: object, expected: str) -> bool:
    return {
        "object": lambda: isinstance(value, dict),
        "array": lambda: isinstance(value, list),
        "string": lambda: isinstance(value, str),
        "integer": lambda: isinstance(value, int) and not isinstance(value, bool),
        "boolean": lambda: isinstance(value, bool),
        "null": lambda: value is None,
    }[expected]()


def assert_schema_subset(value: object, schema: dict, root: dict, path: str = "$") -> None:
    """Validate the JSON Schema keywords used by the checked-in golden artifact."""

    if "$ref" in schema:
        assert_schema_subset(value, _resolve_ref(root, schema["$ref"]), root, path)
        return
    if "const" in schema and value != schema["const"]:
        raise AssertionError(f"{path}: expected const {schema['const']!r}, got {value!r}")
    if "enum" in schema and value not in schema["enum"]:
        raise AssertionError(f"{path}: {value!r} is not in {schema['enum']!r}")

    expected_types = schema.get("type")
    if expected_types is not None:
        if isinstance(expected_types, str):
            expected_types = [expected_types]
        if not any(_matches_type(value, expected) for expected in expected_types):
            raise AssertionError(f"{path}: expected {expected_types!r}, got {type(value).__name__}")

    if "minimum" in schema and isinstance(value, int) and not isinstance(value, bool):
        if value < schema["minimum"]:
            raise AssertionError(f"{path}: {value} is below {schema['minimum']}")

    if "oneOf" in schema:
        matches = 0
        for candidate in schema["oneOf"]:
            try:
                assert_schema_subset(value, candidate, root, path)
            except AssertionError:
                continue
            matches += 1
        if matches != 1:
            raise AssertionError(f"{path}: expected exactly one oneOf match, got {matches}")

    if isinstance(value, dict):
        for required in schema.get("required", []):
            if required not in value:
                raise AssertionError(f"{path}: missing required property {required!r}")
        properties = schema.get("properties", {})
        for key, item in value.items():
            if key in properties:
                assert_schema_subset(item, properties[key], root, f"{path}.{key}")
            elif schema.get("additionalProperties") is False:
                raise AssertionError(f"{path}: unexpected property {key!r}")

    if isinstance(value, list):
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            raise AssertionError(f"{path}: too many items")
        if "items" in schema:
            for index, item in enumerate(value):
                assert_schema_subset(item, schema["items"], root, f"{path}[{index}]")


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
                self.assertIn("execution_policy", schema["required"])
                self.assertIn("limitations", schema["required"])
                execution_policy = schema["properties"]["execution_policy"]
                self.assertEqual(
                    set(execution_policy["required"]),
                    {
                        "mode",
                        "workspace_trust",
                        "cargo_registry_network",
                        "process_network_isolation",
                        "environment",
                        "compiler_wrappers",
                        "target_directory",
                    },
                )

    def test_change_scope_golden_artifact_matches_context_schema(self) -> None:
        schema = json.loads((REPO_ROOT / "schemas/context-v1.schema.json").read_text())
        fixture = json.loads(
            (
                REPO_ROOT / "schemas/fixtures/context-v1-change-scopes.json"
            ).read_text()
        )

        assert_schema_subset(fixture, schema, schema)
        scopes = fixture["diff"]["change_scopes"]
        self.assertEqual(
            [scope["scope"] for scope in scopes],
            ["revision", "staged", "unstaged", "untracked"],
        )
        mixed = fixture["changed_files"][0]["scope_changes"]
        self.assertEqual({change["scope"] for change in mixed}, {"staged", "unstaged"})

    def test_change_scope_schema_rejects_false_zero_line_evidence(self) -> None:
        schema = json.loads((REPO_ROOT / "schemas/context-v1.schema.json").read_text())
        fixture = json.loads(
            (
                REPO_ROOT / "schemas/fixtures/context-v1-change-scopes.json"
            ).read_text()
        )
        binary_change = fixture["changed_files"][1]["scope_changes"][0]
        binary_change["additions"] = 0

        with self.assertRaisesRegex(AssertionError, "oneOf match"):
            assert_schema_subset(fixture, schema, schema)

    def test_current_cli_context_emits_schema_valid_change_scopes(self) -> None:
        schema = json.loads((REPO_ROOT / "schemas/context-v1.schema.json").read_text())
        completed = subprocess.run(
            [
                "cargo",
                "run",
                "-q",
                "-p",
                "nekocode",
                "--",
                "context",
                ".",
                "--compare-ref",
                "HEAD",
                "--working-tree",
                "--budget",
                "8000",
            ],
            cwd=REPO_ROOT / "nekocode-workspace",
            capture_output=True,
            text=True,
            check=True,
            timeout=60,
        )
        payload = json.loads(completed.stdout)

        assert_schema_subset(payload, schema, schema)
        self.assertEqual(
            [scope["scope"] for scope in payload["diff"]["change_scopes"]],
            ["revision", "staged", "unstaged", "untracked"],
        )


if __name__ == "__main__":
    unittest.main()
