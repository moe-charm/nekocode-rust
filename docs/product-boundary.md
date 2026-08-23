# Product boundary

Status: accepted design decision, 2026-08-23.

## Product definition

> NekoCode converts Rust official-tool results and Git changes into
> comparable, budgeted, evidence-backed code context.

NekoCode is a **Rust-first code context layer**, not a Rust semantic analyzer,
IDE backend, or universal language index. Correctness of Rust meaning remains
with Cargo, `rustc`, `cargo check`, Clippy, and rust-analyzer when a later
backend is explicitly enabled.

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
```

`snapshot` describes an explicit workspace observation. `context` describes a
Git change set plus optional compiler observations. No compatibility command
or hidden session entry point is supported.

## Non-goals for the Rust-first MVP

- independent Rust type checking, reference indexing, or symbol resolution;
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
