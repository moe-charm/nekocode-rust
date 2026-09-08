# Symlink input scope v1

Design recorded before implementation, implemented 2026-09-08. Responds to the Hakorune d28498d591
report: five untracked links blocked retention despite unchanged input hashes.
Git tracking status must not decide input relevance.

The existing inventory scope is Rust files and Cargo/toolchain/config files,
plus separately captured backend source locations. This is not arbitrary build
input or external-environment verification. Apply that same scope to links:

- Workspace-local directory links: resolve and enumerate their targets with the
  normal bounds; visit each physical directory once. Record the alias mapping.
  This does not exclude .venv by name. A .venv/lib64 -> lib alias is inspected.
- File links whose logical OR resolved name is a Rust/Cargo input: hash content
  and record the raw target and canonical workspace-local destination. Retarget,
  deletion and content changes invalidate reuse. External/unreadable input
  targets remain unverified and reject reuse.
- Links to regular non-input files: explicitly report outside_input_file_scope,
  with the logical path and target. They have the same limited guarantee as
  ordinary non-input files. No Python-runtime or binary-content guarantee.
- A dangling native build-artifact link (.so/.dll/.dylib/.a/.o) whose lexical target
  is within the excluded workspace target directory is explicitly outside the
  generated-build-output scope. Other dangling links remain unverified; unknown
  directory targets must never be silently excluded.
- Broken/looping/out-of-workspace directory links or unknown targets: incomplete
  scan, explicit reason, no forced reuse. Symbol locations first captured after
  startup remain late inputs; classifying a non-input link is not a claim that
  arbitrary include! or external build inputs were audited.

Keep a bounded mapping inventory (4096 link records maximum) for change detection.
Report bounded path/target/reason examples and counts separately from scan issues;
excluded links do not make the scan incomplete. Compare mapping inventories on
capture finish, replay and session acquire; preserve old packet integrity using
optional fields. Legacy packets without mapping evidence must not gain a new
claim of verified links. Normal file content hashes remain separate from link
mapping changes. Report confirmed source_changed vs source_unverified distinctly.

Validation: recreate the reported five-link layout without changing the real
Hakorune developer workspace; confirm reuse. Input file/directory link content,
retarget (including same-content targets), breakage and external targets must
invalidate or reject reuse. Keep old packet replay, schema and bounded-output
checks. Record exact validation commit and dependency conditions separately from
the original developer report.

Directory aliases are recorded and traversed through their canonical local target;
physical-directory deduplication handles aliases to already visited ancestors.
Unresolvable symlink cycles remain unverified. Mapping counters classify resolved
linkage; unreadable or oversized content can still make the scan incomplete.
Excluded link contents are not monitored; their mapping is recorded, so a changed
destination can conservatively trigger restart even when both targets are outside
input scope. A captured non-Rust file still enters freshness through the existing
source-capture mechanism, and late capture still blocks retention.

A workspace-local `.cargo` directory alias retains the special meaning of its
config/config.toml files, including file links below that alias. The canonical
destination is rescanned when this contextual scope is discovered, so directory
iteration order cannot hide Cargo configuration.

## Validation

Full gate: 77 Rust / 45 Python-MCP tests passed. Tests cover the five environment
links, a Rust file inside .venv (still included), same-content input-link retarget,
content changes, directory retarget, missing/external input targets, unknown
.so links, cycles, .cargo aliases and bounded link records/examples. The 4096-link
limit produces incomplete scans; examples stop at 16 with an omitted count.

A development debug build on Hakorune cf458a28e135178364d873008ae77c12908da61f,
with the five reported logical links reconstructed and locked dependencies cached,
returned OBJ then EXE investigations in 215.774s / 2.704s. Both completed, with
5136 input hashes matching, backend health ok, retained=true and second
backend_reused=true. Link scope: 1 verified directory, 4 excluded non-input/output
links, 0 unverified. The local /usr/bin/python3 resolves to Python 3.10; it is not
the original developer environment. This is a reproduction of the reported link
layout, not a claim to have validated Hakorune d28498d591's original worktree.
Backend synchronization stays unverified and caller-zero remains unproven.

No tracked Hakorune source/manifest/toolchain changes. The five fixture links
remain in the dedicated validation checkout; git status was unchanged/clean and
stdin EOF ended the session with exit 0. No link in the developer's workspace
was changed. Remaining validation on that original environment is a field trial.
