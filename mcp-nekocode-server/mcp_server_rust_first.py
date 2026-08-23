#!/usr/bin/env python3
"""A deliberately small Rust-first MCP server for NekoCode.

The server speaks newline-delimited JSON-RPC 2.0 on stdio and exposes only
the Rust-first ``index`` and ``context`` commands.  It is intentionally
independent from the legacy MCP implementations in this directory.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, Optional, Tuple


PROTOCOL_VERSION = "2024-11-05"
SERVER_NAME = "nekocode-rust-first"
SERVER_VERSION = "0.1.0"
MAX_BUDGET = 100_000
COMMAND_TIMEOUT_SECONDS = 180


class ToolInputError(ValueError):
    """A client supplied invalid input to a supported tool."""


class CommandError(RuntimeError):
    """The Rust CLI could not produce a valid JSON result."""


def _safe_error(message: str) -> str:
    """Keep operational details, paths, and environment values off stdio."""
    return re.sub(r"(?:[A-Za-z]:)?[/\\][^\s'\"]+", "<path>", message)


def _redact_absolute_paths(value: Any) -> Any:
    """Recursively remove absolute paths from CLI data returned to MCP clients."""
    if isinstance(value, dict):
        return {key: _redact_absolute_paths(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_redact_absolute_paths(item) for item in value]
    if isinstance(value, str):
        # Cargo metadata emits the workspace root as an absolute path.  Preserve
        # relative source paths while replacing POSIX and Windows absolute paths.
        return re.sub(r"(?<!\w)(?:[A-Za-z]:[\\/]|/)[^\s\"']+", "<path>", value)
    return value


def _tool_result(data: Any, is_error: bool = False) -> Dict[str, Any]:
    sanitized = _redact_absolute_paths(data)
    text = json.dumps(sanitized, ensure_ascii=False, indent=2, sort_keys=True)
    result: Dict[str, Any] = {
        "content": [{"type": "text", "text": text}],
        "isError": is_error,
    }
    if not is_error and isinstance(sanitized, (dict, list)):
        result["structuredContent"] = sanitized
    return result


class RustFirstMCPServer:
    """Minimal MCP dispatcher backed by the Rust-first NekoCode CLI."""

    def __init__(
        self,
        workspace_dir: Optional[Path] = None,
        binary_path: Optional[Path] = None,
    ) -> None:
        project_root = Path(__file__).resolve().parent.parent
        configured_workspace = os.environ.get("NEKOCODE_WORKSPACE_DIR")
        if workspace_dir is not None:
            self.workspace_dir = workspace_dir
        elif configured_workspace:
            self.workspace_dir = Path(configured_workspace).expanduser()
        else:
            self.workspace_dir = project_root / "nekocode-workspace"

        configured_binary = binary_path
        if configured_binary is None:
            raw_binary = os.environ.get("NEKOCODE_BINARY_PATH")
            if raw_binary:
                configured_binary = Path(raw_binary).expanduser()
        if configured_binary is not None and not configured_binary.is_absolute():
            configured_binary = project_root / configured_binary
        self.binary_path = configured_binary

    @staticmethod
    def tools() -> list[Dict[str, Any]]:
        return [
            {
                "name": "index",
                "description": "Index a Rust workspace using Cargo metadata.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Rust workspace or Cargo.toml path.",
                        }
                    },
                    "required": ["path"],
                    "additionalProperties": False,
                },
            },
            {
                "name": "context",
                "description": "Build a bounded Rust context pack from a Git diff.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Rust workspace or Cargo.toml path.",
                        },
                        "compare_ref": {
                            "type": "string",
                            "description": "Git ref to compare with HEAD.",
                        },
                        "budget": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": MAX_BUDGET,
                            "default": 8000,
                        },
                        "diagnostics": {
                            "type": "boolean",
                            "default": False,
                        },
                        "working_tree": {
                            "type": "boolean",
                            "description": "Include staged, unstaged, and untracked changes.",
                            "default": False,
                        },
                        "all_features": {
                            "type": "boolean",
                            "description": "Run cargo check with all workspace features.",
                            "default": False,
                        },
                    },
                    "required": ["path"],
                    "additionalProperties": False,
                },
            },
        ]

    @staticmethod
    def _path_argument(args: Dict[str, Any]) -> str:
        path = args.get("path")
        if not isinstance(path, str) or not path.strip() or "\x00" in path:
            raise ToolInputError("'path' must be a non-empty string")
        return path

    @staticmethod
    def _context_arguments(args: Dict[str, Any]) -> list[str]:
        command: list[str] = []
        compare_ref = args.get("compare_ref")
        if compare_ref is not None:
            if not isinstance(compare_ref, str) or not re.fullmatch(
                r"[A-Za-z0-9][A-Za-z0-9._/@+~^-]*", compare_ref
            ):
                raise ToolInputError("'compare_ref' must be a simple Git revision")
            command.extend(["--compare-ref", compare_ref])

        budget = args.get("budget", 8000)
        if isinstance(budget, bool) or not isinstance(budget, int):
            raise ToolInputError("'budget' must be an integer")
        if not 1 <= budget <= MAX_BUDGET:
            raise ToolInputError(f"'budget' must be between 1 and {MAX_BUDGET}")
        command.extend(["--budget", str(budget)])

        diagnostics = args.get("diagnostics", False)
        if not isinstance(diagnostics, bool):
            raise ToolInputError("'diagnostics' must be a boolean")
        if diagnostics:
            command.append("--diagnostics")
        working_tree = args.get("working_tree", False)
        if not isinstance(working_tree, bool):
            raise ToolInputError("'working_tree' must be a boolean")
        if working_tree:
            command.append("--working-tree")
        all_features = args.get("all_features", False)
        if not isinstance(all_features, bool):
            raise ToolInputError("'all_features' must be a boolean")
        if all_features:
            command.append("--all-features")
        return command

    def _run_cli(self, tool: str, args: Dict[str, Any]) -> Any:
        path = self._path_argument(args)
        if self.binary_path is not None:
            if not self.binary_path.is_file():
                raise CommandError("Configured Rust CLI is unavailable")
            command = [str(self.binary_path), tool, path]
            configured_cwd = os.environ.get("NEKOCODE_CLI_CWD")
            command_cwd = Path(configured_cwd).expanduser() if configured_cwd else Path.cwd()
        else:
            if not self.workspace_dir.is_dir():
                raise CommandError("Rust workspace is unavailable")
            cargo = shutil.which("cargo")
            if cargo is None:
                raise CommandError("Cargo is unavailable")
            command = [cargo, "run", "-q", "-p", "nekocode", "--", tool, path]
            command_cwd = self.workspace_dir
        if tool == "context":
            command.extend(self._context_arguments(args))

        env = os.environ.copy()
        # Cargo output belongs on stderr.  stdout must remain a parseable JSON CLI
        # response, so do not allow terminal colouring to leak into it.
        env["CARGO_TERM_COLOR"] = "never"
        try:
            completed = subprocess.run(
                command,
                cwd=command_cwd,
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=COMMAND_TIMEOUT_SECONDS,
                check=False,
            )
        except subprocess.TimeoutExpired as exc:
            raise CommandError("Rust command timed out") from exc
        except OSError as exc:
            raise CommandError("Rust command could not start") from exc

        if completed.returncode != 0:
            raise CommandError("Rust command failed; inspect the local Cargo diagnostics")
        try:
            return json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise CommandError("Rust command returned invalid JSON") from exc

    def handle_tool_call(self, params: Any) -> Dict[str, Any]:
        if not isinstance(params, dict):
            return _tool_result({"error": "tools/call params must be an object"}, True)
        name = params.get("name")
        args = params.get("arguments", {})
        if name not in {"index", "context"}:
            return _tool_result({"error": "unknown tool; only index and context are available"}, True)
        if not isinstance(args, dict):
            return _tool_result({"error": "tool arguments must be an object"}, True)
        if set(args) - {
            "path",
            "compare_ref",
            "budget",
            "diagnostics",
            "working_tree",
            "all_features",
        }:
            return _tool_result({"error": "unsupported tool argument"}, True)
        if name == "index" and set(args) - {"path"}:
            return _tool_result({"error": "index accepts only 'path'"}, True)
        try:
            return _tool_result(self._run_cli(name, args))
        except ToolInputError as exc:
            return _tool_result({"error": str(exc)}, True)
        except CommandError as exc:
            return _tool_result({"error": _safe_error(str(exc))}, True)

    def dispatch(self, message: Any) -> Tuple[Optional[Any], Optional[Dict[str, Any]]]:
        """Return (request id, response object), or no response for notifications."""
        if not isinstance(message, dict) or message.get("jsonrpc") != "2.0":
            return None, self._jsonrpc_error(None, -32600, "Invalid Request")
        request_id = message.get("id")
        method = message.get("method")
        if not isinstance(method, str):
            return request_id, self._jsonrpc_error(request_id, -32600, "Invalid Request")

        if method == "initialize":
            result: Dict[str, Any] = {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
            }
        elif method == "tools/list":
            result = {"tools": self.tools()}
        elif method == "tools/call":
            result = self.handle_tool_call(message.get("params", {}))
        elif method == "notifications/initialized":
            return None, None
        else:
            return request_id, self._jsonrpc_error(request_id, -32601, "Method not found")

        if request_id is None:
            return None, None
        return request_id, {"jsonrpc": "2.0", "id": request_id, "result": result}

    @staticmethod
    def _jsonrpc_error(request_id: Any, code: int, message: str) -> Dict[str, Any]:
        return {
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {"code": code, "message": message},
        }

    def run(self) -> None:
        for line in sys.stdin:
            try:
                message = json.loads(line)
            except json.JSONDecodeError:
                response = self._jsonrpc_error(None, -32700, "Parse error")
            else:
                _, response = self.dispatch(message)
            if response is not None:
                sys.stdout.write(json.dumps(response, ensure_ascii=False) + "\n")
                sys.stdout.flush()


if __name__ == "__main__":
    RustFirstMCPServer().run()
