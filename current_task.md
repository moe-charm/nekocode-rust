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
- current CLI Clippy snapshot/context schema-subset validation and explicit
  CLI/MCP Clippy payload parity regression (volatile Cargo timing normalized);
- read-only live Clippy probe on `test-workspace/nyash/src`: producer/profile/
  version markers were present and offline failure was `tool_failed` with
  `incomplete` evidence; the already-dirty target repository was not edited;
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

## Verified repository state

- canonical branch: `master` at `9acbdf1`, matching `origin/master`;
- working tree: intentionally modified with the documentation and P0 fixes
  listed below; no unrelated files are changed;
- legacy recovery tag and remote archive branch: present;
- canonical hashing now excludes raw diagnostic stderr and re-reads metadata
  after compiler observations that may create `Cargo.lock`;
- the repeated Clippy snapshot hash regression is green;
- `make verify` is green locally: fmt, Clippy `-D warnings`, locked Cargo
  check/test, CLI integration tests, MCP/Python tests, and schema parsing;
- the release baseline is now documented and wired as Rust 1.85.0 (MSRV 1.85)
  across package metadata, CI, and Docker;
- the binary release procedure now emits a checksum and provenance record,
  and the MCP adapter's independent 0.2.0 version policy is documented;
- snapshot metadata and context change-scope goldens pass the standard
  Draft 2020-12 validator, including a negative contract-version case;
- the standard-schema CI gate is green for `6540b05` in remote run
  `32718683043`;
- the latest GitHub Actions run is **not green**: its `stable` toolchain
  resolved to Rust/Clippy 1.98, where `-D warnings` rejects five manual
  `Default` implementations as derivable;
- the five derivable defaults are fixed locally, but a public release/tag is
  now cleared by the updated remote workflow run `32717722425` for
  `cad890f` (all jobs green).

## Next implementation focus

### P0 — restore a reproducible green release gate

- [x] Make snapshot hashing deterministic and add a repeated-execution
      regression test.
- [x] Replace the five derivable manual `Default` implementations rejected by
      Clippy 1.98.
- [x] Make the documented `make verify` gate and CI agree: formatting,
      `-D warnings`, CLI integration tests, locked Cargo commands, and the
      complete Python contract/MCP suite.
- [x] Select and document one Rust toolchain policy (MSRV/CI/Docker 1.85).
- [x] Correct stale trust/test-gate wording and limit compiler observations to
      explicitly trusted workspaces.
- [x] Commit/push the verified changes and require the exact remote workflow
      to pass before creating a release tag.

### P1 — release hygiene after P0

- [x] Decide and document the CLI/core 1.2.0 versus MCP 0.2.0 version policy.
- [x] Add the repository license and complete publish-facing Cargo metadata.
- [x] Define the v1.2.0 binary release artifact procedure: tag/commit
      verification, clean-tree check, locked build, SHA-256 checksums,
      provenance, and release notes. Crates.io publication remains a separate
      future gate because `nekocode-core` is not published.
- [x] Harden contract validation with a snapshot metadata golden artifact,
      negative compatibility cases, and a standard JSON Schema validator
      installed from pinned `requirements-dev.txt` in CI.

### Frozen for this release

- fuzzy diagnostic matching;
- a second language or a generic analyzer abstraction;
- rust-analyzer integration;
- additional MCP tools, prompts, resources, Plugin UI, or remote service.

The authoritative decisions are under `docs/`.
