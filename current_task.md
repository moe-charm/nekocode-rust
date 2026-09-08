# Implemented — large-workspace scan diagnostics

See docs/large-workspace-scan-v1.md. Preserve strict freshness gates while
adding explicit large scan limits and per-request reuse explanations.
Hakorune cf458a28: complete 5136-input scans, first 226.401s, second 2.663s
with backend_reused=true. Full gate: 73 Rust / 44 Python-MCP tests.

---

# Review corrections — coverage, freshness and reference comparison

Addressed the three external reviews: text-only errors no longer veto complete
semantic deltas; replay preserves capture change paths; non-file replacements
are changed; derived explanations yield to code under budget pressure; session
save errors report actual reuse without leaking a preceding request's state.
Docs clarify late-input reuse limits and lockfile creation. Full make verify
passed (71 Rust / 43 Python-MCP tests), including extended regression scenarios.

---

# Implemented — coverage explanation and input verification

JSON/summary expose requested vs observed vs unverified scope, and bounded hash
comparison counts/issues with explicit capture/replay basis. Old packets remain
integrity-compatible. Missing observations are not automatically source changes;
backend generation remains unverified. See docs/coverage-freshness-v1.md.

---

# Implemented — unconfirmed text candidates

Live CLI/session/MCP investigations accept text_candidates. Bounded rg matches
absent from captured definitions/references become unconfirmed_text_candidate
items, independent of semantic reference counts. Packet paging/replay remains
backend-free. See docs/text-candidates-v1.md for limits and failure handling.
Coverage explanation and freshness UX remain the next priorities.

---

# Review repairs — explicit CLI sessions

Preserve healthy backends on request/save errors; probe liveness before reuse;
invalidate backend health after observation failures; rehash additional captured
inputs across requests; report didOpen/didChange accurately. Regression tests
cover the reported failure paths. New requested priority: unconfirmed rg matches,
coverage explanation, freshness UX, then stronger reuse/batch workflows.
See docs/hakorune-follow-up.md for exact implementation status.

---

# Implemented follow-up — explicit CLI analysis reuse

`context PATH --session` keeps an explicitly owned foreground backend for
sequential JSON-line requests. Unchanged observed inputs/settings allow reuse;
source/config changes and incomplete scans force restart. Existing MCP calls
remain one-shot; coverage explanation and freshness UX remain next priorities.

Local Nyash observations: first query 6.652 seconds, second different function
1.660 seconds with confirmed backend reuse. See docs/symbol-session-v1.md and
docs/symbol-context-validation.md for protocol, limits and test evidence.

---

# Next priorities — Hakorune investigation workflow

Requested order: reuse a backend for new investigations, explain coverage gaps,
then clarify source freshness versus backend synchronization. Design and
acceptance criteria: [Hakorune follow-up](docs/hakorune-follow-up.md).
These features are not implemented by the timeout fix.

---

# Completed follow-up — large-workspace observation timeout

- CLI/core and MCP accept 1..600 seconds, default 60; MCP child deadline is 660 seconds.
- Added timeout boundary and MCP argument-forwarding regression coverage.
- Hakorune developer reports completion in ~117 seconds with a 300-second limit:
  definition, type information, five references, one test candidate, saved replay.
- Reported `assert!` call omission remains a known limit; caller-zero/deletion
  checks require ordinary search and project validation as well.
- Field results and their provenance are recorded in docs/symbol-context-validation.md;
  docs/hakorune-nekocode-handoff.md includes the operating instructions.

---

# Active follow-up — saved reference comparisons

Implemented and verified after the documentation update.
Contract: [symbol-delta-v1](docs/symbol-delta-v1.md).

- `context --packet AFTER --compare-packet BEFORE` returns observed reference
  additions/removals, matched anchors and unresolved cases.
- Full saved captures, capture-time conditions, integrity, paging and budgets
  are shared through core/CLI/MCP; comparison starts no backend.
