# NekoCode workspace

This directory is the canonical Rust-first Cargo workspace. The repository
root contains migration and recovery material; new Rust-first work starts here.

## English

The workspace provides the `nekocode` package and the shared `nekocode-core`
context model. The supported entry points are:

```bash
cargo run -q -p nekocode -- snapshot .
cargo run -q -p nekocode -- context . --compare-ref HEAD~1 --budget 8000
cargo run -q -p nekocode -- context . --compare-ref HEAD~1 --format summary
cargo run -q -p nekocode -- context . --working-tree --include-untracked-content
```

`snapshot` records Cargo workspace/package/target metadata. `context` adds a
bounded Git diff, optional source excerpts, and optional `cargo check` or
Clippy diagnostics. Compiler observations are explicit opt-ins and require a
trusted workspace; the current release does not claim an OS sandbox. The
external contracts are `snapshot-v1` and `context-v1`; both carry tool
provenance, evidence, execution policy, comparability, budget, and limitations.
Untracked contents are markers unless explicitly requested. The core is
deliberately not an independent Rust semantic analyzer. JSON is the default
machine contract; `--format summary` is a deterministic human-readable view of
the same evidence and does not add semantic inference.

Useful development checks:

```bash
python3 -m pip install -r ../requirements-dev.txt
cargo test --locked
cargo check --locked --all-targets
```

The workspace contains exactly two members: the core and CLI. The retired
implementation is available only through the recovery refs documented at the
repository root.

The release baseline is Rust 1.85.0 (MSRV 1.85); CI and the Docker builder use
that pinned toolchain. Newer local toolchains may work but are not the release
reproducibility baseline.

See the root [README](../README.md), the [Rust-first MVP contract](../docs/RUST_FIRST_MVP.md),
and the [repository layout](../docs/REPOSITORY_LAYOUT.md).

## 日本語

このディレクトリが正規のRust-first Cargo workspaceです。新しいRust-firstの
実装・テストはここを起点にします。

`nekocode-core`がCargo/Git/diagnosticのコンテキストモデルを持ち、`nekocode`
が次の2コマンドを提供します。

```bash
cargo run -q -p nekocode -- snapshot .
cargo run -q -p nekocode -- context . --compare-ref HEAD~1 --budget 8000
cargo run -q -p nekocode -- context . --compare-ref HEAD~1 --format summary
```

`snapshot`はCargo workspace・package・targetの構造を取得し、`context`は予算制限付き
Git差分、任意のsource excerpt、任意の`cargo check`またはClippy診断を追加します。
compiler observationは明示指定時だけ実行し、trusted workspaceを必要とします。現行
releaseはOS sandbox済みとは表現しません。外部契約は`snapshot-v1`と`context-v1`で、
tool provenance・evidence・execution policy・比較可能性・budget・limitationsを含みます。
未追跡ファイルの内容は明示指定時だけ読み込みます。既定JSONが機械契約で、
`--format summary`は同じ根拠を人向けに読みやすく表示するだけです。意味を推測せず、
Rustの意味解析を独自に置き換えません。

workspace memberはcoreとCLIの2つだけです。退役した実装はmainに残さず、ルート文書に
記載したrecovery tag/branchからのみ取得できます。

release基準のtoolchainはRust 1.85.0（MSRV 1.85）です。CIとDocker builderは同じ
toolchainを固定します。より新しいlocal toolchainで動く場合があっても、releaseの
再現性基準にはしません。

詳細はルートの[README](../README.md)、[Rust-first MVP契約](../docs/RUST_FIRST_MVP.md)、
[repository layout](../docs/REPOSITORY_LAYOUT.md)を参照してください。
