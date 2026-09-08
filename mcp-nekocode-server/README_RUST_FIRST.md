# NekoCode Rust-first MCP gateway

`mcp_server_rust_first.py` is a small, read-only stdio adapter around the
canonical `nekocode` CLI. It exposes exactly two tools:

- `nekocode_snapshot` → `nekocode snapshot <path> [--analysis cargo-check|clippy]`
- `nekocode_context` → Git context, symbol investigation, or saved packet replay

The gateway does not define a second JSON contract or semantic analyzer. It
validates inputs, invokes the same CLI use cases, and returns the same core
payload apart from MCP transport fields. The core sanitizes machine paths in
snapshot/Git context. Symbol context retains its actual workspace root and
explicit packet path for navigation. The gateway preserves source text verbatim.

The gateway adapter version (`0.2.0`) is independent from the Rust CLI/core
product version (`1.2.0`). `snapshot-v1`, Git-focused `context-v1`, and explicit
symbol-selection `symbol-context-v1` are the shared artifact contracts.

## Run

```bash
python3 mcp-nekocode-server/mcp_server_rust_first.py
```

The server speaks newline-delimited JSON-RPC 2.0 on stdin/stdout. Logs and
Cargo diagnostics are kept off stdout. `NEKOCODE_BINARY_PATH` can point to a
prebuilt canonical CLI and is the recommended production configuration. If
unset, the gateway first uses `nekocode` on PATH, then falls back to launching
the nested workspace with Cargo for source development. Set the binary path
explicitly to select an exact installation. `NEKOCODE_WORKSPACE_DIR` selects
only the Cargo fallback's workspace.
All caller paths (`path`, `baseline`, `output`, `save_packet`, and `packet`)
are resolved against `NEKOCODE_CLI_CWD`, or the gateway's current directory
when unset, before either execution mode starts. Cargo fallback does not
reinterpret relative paths against the NekoCode source workspace.
`NEKOCODE_RUST_ANALYZER_PATH` selects the external analyzer executable; otherwise
the core searches PATH for `rust-analyzer`.

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
workspace. `clippy` is an explicit alternative producer with the same trust
boundary; it observes Clippy's default lint behavior without adding a custom
lint set. An explicit output path is the only persisted artifact.

### `nekocode_context`

```json
{
  "path":".",
  "compare_ref":"HEAD",
  "baseline":"/tmp/nekocode-baseline.json",
  "diagnostics":true,
  "diagnostic_producer":"cargo-check",
  "working_tree":true,
  "include_untracked_content":false,
  "all_features":false,
  "budget":8000,
  "excerpt_lines":8
}
```

Set `diagnostic_producer` to `clippy` only together with `diagnostics: true`.
The returned diagnostic run records `producer`, `profile`, and
`producer_version`; exact deltas are never computed across cargo-check and
Clippy profiles.

The result contains Cargo metadata, normalized Git refs and change markers,
diff hunks, bounded excerpts, optional structured diagnostics, and an exact
diagnostic delta only when the baseline is comparable. A missing diagnostic
baseline is `baseline_missing`; incompatible toolchain/features/targets are
`not_comparable`. Untracked contents remain markers unless
`include_untracked_content` is explicitly set with `working_tree`. Clients
must not infer a compiler delta from `compare_ref`. `all_features` requires
`diagnostics` for Git context; it also selects features for live symbol analysis.
The shared context payload also keeps revision, staged, unstaged, and untracked
Git scopes distinct and reports numstat aggregates that do not disappear when
the bounded patch body is omitted. MCP does not define a second DTO for these
fields.

### Symbol investigation through `nekocode_context`

```json
{
  "path": "/path/to/rust/workspace",
  "at": "src/config.rs:42:5",
  "save_packet": "/tmp/investigation.json",
  "max_items": 8,
  "budget": 8000,
  "timeout_seconds": 60
}
```

