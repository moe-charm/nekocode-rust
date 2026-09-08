# Explicit symbol session v1

This extends the previous no-session boundary with an explicitly owned foreground
process. `nekocode context PATH --session` reads one JSON request per stdin line
and writes one JSON response per stdout line. It never starts a detached daemon
or opens a network listener. EOF ends the process and drops the backend. Requests
are sequential; each session owns at most one backend. Clients must drain stdout
and close stdin when finished. Separate processes own separate sessions.

Example request lines:

```json
{"at":"src/runner.rs:41","timeout_seconds":300}
{"symbol":"another_function","timeout_seconds":300,"save_packet":"/tmp/another.json"}
```

Workspace is fixed by PATH. Requests use SymbolContextRequest fields and defaults;
`path` is not accepted in the stream. Replay is available via ordinary `--packet`,
not in a session. Each response has `contract_version: symbol-session-v1`,
`backend_reused`, `context` (the existing symbol-context-v1 artifact), and `error`.
A rejected request has null context and a message; the stream continues. Input
lines are limited to 64 KiB including the newline delimiter; an oversized line terminates the session.

Reuse is conservative: the previous observation must be healthy and source-stable,
its bounded input inventory complete, and root, input hashes, feature/build options,
and backend selection unchanged. Any change restarts the backend. This first
version accelerates repeated questions between edits; it does not incrementally
synchronize a changed project. Each observation has a fresh timeout (1..600).
Timeouts, backend failures, incomplete scans, and uncertain freshness discard the
backend. Resetting configuration by ending the process is always available.

Source inventory excludes external configuration/dependencies/generated inputs;
backend synchronization remains unverified. If those external inputs change,
close and reopen the session. Per-backend bounded transport limits still apply;
long sessions may exhaust those limits and a subsequent query restarts analysis.
Saved packets retain the existing evidence and replay contracts. The MCP gateway
continues to use one-shot CLI invocations in this delivery; direct CLI session
clients can already reuse analysis. Do not imply that existing MCP calls reuse it.

## Client example

Keep this subprocess open while choosing successive questions. Start the updated
CLI (older binaries do not recognize `--session`); set
`NEKOCODE_RUST_ANALYZER_PATH` to the working independent backend if needed.

```python
import json
import subprocess

with subprocess.Popen(
    ["/absolute/path/to/nekocode", "context", "/path/to/workspace", "--session"],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
) as process:
    def investigate(at):
        process.stdin.write(json.dumps({"at": at, "timeout_seconds": 300}) + "\n")
        process.stdin.flush()
        return json.loads(process.stdout.readline())

    first = investigate("src/runner.rs:41")
    # Choose another location from the first result; these example lines are
    # from the local test Nyash, not universal Hakorune source positions.
    second = investigate("src/runner.rs:36")
    print(second["backend_reused"])
    process.stdin.close()  # EOF releases the backend and exits the session.
```

Use normal `context --packet ... --cursor ...` for saved continuation; it does
not need the session to remain open. Default source-stability scans are bounded to 4096 files, 32768 entries and
64 MiB of input content. Explicit `scan_profile: "large"` raises those limits
to 16384 files, 262144 entries and 256 MiB; per-file input reads remain 8 MiB. A workspace
exceeding those limits may report unknown source freshness and will not reuse a
backend in this conservative delivery; do not equate its total line count with
these scan limits. Ending a client forcibly is not a substitute for the documented
EOF cleanup; hosts should manage their process trees on cancellation.

## Review fixes

Validation errors and packet-save errors preserve a healthy cached backend.
Before reuse, an already exited backend is replaced automatically. Failures during
an observation still return partial/timed-out evidence; backend health/readiness
are invalidated on protocol failures, and that client is not retained. A crash
racing with the liveness probe can still fail the current request; retrying then
starts a new backend. No automatic query retry hides a failed observation.

Additional source files recorded before backend acquisition (for example a
non-.rs file explicitly selected with `at`)
are reread under the existing source capture limits on the next request and
remain in the packet's freshness inputs. Unchanged additional files do not
cause a restart merely because another symbol is selected. Changes, deletion,
unreadability or exceeded capture bounds prevent reuse. This does not extend
coverage to arbitrary unobserved generated inputs.

Failure records identify the actual document notification (`didOpen` or
`didChange`). The `backend_reused` flag describes this observation's use, not
whether a backend remains cached afterward.

Files first discovered through backend locations after startup make that capture
incomplete and prevent retention. The current additional-input refresh guarantee
does not cover reuse after such a late discovery.

A save error after an observation can return `backend_reused: true` with an error
and null context. Parse, fixed-path and pre-observation validation errors return
false for that request, even after a previous reused observation. The session
schema allows both booleans on errors to represent actual observation use.

## Reuse diagnostics and large workspaces

Requests accept `scan_profile: "default" | "large"`. Changing profile starts a
fresh backend, even if the observed file hashes are equal. Response `reuse`
reports acquisition (`not_observed`, `fresh_backend`, `reused`), `reasons`,
`retained` and `retention_reasons`. Retained describes whether this observation
was kept for the next request, not whether a previously cached backend survived
a rejected request. Parse errors report not_observed; save errors preserve the
actual observation report. `no_cached_backend` can include the preceding
observation's rejection reasons. Incomplete scans and backend warnings remain
reasons to reject retention. See [scan diagnostics](large-workspace-scan-v1.md).
