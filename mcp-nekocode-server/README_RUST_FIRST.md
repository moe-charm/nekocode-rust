# NekoCode Rust-first MCP gateway

`mcp_server_rust_first.py` is a small, read-only stdio MCP server.  It exposes
only two tools backed by the canonical `nekocode-workspace` CLI:

- `index` → `nekocode index <path> [--snapshot FILE] [--diagnostics] [--all-features]`
- `context` → `nekocode context <path> [--compare-ref REF] [--budget N] [--diagnostics] [--working-tree] [--all-features] [--excerpt-lines N] [--baseline FILE]`

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
to 100000, and booleans for compiler diagnostics, working-tree changes, and
feature coverage:

```json
{
  "path":".",
  "compare_ref":"HEAD~1",
  "budget":2000,
  "diagnostics":true,
  "working_tree":true,
  "all_features":true
}
```

The context response includes Cargo package/target metadata, input-file
digests, resolved Git refs, unified-diff hunks and a bounded patch.  With
`diagnostics`, it also includes structured rustc spans, rendered messages,
package/target provenance, stderr, exit status, and the exact Cargo command.
Every response reports its evidence level and whether the requested budget was
exceeded; no accuracy percentage is inferred from these fields.

## Current context contract

The Rust-first gateway exposes three opt-in capabilities without replacing
rustc or rust-analyzer:

- `index --snapshot FILE --diagnostics` persists a reproducible JSON snapshot
  for later comparison. It is an explicit file, not a hidden database.
- `context --excerpt-lines N` returns bounded source excerpts around Git
  hunks. Excerpts are syntax-only display context, not symbol resolution.
- `context --baseline FILE --diagnostics` returns `added`, `resolved`, and
  `persisting` compiler diagnostics only when the baseline has matching
  toolchain/features/target provenance. Git changes and diagnostic changes stay
  separate.

Clients must not infer a diagnostic delta from `compare_ref` alone: a Git commit
does not contain the result of a past `cargo check`.

The gateway uses argument-vector subprocess execution (no shell), rejects
unknown arguments, and returns client-safe MCP errors when Cargo or CLI JSON
fails.

## Smoke test

```bash
python3 -m unittest discover -s mcp-nekocode-server/tests -p 'test_*.py'
```

## Prebuilt CLI / Docker

配布環境では、gatewayに`NEKOCODE_BINARY_PATH`を渡すとCargo workspaceなしで
canonical CLIを実行できる。リポジトリにはこのモードを使う
`Dockerfile.rust-first`を用意している。

```bash
docker build -f Dockerfile.rust-first -t nekocode-rust-first .
docker run --rm -i -v "$PWD":/work -w /work nekocode-rust-first
```

既存の`Dockerfile`と`mcp_server_real.py`はlegacy互換用であり、このgatewayから
呼び出さない。

The test starts the server over stdio, checks `initialize` and `tools/list`,
then runs an `index` call against the canonical workspace.
