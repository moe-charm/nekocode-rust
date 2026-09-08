# Symbol context v1 — AI investigation workflow

Status: implemented first delivery, 2026-09-08. The user-approved redesign
supersedes the earlier decision to defer rust-analyzer-backed investigation.

## Product and first delivery

Given one function/source position, return its definition, observed references,
containing symbols, type/contract information, related test candidates, exact
source excerpts, and observation conditions/gaps. Let the AI retrieve more
items or expand one item without repeating the investigation.

First delivery: investigation plus continuation. The follow-up
[Symbol delta v1](symbol-delta-v1.md) now compares saved reference observations.
Forbidden-dependency rules, automatic edits and permanent whole-repo call graphs
remain later work. Rust meaning comes from external rust-analyzer LSP.

## CLI and compatibility

Existing snapshot and Git-focused context invocations retain snapshot-v1 and
context-v1. Explicit selectors return the separate symbol-context-v1 artifact.

```sh
# Positions are 1-based; columns count Unicode characters.
nekocode context PATH --at src/config.rs:42 --save-packet /tmp/investigation.json
nekocode context PATH --symbol parse_config --max-items 8
# The compiled CLI starts neither Cargo nor rust-analyzer for follow-up reads.
nekocode context PATH --packet /tmp/investigation.json --cursor CURSOR
nekocode context PATH --packet /tmp/investigation.json --item ITEM_ID
nekocode context PATH --packet /tmp/investigation.json --format summary
```

--at, --symbol, and --packet are mutually exclusive. Ambiguous names return
candidates with concrete positions, never an arbitrary selection. PATH may
default to `.` for collection. Replay uses the recorded workspace root; an
explicit PATH must match it. --max-items defaults to 8. --budget retains the advisory-token
convention (4 bytes per requested token); final compact JSON is the size
boundary. An envelope that cannot fit is explicitly output-limited.

--save-packet explicitly writes a disposable local evidence cache, separate
from --output (the displayed response). Without a saved packet, do not
advertise a continuation another process cannot resolve. --cursor and --item
require --packet. Cursors belong to one packet ID; item expansion uses captured
source and the containing symbol when available. Saved packets have a versioned
envelope, integrity digest, and bounded captured sources.

Reject Git-only flags (--compare-ref, --working-tree, --baseline) and compiler
flags in symbol mode rather than ignoring them. Initial analysis scope is the
host/default configuration or explicit --all-features. --timeout-seconds bounds
backend observation. --allow-build-scripts explicitly enables build-script and
proc-macro preparation. No intent selector is needed in this first version.

## Backend, execution, freshness

For line-only positions, select the containing declaration using backend
document outlines; explicit columns identify the requested token.
Use standard LSP for symbols, definitions, references, hover/type information,
and document outlines. Isolate optional rust-analyzer/relatedTests. Keep
per-query success, unsupported, timeout or failure. An unavailable backend is
an actionable status, never zero references. Use NEKOCODE_RUST_ANALYZER_PATH
or rust-analyzer on PATH. The backend health/message and query result counts
remain visible; failed queries have null counts, not zero.

Initial collection may start rust-analyzer and Cargo metadata. Disable
check-on-save, build scripts and proc macros by default. Enabling preparation
is for trusted workspaces and is not an OS sandbox. Record effective options,
cfg(test), backend version and unobserved external configuration. No tests run
as a context side effect. Replay starts no backend. A resident daemon is not
required in this first delivery; measure startup separately. A development
`cargo run` wrapper or the MCP Cargo fallback still launches Cargo to run the
CLI itself; configure `NEKOCODE_BINARY_PATH` or an installed `nekocode` for
backend-free replay.

Capture source/input hashes around observation and recheck captured inputs
on replay. Changed inputs are stale. File stability and backend quiescence do
not prove a common LSP analysis generation: record backend synchronization as
unverified when it cannot be proved. Query `completed` means the request
finished, not that the backend proved coverage. References may include inactive
cfg branches; macro-expanded or indirect uses and tests may be absent.
Unobserved external build inputs remain
outside the freshness guarantee. Never silently label old results current.

