# Repository layout during the Rust-first migration

The repository is intentionally kept recoverable while the new contract is
being validated. Existing code and uncommitted work are not deleted or moved
under a broad archive operation.

## Canonical path

The current Rust-first implementation lives under `nekocode-workspace/`:

- `nekocode-core/src/rust_context.rs`: Cargo/Git/diagnostic context model
- `nekocode-core/tests/`: Rust fixture and integration coverage
- `nekocode/src/cli.rs` and `nekocode/src/main.rs`: `index` and `context` CLI
- `mcp-nekocode-server/mcp_server_rust_first.py`: read-only stdio MCP adapter
- `scripts/update_rust_first_release.sh`: canonical CLI-only release staging
- `Dockerfile.rust-first`: prebuilt CLI + minimal MCP image
- `.github/workflows/rust-first-mvp.yml`: focused verification workflow

Use the nested workspace for development and CI:

```bash
cd nekocode-workspace
cargo test -p nekocode-core
cargo check -p nekocode
```

## Legacy boundary

The root single-package Cargo project and the remaining five-binary features
are retained as legacy history for now. Their dead-code, multi-language,
refactor, split, watch, and impact claims are not part of the Rust-first MVP.
They may be archived to a tag or separate branch after the working tree is
committed and a release owner confirms that no downstream path still depends
on them.

This avoids mixing an irreversible archive operation with the accuracy work.
The migration gate is: Rust fixtures pass, the JSON schema is stable, and the
CLI smoke tests pass on a clean checkout.

Current status: Stage A (logical archive and canonical workspace selection) is
complete. The dependency audit is recorded in [`LEGACY_DEPENDENCY_AUDIT.md`](LEGACY_DEPENDENCY_AUDIT.md).
Stage B's clean-checkout gate, Rust-first MCP smoke, the legacy recovery tag
`legacy-2026-pre-rust-first`, manual-only isolation of old build/release/PR
workflows, and the canonical CLI-only release/prebuilt-Docker path are complete.
Five-binary setup/build, legacy Docker, and root cargo-deny/CodeQL checks still
need switching before Stage C.
Stage C (physical move) follows the root/MCP/distribution migration.
