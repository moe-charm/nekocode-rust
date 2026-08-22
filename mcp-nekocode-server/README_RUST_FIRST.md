# NekoCode Rust-first MCP gateway

`mcp_server_rust_first.py` is a small, read-only stdio MCP server.  It exposes
only two tools backed by the canonical `nekocode-workspace` CLI:

- `index` → `nekocode index <path>`
- `context` → `nekocode context <path> [--compare-ref REF] [--budget N] [--diagnostics]`

It does not invoke or replace `mcp_server_real.py`; the two servers may coexist
while the repository moves to the Rust-first surface.

## Run

Run the script from any directory where `python3` and `cargo` are available:

```bash
python3 mcp-nekocode-server/mcp_server_rust_first.py
```

Configure the client with a relative checkout-local path or its own launch
mechanism.  Do not commit machine-specific paths or secrets in client config.

The server uses newline-delimited JSON-RPC 2.0 on stdin/stdout.  Logs and Cargo
diagnostics are not sent to stdout.  Tool results are parsed as JSON, and
absolute paths in returned data are replaced with `<path>`.

## Tool inputs

`index` requires:

```json
{"path":"."}
```

`context` requires `path` and accepts a simple Git reference, a budget from 1
to 100000, and a `diagnostics` boolean:

```json
{"path":".","compare_ref":"HEAD~1","budget":2000,"diagnostics":false}
```

The gateway uses argument-vector subprocess execution (no shell), rejects
unknown arguments, and returns client-safe MCP errors when Cargo or CLI JSON
fails.

## Smoke test

```bash
python3 -m unittest discover -s mcp-nekocode-server/tests -p 'test_*.py'
```

The test starts the server over stdio, checks `initialize` and `tools/list`,
then runs an `index` call against the canonical workspace.
