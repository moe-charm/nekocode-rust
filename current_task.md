# Current task — Rust-first context layer

Updated: 2026-08-23

## Product

NekoCode converts Rust official-tool results and Git changes into comparable,
budgeted, evidence-backed code context. It is not a Rust parser, type checker,
IDE backend, refactor engine, or universal language index.

The public surface is exactly:

```text
nekocode snapshot PATH
nekocode context PATH --baseline SNAPSHOT.json
```

The semantic source of truth is `nekocode-core`. The CLI is the canonical
entry point, MCP is a two-tool transport adapter, and the Codex Skill owns only
workflow and stop conditions.

## Completed

- versioned `snapshot-v1` and `context-v1` contracts;
- Cargo/Git/rustc evidence, diagnostic comparability, budgets, and omissions;
- explicit trusted-workspace execution policy and safety fixtures;
- CLI/MCP payload parity and two-tool gateway;
- deterministic `context --format summary` projection while JSON remains the
  machine contract;
- Codex Skill v0;
- final legacy recovery tag and archive branch;
- physical removal of the old root crate, multi-binary workspace, analyzers,
  hidden sessions, prebuilt binaries, old workflows, and old MCP gateway.

Recovery points:

- tag: `legacy-multilang-final`;
- branch: `archive/legacy-multilang-final`.

## Next implementation focus

1. Stabilize and validate schema v1 golden artifacts.
2. Strengthen diagnostic fingerprints and comparability fixtures, including
   human-summary views of added/resolved diagnostics.
3. Add property tests for byte/item/line budget invariants.
4. Evaluate the local Skill workflow; do not add MCP tools or another language.

The authoritative decisions are under `docs/`.
