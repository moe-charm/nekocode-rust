# Rust-first MVP contract

Status: canonical design, 2026-08-23.

NekoCode is a Rust-first code context layer. It does not reimplement Rust
semantic analysis. Cargo metadata, `rustc`/`cargo check`, Git, and later
explicitly enabled official backends remain the sources of meaning.

The product definition is:

> NekoCode converts Rust official-tool results and Git changes into
> comparable, budgeted, evidence-backed code context.

Read the companion decisions first:

- [Product boundary](product-boundary.md)
- [Execution trust model](execution-trust.md)
- [Artifact contract](artifact-contract.md)
- [Legacy retirement](legacy-retirement.md)

## Canonical architecture

```text
human / CI ────────> nekocode CLI
                         │
Skill / Plugin ───> thin MCP gateway
                         │
                         ▼
                   nekocode-core
              snapshot / context / delta
                         │
                    Git / Cargo / rustc
```

`nekocode-core` owns request/response meaning, evidence, comparability,
budget, provenance, and safety. The CLI is the canonical execution entry.
MCP is a transport adapter. Skill is workflow guidance. Plugin is packaging.
App UI is optional and never the required execution path.

## Canonical commands

The public command names are deliberately not `index` or `analyze`:

```bash
cd nekocode-workspace

# Metadata-only workspace observation (default; no workspace code execution)
cargo run -q -p nekocode -- snapshot .

# Explicit diagnostic observation for a reusable baseline
cargo run -q -p nekocode -- snapshot . \
  --analysis cargo-check --output /tmp/nekocode-baseline.json

# Git context with optional current diagnostics and a saved baseline
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD --baseline /tmp/nekocode-baseline.json \
  --diagnostics --budget 8000
```

For one migration release, the CLI may accept `index` as a hidden compatibility
alias. Documentation and MCP expose `snapshot` only. There is no canonical
`analyze` command.

## Snapshot contract

The external artifact is `snapshot-v1`. It is an explicit caller-selected JSON
file; NekoCode never chooses an implicit latest snapshot or hidden database.

Metadata includes workspace/package/target/features, toolchain and target
profile, relevant input digests, Git state, execution policy, and provenance.
`--analysis cargo-check` adds a structured diagnostic observation. The default
is `metadata-only` and must not run `build.rs` or procedural macros.

The canonical payload hash excludes timestamps and storage paths. Path values
in the artifact are normalized to the workspace boundary where possible.

## Context contract

The external artifact is `context-v1`. It contains the caller-selected Git
base, staged/unstaged/working-tree change markers, hunks, bounded excerpts,
optional compiler diagnostics, and explicit status/provenance/budget fields.

`compare_ref` selects a Git change set; it never recreates compiler output for
an old commit. A diagnostic delta requires a saved snapshot containing a
diagnostic observation under compatible conditions.

## Diagnostic comparability

The response must distinguish:

```text
comparable
baseline_missing
not_comparable
partial
```

Toolchain, target, package, feature/default-feature, compiler-affecting
configuration, or analysis-profile changes produce `not_comparable` with
reasons. A baseline without diagnostics produces `baseline_missing`, never an
empty successful delta. MVP matching is exact and multiset-based; fuzzy line
movement matching is not implemented.

Compiler result states are also explicit:

```text
not_run
completed_clean
completed_with_diagnostics
tool_failed
timed_out
output_limited
partial
```

Compiler errors are observations. Operational failures are separate from a
valid diagnostic stream.

## Budget and evidence

Hard limits are bytes, item counts, and lines. Token estimates are advisory.
The envelope always keeps status, provenance, and omission information. Every
omitted group records a reason, count, and priority; silent truncation is not
allowed.

Evidence levels are:

- `tool-confirmed`: directly reported by Cargo, rustc, or Git;
- `semantic-resolved`: a future explicitly enabled semantic backend;
- `syntax-only`: display-only syntax information;
- `incomplete`: unavailable, incompatible, failed, or budget-truncated data.

These levels are evidence categories, not accuracy percentages.

## Execution trust

`metadata-only` is the default. `cargo-check` is opt-in and requires a trusted
workspace because Cargo may execute build scripts, procedural macros, and
compiler wrappers. Until OS-level isolation exists, NekoCode must report that
process network isolation is not enforced and must not call the operation
"sandboxed".

The implementation must bound process time/output, redact secrets, keep source
reads inside the workspace, reject symlink escapes, and disable Git external
diff/textconv. See [execution-trust.md](execution-trust.md) for the release
blockers and fixtures.

## Current implementation and promotion gates

The current branch has the Rust context core, explicit snapshot persistence,
Git hunk/excerpt support, diagnostic parsing, and a minimal two-tool stdio
gateway. The next implementation step makes `snapshot` canonical, adds the
versioned artifact envelope/schema, and preserves only the short-lived CLI
`index` alias.

Before a semantic backend or a second language is promoted, fixtures must cover
trait/impl/macro/cfg/features, workspaces and targets, toolchain/profile
changes, diagnostics, UTF-8 boundaries, rename/delete/untracked/binary Git
states, budget omissions, and CLI/MCP payload parity.

## Legacy boundary

The root package, old five binaries, multi-language analyzers, refactoring,
dead-code/impact heuristics, watch, security/quality suites, and extra MCP
tools remain recoverable legacy only. They receive no new product claims.

Physical archive waits for the gates in
[legacy-retirement.md](legacy-retirement.md): final tag, clean canonical
dependency graph, golden artifacts, parity tests, one install binary, and no
legacy route in primary docs.

## Version policy

The checked-in schemas under `schemas/` are the external contract. Additive
optional fields remain within v1. A breaking field/status/meaning change is v2.
Core Rust types are the semantic source; CLI and MCP must consume the same
payload rather than defining adapter-specific DTOs.
