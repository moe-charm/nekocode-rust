# Core module boundaries

Status: implemented responsibility split, 2026-08-25.

`nekocode-core/src/rust_context.rs` grew into a 3,700-line mixed module. The
next implementation keeps the public API unchanged while separating the
evidence providers and presentation rules. No module may become a second
semantic analyzer or duplicate the CLI/MCP contract.

## Target layout

```text
nekocode-core/src/
├── rust_context.rs       # public facade, stable model types, re-exports
├── rust_context/
│   ├── snapshot.rs       # snapshot persistence, hashing, path redaction
│   ├── workspace.rs      # Cargo metadata and input provenance
│   ├── diagnostics.rs    # Cargo/Clippy observations and exact deltas
│   ├── git.rs            # bounded Git scopes, hunks, and line metrics
│   ├── execution.rs      # bounded process runner and trusted execution
│   ├── budget.rs         # byte truncation, omissions, and excerpts
│   └── summary.rs        # deterministic human projection
```

The facade currently retains the stable public model/request types so the
wire/API compatibility surface remains obvious; it owns only those types plus
request composition. Provider modules own
their external command boundary; `diagnostics` may call `execution`, while
`git` may call `execution`, but neither may call the CLI or MCP. The stable
model definitions have no process or filesystem side effects. `summary` reads completed artifacts and
does not infer new facts.

## Compatibility rules

- `lib.rs` keeps the existing public re-exports and `snapshot-v1`/
  `context-v1` wire shapes;
- module visibility is `pub(crate)` unless a symbol is part of the existing
  core API;
- tests remain provider-focused, with at least one facade parity test;
- no generic `LanguageAnalyzer`, parser registry, or semantic backend is
  introduced by the split;
- the facade and provider modules should stay near or below roughly 1,000
  lines; a future split is preferred over recreating another monolith. The
  comparability provider is currently the largest at about 1,000 lines because
  it owns the matrix evaluator and its focused unit cases.

## Release blockers tracked with this split

The 2026-08-24 Codex audit found no regression in the current tests but gave a
release **No-Go** until these were closed:

1. Cargo configuration/toolchain inputs must be closed across the actual Cargo
   search hierarchy, or untracked/ignored configuration must be rejected by
   release checks;
2. diagnostic comparability must include workspace/package/target and input
   digests, not only producer and toolchain markers;
3. CLI Git revisions must be validated and passed as data, never as options;
4. MCP fallback and compiler observations must use the locked/offline policy;
5. Docker and release staging must exclude ignored legacy residues;
6. schema identity and snapshot envelope validation must match the repository.

All six implementation items are now covered by code and regression tests. The
remaining No-Go is operational: release packaging must run in a clean,
approved environment with no effective untracked Cargo configuration, and the
Docker/staging smoke gates must complete.

The implementation order was documentation first, then the safety and
comparability fixes, then the mechanical module split. The split is now
implemented: the facade is about 1,028 lines and the largest provider module
is about 993 lines. The remaining release gate is operational (clean inputs and Docker/
staging smoke), not another parser or language feature.

## Function investigation extension (2026-09-08)

The approved [symbol context contract](symbol-context-v1.md) adds a separate
`symbol_context` module, preserving the snapshot/Git providers above:

- `model.rs`: request and `symbol-context-v1` response types;
- `lsp.rs`: bounded rust-analyzer transport and backend state;
- `capture.rs`: exact source, Unicode locations and input hashes;
- `collect.rs`: one-target observation, evidence selection and query statuses;
- `packet.rs`: explicit saved evidence, freshness checks, paging and budgets;
- `mod.rs`: public entry point and human-readable summary.

Only the LSP adapter supplies semantic relationships. CLI/MCP forward requests
to this core API; saved replay reads captured sources without launching backend
processes. This extension does not introduce a second parser or a daemon.

`symbol_context/delta.rs` compares saved observations using conservative source
anchors; `delta_model.rs` owns the independent symbol-delta-v1 response. It
reads packets, not live semantic queries, and leaves collection/replay unchanged.
