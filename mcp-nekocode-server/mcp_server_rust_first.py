#!/usr/bin/env python3
"""A deliberately small Rust-first MCP server for NekoCode.

The server speaks newline-delimited JSON-RPC 2.0 on stdio and exposes only
the Rust-first ``snapshot`` and ``context`` commands.
"""

from __future__ import annotations

import json
import os
import re
import signal
import shutil
import subprocess
import sys
import threading
from pathlib import Path
from typing import Any, Dict, Optional, Tuple


PROTOCOL_VERSION = "2024-11-05"
SERVER_NAME = "nekocode-rust-first"
SERVER_VERSION = "0.2.0"
MAX_BUDGET = 100_000
COMMAND_TIMEOUT_SECONDS = 180
MAX_STDOUT_BYTES = 8 * 1024 * 1024
MAX_STDERR_BYTES = 2 * 1024 * 1024
SNAPSHOT_TOOL = "nekocode_snapshot"
CONTEXT_TOOL = "nekocode_context"
SAFE_ENVIRONMENT_KEYS = (
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "USERPROFILE",
    "SystemRoot",
    "WINDIR",
    "TEMP",
    "TMP",
    "TMPDIR",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
)


class ToolInputError(ValueError):
    """A client supplied invalid input to a supported tool."""


class CommandError(RuntimeError):
    """The Rust CLI could not produce a valid JSON result."""


def _read_capped(stream: Any, limit: int) -> tuple[bytes, bool]:
    chunks: list[bytes] = []
    total = 0
    truncated = False
    while True:
        chunk = stream.read(16 * 1024)
        if not chunk:
            break
        kept = 0
        if total < limit:
            keep = chunk[: limit - total]
            chunks.append(keep)
            total += len(keep)
            kept = len(keep)
        if kept < len(chunk):
            truncated = True
    return b"".join(chunks), truncated


def _capture_capped(stream: Any, limit: int, result: Dict[str, Any], key: str) -> None:
    try:
        result[key] = _read_capped(stream, limit)
    except BaseException as exc:  # pragma: no cover - defensive thread bridge
        result[key] = exc


