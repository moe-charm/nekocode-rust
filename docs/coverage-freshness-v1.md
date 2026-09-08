# Coverage explanation and input verification

Symbol-context-v1 responses add two optional fields without changing existing
scope/packet contracts. Old saved packets remain readable and integrity-checked;
missing fields are omitted during serialization, then explanations are derived
when replaying them.

## coverage

Each entry separates `requested`, `observed`, `verification`, and `limitation`.
Features, cfg(test), macros/build scripts, tests, and optional text scanning are
shown individually. Successful protocol queries and healthy backend readiness
are observations, not proof of semantic coverage. Proc-macro enablement does not
establish builtin/declarative macro coverage. Related tests are candidates, not
executed tests. Unknown legacy scope is kept unknown. Coverage is derived from
recorded scope/query evidence, including on packet replay; it never starts tools.

## freshness.verification

The `basis` distinguishes capture-start/end comparison from saved-packet/current
comparison. Counts distinguish matching hashes, modified hashes, missing inputs,
unreadable inputs, inputs not observed by the bounded scan, newly observed inputs,
and captured-source/current mismatches. Up to 16 issue paths are shown with an
explicit omitted count. This is bounded observation, not a complete directory
or external-dependency audit.

`verdict` is `changed`, `match`, or `unknown`. Read failures or scan omissions
alone are unknown, not proof that source changed. New observations count as
confirmed additions only if the baseline scan was complete. A matching hash
count does not imply a complete scan; start/end scan completeness is shown.
Captured-source mismatches can detect differing evidence even if the initial and
final inventory hashes happen to match.

Backend synchronization remains separately `unverified`: matching source hashes
do not prove rust-analyzer analyzed that exact generation. Existing overall
`completed` means completed investigation protocol, not exhaustive references or
verified synchronization. Plain-text summary output includes both explanations.

A replay's verification describes the current comparison; it does not
retroactively upgrade a partial or inconsistent capture. Capture-time status
may remain partial even if current hashes match. Saved reference deltas keep
showing capture-time verification, not a new filesystem check.

## Review corrections

Replay `changed_inputs` retains the union of capture-time and current confirmed
change paths (bounded to 4096); `verification` counts/issues describe only the
current comparison. Thus matching current files do not erase capture-time
inconsistency. A former input now replaced by a directory or another non-file,
non-symlink object counts as modified. Symlink changes are now checked using a separate mapping inventory; legacy
packets without mapping evidence remain unverified when links are present. Unreadable files remain unknown.

Under byte pressure, derived coverage prose and verification issue examples are
omitted before code items. Coverage omissions and `issues_omitted` explicitly
record this; verification counts, verdict and basis remain present. Replaying
with a larger budget restores the explanations.

A lockless workspace may get a new Cargo.lock during backend startup, making the
first capture partial and preventing backend retention. This is a detected input
change, not a backend failure. NekoCode does not exclude lockfiles from freshness
checks or silently prepare them before capturing the baseline.

## Inventory diagnostics

Optional `freshness.scans.baseline/current` show inventory profile, numeric limits,
examined entries, hashed files/bytes and bounded reason/path examples. These walk
counters exclude separately captured sources; `checked_inputs` can include those
additional sources. Late captures are explicitly listed as `late_input`; omissions
are counted. Legacy packet baseline diagnostics are null, not invented; current
replay diagnostics use default limits for such packets. New packets replay their
recorded profile. Under byte pressure the scan reports may be omitted before code,
with an `input_scans` omission; retry with a larger budget to read the diagnostics.
Full details: [large-workspace-scan-v1.md](large-workspace-scan-v1.md).

## Symlink scope

`scans.*.link_scope` separates verified mappings, explicitly excluded non-input
links and unverified links, with at most 16 path/target/resolved/reason examples
and an omitted count. Mapping verification is distinct from successful bounded
content hashing; scan issues/completeness still gate reuse. `verification.link_changes`
counts confirmed mapping changes separately from modified content hashes, and
issues use `symlink_changed`. Old optional-field serialization remains compatible.
See [symlink-input-scope-v1.md](symlink-input-scope-v1.md) for the exact boundary.
