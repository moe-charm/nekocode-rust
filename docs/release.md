# Release procedure

Status: accepted for the Rust-first binary release, 2026-08-24.

The current distribution target is a GitHub/tag binary release plus the
matching Docker image. It is not a tested multi-platform matrix and it does
not imply a crates.io publication. A crates.io release requires a separate
`cargo package`/registry gate. At present the CLI depends on the unpublished
workspace `nekocode-core`, so the current release procedure intentionally does
not claim crates.io support.

## Version policy

- `nekocode` and `nekocode-core` use the product release version (`1.2.0` for
  the first Rust-first release).
- The stdio MCP gateway is an independent adapter and uses its own semver
  (`0.2.0` currently). Its transport version is not a second artifact or
  contract version; the shared `snapshot-v1` and `context-v1` contracts remain
  the compatibility boundary.
- A product release must document both versions when they are distributed
  together.

## Preconditions

1. The release commit is on `master` and the worktree is clean.
2. The exact commit has a successful GitHub Actions run.
3. The release tag resolves to that exact commit.
4. `make verify` passes locally with the locked workspace.

## Binary staging

From the repository root, after creating the tag:

```bash
scripts/update_rust_first_release.sh \
  --tag v1.2.0 \
  --output dist/v1.2.0
```

The script requires a clean worktree and emits:

- `nekocode` — the host-target release binary;
- `nekocode.sha256` — a SHA-256 checksum for that binary;
- `nekocode.provenance.json` — commit, tag, binary version, toolchain
  versions, checksum, and generation time.

Verify the staged artifact before uploading it:

```bash
(cd dist/v1.2.0 && sha256sum --check nekocode.sha256)
python3 -m json.tool dist/v1.2.0/nekocode.provenance.json >/dev/null
```

The provenance timestamp is informational and must not be used as an artifact
identity. The binary checksum and tag/commit relationship are the release
evidence. Do not publish an untested target matrix or describe compiler
execution as sandboxed; compiler observations remain trusted-workspace only.