Use `symbol: "parse_config"` instead of `at` to resolve a name. Ambiguous names
return concrete candidates. Positions are one-based; columns count Unicode
characters. PATH defaults to the caller directory for a live investigation.
The artifact contains definitions, observed references, containing symbols,
type information and related test candidates with captured code, per-query
states, freshness, omissions and continuation data. References are not
necessarily calls, and test candidates have not been executed.

`at`, `symbol`, and `packet` are mutually exclusive. Symbol context rejects
Git and compiler-diagnostic options, including `compare_ref`, `working_tree`,
`baseline`, `diagnostics`, `diagnostic_producer` and `excerpt_lines`.
`allow_build_scripts: true` explicitly enables build-script/proc-macro
preparation for a trusted workspace; both are disabled by default. Analysis
can still start rust-analyzer and Cargo metadata. It runs no tests.

`save_packet` writes the full bounded capture for later reads; `output` writes
the displayed JSON response. Without `save_packet`, continuation is unavailable.
Page or expand captured evidence using values returned by the artifact:

```json
{"packet":"/tmp/investigation.json","cursor":"RETURNED_CURSOR","max_items":8}
```

```json
{"packet":"/tmp/investigation.json","item":"RETURNED_ITEM_ID"}
```

Replay starts neither Cargo nor rust-analyzer inside the core; configure a
prebuilt `NEKOCODE_BINARY_PATH` to also avoid the gateway's Cargo launch fallback.
It verifies packet integrity and checks captured inputs for staleness. Omit
`path` to use the captured workspace; an explicit path must belong to that
workspace. `item` and `cursor` cannot be combined;
analysis configuration cannot be changed during replay. `max_items` accepts
1–100 and live `timeout_seconds` accepts 1–600. The byte budget is four times
the advisory token budget; omissions and an oversized minimum envelope remain
explicit. See [the symbol contract](../docs/symbol-context-v1.md).

## Safety and parity

The gateway uses argument-vector subprocess execution (no shell), rejects
unknown arguments, enforces input/output limits and timeout, terminates the
child process group on timeout, and preserves each core contract's path policy
in both text and structured results. Symbol context includes navigation paths.
The gateway never applies path regexes to source
snippets, patches, URLs or diagnostic code. The CLI subprocess receives a small environment allowlist;
compiler wrapper variables are not forwarded by the adapter. It does not claim
OS-level sandboxing for Cargo execution; callers must trust the workspace when
requesting `cargo-check` or `clippy`.

MCP exposes no prompts, resources, UI, refactor operations, separate symbol or
dead-code tool, arbitrary command, or server-side snapshot database.

## Smoke test

```bash
python3 -m unittest discover -s mcp-nekocode-server/tests -p 'test_*.py'
```

The test starts the server over stdio, checks `initialize` and `tools/list`,
and calls both canonical tool names against the Rust-first workspace. Final
`tools/call` tests check source preservation, caller-path parity, selector
validation and saved packet navigation.

The retired gateway is recoverable from the `legacy-multilang-final` tag and
`archive/legacy-multilang-final` branch; it is not present on `main`.

## Saved reference comparison

Through `nekocode_context`:

```json
{"packet":"/tmp/after.json","compare_packet":"/tmp/before.json","max_items":8}
```

Returns `symbol-delta-v1` from the same core as the CLI. Use its `next_cursor`
with both packet paths to page changes. Expand an evidence item with its
original packet and `item` (without `compare_packet`). `compare_packet` requires
`packet`, rejects `item` and live/Git options, and follows the same caller-path
resolution as other explicit files. Snapshot inputs are unaffected.

Comparison reads complete saved captures without backend processes or live
source checks. Capture-time stability, matching conditions and query completion
are required; incompatible or partial observations have null delta counts.
A changed current source does not invalidate historical comparison. Counts
remain independent of display budgets. See [the contract](../docs/symbol-delta-v1.md).

The gateway process timeout is 660 seconds, allowing the maximum 600-second
backend observation plus 60 seconds of CLI overhead. MCP clients may impose
their own shorter deadlines.
