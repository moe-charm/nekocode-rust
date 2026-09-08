# NekoCode documentation

This directory documents the Rust-first code context layer. The primary
contract is deliberately small: `snapshot` and `context`.

## Current design decisions

- [Symbol delta v1](symbol-delta-v1.md) — on-demand comparison of saved reference observations;

- [Symbol context v1](symbol-context-v1.md) — approved AI investigation and continuation;

- [Product boundary](product-boundary.md) — what NekoCode is and is not;
- [Execution trust model](execution-trust.md) — metadata-only vs opt-in Cargo execution;
- [Artifact contract](artifact-contract.md) — snapshot-v1/context-v1, comparability, and omissions;
- [Comparability Matrix v1](comparability-matrix-v1.md) — the executable
  comparison contract and one-axis golden matrix;
- [Legacy retirement](legacy-retirement.md) — completed archive and recovery refs;
- [Rust-first MVP contract](RUST_FIRST_MVP.md) — canonical architecture and acceptance gates;
- [Repository layout](REPOSITORY_LAYOUT.md) — current canonical paths;
- [Core module boundaries](module-boundaries.md) — the implemented split of the
  core evidence providers and current release blockers;
- [Legacy dependency audit](LEGACY_DEPENDENCY_AUDIT.md) — final migration result.

## Current implementation guides

- [Hakorune担当向け実用評価の指示書](hakorune-nekocode-handoff.md) — 進行中の作業を優先する試用手順;

- [Rust-first MCP gateway](../mcp-nekocode-server/README_RUST_FIRST.md);
- [Release procedure](release.md) — version policy, tag checks, checksums, and provenance;
- [Root README](../README.md);
- [Canonical workspace README](../nekocode-workspace/README.md).

Historical source and documents are available only through the recovery tag
and archive branch listed in the retirement decision.

## English / 日本語

The root README contains the current English and Japanese quick starts. CLI
and MCP names are intentionally the same in both languages:

```text
nekocode snapshot PATH
nekocode context PATH --baseline SNAPSHOT.json
```

- [Symbol context implementation validation](symbol-context-validation.md): live backend cases, saved replay and observed limitations.
