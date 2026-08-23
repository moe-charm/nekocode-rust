# Legacy retirement

Status: migration policy, 2026-08-23.

The repository remains recoverable while the Rust-first contract is validated.
Keeping code recoverable does not make it a supported product surface.

## Freeze now

The following receive no new product features or accuracy claims:

- root single-package Cargo entry points;
- old five-binary install/release paths;
- multi-language analyzers;
- heuristic dead-code and impact conclusions;
- refactor, split, strip-comments, watch, security, and quality suites;
- generic symbol/reference indexing;
- additional MCP tools, prompts, resources, UI, or remote services.

The canonical docs, release scripts, and README must describe only
`snapshot`/`context` and the two-tool Rust-first gateway.

## Physical archive gate

Do not move or delete the legacy tree until all of the following are true:

1. a final tag such as `legacy-multilang-final` exists;
2. a read-only archive branch or repository is available if downstream users
   need recovery;
3. useful fixtures have moved to the canonical Rust-first test corpus;
4. no canonical crate depends on legacy code;
5. `snapshot`/`context` golden artifacts and CLI/MCP parity tests pass;
6. install, release, CI, Docker, and README paths advertise one CLI only;
7. searches for old binary names succeed only in migration documentation.

Plugin or App completion is not an archive prerequisite. Those are packaging
and presentation layers, not replacements for the legacy analyzer.

## Migration sequence

1. Keep the old implementation recoverable and mark it legacy.
2. Add the Rust-first contract and fixtures.
3. Make `snapshot` the public command; retain only a short-lived CLI `index`
   alias for existing scripts.
4. Remove old release/install/CI defaults from the canonical path.
5. Create the final legacy tag and archive branch.
6. Delete the old tree from `main` only after the gate is green.

The migration is complete when a clean checkout has one supported CLI, two
MCP tools, no hidden state, and no legacy feature claim in the primary docs.
