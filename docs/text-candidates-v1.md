# Unconfirmed textual candidates

Opt in with `context PATH --at FILE:LINE --text-candidates`, or
`{"at":"FILE:LINE","text_candidates":true}` in a foreground session/MCP live
investigation. Existing one-shot invocations remain unchanged by default.

After successful semantic reference collection, NekoCode runs bounded `rg`
fixed-string whole-word search for the selected definition's identifier. Matches
not overlapping a captured definition/reference become items with relation
`unconfirmed_text_candidate`, backend `ripgrep`. These are text matches, NOT
additional semantic references: comments, strings, inactive code and different
symbols can match. Existing reference query counts and saved reference deltas
continue counting only semantic `reference` items. Saved packets can page and
expand candidates without restarting rg or the backend.

Scope: workspace-local *.rs files, including hidden/ignored Rust source, excluding
.git, target and .nekocode directories; no followed symlinks. `--no-config` and
`--no-ignore` prevent user ignore/config rules from silently narrowing that scope.
Non-Rust files, external dependencies, alternate spellings and indirect/FFI calls
are outside this text scan. No matches is not proof of no callers.

The initial version requires one captured definition with an extractable ordinary
identifier and complete retained semantic references. Ambiguity, unsupported
identifier spelling or incomplete references yields a `not_applicable` detail
on an unsupported `rg/text_candidates` query. Missing rg, scan timeout (10 seconds),
output bounds (1 MiB stdout/64 KiB stderr), source mismatch and candidate limits
are reported explicitly; failure is not a zero count. Up to 64 candidates fit
within the existing overall 256-item capture cap. Display limits and continuation
remain independent of capture limits. Text comparison uses path plus overlapping
Unicode source ranges, not just matching line numbers.

Text scanning is additional to the backend observation timeout, so total CLI
wall time can grow. Toggling this option does not reconfigure rust-analyzer.
Candidate capture remains subject to the existing source/freshness bounds.

An rg-only failure does not poison a healthy reusable backend. The combined
artifact may be partial while the semantic backend is retained, provided the
semantic observation and source-freshness checks independently allow reuse.

Candidate excerpts are capped at 4096 UTF-8 bytes with an explicit truncated
flag; the saved source remains expandable under the existing packet budget.
Ripgrep must be available on PATH; the independent rust-analyzer bundle does
not itself install ripgrep.

Text-only failure, timeout or candidate omissions can make the combined artifact
partial/timed_out, but do not alone prevent saved semantic reference comparison.
Comparisons still require complete semantic queries, no other capture omissions,
stable source inputs and a healthy ready backend. Candidate-limit query counts
are null (not zero); retained candidates and omitted counts remain explicit.
`totals.observed` spans all evidence relations, including text candidates; use
`textDocument/references.result_count` for the semantic reference count.
