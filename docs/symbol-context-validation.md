# Symbol context v1 validation

Recorded 2026-09-08 on Linux, Rust/Cargo 1.89.0 and rust-analyzer
1.89.0 (2948388 2025-08-04), with rust-src installed. These are functional
smoke observations on the local implementation, not a speed or accuracy benchmark.

## Live backend observations

| Target | Definition / hover | References | Related tests | Startup / total observation |
| --- | --- | --- | --- | --- |
| Fixture `parse_config`, with a direct test call | 1 / 1 | 5 | 1 | 780 / 2312 ms |
| NekoCode `build_context` | 1 / 1 | 6 | 2 | 812 / 3588 ms |
| NekoCode `parse_unified_hunks` | 1 / 1 | 5 | 2 | 813 / 3692 ms |

All three reported healthy backend readiness and stable captured source inputs.
Backend synchronization remained `unverified`. Counts are responses observed
in these runs; they are not independently established completeness counts.
Startup is included in total observation time.

The two repository names also returned import/re-export locations when selected
by name. NekoCode exposed the candidates as ambiguous. Selecting the function
location with line-only `--at` then resolved its identifier via the backend
document outline. Example from `nekocode-workspace`:

```sh
nekocode context . --symbol build_context
# Select the definition from the candidates; line numbers refer to this run.
nekocode context . --at nekocode-core/src/rust_context.rs:741 \
  --max-items 3 --save-packet /tmp/build-context.json
nekocode context . --at nekocode-core/src/rust_context/git.rs:603 \
  --max-items 3 --save-packet /tmp/hunks.json
```

## Continuation and automated checks

A real fixture packet retained eight items while initially displaying two.
With `PATH=/no-executables` and a nonexistent analyzer path, the prebuilt CLI
returned the next six items and the selected function's captured body. The
final MCP `tools/call` response replayed the same packet under those conditions.
CLI responses and MCP `structuredContent` passed the standard Draft 2020-12
schema validator; MCP text decoded to the identical structured payload.

Automated coverage includes bounded LSP framing and deadlines, blocked writes,
process cleanup, Unicode positions/paths, exact URLs/division/backslashes,
ambiguous names, missing backend and zero results, packet integrity and wrong
cursors, changed captured inputs (including non-`.rs` source), explicit replay
workspace mismatch, and budgets that would otherwise create empty looping pages.
CLI mode conflicts and final MCP payloads are covered as well.

The existing Git/diagnostic workflow has regression checks for unchanged warning
identity after snapshot persistence and distinct revision/index/disk excerpts,
including filenames containing spaces.

The complete `make verify` gate passed: formatting, workspace Clippy with
warnings denied, Cargo check, 65 Rust tests, 29 Python/MCP tests, and schema
parsing. Run it after installing `requirements-dev.txt`.
The Skill also passes `skill-creator/scripts/quick_validate.py`. Live RA is a
manual acceptance check; normal CI uses a bounded fake LSP fixture.

## Observed backend limits

In the first small fixture, RA itself failed to resolve standard-library
`Result` and `assert_eq!`, missed a macro-wrapped reference, and returned no
related tests. Direct LSP checks reproduced this with build/proc-macro
preparation enabled and after additional waiting. Adding a direct test call
in a separate fixture produced a related-test candidate. The reference result
also included a feature-disabled branch under the default feature request.

The tool reports these as backend observations with explicit scope and coverage
limits. A completed request is not proof that all references or tests were
found, and stable source files do not prove a common backend analysis generation.
No tests execute during investigation. Comparative AI usefulness and timing
evaluation across untuned tasks remains separate product validation.

## Hakorune field report — 2026-09-08

The Hakorune developer reported successful investigation on a workspace described
as approximately one million lines, using the `eff8d17-timeout600` runtime and a
300-second observation limit. The run completed in approximately 117 seconds
and returned definition and type information, five references, and one related
test candidate. Saved packet continuation worked without restarting the backend.
These are user-relayed field observations; the raw successful-run packet and
logs were not independently inspected here. They are not a throughput benchmark
or evidence of measured search/time savings.

Earlier attempts hit the default 60-second timeout. The original eff8d17 binary
then rejected a requested 300 seconds before analysis because its maximum was
120. The fix accepts 1..600 seconds (default remains 60) and extends the MCP
child-process deadline to 660 seconds. Boundary tests cover accepted 300/600
values and rejected out-of-range values. A local prepared test Nyash also
completed with the updated executable using a 300-second limit.

