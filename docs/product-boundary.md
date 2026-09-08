# Product boundary

Status: updated by user-approved redesign, 2026-09-08.

## Product definition

> NekoCode gives an AI the code and observed relationships needed to investigate
> a change, with exact evidence, continuation, and explicit limits.

The first redesign delivery is [Symbol context v1](symbol-context-v1.md).
Existing snapshot and Git/diagnostic context remain supported as its foundation.

NekoCode is a **Rust-first code context layer**, not a Rust semantic analyzer,
IDE backend, or universal language index. Correctness of Rust meaning remains
with Cargo, `rustc`, `cargo check`, Clippy, and an external rust-analyzer LSP
backend. NekoCode integrates their evidence.

## Responsibility map

| Layer | Single responsibility | Must not become |
| --- | --- | --- |
| `nekocode-core` | Snapshot/context use cases, comparability, budget, provenance, safety | A generic language framework or UI |
| `nekocode` CLI | The canonical execution entry, arguments, explicit files, human/JSON output, exit codes | A second analysis implementation |
| Rust-first MCP gateway | Thin transport and input validation for the same two use cases | An analyzer, prompt, or workflow engine |
| Codex Skill | Call order, stop conditions, and evidence presentation | Diagnostic or semantic truth |
| Plugin | Packaging, install, permissions, and distribution | New analysis logic |
| App UI | Optional comparison and review display | The required execution path |

The core owns the meaning of the request/response contract. CLI and MCP must
not define separate DTOs or independently reimplement delta, budget, or
comparability rules.

## Canonical use cases

The public vocabulary is intentionally small:

```text
nekocode snapshot PATH
nekocode context PATH --baseline SNAPSHOT.json
nekocode context PATH --symbol NAME --save-packet PACKET.json
nekocode context --packet PACKET.json --item ITEM_ID
```

`snapshot` describes an explicit workspace observation. `context` without a
symbol selector retains its Git/diagnostic behavior. `context --at` or --symbol
investigates code; `context --packet` reads a saved investigation. Symbol modes
use symbol-context-v1. There is no hidden session entry point.

## Non-goals for the Rust-first MVP

- independent Rust type checking or symbol resolution (LSP-backed queries are in scope);
- heuristic dead-code or breaking-change conclusions;
- refactoring, source rewriting, watch, security, or quality suites;
- a hidden global session/snapshot database;
- multi-language support before a Rust promotion gate is passed;
- direct dependency on rust-analyzer implementation crates;
- arbitrary command execution or remote source retention.

Do not introduce generic abstractions such as `LanguageAnalyzer`,
`LanguagePlugin`, `SemanticBackend`, `UniversalParser`, or `AnalyzerRegistry`
until a second language is an approved product requirement.

## Source of truth

- Core Rust types and use cases are the semantic SSOT.
- Versioned JSON schemas are the external artifact contract.
- CLI and MCP consume the same core payload.
- Skill instructions describe workflow only.
- Plugin metadata describes packaging only.

Breaking artifact changes require a new contract version. Additive fields are
allowed within a version when old readers can safely ignore them.

The 2026-09-08 follow-up adds explicit saved reference comparisons under
[Symbol delta v1](symbol-delta-v1.md). This supersedes the earlier
deferral of reference comparisons only; other later features remain deferred.
