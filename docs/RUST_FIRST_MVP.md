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

# Human-readable projection of the same context evidence
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD --format summary
```

There is no compatibility alias or `analyze` command.

## Snapshot contract

The external artifact is `snapshot-v1`. It is an explicit caller-selected JSON
file; NekoCode never chooses an implicit latest snapshot or hidden database.

Metadata includes workspace/package/target/features, toolchain and target
profile, relevant input digests, Git state, execution policy, and provenance.
`--analysis cargo-check` adds a structured diagnostic observation. The default
is `metadata-only` and must not run `build.rs` or procedural macros.

The canonical payload hash excludes timestamps and storage paths. Path values
in the artifact are normalized to the workspace boundary where possible.
The input path may be the workspace/package root, a nested directory, or an
existing file. NekoCode searches ancestors for the nearest manifest and then
uses Cargo metadata's canonical workspace root for subsequent observations.

## Context contract

The external artifact is `context-v1`. It contains the caller-selected Git
base, staged/unstaged/working-tree change markers, hunks, bounded excerpts,
optional compiler diagnostics, and explicit status/provenance/budget fields.
Git filename lists use NUL delimiters and patches disable non-ASCII path
quoting, preserving UTF-8 paths and their hunk association.

`compare_ref` selects a Git change set; it never recreates compiler output for
an old commit. A diagnostic delta requires a saved snapshot containing a
diagnostic observation under compatible conditions.

JSON is the canonical machine artifact and remains the default output.
`--format summary` renders a deterministic human-readable projection of that
same `ContextV1`: file/hunk counts, visible patch line counts, diagnostics and
delta, comparability, budget, omissions, and limitations. The formatter must
not add semantic inference or become a second contract. When the byte budget
removes the complete patch body, the summary reports it as omitted instead of
displaying a misleading `+0/-0` line count.

`--all-features` changes only a compiler observation. It is rejected for
`context` unless `--diagnostics` is also present, matching the snapshot rule
that requires `--analysis cargo-check`.

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
movement matching is not implemented. Error and warning observations enter the
delta; auxiliary note/help/failure-note messages stay available in the full
diagnostic run. JSON preserves multiplicity, while the human summary explicitly
reports unique fingerprints and raw observation counts. Public output redacts
both workspace-local and external baseline storage paths.

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
valid diagnostic stream. A failed, timed-out, output-limited, or partial
diagnostic run sets the enclosing artifact evidence to `incomplete`.

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
Explanatory `limitations` can describe a deliberately selected scope without
making the returned facts incomplete. Marker-only reporting of untracked files
is the default requested mode and remains `tool-confirmed` when Git completed
and no evidence was omitted. Failed/partial tools, unavailable or incompatible
diagnostic comparisons, and actual budget omissions still produce
`incomplete`.

## Execution trust

`metadata-only` is the default. `cargo-check` is opt-in and requires a trusted
workspace because Cargo may execute build scripts, procedural macros, and
compiler wrappers. Until OS-level isolation exists, NekoCode must report that
process network isolation is not enforced and must not call the operation
"sandboxed".

The implementation now bounds process time/output, filters the Cargo
environment, uses offline mode and a dedicated target, keeps source reads
inside the workspace, rejects symlink escapes, and disables Git external
diff/textconv. OS-level process/network isolation is still not enforced.
See [execution-trust.md](execution-trust.md) for the remaining release
blockers and fixtures.

## Current implementation and promotion gates

The current branch has the Rust context core, explicit snapshot persistence,
Git hunk/excerpt support, diagnostic parsing, versioned artifact envelopes,
execution policy, safe public path views, a deterministic human summary, and a
minimal two-tool stdio gateway.
Untracked file content is opt-in. `snapshot` is the canonical CLI/MCP name.
The execution fixture suite covers build-script
and procedural-macro execution evidence without claiming sandboxing.

Before a semantic backend or a second language is promoted, fixtures must cover
trait/impl/macro/cfg/features, workspaces and targets, toolchain/profile
changes, diagnostics, UTF-8 boundaries, rename/delete/untracked/binary Git
states, budget omissions, and CLI/MCP payload parity.

## Legacy boundary

The former multi-language, session, refactor, impact, watch, and extra MCP
implementation has been removed from `main`. It remains recoverable only from
the refs recorded in [legacy-retirement.md](legacy-retirement.md).

## Version policy

The checked-in schemas under `schemas/` are the external contract. Additive
optional fields remain within v1. A breaking field/status/meaning change is v2.
Core Rust types are the semantic source; CLI and MCP must consume the same
payload rather than defining adapter-specific DTOs.
