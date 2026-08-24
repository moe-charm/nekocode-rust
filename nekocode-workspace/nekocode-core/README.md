# nekocode-core

`nekocode-core` is the Rust-first semantic source of truth for NekoCode's
versioned `snapshot-v1` and `context-v1` artifacts. It delegates Rust meaning
to Cargo, rustc, and Clippy, then adds bounded Git context, comparability,
provenance, and explicit omissions.

The canonical user entry point is the `nekocode` CLI in the sibling package.
See the repository [README](../../README.md) and [Rust-first MVP contract](../../docs/RUST_FIRST_MVP.md).
