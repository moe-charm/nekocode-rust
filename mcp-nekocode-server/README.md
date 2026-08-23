# NekoCode MCP gateway

このファイルは、旧多言語・セッション型MCPの案内を置き換えた移行案内です。
現行の正規gatewayは [`README_RUST_FIRST.md`](README_RUST_FIRST.md) に記載した
`mcp_server_rust_first.py` だけです。

## Current contract / 現行契約

The Rust-first stdio gateway exposes exactly two tools:

- `nekocode_snapshot` → `nekocode snapshot PATH`
- `nekocode_context` → `nekocode context PATH`

Rust/Cargo metadata, Git changes, optional explicitly-authorized `cargo check`
diagnostics, provenance, comparability, and hard-budget omissions are returned
using the same `snapshot-v1` / `context-v1` payloads as the CLI.

Rust-first stdio gatewayはCLIと同じ `snapshot-v1` / `context-v1` payloadを返し、
`nekocode_snapshot` と `nekocode_context` の2 toolだけを公開します。MCP側に
独自解析、prompt、refactor、dead-code判定、任意コマンド実行はありません。

## Run / 実行

```bash
python3 mcp-nekocode-server/mcp_server_rust_first.py
python3 -m unittest discover -s mcp-nekocode-server/tests -p 'test_*.py'
```

詳細な入力、実行信頼モデル、redaction、移行条件は
[`README_RUST_FIRST.md`](README_RUST_FIRST.md) と
[`docs/execution-trust.md`](../docs/execution-trust.md) を参照してください。

`mcp_server_real.py`、`mcp_server_nekocode.py`、旧5 binary用wrapper、旧多言語
READMEは復旧用legacyです。現行機能として利用・配布・拡張しません。
