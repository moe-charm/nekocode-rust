# Current task — Rust-first context layer

Updated: 2026-08-24

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
- external baseline path redaction plus exact error/warning multiset deltas,
  covered by the repository's Rust golden fixture;
- Cargo workspace discovery from a workspace root, nested directory, or source
  file, with Cargo's reported workspace root used as the canonical boundary;
- NUL-delimited Git path collection and unquoted UTF-8 patch paths, so Japanese
  tracked and untracked paths remain readable and hunks stay associated;
- incomplete snapshot evidence and an explicit limitation when `cargo check`
  fails operationally, times out, or exceeds its output limit;
- a read-only Nyash repository probe using its nested `src` path, confirming
  workspace discovery, readable Japanese paths, incomplete offline-failure
  evidence, and an unchanged target Git status;
- independent AI review follow-up for evidence/limitation separation,
  `--all-features` validation across core/CLI/MCP, and explicit fully omitted
  patch presentation;
- read-only follow-up review of commit `f1f8f4c`: all three findings closed,
  no new findings, release-ready verdict, and unchanged NekoCode/Nyash Git
  status hashes;
- Codex Skill v0;
- final legacy recovery tag and archive branch;
- physical removal of the old root crate, multi-binary workspace, analyzers,
  hidden sessions, prebuilt binaries, old workflows, and old MCP gateway.

Recovery points:

- tag: `legacy-multilang-final`;
- branch: `archive/legacy-multilang-final`.

## Next implementation focus

1. Stabilize and validate schema v1 golden artifacts.
2. Strengthen diagnostic fingerprint fields and comparability fixtures.
3. Add property tests for byte/item/line budget invariants.
4. Evaluate the local Skill workflow; do not add MCP tools or another language.

The authoritative decisions are under `docs/`.
