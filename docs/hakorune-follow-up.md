# Hakorune follow-up priorities

## Latest requested order

1. Show textual `rg` matches absent from semantic results as **unconfirmed
   candidates**, never as verified references. Implemented as opt-in
   `--text-candidates`; see [scope and limits](text-candidates-v1.md).
2. Explain requested/observed/unverified features, cfg(test), and macro scope.
   Implemented explanatory `coverage` rows; effective coverage remains unverified.
3. Improve change detection and freshness interpretation. Input hashes exist;
   `freshness.verification` now separates hash matches/changes and missing/unreadable/
   unobserved inputs; common backend generation remains unverified.
4. Strengthen backend reuse and batch investigation. Sequential CLI sessions
   exist; MCP reuse and a dedicated batch interface are not implemented.

The current repair completes the reviewed CLI reuse work first. Subsequent
feature work follows the order above. Historical rationale follows below.


User-relayed feedback, 2026-09-08. The timeout fix did not implement these items. The first follow-up now adds
explicit CLI sessions for item 1; MCP reuse and items 2/3 remain future work. NekoCode currently gathers related-code evidence; deletion of
legacy paths still requires ordinary search and project tests.

## 1. Reuse analysis for successive investigations

Initial CLI delivery: [foreground sessions](symbol-session-v1.md), with conservative
restart after observed input or configuration changes.

Allow investigation of another function in the same workspace without repeating
the full backend initialization. Existing saved packet replay only rereads
previously captured evidence; it does not provide a warm backend for new queries.

Before implementation, define an explicit backend lifecycle, workspace and
configuration identity, source-change handling, termination, and concurrent
request behavior. Measure first and subsequent queries on the same workspace;
verify that edits and feature changes cannot silently reuse obsolete evidence.
A backend session would extend the current no-hidden-session product boundary
and requires a documented design, not an implicit cache behind existing replay.

## 2. Explain coverage gaps

Expose the distinction between requested settings, backend-reported capability,
and verified coverage for macros, features/cfg, and test code. Show unknown
coverage explicitly. The reported missing `assert!` call is a concrete regression
fixture candidate, but reproducing or fixing that example would not prove
coverage of all macro calls. Enabled build scripts/proc macros and successful
LSP responses must not be presented as completeness guarantees.

Acceptance: users can identify which scopes were requested and which remain
unverified, and zero references never implies safe deletion.

## 3. Make freshness easier to interpret

The developer reported `completed` together with `freshness=unknown`. Inspect the
actual packet/response fields before diagnosing this: source-file stability and
backend synchronization are separate claims in the current contract. A completed
query does not establish either claim by itself.

Provide a clear way to check captured file hashes against current files and show
changed, matching, unreadable, and unobserved inputs separately from whether the
backend analyzed that exact source generation. Preserve unknown synchronization
unless the backend provides evidence for it. Test unchanged files, edits during
capture, edits after capture, and unreadable inputs.

## Evaluation boundary

Hakorune reported ~117 seconds for one successful investigation, five references,
one test candidate, and backend-free saved continuation. Search also found one
`assert!` call missing from those references. Search savings and productivity
improvements have not been measured. Keep raw project source in local packets;
record field reports as reported evidence rather than independently reproduced
measurements.
