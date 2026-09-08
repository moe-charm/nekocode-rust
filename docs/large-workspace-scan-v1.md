# Large-workspace scan and reuse diagnostics

Implemented design (2026-09-08): address the local Hakorune trial at cf458a28.
The checkout has 5086 Rust files and 33467 directory entries; existing limits
4096 inputs / 32768 entries prevent complete observation even with ample time.
Offline dependency availability is a separate backend-readiness condition.

Expose an explicit live `--scan-profile large` / JSON `scan_profile: "large"`.
Default remains 4096 files / 32768 entries / 64 MiB total / 8 MiB per input.
Large permits 16384 files / 262144 entries / 256 MiB total / 8 MiB per input.
These are finite limits, not permission to reuse incomplete evidence. Saved
packets record the chosen limits and replay with that profile; legacy packets
use default limits. The packet input-count validation bound must accommodate
large inventories plus separately captured files; source-text and packet-byte
limits remain unchanged.

Freshness adds optional `scans` with baseline/current inventory statistics:
profile/limits, examined entries, hashed files/bytes, completeness and bounded
issue examples with reason/path and omitted count. Examples include entry/file/
byte limits, unreadable directories/files, symlinks and late-discovered inputs.
Counters describe the inventory walk; separately captured source files can add
freshness inputs without increasing the inventory hash counters. Unknown remains
unknown. Old packets missing diagnostics remain readable and are not re-signed.

Session responses add a `reuse` report distinguishing this request's acquisition
(reused, fresh backend, or no observation), reasons, and whether the resulting
backend was retained for the next request. Parse/validation failures must not
leak the previous request's report. Missing cache, changed inputs/settings,
incomplete scans, dead backend and unsuccessful observation are distinct.
Changing scan profile invalidates reuse. Source/freshness/backend gates remain.
Late-discovered inputs still prevent retention; no forced reuse or silent retry.

Validation: small bounded scan fixtures, schema/legacy replay regression,
complete-vs-incomplete session cases, changed-profile invalidation, then real
Hakorune sequential queries. Prepare its existing generated lockfile's Cargo
cache with `cargo fetch --locked`, without changing dependency selections,
source, manifest or toolchain. Report actual commit/settings, timings, reuse and
remaining limitations. Never claim large-workspace success from synthetic tests.

## Real Hakorune validation

At cf458a28e135178364d873008ae77c12908da61f, after locked dependency fetch,
the local debug build queried OBJ then EXE entrypoints in one foreground session
with scan_profile=large, timeout_seconds=300 and text_candidates=true:

| Query | Seconds | backend_reused | Retained | Status |
| --- | ---: | --- | --- | --- |
| try_compile_published_view_object | 226.401 | false | true | completed |
| emit_published_view_exe | 2.663 | true | true | completed |

Both inventories completed: 5136 inputs, 33468 entries, 38121077 hashed bytes,
zero scan issues; all hashes matched and backend health was ok. Semantic refs
were 5/6; unconfirmed text candidates 1/3. The known assert! at
published_native_array_c_tests.rs:68 remained a text candidate. Queries differ,
so this is a reuse observation, not a general speedup benchmark. Backend semantic
generation remains unverified; complete file scans do not prove caller-zero.
Hakorune source/manifests/toolchain were not edited, git status remained clean,
and stdin EOF stopped the session with exit 0. Only dependency cache preparation
was needed after the previous trial had generated Cargo.lock.

Full verification: 73 Rust tests / 44 Python-MCP tests, formatting, Clippy and
schema checks passed. Legacy 873c4f45 packets replay with null baseline scan report
and explicit current default-scan limits. Changing the profile or incomplete
inputs still prevents reuse; tests exercise those paths independently.
