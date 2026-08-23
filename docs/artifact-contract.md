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

## Context

`context` combines a caller-selected baseline revision and working-tree state
with the current workspace observation. It may contain:

- staged and unstaged Git changes, renames, deletes, untracked/binary markers;
- untracked file content only when explicitly requested;
- unified-diff hunks and bounded source excerpts;
- optional structured Cargo/rustc diagnostics;
- an exact diagnostic delta against a compatible saved snapshot;
- status, provenance, budget, and omission information.

The default Git base is explicit `HEAD` or a caller-provided ref. NekoCode does
not infer a merge base. Untracked content is reported as a marker by default;
reading it requires an explicit option.

JSON is the canonical `context-v1` representation. A human-readable summary
may project fields from the completed core artifact, but it must not infer new
facts, define adapter-specific statuses, or change the underlying artifact.
The summary must make truncated patch counts, omissions, incomplete evidence,
and non-comparability visible.

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

## Versioning and parity

The checked-in schemas under `schemas/` are the external contract. Core Rust
types are the semantic source and schema validation is part of CI. CLI and MCP
payloads must match for the same request except for transport fields and
volatile timestamps. A breaking field or status change requires `v2`; additive
optional fields remain within `v1`.
