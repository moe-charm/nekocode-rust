# Change Scope v1 evaluation

This is a maintainer reference for checking that the Skill interprets the
current `context-v1` artifact correctly. It is not a second parser or a
replacement for the Rust integration tests.

## Cases

Run the canonical CLI against a disposable Cargo Git fixture, or inspect an
artifact produced by the same command:

```text
nekocode context PATH --compare-ref HEAD --working-tree --budget 8000
```

| Case | Required observation | Required Skill behavior |
| --- | --- | --- |
| Clean working tree | four scope entries, all zero | Say there is no observed change; do not claim that semantic impact was checked. |
| One path staged and edited again | one `changed_files` entry with staged and unstaged `scope_changes` | Keep both observations; do not overwrite the staged state with `M`. |
| Rename or deletion | destination/old path and a path-associated hunk | Explain the observed Git status; do not invent a breaking-change conclusion. |
| Binary file | `line_count_status=binary`, null additions/deletions | Say line counts are unknown, not `+0/-0`. |
| Untracked marker | `line_count_status=not_read` | Say the path is present but its line count is unknown. Do not read or quote it unless content was explicitly requested. |
| Tiny byte budget | `output_limited` or omissions, retained `diff.change_scopes` | Use pre-budget totals, then state which patch/file details were omitted. Never summarize the artifact as “no changes”. |
| No first commit | `resolved_head=null`, staged scope from Git's empty-tree comparison | Continue with the explicit incomplete history state; do not require or invent `HEAD`. |

## Acceptance checklist

- `status`, `comparison_status`, `execution_policy`, `evidence`, and
  `omissions` were read before source interpretation.
- Each reported `+/-` number came from a `counted` scope or a visible patch
  explicitly labeled as such.
- `binary` and `not_read` were called unknown.
- Pre-budget totals were not confused with retained `changed_files`.
- Missing or incompatible baselines were not treated as clean diagnostic
  comparisons.
- No repository comment, source excerpt, or diagnostic text was executed as
  an instruction.