## Response and navigation

Include contract identity, packet ID, target/disambiguation candidates, scope,
backend state, per-query statuses, freshness, items, observed/retained totals,
omissions and continuation data. Items include ID, relation, backend/source,
workspace-relative location, reason, exact code, containing symbol when known,
and truncation. Workspace root and the caller-selected packet path stay visible
for navigation; source excerpts are verbatim and are not path-redacted.
Distinguish definition, reference, type definition and test
candidate. A reference is not necessarily an executed call; a test candidate
is not an executed test result.

Order definition/contracts before references and tests; deduplicate locations.
Begin at one hop. Include usable code in the first page. Ranking is navigation,
not proof of importance or complete impact. Preserve source verbatim through
CLI and final MCP output, including URLs, division and backslashes.

Separate completed-zero, acquired-but-not-displayed, not-acquired, stale, and
backend-synchronization-unverified states. Bound collection, source capture,
saved files, protocol frames and final output separately. Budget trimming
retains failures and continuation/omission data. The existing nekocode_context
MCP tool forwards selectors and replay arguments to the same core implementation.

## Acceptance

1. A real Rust fixture yields a definition, same-name-safe references,
   containing code and test candidates or explicit unsupported status.
2. Ambiguous names and Unicode positions/paths behave predictably.
3. Saved paging/expansion works with Cargo/RA unavailable; reject wrong cursors
   and tampered packets, and detect stale captured inputs.
4. Missing backend, timeout, unsupported queries, completed zero references and
   budget omission remain distinct through final MCP tools/call output.
5. Existing tests/contracts pass. Add schema, fake-LSP, CLI and MCP tests.
   Record a live RA smoke test; normal CI need not install rust-analyzer.
6. make verify passes. After tuning two real examples, compare six untuned
   tasks with equal model/RA capabilities/workspace/time. Measure startup,
   extra reads, missed/stale evidence and human clarification. No unmeasured
   speedup claim. Comparative evaluation is later product validation, separate
   from the implementation gate.

Recorded implementation checks and live backend observations are in
[the validation record](symbol-context-validation.md).

The 2026-09-08 follow-up adds explicit saved reference comparisons under
[Symbol delta v1](symbol-delta-v1.md). This supersedes the earlier
deferral of reference comparisons only; other later features remain deferred.

## Large-workspace timeout

Live `--timeout-seconds` accepts 1 through 600 seconds (default 60).
For a large workspace, retry the same source position with
`--timeout-seconds 300`; 600 is also supported. This bounds backend observation,
not total CLI wall time including input capture and output serialization.
A timeout does not mean there are no references. The MCP gateway allows 660
seconds for its CLI process; client-side timeouts may be shorter.

The original eff8d17 distribution accepted only 1 through 120 seconds. Use the
updated timeout600 runtime for longer observations; that original binary cannot
accept 300 seconds.

## Optional unconfirmed text evidence

`--text-candidates` adds bounded ripgrep observations after semantic collection.
See [text-candidates-v1](text-candidates-v1.md). Relation
`unconfirmed_text_candidate` is navigation evidence only, excluded from semantic
reference counts and reference-delta matching. A separate `rg/text_candidates`
query reports scan status and scope. Existing output/packet schemas already allow
new relation and producer strings; older saved packets remain readable.

## Coverage and current input verification

Additive optional `coverage` and `freshness.verification` fields explain observed
scope and input hash comparisons. See [coverage/freshness](coverage-freshness-v1.md).
Saved packets lacking these fields still pass integrity checks and receive
derived explanations on replay. Update public schema consumers for these fields.
New captures record cfg_test as requested_true_unverified to distinguish the
requested cfg.setTest=true setting from verified effective test coverage.
