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

## Required controls

Before exposing `cargo-check` as a supported operation, the implementation
must provide or explicitly report:

- an opt-in execution mode with metadata-only as the default;
- a timeout and process-tree termination;
- bounded stdout, stderr, diagnostics, and serialized output;
- a dedicated target directory where practical;
- a documented environment allowlist and secret redaction;
- workspace-root/path-jail checks for excerpts and artifacts;
- symlink escape rejection for files read as source context;
- Git invocation with external diff/textconv disabled;
- an execution policy and tool provenance in every observation;
- `not_run`, `tool_failed`, `timed_out`, `output_limited`, and `partial`
  states distinct from a clean compiler result.

The current implementation is allowed to be conservative: if a control is
not enforced, it must be represented as a limitation rather than described as
"sandboxed".

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
