# Legacy dependency audit — final result

Audit completed: 2026-08-23.

Before retirement, the repository had three competing dependency roots: a
root parser package, a multi-binary nested workspace, and several MCP servers.
They also controlled release binaries, Docker, CI, and checked-in session
state.

The migration established the Rust context core, two-command CLI, versioned
schemas, safety fixtures, and CLI/MCP parity first. Commit `ba71a05` then made
legacy dependencies opt-in, after which the complete recovery snapshot was
pushed as `legacy-multilang-final` and
`archive/legacy-multilang-final`.

The main branch now has one Cargo workspace with two members, one CLI binary,
one MCP server with two tools, one Docker path, and no prebuilt executables or
hidden sessions. Historical source and documentation must be consulted from
the recovery refs, not copied back into the canonical dependency graph.
