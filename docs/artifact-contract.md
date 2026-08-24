# Artifact contract

Status: accepted design decision, 2026-08-23.

The public artifacts are versioned independently from internal module names.
The first external contracts are `snapshot-v1` and `context-v1`.

## Snapshot

```text
nekocode snapshot PATH
    -> metadata-only SnapshotV1

nekocode snapshot PATH --analysis cargo-check
    -> SnapshotV1 with a diagnostic observation
```

The snapshot is an explicit file chosen by the caller. There is no hidden
latest snapshot, automatic history database, or implicit baseline selection.
The artifact records, as applicable:

- workspace root, packages, targets, features, and editions;
- Cargo/rustc versions, host/target profile, and execution policy;
- relevant manifest, lockfile, toolchain, and configuration digests;
- Git HEAD/dirty state and normalized workspace-relative paths;
- analysis mode and tool provenance;
- an execution policy describing trust, offline mode, environment filtering,
  compiler-wrapper handling, and sandbox gaps;
- optional compiler diagnostic observation;
- a canonical payload hash that excludes volatile timestamps and storage paths.

Cargo metadata parsing must pin a machine-readable format version, tolerate
unknown fields, and avoid assuming a fixed future enum set.
`PATH` may identify a manifest root, nested directory, or existing file. The
nearest ancestor manifest is used to invoke Cargo, and Cargo's canonical
workspace root becomes the artifact and Git boundary.

## Context

`context` combines a caller-selected baseline revision and working-tree state
with the current workspace observation. It may contain:

- staged and unstaged Git changes, renames, deletes, untracked/binary markers;
- untracked file content only when explicitly requested;
- unified-diff hunks and bounded source excerpts;
- optional structured Cargo/rustc diagnostics;
- an exact diagnostic delta against a compatible saved snapshot;
- status, provenance, budget, and omission information.

### Git change scopes and line metrics

When Git context is requested, `diff.change_scopes` is a bounded aggregate
that remains present even if patch bodies or per-file details are removed by
the output budget. Scope names and meanings are fixed:

- `revision`: `compare_ref...HEAD`;
- `staged`: `HEAD` to the index, or Git's empty-tree comparison when no first
  commit exists yet;
- `unstaged`: the index to the working tree;
- `untracked`: paths not present in the index.

Each scope reports file/Rust-file counts, counted additions/deletions, binary
file count, and files whose contents were deliberately not read. Each included
`changed_files` entry carries zero or more `scope_changes`; a path may contain
both staged and unstaged observations. A single stage enum on the file is not
sufficient and must not overwrite one scope with another.

Line metrics come from NUL-delimited Git numstat output, not from the retained
patch prefix. Binary files have unknown additions/deletions and are counted as
`binary`. Untracked contents remain `not_read` unless an explicit future
contract defines how their metrics are observed; marker-only mode does not
silently claim zero changed lines. NekoCode never stages or edits files.

The default Git base is explicit `HEAD` or a caller-provided ref. NekoCode does
not infer a merge base. Untracked content is reported as a marker by default;
reading it requires an explicit option.
Filename collection uses NUL-delimited Git output so UTF-8 names are not
stored as quoted octal text. Patch collection disables non-ASCII pathname
quoting so a changed file and its hunks remain associated.

JSON is the canonical `context-v1` representation. A human-readable summary
may project fields from the completed core artifact, but it must not infer new
facts, define adapter-specific statuses, or change the underlying artifact.
The summary must make truncated patch counts, omissions, incomplete evidence,
and non-comparability visible.
If every patch body line is removed by the byte budget, the summary says the
patch was omitted; it must not present the retained line count as `+0/-0`.

Feature selection is meaningful only for compiler diagnostics. A context
request with `all_features=true` and diagnostics disabled is invalid rather
than a silently ignored option.

## Comparability

Diagnostic delta is not an empty list when a comparison cannot be made. The
response carries one of:

```text
comparable
baseline_missing
not_comparable
partial
```

`not_comparable` reasons include toolchain, target, package, target selection,
feature/default-feature, compiler-affecting configuration, or analysis-profile
changes. A baseline without a diagnostic observation is always
`baseline_missing`.

