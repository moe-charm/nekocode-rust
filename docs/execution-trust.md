# Execution trust model

Status: accepted design decision, 2026-08-23.

NekoCode has two different operations and must not describe both as harmless
"read-only parsing".

## Analysis modes

| Mode | Default | Runs workspace code? | Purpose |
| --- | --- | --- | --- |
| `metadata-only` | Yes | No | Cargo workspace/toolchain observation |
| `cargo-check` | No, explicit opt-in | Potentially | Structured compiler diagnostics and a baseline |

Cargo can execute `build.rs`, procedural macros, compiler wrappers, and related
build configuration. `cargo check` therefore requires a trusted workspace and
is an execution operation, even when it does not produce a final binary.

NekoCode must never claim that `--offline` is an OS network sandbox. Until an
OS-level sandbox exists, provenance reports the limitation explicitly:

```text
workspace_trust: required
cargo_registry_network: policy-controlled
process_network_isolation: not-enforced
```

## Current controls and gaps

The current implementation exposes `cargo-check` only as an explicit opt-in
and records tool provenance, compiler status, structured diagnostics, and
budget omissions. The MCP subprocess adapter also applies an input limit,
redacts absolute paths, and has a wall-clock timeout. Git diff calls disable
external diff helpers; this is not an OS sandbox.

The following controls are **not yet enforced** by the core and remain release
blockers for untrusted workspaces:

- process-tree termination after a Cargo timeout;
- a dedicated target directory and environment allowlist;
- OS-level network/process isolation;
- canonical-root and symlink escape checks for every source read;
- bounded Cargo stdout/stderr before parsing;
- a sandboxed build-script/procedural-macro execution boundary.

Until those controls are implemented and tested, callers must trust the
workspace and the product must not describe `cargo-check` as sandboxed. An
unimplemented control is reported as a limitation rather than implied by a
generic “read-only” label.

## Compiler result states

Compiler errors are data, not automatically NekoCode failures:

```text
not_run
completed_clean
completed_with_diagnostics
tool_failed
timed_out
output_limited
partial
```

The CLI exit status may still be non-zero for operational failures. A Cargo
non-zero exit with a valid diagnostic stream is `completed_with_diagnostics`.

## Acceptance fixtures

The trust gate must include a build script sentinel, a procedural-macro
sentinel, a timeout fixture, output-limit coverage, an out-of-root symlink,
and a Git external-diff sentinel. Until those tests pass, the product must
require a trusted workspace and disclose the missing sandbox guarantee.
