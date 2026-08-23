# NekoCode repository guidance

The supported product is a Rust-first evidence context layer, not an
independent semantic analyzer. Its only CLI use cases are `snapshot` and
`context`.

Canonical paths:

- `nekocode-workspace/nekocode-core`: artifact and execution semantics;
- `nekocode-workspace/nekocode`: CLI adapter;
- `mcp-nekocode-server/mcp_server_rust_first.py`: two-tool stdio gateway;
- `schemas/`: public JSON contracts;
- `skills/nekocode-rust-context`: agent workflow and stop conditions.

Use `make verify` before committing. `cargo check` may execute workspace code
through build scripts or procedural macros, so diagnostic execution must stay
explicit and limited to trusted workspaces. Never infer missing, partial, or
non-comparable evidence.

The removed multi-language implementation is recoverable from the
`legacy-multilang-final` tag and `archive/legacy-multilang-final` branch. Do
not reintroduce its session, dead-code, refactor, impact, or multi-binary APIs
into the canonical path.
