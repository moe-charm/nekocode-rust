# NEKOCODE-COMPARABILITY-MATRIX-V1

Status: implemented locally; review follow-ups applied, pending the separate
commit and exact remote CI gate, 2026-08-25.

This is a feature name, not a new artifact contract version. The public
artifacts remain `snapshot-v1` and `context-v1`; the matrix adds optional,
additive comparison evidence within those contracts.

## Decision

NekoCode's next feature is an executable comparability contract. It makes
the conditions used for a diagnostic delta machine-readable and will prove
the decision with one-axis golden fixtures across core, CLI, MCP, and schema
validation.

The goal is to make a false `comparable` result structurally difficult, not to
add another parser, diagnostic producer, CLI command, or MCP tool.

```text
NEKOCODE-COMPARABILITY-MATRIX-V1

same observation basis       -> comparable + exact multiset delta
missing baseline observation -> baseline_missing
changed/unknown basis        -> not_comparable + stable reason code
incomplete run               -> partial + incomplete evidence
```

## What is already implemented

The current local implementation already covers several findings from the
external review and they must not be reimplemented as duplicate work:

- workspace, package, target, manifest, lockfile, toolchain, and effective
  Cargo configuration inputs are observed and compared;
- parent and Cargo-home configuration files are included when present, while
  the release boundary rejects contaminated ignored/untracked configuration;
- `all_targets`, `all_features`, producer, profile, producer version, and tool
  provenance participate in comparability;
- compiler failures with parsed messages remain useful evidence, but are
  incomplete for exact comparison; only successful producer runs are treated
  as complete comparison observations;
- exact multiset matching and multiplicity preservation are already used;
- snapshot `contract_version`, `artifact_kind`, and `schema_version` are
  validated on read;
- CLI and MCP use the same core payload and current parity tests pass.

These facts are why this slice is a contract hardening task, not a restart of
the Rust workspace observer.

## Implemented changes

### 1. Comparison basis

The implementation adds an optional `comparison_basis` to the diagnostic
observation. It records
normalized, non-secret evidence such as:

- workspace member set digest;
- package set digest;
- package/target coverage digest;
- feature definition and selected-feature coverage;
- compiler-affecting configuration status and digest;
- producer/profile/version and toolchain identity;
- observation completeness.

The basis is evidence for the decision, not a second source of truth. Core
remains the only owner of the comparison rule.

Configuration status must distinguish at least `observed`, `absent`, and
`unknown`. An unavailable external configuration source must not silently be
treated as identical to an observed empty configuration.

### 2. Stable reason codes

The implementation keeps the existing human-readable `limitations` for
compatibility and adds machine-readable reason objects to `diagnostic_delta`:

```text
baseline_missing
baseline_integrity_unavailable
baseline_integrity_mismatch
comparison_basis_unavailable
toolchain_mismatch
producer_mismatch
producer_version_unknown
producer_version_mismatch
analysis_profile_mismatch
package_set_mismatch
target_coverage_mismatch
feature_coverage_mismatch
feature_definition_mismatch
workspace_inputs_mismatch
compiler_config_mismatch
compiler_config_unknown
tool_provenance_mismatch
baseline_observation_incomplete
current_observation_incomplete
```

When the status is `not_comparable` or `partial`, `added`, `resolved`, and
`persisting` must not be interpreted as a successful empty delta. The reason
codes and status remain part of the mandatory envelope and are never removed
by the output budget when the envelope fits.

### 3. Exact diagnostic identity

The current model stores `package_id`, `target`, and structured spans, and the
fingerprint now uses the contract's complete primary identity, including:

- producer and profile;
- package and target;
- diagnostic code or level;
- normalized message;
- workspace-relative primary path;
- primary span start/end and primary label.

Matching remains exact and multiset-based. Line movement, message similarity,
or fuzzy relocation is explicitly out of scope.

### 4. Baseline integrity

Reading a baseline validates the stored canonical hash when present. A
wrong contract/artifact/schema identity is already rejected. A missing hash
may be read for migration, but diagnostic comparison returns
`baseline_integrity_unavailable` rather than silently treating the artifact
as fully trusted.

## External review follow-up (2026-08-25)

Two independent read-only reviews were run against the local working tree.
The first review completed a broad pass; the second completed a focused pass
after its full-context request exceeded the wait budget. Review agents were
instructed not to edit or commit files.