def _terminate_process_tree(process: subprocess.Popen[bytes]) -> None:
    if os.name == "posix":
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    elif os.name == "nt":
        subprocess.run(
            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    try:
        process.kill()
    except ProcessLookupError:
        pass


def _safe_error(message: str) -> str:
    """Keep operational details, paths, and environment values off stdio."""
    return re.sub(r"(?:[A-Za-z]:)?[/\\][^\s'\"]+", "<path>", message)


def _safe_cli_environment() -> Dict[str, str]:
    """Pass only the execution inputs needed by the local Rust CLI."""
    environment = {
        key: os.environ[key] for key in SAFE_ENVIRONMENT_KEYS if key in os.environ
    }
    environment["CARGO_TERM_COLOR"] = "never"
    return environment


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
                "name": SNAPSHOT_TOOL,
                "description": "Create an explicit Rust workspace snapshot.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Rust workspace or Cargo.toml path.",
                        },
                        "analysis": {
                            "type": "string",
                            "enum": ["metadata-only", "cargo-check", "clippy"],
                            "default": "metadata-only",
                        },
                        "output": {
                            "type": "string",
                            "description": "Explicit JSON snapshot path to write.",
                        },
                        "all_features": {
                            "type": "boolean",
                            "description": "Run the snapshot check with all workspace features.",
                            "default": False,
                        },
                    },
                    "required": ["path"],
                    "additionalProperties": False,
                },
            },
            {
                "name": CONTEXT_TOOL,
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
                        "diagnostic_producer": {
                            "type": "string",
                            "enum": ["cargo-check", "clippy"],
                            "default": "cargo-check",
                            "description": "Explicit diagnostic producer; clippy requires diagnostics.",
                        },
                        "working_tree": {
                            "type": "boolean",
                            "description": "Include staged, unstaged, and untracked changes.",
                            "default": False,
                        },
                        "include_untracked_content": {
                            "type": "boolean",
                            "description": "Read untracked file contents; requires working_tree.",
                            "default": False,
                        },
                        "all_features": {
                            "type": "boolean",
                            "description": "Run the selected compiler diagnostic producer with all workspace features; requires diagnostics.",
                            "default": False,
                        },
                        "excerpt_lines": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 200,
                            "default": 8,
                        },
                        "baseline": {
                            "type": "string",
                            "description": "Explicit JSON snapshot used for diagnostic delta.",
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
        diagnostic_producer = args.get("diagnostic_producer", "cargo-check")
        if not isinstance(diagnostic_producer, str) or diagnostic_producer not in {
            "cargo-check",
            "clippy",
        }:
            raise ToolInputError(
                "'diagnostic_producer' must be cargo-check or clippy"
            )
        if diagnostic_producer == "clippy" and not diagnostics:
            raise ToolInputError("'diagnostic_producer' clippy requires 'diagnostics'")
        if diagnostic_producer != "cargo-check":
            command.extend(["--diagnostic-producer", diagnostic_producer])
        working_tree = args.get("working_tree", False)
        if not isinstance(working_tree, bool):
            raise ToolInputError("'working_tree' must be a boolean")
        if working_tree:
            command.append("--working-tree")
        include_untracked_content = args.get("include_untracked_content", False)
        if not isinstance(include_untracked_content, bool):
            raise ToolInputError("'include_untracked_content' must be a boolean")
        if include_untracked_content and not working_tree:
            raise ToolInputError("'include_untracked_content' requires 'working_tree'")
        if include_untracked_content:
            command.append("--include-untracked-content")
        all_features = args.get("all_features", False)
        if not isinstance(all_features, bool):
            raise ToolInputError("'all_features' must be a boolean")
        if all_features and not diagnostics:
            raise ToolInputError("'all_features' requires 'diagnostics'")
        if all_features:
            command.append("--all-features")
        excerpt_lines = args.get("excerpt_lines", 8)
        if isinstance(excerpt_lines, bool) or not isinstance(excerpt_lines, int):
            raise ToolInputError("'excerpt_lines' must be an integer")
        if not 0 <= excerpt_lines <= 200:
            raise ToolInputError("'excerpt_lines' must be between 0 and 200")
        command.extend(["--excerpt-lines", str(excerpt_lines)])
        baseline = args.get("baseline")
        if baseline is not None:
            if not isinstance(baseline, str) or not baseline.strip() or "\x00" in baseline:
                raise ToolInputError("'baseline' must be a non-empty string")
            command.extend(["--baseline", baseline])
        return command

    @staticmethod
    def _snapshot_arguments(args: Dict[str, Any]) -> list[str]:
        command: list[str] = []
        analysis = args.get("analysis", "metadata-only")
        if not isinstance(analysis, str) or analysis not in {
            "metadata-only",
            "cargo-check",
            "clippy",
        }:
            raise ToolInputError("'analysis' must be metadata-only, cargo-check, or clippy")
        if analysis != "metadata-only":
            command.extend(["--analysis", analysis])
        output = args.get("output")
        if output is not None:
            if not isinstance(output, str) or not output.strip() or "\x00" in output:
                raise ToolInputError("'output' must be a non-empty string")
            command.extend(["--output", output])
        all_features = args.get("all_features", False)
        if not isinstance(all_features, bool):
            raise ToolInputError("'all_features' must be a boolean")
        if all_features:
            command.append("--all-features")
        return command

    def _run_cli(self, tool: str, args: Dict[str, Any]) -> Any:
        path = self._path_argument(args)
        cli_tool = "snapshot" if tool == SNAPSHOT_TOOL else "context"
        if self.binary_path is not None:
            if not self.binary_path.is_file():
                raise CommandError("Configured Rust CLI is unavailable")
            command = [str(self.binary_path), cli_tool, path]
            configured_cwd = os.environ.get("NEKOCODE_CLI_CWD")
            command_cwd = Path(configured_cwd).expanduser() if configured_cwd else Path.cwd()
        else:
            if not self.workspace_dir.is_dir():
                raise CommandError("Rust workspace is unavailable")
            cargo = shutil.which("cargo")
            if cargo is None:
                raise CommandError("Cargo is unavailable")
            command = [cargo, "run", "-q", "-p", "nekocode", "--", cli_tool, path]
            command_cwd = self.workspace_dir
        if tool == SNAPSHOT_TOOL:
            command.extend(self._snapshot_arguments(args))
        elif tool == CONTEXT_TOOL:
            command.extend(self._context_arguments(args))

        # Cargo output belongs on stderr. stdout must remain a parseable JSON CLI
        # response, so do not allow terminal colouring to leak into it. The
        # adapter also avoids forwarding compiler wrappers and arbitrary build
        # configuration from the MCP host process.
        env = _safe_cli_environment()
        process_options: Dict[str, Any] = {}
        if os.name == "posix":
            process_options["start_new_session"] = True
        elif os.name == "nt":
            process_options["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP
        try:
            process = subprocess.Popen(
                command,
                cwd=command_cwd,
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                **process_options,
            )
        except OSError as exc:
            raise CommandError("Rust command could not start") from exc

        assert process.stdout is not None
        assert process.stderr is not None
        captured: Dict[str, Any] = {}
        stdout_thread = threading.Thread(
            target=_capture_capped,
            args=(process.stdout, MAX_STDOUT_BYTES, captured, "stdout"),
            daemon=True,
        )
        stderr_thread = threading.Thread(
            target=_capture_capped,
            args=(process.stderr, MAX_STDERR_BYTES, captured, "stderr"),
            daemon=True,
        )
        stdout_thread.start()
        stderr_thread.start()
        try:
            completed_returncode = process.wait(timeout=COMMAND_TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired as exc:
            _terminate_process_tree(process)
            process.wait(timeout=5)
            stdout_thread.join(timeout=5)
            stderr_thread.join(timeout=5)
            process.stdout.close()
            process.stderr.close()
            raise CommandError("Rust command timed out") from exc
        stdout_thread.join(timeout=5)
        stderr_thread.join(timeout=5)
        process.stdout.close()
        process.stderr.close()
        if stdout_thread.is_alive() or stderr_thread.is_alive():
            _terminate_process_tree(process)
            raise CommandError("Rust command output reader timed out")
        stdout = captured.get("stdout")
        stderr = captured.get("stderr")
        if completed_returncode != 0:
            raise CommandError("Rust command failed; inspect the local Cargo diagnostics")
        if stdout is None or stderr is None:
            raise CommandError("Rust command output could not be collected")
        if isinstance(stdout, BaseException) or isinstance(stderr, BaseException):
            raise CommandError("Rust command output could not be collected")
        stdout_bytes, stdout_truncated = stdout
        _, stderr_truncated = stderr
        if stdout_truncated or stderr_truncated:
            raise CommandError("Rust command output exceeded the safety limit")
        try:
            return json.loads(stdout_bytes.decode("utf-8"))
        except json.JSONDecodeError as exc:
            raise CommandError("Rust command returned invalid JSON") from exc

    def handle_tool_call(self, params: Any) -> Dict[str, Any]:
        if not isinstance(params, dict):
            return _tool_result({"error": "tools/call params must be an object"}, True)
        name = params.get("name")
        args = params.get("arguments", {})
        if name not in {SNAPSHOT_TOOL, CONTEXT_TOOL}:
            return _tool_result(
                {"error": "unknown tool; only nekocode_snapshot and nekocode_context are available"},
                True,
            )
        if not isinstance(args, dict):
            return _tool_result({"error": "tool arguments must be an object"}, True)
        if set(args) - {
            "path",
            "compare_ref",
            "budget",
            "diagnostics",
            "diagnostic_producer",
            "working_tree",
            "include_untracked_content",
            "all_features",
            "analysis",
            "output",
            "excerpt_lines",
            "baseline",
        }:
            return _tool_result({"error": "unsupported tool argument"}, True)
        if name == SNAPSHOT_TOOL and set(args) - {
            "path",
            "analysis",
            "output",
            "all_features",
        }:
            return _tool_result({"error": "unsupported snapshot argument"}, True)
        if name == CONTEXT_TOOL and set(args) - {
            "path",
            "compare_ref",
            "budget",
            "diagnostics",
            "diagnostic_producer",
            "working_tree",
            "include_untracked_content",
            "all_features",
            "excerpt_lines",
            "baseline",
        }:
            return _tool_result({"error": "unsupported context argument"}, True)
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