- Live RA before/after fixture passed: added 1, removed 1, matched 1.
- `make verify` passed: 65 Rust tests, 40 Python/MCP tests, formatting,
  Clippy, Cargo check and schemas. Workflow Skill validation passed.
- Project role rules, forbidden dependencies and automatic historical
  worktrees remain later work. Existing local changes are preserved.

---

# Active task — Symbol context v1

Updated: 2026-09-08. The user approved documentation first, then implementation
of per-function investigation and saved follow-up reads.

Contract: [docs/symbol-context-v1.md](docs/symbol-context-v1.md).
Implementation status: first delivery implemented and verified. Existing local changes and contracts are preserved. Later
relationship-diff and forbidden dependency features are outside this delivery.

Implemented: external LSP collection, name/position selection, exact source and
query statuses, explicit saved packets, paging/item expansion, input staleness,
bounded transport/output, CLI/MCP forwarding, schema and workflow updates.
Live rust-analyzer observations and backend-free CLI/MCP saved replay passed.
`make verify` passed: formatting, workspace Clippy with warnings denied, Cargo
check, 65 Rust tests, 29 Python/MCP tests, and schema parsing. The Skill
validation passed. See [the validation record](docs/symbol-context-validation.md).
Known backend coverage gaps remain explicit in each response.

---

# Current task — Rust-first context layer

Updated: 2026-08-25

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
- independent `agy` release-readiness review of `3f4711b`: conditional pass
  for its narrower scope; a later full Codex audit found a release No-Go for
  effective Cargo configuration/toolchain closure, input-aware diagnostic
  comparability, CLI Git revision validation, locked/offline fallback,
  Docker/staging exclusion, and contract identity validation;
- the independent Codex audit was read-only: no tracked file was edited and no
  commit, push, deletion, or tag was made;
- the former 3,754-line core monolith is now split into a roughly 1,028-line
  public facade plus seven provider modules (largest about 993 lines), under
  [docs/module-boundaries.md](docs/module-boundaries.md);
- Codex P0 fixes are implemented: effective Cargo hierarchy digests and
  release rejection, input-aware diagnostic comparability, option-safe Git
  revisions, locked/offline compiler/MCP fallback, workspace refresh after
  diagnostics, Docker/staging exclusion, schema identity, and snapshot
  envelope validation;
- `NEKOCODE-COMPARABILITY-MATRIX-V1` is implemented locally: diagnostic
  comparison basis and stable reason codes, package/target/span-aware exact
  fingerprints, canonical baseline-hash validation, additive schemas, and
  core/CLI/MCP/golden coverage;
- external AI review findings are documented in
  [docs/comparability-matrix-v1.md](docs/comparability-matrix-v1.md): custom
  `CARGO_HOME` detection, tiny-budget diagnostic-envelope retention, and
  failed-run comparison safety were fixed locally; baseline hash validation
  was verified as an existing safeguard and diagnostic path normalization was
  regression-tested;
- the existing M1/L1/L2 changes remain local review follow-ups, not release
  approval.

Recovery points:

- tag: `legacy-multilang-final`;
- branch: `archive/legacy-multilang-final`.

## Verified repository state

- canonical branch: `master` at `3f4711b`, matching `origin/master`;
- working tree: contains the documented Codex P0 fixes, the core module split,
  their regression tests, and the related documentation; no review tool or
  unrelated task changed files;
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
- the release-hygiene workflow is green for `bd9faf5` in remote run
  `32718178584`, and the schema-gate workflow is green for `6540b05` in
  remote run `32718683043`;
- the latest documentation-only commit is `3f4711b`; no public `v1.2.0` tag
  has been created yet, because that remains an explicit release decision;
- the independent `agy` review found no blocker, but identified one
  reproducibility follow-up and three lower-priority hardening/cleanup items:
  `.cargo/config*` input digests (M1), ignored local legacy residues (M2),
  Git timeout/output bounds (L1), and the `jsonschema` developer message (L2).
