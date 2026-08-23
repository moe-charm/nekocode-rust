# NekoCode workspace

This directory is the canonical Rust-first Cargo workspace. The repository
root contains migration and recovery material; new Rust-first work starts here.

## English

The workspace provides the `nekocode` package and the shared `nekocode-core`
context model. The supported entry points are:

```bash
cargo run -q -p nekocode -- index .
cargo run -q -p nekocode -- context . --compare-ref HEAD~1 --budget 8000
```

`index` records Cargo workspace/package/target metadata. `context` adds a
bounded Git diff, optional source excerpts, and optional `cargo check`
diagnostics. The JSON schema is v3 and carries tool provenance, evidence, and
limitations. It is deliberately not an independent Rust semantic analyzer.

Useful development checks:

```bash
cargo test --workspace
cargo check --workspace --all-targets
```

The workspace still contains legacy members (`nekorefactor`, `nekoimpact`,
`nekoinc`, and others) for recovery. Their old multi-language, session,
refactoring, impact, and watch commands are not the Rust-first contract.

See the root [README](../README.md), the [Rust-first MVP contract](../docs/RUST_FIRST_MVP.md),
and the [repository layout](../docs/REPOSITORY_LAYOUT.md).

## 日本語

このディレクトリが正規のRust-first Cargo workspaceです。新しいRust-firstの
実装・テストはここを起点にします。

`nekocode-core`がCargo/Git/diagnosticのコンテキストモデルを持ち、`nekocode`
が次の2コマンドを提供します。

```bash
cargo run -q -p nekocode -- index .
cargo run -q -p nekocode -- context . --compare-ref HEAD~1 --budget 8000
```

`index`はCargo workspace・package・targetの構造を取得し、`context`は予算制限付き
Git差分、任意のsource excerpt、任意の`cargo check`診断を追加します。JSON schemaは
v3で、tool provenance・evidence・limitationsを含みます。Rustの意味解析を独自に
置き換えるものではありません。

workspace内には復旧用のlegacy member（`nekorefactor`、`nekoimpact`、`nekoinc`など）
が残っています。旧多言語・session・refactor・impact・watch機能は現行契約外です。

詳細はルートの[README](../README.md)、[Rust-first MVP契約](../docs/RUST_FIRST_MVP.md)、
[repository layout](../docs/REPOSITORY_LAYOUT.md)を参照してください。
