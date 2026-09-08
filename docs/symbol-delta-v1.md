# Symbol reference delta v1

Status: implemented and verified, 2026-09-08. This extends saved symbol
investigation with an on-demand comparison. No resident watcher is required.

## Workflow

```sh
nekocode context PATH --at src/lib.rs:12 --save-packet /tmp/before.json
# Make the intended code change, then select the same definition again.
nekocode context PATH --at src/lib.rs:14 --save-packet /tmp/after.json
nekocode context --packet /tmp/after.json --compare-packet /tmp/before.json
nekocode context --packet /tmp/after.json --compare-packet /tmp/before.json --format summary
# Use the returned delta cursor to read further changes.
nekocode context --packet /tmp/after.json --compare-packet /tmp/before.json --cursor CURSOR
# Expand evidence using the corresponding original packet and item ID.
nekocode context --packet /tmp/before.json --item BEFORE_ITEM_ID
```

The compiled CLI compares captured packets without launching Git, Cargo or
rust-analyzer, or consulting live source. Older source is expected to differ
from the current disk; eligibility uses freshness **at capture**, not freshness
now. Initial collection is still explicit. Automatic historical worktrees,
project role rules, forbidden dependency checks and test-delta analysis are
separate later work.

`--compare-packet BEFORE` requires `--packet AFTER`; it cannot be combined with
`--item` or live/Git/diagnostic flags. `--budget`, `--max-items`, `--cursor`,
`--format` and `--output` remain available. MCP uses `compare_packet` through
the existing `nekocode_context` tool. This mode returns `symbol-delta-v1`;
existing symbol-context and snapshot/Git contracts remain unchanged.

## What the result means

Return matched, added, removed and unresolved **reference observations**, with
before/after packet IDs, original item IDs, source locations, exact line
evidence and the reason for each classification. Added/removed observations
are not proof of runtime dependency changes or safe deletion.

Only compare captures with the same recorded workspace root, requested scope,
backend version, and captured Cargo/toolchain/configuration input digests.
Both captures must have a selected, identifiable definition, healthy completed
observation, stable captured inputs, and a completed reference query with all
its results retained. Otherwise return `not_comparable`, machine-readable
reasons and null delta counts; missing evidence must never become removed refs.
Host/target and effective external compiler configuration are not recorded by
these packets. External configuration and backend synchronization remain unverified even when
these observed conditions match. No cross-target coverage is inferred.

Match the target using its path, backend containing-symbol identity and an
unchanged declaration line that is unique within each captured file. Renamed,
ambiguous or differently declared targets require a new investigation decision.
Reference matching uses path, a unique containing-declaration line/name/kind,
exact reference line and its column range; absolute line numbers and packet
item IDs are not identity. Thus inserting unrelated lines does not invent
reference additions/removals. Duplicate anchors, missing containing symbols
and edits leaving unmatched observations on both sides of one container are
`unresolved`; do not guess a pairing. This is conservative source anchoring,
not an independent Rust semantic analyzer or rename tracker.

Counts are computed from the full captures before presentation trimming.
Changes are displayed before matched observations. Paging cursors bind both
packet contents and the comparison algorithm. Tiny budgets preserve comparison
reasons/counts and explicitly report `output_limited` if the required envelope
cannot fit or no item can be displayed. Empty pages never repeat a cursor.

## Verification

Cover inserted-line stability, actual added/deleted callers, duplicate anchors,
changed reference lines, target/scope/backend/config mismatch, partial or failed
observations, completed zero references, capture-time freshness, packet integrity,
cursor binding, budget invariants, original-code expansion and final MCP output.
Run `make verify` and a live rust-analyzer before/after fixture.

## Live acceptance record

On 2026-09-08, Linux / Rust 1.89.0 / rust-analyzer 1.89.0:

- Created a standalone Cargo fixture with `target`, `kept` and `removed`.
- Captured before: two references, healthy backend, stable source.
- Inserted two unrelated lines, retained `kept`, replaced `removed` with `added`.
- Captured after: two references under the same recorded conditions.
- Compared with `PATH=/no-executables` and a nonexistent analyzer path.
- Result: **added 1, removed 1, matched 1, unresolved 0**; the new schema passed.

Both initial responses displayed only one item, confirming that comparison
uses full saved evidence. This is a functional acceptance case, not a measured
AI productivity improvement. Automated integration tests additionally cover
final MCP parity, Unicode, stale-on-disk historical packets, condition failures,
duplicate anchors, edited callers, original-code expansion and bounded paging.

Final `make verify` passed: Rust formatting, workspace Clippy with warnings
denied, Cargo check, 65 Rust tests, 40 Python/MCP tests and schema parsing.
The workflow Skill validation passed. LSP unit tests use the immutable executable
fixture to avoid Linux ETXTBSY races from freshly written test executables.