MVP matching is exact and multiset-based. It does not fuzzy-match diagnostics
that moved to another line. A fingerprint includes the producer, code/level,
normalized message, workspace-relative primary path/label, and a stable span
component. Repeated error/warning observations retain their multiplicity in
JSON. Auxiliary note/help/failure-note messages remain in the producer run but
do not enter `added`, `resolved`, or `persisting`. A human summary may collapse
identical fingerprints only when it labels those counts as unique and keeps the
raw observation count visible.

Public artifacts replace workspace-local paths with `$WORKSPACE` and other
absolute path fields with `$EXTERNAL`. This includes both `baseline` and
`diagnostic_delta.baseline_path`; artifact storage locations must not leak.

## Budget and omissions

Core limits are bytes, item counts, and lines. Token estimates are advisory
for Skills and model callers, not the hard correctness boundary.

The fixed truncation order is:

1. envelope/status/provenance/omissions;
2. new error diagnostics and primary excerpts;
3. changed hunks;
4. warnings;
5. surrounding context;
6. verbose metadata.

Every omitted group has a reason, count, and priority. Silent truncation and
an incomplete artifact labelled complete are forbidden.
Operational diagnostic failures, timeouts, output limits, and partial runs set
the enclosing artifact evidence to `incomplete` and record a limitation.
`limitations` also records requested scope and safety boundaries. Its presence
alone does not make evidence incomplete. In particular, the default choice to
report untracked files as markers without reading their contents is a complete
observation of that requested mode. Evidence is downgraded only for an actual
failed/partial observation, unavailable or incompatible diagnostic comparison,
or recorded omission/truncation.

### Budget invariant gate

The core budget is a serialized-byte limit; token estimates are advisory. The
property-test gate for any truncation change must assert that:

- `diff.change_scopes` is present, ordered, and unchanged across budget values;
- each `counted` file has numeric additions/deletions, while `binary` and
  `not_read` files retain null line counts;
- every omitted group has a nonzero reasoned ledger entry and retained
  `changed_files` never claims to be complete after omission;
- UTF-8 patch and excerpt truncation stops only at character boundaries;
- when the envelope can fit, `serialized_bytes <= max_bytes`; when it cannot,
  the artifact explicitly reports `output_limited`, `budget.exceeded`, and
  `evidence=incomplete` rather than pretending the result is complete;
- CLI and MCP preserve the same budget outcome and scope aggregates.

The tests compare several budgets against the same fixture and treat the
pre-budget scope aggregate as the invariant evidence. They do not require
smaller budgets to preserve per-file detail or patch text.

### Optional Clippy producer

Clippy is a deferred, explicit diagnostic producer. It must not be silently
added to the default `cargo-check` profile or compared with a rustc-only
baseline. Before implementation, the contract requires:

- a producer/profile marker distinguishing `clippy` from `cargo_check`;
- comparability fingerprints covering Clippy, Cargo, rustc, package/target,
  feature/default-feature, compiler-affecting configuration, and command
  profile;
- exact multiset diagnostic matching within the same producer/profile only;
- the same `not_run`, `completed_*`, `tool_failed`, `timed_out`,
  `output_limited`, and `partial` states as Cargo diagnostics;
- explicit trusted-workspace permission and the existing execution-policy
  disclosure, because Clippy also runs build scripts and procedural macros;
- fixtures for clean, warning, compiler-error, tool-failure, and changed
  profile cases before exposing a CLI or MCP option.

No Clippy-specific lint set, accuracy percentage, or cross-producer delta is
part of `context-v1` until those gates are implemented and reviewed.

## Versioning and parity

The checked-in schemas under `schemas/` are the external contract. Core Rust
types are the semantic source and schema validation is part of CI. CLI and MCP
payloads must match for the same request except for transport fields and
volatile timestamps. A breaking field or status change requires `v2`; additive
optional fields remain within `v1`. Change Scope v1 fields are additive: older
`context-v1` readers may ignore them, while golden artifacts verify that the
current producer emits and validates them.
