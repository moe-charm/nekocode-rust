# NekoCode repository guidance

The supported product is a Rust-first evidence context layer, not an
independent semantic analyzer. Its only CLI use cases are `snapshot` and
`context`.

The user-approved 2026-09-08 redesign adds external rust-analyzer investigation
and saved continuation inside context. Follow docs/symbol-context-v1.md; it
supersedes older backend-deferral statements. Preserve snapshot-v1/context-v1.
This does not restore legacy analyzers, automatic edits, or hidden sessions.

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

The 2026-09-08 follow-up adds explicit saved reference comparisons under
[Symbol delta v1](docs/symbol-delta-v1.md). This supersedes the earlier
deferral of reference comparisons only; other later features remain deferred.

Explicit user-requested backend reuse is implemented as the foreground
`context --session` interface described in docs/symbol-session-v1.md. This
supersedes the no-session boundary only for explicit process-owned sessions.
