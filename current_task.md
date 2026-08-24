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
- next-feature consultation grounded in Nyash and the current implementation;
  the selected vertical slice is budget-independent Git line metrics plus
  explicit revision/staged/unstaged/untracked scopes;
- Change Scope v1 implemented as additive `context-v1` fields: per-file
  multi-scope observations plus fixed-size, pre-budget Git numstat aggregates;
- regression coverage for non-empty revision changes, mixed staged/unstaged
  changes on one path, rename, binary, UTF-8 untracked paths, tiny-budget
  removal of patch/file details, schema golden validation, and live CLI schema
  validation;
- independent read-only implementation review followed by fixes for deleted
  hunk association and pre-first-commit staged comparison, both covered by
  integration tests;
- Codex Skill updated with Change Scope v1 interpretation rules, explicit
  unknown/omission handling, and a maintainer evaluation reference;
- independent read-only Skill forward evaluation against the Nyash artifact:
  all six acceptance checks passed, with no instruction ambiguity or behavior
  mismatch;
- documented budget invariant gate implemented as multi-budget Rust fixture
  checks for scope stability, omission ledgers, line-count states, UTF-8
  boundaries, and explicit output-limited evidence;
- CLI/MCP bounded-budget parity coverage added and passing for the same
  serialized outcome, omission ledger, and `diff.change_scopes` aggregates;
- explicit Clippy producer/profile/version markers with separate execution
  policy and same-producer comparability;
- Clippy clean/warning/compiler-error/tool-failure/profile-mismatch fixtures,
  tiny-budget marker retention, CLI coverage, and MCP forwarding validation;
- read-only Nyash Change Scope probe: even with the patch and all 41 file
  details omitted, the summary retained 16 unstaged files (`+288/-1663`) and
  25 marker-only untracked files; the target Git status hash remained
  unchanged;
- Codex Skill v0;
- final legacy recovery tag and archive branch;
- physical removal of the old root crate, multi-binary workspace, analyzers,
  hidden sessions, prebuilt binaries, old workflows, and old MCP gateway.

Recovery points:

- tag: `legacy-multilang-final`;
- branch: `archive/legacy-multilang-final`.

## Next implementation focus

1. Run a read-only live Clippy probe on the permitted Nyash workspace and
   perform the final schema/parity review before release tagging.
2. Do not add fuzzy diagnostic matching, another language, or another MCP tool.

The authoritative decisions are under `docs/`.
