# Repository layout during the Rust-first migration

Status: canonical layout policy, 2026-08-23.

The repository is intentionally recoverable while the new artifact contract
and execution-trust gates are validated. The logical boundary is active now;
physical removal waits for [legacy-retirement.md](legacy-retirement.md).

## Current canonical path

The supported implementation currently lives under `nekocode-workspace/`:

- `nekocode-core/src/rust_context.rs`: Cargo/Git/diagnostic context model;
- `nekocode-core/tests/`: Rust fixtures and integration coverage;
- `nekocode/src/cli.rs` and `nekocode/src/main.rs`: canonical CLI;
- `mcp-nekocode-server/mcp_server_rust_first.py`: thin stdio adapter;
- `scripts/update_rust_first_release.sh`: CLI-only release staging;
- `Dockerfile.rust-first`: prebuilt CLI/minimal gateway image.

The four design decisions are kept at the repository root under `docs/`:

- `product-boundary.md`;
- `execution-trust.md`;
- `artifact-contract.md`;
- `legacy-retirement.md`.

The checked-in external schemas live under `schemas/`. They are contract
artifacts, not a second implementation of the core model.

Use the nested workspace for development and CI:

```bash
cd nekocode-workspace
cargo test -p nekocode-core
cargo check -p nekocode
```

The `nekocode` package has `autolib = false`: its canonical binary no longer
compiles or exposes the retained analyzer/session library. Cargo
`default-members` likewise excludes the old standalone binaries. Building a
legacy member now requires an explicit package name or `--workspace`.

## Target physical shape

The long-term shape is a small core plus adapters. The current workspace has
not yet been physically renamed to `crates/`; do not create a second parallel
workspace just to match a diagram.

```text
nekocode-workspace/
├── nekocode-core/       # canonical core (current path)
├── nekocode/            # canonical CLI (current path)
└── ...                  # recoverable legacy members
schemas/
docs/
```

The core may later be split into `snapshot/`, `context/`, `rust/`, `git/`,
`contract/`, `comparability/`, `budget/`, `provenance/`, and `execution/`
modules. It should not be split into many crates until a real dependency
boundary requires it. CLI and MCP depend on core in one direction.

## Legacy boundary

The root single-package Cargo project and remaining five-binary features are
legacy recovery material. Their dead-code, multi-language, refactor, split,
watch, and impact claims are not part of the Rust-first contract. The old
MCP/server files remain available for recovery but are not the canonical
gateway.

Legacy code may be moved to a final tag/read-only branch after:

1. snapshot/context golden artifacts and parity tests pass;
2. canonical install/release/CI paths use one CLI;
3. no canonical crate depends on legacy code;
4. old binary names appear only in migration documentation.

This avoids mixing an irreversible archive operation with contract and
execution-safety work.
