# Legacy retirement

Status: completed on `main`, 2026-08-23.

The former multi-language/session/refactor implementation is no longer part of
the main branch. It remains fully recoverable from:

- annotated tag `legacy-multilang-final`;
- archive branch `archive/legacy-multilang-final`.

## Completed gate

1. The final tag exists and was pushed.
2. The archive branch exists and was pushed.
3. Rust trait/impl/macro/cfg/feature and execution fixtures are in the
   canonical test corpus.
4. Canonical crates contain no dependency on retired code.
5. snapshot/context artifacts and CLI/MCP parity tests pass.
6. install, package, CI, Docker, README, and MCP paths expose one CLI and two
   use cases.
7. Retired binary names occur only in migration records.

## Removed from main

- the root Cargo package and parser tree;
- standalone analysis/refactor/impact/watch/MCP crates;
- session, SQLite, generic symbol, and multi-language modules;
- old MCP servers, prompts, configuration, and guides;
- prebuilt binaries and multi-binary install/release helpers;
- legacy CI, security, impact, and release workflows;
- hidden checked-in session artifacts and unsupported product claims.

No compatibility wrapper or hidden command is retained. Recovery must use the
tag or archive branch rather than reintroducing a second supported surface.
