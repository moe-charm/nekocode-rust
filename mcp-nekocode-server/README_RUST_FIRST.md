# NekoCode Rust-first MCP gateway

`mcp_server_rust_first.py` is a small, read-only stdio adapter around the
canonical `nekocode` CLI. It exposes exactly two tools:

- `nekocode_snapshot` → `nekocode snapshot <path> [--analysis cargo-check]`
- `nekocode_context` → `nekocode context <path> [--compare-ref REF] ...`

The gateway does not define a second JSON contract or semantic analyzer. It
validates inputs, invokes the same CLI use cases, and returns the same core
payload apart from MCP transport fields and redacted machine paths.

## Run

```bash
python3 mcp-nekocode-server/mcp_server_rust_first.py
```

The server speaks newline-delimited JSON-RPC 2.0 on stdin/stdout. Logs and
Cargo diagnostics are kept off stdout. `NEKOCODE_BINARY_PATH` can point to a
prebuilt canonical CLI; otherwise the gateway launches the nested workspace
with Cargo. `NEKOCODE_WORKSPACE_DIR` can select the workspace explicitly.

## Tools

### `nekocode_snapshot`

Required input:

```json
{"path":"."}
```

Optional inputs:

```json
{
  "analysis": "cargo-check",
  "output": "/tmp/nekocode-baseline.json",
  "all_features": false
}
```

`metadata-only` is the default. `cargo-check` is opt-in because Cargo may run
build scripts, procedural macros, and compiler wrappers in a trusted
workspace. An explicit output path is the only persisted artifact.

### `nekocode_context`

```json
{
  "path":".",
  "compare_ref":"HEAD",
  "baseline":"/tmp/nekocode-baseline.json",
  "diagnostics":true,
  "working_tree":true,
  "include_untracked_content":false,
  "all_features":false,
  "budget":8000,
  "excerpt_lines":8
}
```

The result contains Cargo metadata, normalized Git refs and change markers,
diff hunks, bounded excerpts, optional structured diagnostics, and an exact
diagnostic delta only when the baseline is comparable. A missing diagnostic
baseline is `baseline_missing`; incompatible toolchain/features/targets are
`not_comparable`. Untracked contents remain markers unless
`include_untracked_content` is explicitly set with `working_tree`. Clients
must not infer a compiler delta from `compare_ref`.

## Safety and parity

The gateway uses argument-vector subprocess execution (no shell), rejects
unknown arguments, enforces input/output limits and timeout, terminates the
child process group on timeout, and redacts absolute paths in content and
structured results. The CLI subprocess receives a small environment allowlist;
compiler wrapper variables are not forwarded by the adapter. It does not claim
OS-level sandboxing for Cargo execution; callers must trust the workspace when
requesting `cargo-check`.

MCP exposes no prompts, resources, UI, refactor operations, symbol search,
dead-code tool, arbitrary command, or server-side snapshot database.

## Smoke test

```bash
python3 -m unittest discover -s mcp-nekocode-server/tests -p 'test_*.py'
```

The test starts the server over stdio, checks `initialize` and `tools/list`,
and calls both canonical tool names against the Rust-first workspace.

The legacy `mcp_server_real.py` and five-binary wrappers remain recoverable
but are not the canonical gateway.
