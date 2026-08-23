# NekoCode documentation

This directory documents the Rust-first code context layer. The primary
contract is deliberately small: `snapshot` and `context`.

## Current design decisions

- [Product boundary](product-boundary.md) — what NekoCode is and is not;
- [Execution trust model](execution-trust.md) — metadata-only vs opt-in Cargo execution;
- [Artifact contract](artifact-contract.md) — snapshot-v1/context-v1, comparability, and omissions;
- [Legacy retirement](legacy-retirement.md) — freeze and physical-archive gates;
- [Rust-first MVP contract](RUST_FIRST_MVP.md) — canonical architecture and acceptance gates;
- [Repository layout](REPOSITORY_LAYOUT.md) — current paths and migration boundary;
- [Legacy dependency audit](LEGACY_DEPENDENCY_AUDIT.md) — audited recovery dependencies.

## Current implementation guides

- [Rust-first MCP gateway](../mcp-nekocode-server/README_RUST_FIRST.md);
- [Root README](../README.md);
- [Canonical workspace README](../nekocode-workspace/README.md).

The older usage, performance, refactoring, and multi-language documents are
historical recovery material. They are not the current product contract and do
not override the documents above.

## English / 日本語

The root README contains the current English and Japanese quick starts. CLI
and MCP names are intentionally the same in both languages:

```text
nekocode snapshot PATH
nekocode context PATH --baseline SNAPSHOT.json
```