The developer found one additional call inside `assert!` with ordinary search
that was absent from the returned reference list. This gap is not fixed by the
timeout change. Before deletion or a caller-zero conclusion, combine semantic
observations with `rg`, inspect macro-wrapped calls and indirect/FFI boundaries,
and run the relevant project checks. Neither a zero reference count nor an
empty text search alone establishes safe deletion. Test candidates are not
executed tests.

## Foreground session acceptance — 2026-09-08

Local prepared test Nyash, debug CLI and independent RA 1.89.0: the first
`src/runner.rs:41` query completed in 6.652 seconds with backend_reused=false;
the subsequent `src/runner.rs:36` query completed in 1.660 seconds with
backend_reused=true. The requests selected different functions, so this is one
observed sequence, not a controlled benchmark or a Hakorune speed prediction.
Build scripts/proc macros remained disabled and synchronization unverified.

Protocol-fixture integration checks count backend process starts independently
of the reuse flag, verify restart after source/feature changes and incomplete
input scans, renew the observation deadline, reject malformed requests and
workspace overrides, validate response schemas, and confirm EOF process cleanup.

## Review-fix regression checks — 2026-09-08

The agy, Claude and claude-glm reviews identified avoidable cache loss on invalid
requests, stale health after a cached process exits, extra restarts across
additional include/generated source files, and inaccurate didOpen/didChange
failure labels. Independent fixture reproductions confirmed invalid-request
cache loss and stale health before the fix.

Regression coverage now verifies reuse after invalid budget/timeout/line requests
and failed packet saving; reuse when moving from a non-.rs target to a normal
source; restart after that additional file changes or disappears; replacement of
an exited backend before the next observation; and health invalidation when
transport fails during an observation. Existing EOF cleanup, configuration
invalidation and incomplete-scan tests remain in place.

## Text-candidate acceptance — 2026-09-08

A live RA 1.89.0 run on a dependency-free temporary Rust crate completed in
3.359 seconds (debug CLI). Source contained a normal `target()` call and an
`assert!(target())` call on the same line, plus a comment mentioning `target`.
The backend returned one semantic reference. The opt-in rg scan returned two
unconfirmed candidates: the macro-wrapped call and the comment. Backend health
was ok; synchronization remained unverified. This is a fixture observation, not
a general reference-completeness or performance claim.

The deterministic integration fixture tests independent reference counts,
same-line candidate separation, non-ASCII preceding text, excluded target/
files, saved candidate expansion without backend/rg, semantic-only delta input
counts, explicit rg failure with null count, preservation of a warm backend after
rg-only failure, and bounded candidate capture with explicit omissions.

## Coverage/freshness acceptance — 2026-09-08

Added optional structured scope explanations and input-verification counts with
bounded issue examples. Tests cover equal hashes, modified and missing inputs,
unobserved inputs without false change claims, incomplete baselines, mismatching
captured content, and issue-example limits. Schema tests include the nested
capture-time verification inside saved reference deltas.

Old-packet compatibility is tested by omitting the additive fields and verifying
the original serialization/integrity behavior. A genuine packet from the earlier
text-candidates trial also replayed successfully: three matching input hashes,
zero changes, current comparison match, backend synchronization still unverified.
No backend or rg was needed for replay. Full make verify passed: 71 Rust tests
and 43 Python/MCP tests, formatting, Clippy, Cargo check and schema parsing.

## Coverage/freshness review repairs

Full `make verify`: 71 Rust tests and 43 Python/MCP tests pass. Existing integration
tests now cover text-only failed/timed_out/output_limited delta eligibility,
rejection of semantic failures/omissions, actual rg candidate-limit packets,
capture-change path retention with current-match replay, directory replacement,
budget-prioritized code output, and per-request reuse flags on save/parse errors.
Session schema validation includes error responses with an observed reuse.

The original review repro packets also pass the intended checks: semantic delta
becomes observed_comparable, synthetic capture history retains src/lib.rs,
directory replacement becomes stale, and budget=1000 returns one code item in
3651 bytes (previously zero items with an exceeded budget). These are local
regressions, not a new Hakorune field evaluation. No reuse guarantee was broadened
for late-discovered include files or unreadable inputs.

## Hakorune large-workspace reuse

The large scan profile completed all 5136 Rust/Cargo inputs on Hakorune cf458a28.
With the locked dependency cache prepared, the same foreground backend served
OBJ/EXE investigations in 226.401s / 2.663s; second backend_reused=true, both
completed/source_stable and backend health ok. See
[large-workspace-scan-v1.md](large-workspace-scan-v1.md) for exact scope and limits.
Full gate now passes 73 Rust tests and 44 Python/MCP tests.