- M1/L1/L2 are implemented locally; the core suite, formatting, Clippy,
  locked workspace tests, CLI tests, 21 MCP/Python tests, and schema parsing
  pass with the pinned schema dependency path used by CI.
- the post-implementation AI review found one accepted P0 comparison gap
  (`failed` producer runs with parsed messages), one already-closed false
  positive (baseline hash presence), and one path-normalization hardening case;
  the accepted fixes and remaining test-hardening follow-up are recorded in
  the comparability matrix document.

## Next implementation focus

### P0 — close the Codex No-Go before release

- [x] Include or explicitly reject effective parent/ignored Cargo configuration
      and toolchain inputs at the workspace/release boundary.
- [x] Include workspace/package/target/input digests in diagnostic
      comparability and add manifest/config mutation regressions.
- [x] Validate CLI `compare_ref` and pass Git revisions with an option-safe
      boundary; keep the MCP validation and CLI behavior aligned.
- [x] Make MCP Cargo fallback and compiler observations locked/offline, and
      refresh workspace provenance after observations.
- [x] Exclude ignored legacy residues from Docker context and release staging;
      do not delete them without an explicit cleanup decision.
- [x] Align schema `$id` and validate the full snapshot envelope.
- [ ] Execute the release/Docker smoke gates in a clean, approved environment.

### P1 — restore a reproducible green release gate

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
- [ ] Commit/push the current verified changes and require the exact remote
      workflow to pass before creating a release tag.

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
- [x] Add workspace-local, parent, and Cargo-home `.cargo/config*` plus
      toolchain files to the artifact input digest set, with regression
      fixtures proving configuration changes alter the observed input set.
- [x] Apply a 60-second timeout and bounded stdout/stderr capture to Git
      observations, using the same process-group cleanup as Cargo (review L1).
- [x] Keep ignored migration tools, sessions, and legacy directories outside
      Docker/release staging; the release script rejects contaminated inputs and
      does not delete local residues (review M2).
- [x] Keep the `jsonschema` dependency failure actionable for local developers
      while retaining the CI-installed pinned dependency (review L2).

### Frozen for this release

- fuzzy diagnostic matching;
- a second language or a generic analyzer abstraction;
- rust-analyzer integration;
- additional MCP tools, prompts, resources, Plugin UI, or remote service.

### P1 — Comparability Matrix v1 local gate

- [x] Add comparison-basis evidence for workspace/package/target/features,
      compiler configuration, toolchain, producer/profile, and completeness.
- [x] Add stable machine-readable reason codes while retaining human
      `limitations`.
- [x] Include package/target/primary-span identity in exact fingerprints.
- [x] Reject canonical-hash tampering and preserve integrity reasons under a
      tiny budget.
- [x] Treat failed producer runs with parsed messages as incomplete for exact
      comparison while retaining their snapshot evidence.
- [x] Retain the diagnostic run envelope and comparison basis when a tiny
      budget omits all diagnostic messages.
- [x] Recognize explicit `CARGO_HOME/config*` files in compiler-config basis
      observation and assert Cargo diagnostic paths are workspace-relative.
- [x] Add one-axis core matrix coverage, additive schema fixtures, and
      CLI/MCP parity assertions.
- [ ] Commit the mechanical split plus matrix behavior separately and verify
      the exact remote SHA before release/tag decisions.

## Implemented feature record

**NEKOCODE-COMPARABILITY-MATRIX-V1** is now implemented locally as an additive
contract hardening feature, not a new language or analyzer. The remaining
operational work is the separate commit, exact remote CI verification, and the
already tracked clean release/Docker smoke gate.

The design and 16-case acceptance matrix are recorded in
[docs/comparability-matrix-v1.md](docs/comparability-matrix-v1.md). No public
CLI option, MCP tool, fuzzy matcher, rust-analyzer integration, or second
language is part of this slice.

The authoritative decisions are under `docs/`.