| Review finding | Disposition |
| --- | --- |
| A custom `CARGO_HOME/config*` path was not recognized as compiler configuration | **Fixed.** The basis collector now recognizes explicit Cargo-home config paths, with a unit regression. |
| A tiny budget could remove the whole diagnostic run and hide its basis | **Fixed.** Messages may be omitted, but the run envelope, basis, producer, status, and provenance remain; omission counts cover messages only. |
| `failed` with parsed diagnostics could be treated as a complete comparison | **Fixed.** Compiler failures remain snapshot evidence, but their comparison basis is incomplete and deltas are `partial` with an observation reason. |
| Baseline integrity was only checked by hash presence | **Closed as a false positive.** `read_rust_snapshot` recomputes and verifies the canonical hash before the context evaluator receives the presence bit. Missing hashes still produce `baseline_integrity_unavailable`. |
| Diagnostic paths might remain absolute across checkout roots | **Mitigated and tested.** Production Cargo JSON parsing strips the canonical workspace root; the golden fixture asserts captured diagnostic paths are relative. |
| Producer-version, baseline-incomplete, and profile-only branches lacked direct matrix assertions | **Follow-up test gap recorded.** Producer/profile mismatch and current partial runs are covered; dedicated branches remain post-MVP hardening. |
| Unknown global Cargo configuration can make minimal environments non-comparable | **Intentional fail-closed behavior.** `unknown` is safer than assuming an unobserved external config is empty. |

The accepted fixes are deliberately narrow: no new producer, language, public
option, MCP tool, or fuzzy matcher was introduced. Timed-out review commands
are recorded as operational outcomes, not as code findings.

## One-axis golden matrix

The fixture cases use a common base and change one comparison dimension at a
time. They must not introduce new public package/target-selection options.
Where the current CLI intentionally has fixed coverage, dimension changes may
be represented by a controlled artifact mutation or a dedicated fixture
profile rather than by broadening the CLI surface.

| ID | One changed condition | Expected result |
| --- | --- | --- |
| C01 | same basis, diagnostic added | `comparable`, `added` |
| C02 | same basis, diagnostic fixed | `comparable`, `resolved` |
| C03 | same basis, diagnostic persists | `comparable`, `persisting` |
| C04 | workspace/package set changed | `not_comparable`, `package_set_mismatch` |
| C05 | target coverage changed | `not_comparable`, `target_coverage_mismatch` |
| C06 | selected feature coverage changed | `not_comparable`, `feature_coverage_mismatch` |
| C07 | feature definition changed | `not_comparable`, `feature_definition_mismatch` |
| C08 | compiler-affecting Cargo config changed | `not_comparable`, `compiler_config_mismatch` |
| C09 | producer changed (Cargo/Clippy) | `not_comparable`, `producer_mismatch` |
| C10 | toolchain changed | `not_comparable`, `toolchain_mismatch` |
| C11 | current run incomplete | `partial`, `current_observation_incomplete` |
| C12 | baseline has no diagnostic run | `baseline_missing` |
| C13 | baseline canonical hash tampered | read/compare failure or integrity reason |
| C14 | target or primary span changed | no false `persisting` match |
| C15 | tiny budget | status, basis, reasons, and omissions retained |
| C16 | CLI/MCP same request | normalized payloads agree |

At least these 16 named cases are required. Negative cases should mutate one
basis dimension wherever possible. Full compiler-rendered text and elapsed
time must not be golden values; assert status, reason, code, target, relative
span, and exact counts instead.

## Contract and adapter scope

The schema change is additive within `context-v1`/`snapshot-v1`:

- optional `comparison_basis` evidence;
- optional `diagnostic_delta.reasons` with stable codes and dimensions;
- existing `limitations` retained as human explanations.

Core defines the fields and decision. CLI and MCP only transport the same
serialized result. Human summary may explain a reason code but may not invent a
new status. JSON Schema, core serialization, CLI output, and MCP output must
be checked for the same representative comparable, not-comparable,
baseline-missing, and partial cases.

## Acceptance gate

The local implementation gate is complete for the code/schema/test slice:

1. named matrix cases are covered by network-independent core, integration,
   schema, CLI, and MCP tests;
2. every required mismatch or unknown basis returns a non-comparable status;
3. comparable cases have no reason codes and deterministic exact deltas;
4. partial/baseline-missing cases cannot be mistaken for an empty delta, and
   a failed producer run with parsed messages is never `comparable`;
5. wrong envelope identity and canonical-hash tampering are rejected or
   explicitly marked untrusted;
6. CLI/MCP payload parity and standard Draft 2020-12 schema validation pass;
7. repeated execution has the same canonical hash after volatile fields are
   excluded;
8. `cargo fmt --all -- --check`, locked workspace tests, Clippy with
   `-D warnings`, Python/MCP tests, and `make verify` pass.

The clean release/Docker smoke and exact remote SHA remain separate operational
gates. They should be completed before packaging a release, but they are not
a reason to add new
languages, new MCP tools, rust-analyzer, fuzzy matching, semantic hunk
linking, or an OS sandbox to this slice.

## Promotion order

1. Commit the mechanical split and matrix behavior as reviewable changes.
2. Verify the exact remote SHA with the Rust, Python, schema, and adapter gates.
3. Run the clean release/Docker smoke gate, then make the release/tag decision.

This order keeps the mechanical refactor and behavior change reviewable as
separate commits.
