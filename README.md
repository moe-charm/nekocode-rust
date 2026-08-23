# 🦀 NekoCode

## Rust-first code context layer / Rust-first コードコンテキスト層

[English](#english) · [日本語](#日本語)

NekoCode is a read-only context layer for Rust workspaces. It collects
Cargo metadata, Git changes, and optional `cargo check` diagnostics, then
returns bounded, provenance-aware JSON for humans, AI clients, and MCP.

NekoCode does not replace `rustc`, Cargo, or rust-analyzer. Its value is the
reproducible snapshot, diff context, diagnostic delta, and explicit budget
handling around those tools.

## English

### Current contract

- **Rust-first:** the supported MVP target is a Cargo workspace.
- **Two canonical commands:** `snapshot` and `context`.
- **Versioned artifacts:** external contracts are `snapshot-v1` and `context-v1`.
- **Read-only:** no source editing, hidden database, or automatic commit/push.
- **No unmeasured accuracy claims:** NekoCode does not claim independent
  dead-code, reference, type, or breaking-change accuracy.
- Other languages and the older multi-binary/refactoring surface remain
  legacy or experimental; they are not part of this contract.

### Quick start

```bash
cd nekocode-workspace

# Cargo workspace/package/target structure
cargo run -q -p nekocode -- snapshot .

# Bounded Git context for an AI or review workflow
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --budget 8000 --diagnostics

# Include staged, unstaged, and untracked working-tree markers
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --working-tree

# Read untracked file contents only when explicitly requested
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --working-tree --include-untracked-content
```

### Snapshots and diagnostic deltas

Snapshots are explicit JSON files supplied by the caller. They are not a
hidden database and are not created automatically.

```bash
# Save a reproducible Cargo/toolchain/diagnostic baseline
cargo run -q -p nekocode -- snapshot . \
  --analysis cargo-check --output /tmp/nekocode-baseline.json --all-features

# Compare the current check with that saved baseline
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 \
  --baseline /tmp/nekocode-baseline.json \
  --diagnostics --excerpt-lines 8
```

`compare_ref` describes a Git change set; it does not recreate the compiler
result of an older commit. A diagnostic delta is reported only when the saved
and current toolchain, features, and targets are compatible. The result keeps
`added`, `resolved`, and `persisting` diagnostics separate from Git changes.

### What the JSON contains

`snapshot` records Cargo workspace/package/target information, input file
digests, toolchain information, and command provenance. `context` adds the
resolved Git refs, changed files, diff hunks/patch, optional source excerpts,
optional structured compiler diagnostics, and diagnostic delta information.

Every response reports its contract version, `evidence`, execution policy,
budget fields, and `limitations`. When a request is too large, NekoCode
records what was omitted instead of silently presenting an incomplete result as
complete. Source excerpts are display context around Git hunks; they are not
symbol resolution. Untracked contents are markers by default and require
`--include-untracked-content`.

### MCP and workflow integrations

The Rust-first stdio gateway exposes the same two operations:

```bash
python3 mcp-nekocode-server/mcp_server_rust_first.py
```

`snapshot` and `context` are the only public tools in this gateway. It uses
argument-vector execution, keeps logs out of stdout, and redacts absolute
paths in responses. See
[`mcp-nekocode-server/README_RUST_FIRST.md`](mcp-nekocode-server/README_RUST_FIRST.md)
for the protocol details.

A Skill or Plugin may describe when to call the CLI/MCP tools and how
to present their evidence. It is a workflow layer, not a replacement semantic
backend. Cargo/rustc/rust-analyzer remain the sources of Rust meaning.

The first local Codex workflow is
[`skills/nekocode-rust-context/SKILL.md`](skills/nekocode-rust-context/SKILL.md).
It only defines call order, stop conditions, and evidence presentation; it
does not add another analyzer or execution path.

### Development and tests

```bash
cd nekocode-workspace
cargo test
cargo check --all-targets
cd ..
python3 -m unittest discover -s mcp-nekocode-server/tests -p 'test_*.py'
```

Plain `cargo build`, `cargo check`, and `cargo test` use only the canonical
core and CLI through Cargo `default-members`. Recoverable legacy crates require
an explicit package or `--workspace`. Rust-first fixtures, schema,
safety fixtures, and CLI/MCP smoke tests are the promotion gate for future
semantic backends.

### Repository boundaries

The canonical implementation and migration boundary are documented in
[`docs/RUST_FIRST_MVP.md`](docs/RUST_FIRST_MVP.md) and
[`docs/REPOSITORY_LAYOUT.md`](docs/REPOSITORY_LAYOUT.md). The root Cargo
package, old five-binary commands, multi-language analyzers, refactoring,
watch, impact, and legacy MCP paths are retained for recovery only. They are
not advertised as current features.

`archived/README_jp.md` is a historical document and is intentionally not the
current Japanese specification.

## 日本語

### 現在の位置づけ

NekoCodeは、RustのCargo workspaceを対象に、Cargo metadata・Git差分・
必要に応じた`cargo check`診断を読み取り、AI/MCP向けの根拠付きJSONへ
まとめる読み取り専用のコンテキスト層です。

Rustの意味解析を独自に再実装するものではありません。正しさの一次情報は
Cargo、`rustc`、`cargo check`、rust-analyzer、Gitです。NekoCode固有の役割は、
それらの結果を再現可能なsnapshot、差分、診断delta、予算制限付きの形で返すことです。

現行MVPの契約は次の通りです。

- 対象はRust/Cargo workspaceを優先する。
- 正規CLIは`snapshot`と`context`の2コマンド。
- 外部artifact契約は`snapshot-v1`と`context-v1`で、provenance・evidence・制限情報を含む。
- ソース編集、隠しDB、自動commit/pushは行わない。
- dead code・参照・型・breaking changeの独自精度や、未測定の精度パーセントは主張しない。
- 他言語と旧5バイナリ、旧refactor/watch/impact/MCPはlegacyまたはexperimentalであり、現行契約外。

### 最短手順

```bash
cd nekocode-workspace

# Cargo workspaceの構造を取得
cargo run -q -p nekocode -- snapshot .

# Git差分とcompiler診断を含む、予算制限付きコンテキスト
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --budget 8000 --diagnostics

# 未追跡ファイルは既定でmarkerだけ返す。内容を読む場合だけ明示する。
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --working-tree --include-untracked-content

# 明示的なbaselineを保存し、後でdiagnostic deltaを比較
cargo run -q -p nekocode -- snapshot . \
  --analysis cargo-check --output /tmp/nekocode-baseline.json --all-features
cargo run -q -p nekocode -- context . \
  --baseline /tmp/nekocode-baseline.json --diagnostics
```

`compare_ref`はGitの変更範囲を指定するだけで、過去commitのcompiler結果を
再現しません。diagnostic deltaは、保存したbaselineと現在のtoolchain・features・
targetsが互換する場合だけ比較し、`added`・`resolved`・`persisting`を返します。
予算超過時は省略数と`limitations`をJSONへ残します。source excerptはGit hunk周辺の
表示補助であり、symbol/reference解決ではありません。cargo-checkはtrusted
workspaceでのみ明示的に実行し、応答の`execution_policy`にoffline・環境allowlist・
専用target・未実装のOS network isolationを記録します。

### MCP・Skill・Pluginの境界

Rust-first MCP gatewayは、CLIと同じ`snapshot`/`context`だけをstdioで公開します。

```bash
python3 mcp-nekocode-server/mcp_server_rust_first.py
```

MCPは実行経路、SkillやPluginは呼び出し方・提示方法を定義するworkflow層です。
どれもRustの意味解析の代替にはしません。絶対パスは応答からredactされ、shellを
経由せずにCLIを呼び出します。詳細は
[`mcp-nekocode-server/README_RUST_FIRST.md`](mcp-nekocode-server/README_RUST_FIRST.md)
を参照してください。

ローカルCodex向けの最初のSkillは
[`skills/nekocode-rust-context/SKILL.md`](skills/nekocode-rust-context/SKILL.md)です。
呼び出し順序・停止条件・根拠の提示だけを定義し、解析器や別の実行経路は追加しません。

### 開発・テスト

```bash
cd nekocode-workspace
cargo test --workspace
cargo check --workspace --all-targets
cd ..
python3 -m unittest discover -s mcp-nekocode-server/tests -p 'test_*.py'
```

legacy crate由来のwarningが残るため、workspace全体のwarning-freeは現行契約では
ありません。Rust fixture、schema、実行安全性fixture、CLI/MCP smoke testを、
今後のsemantic backendを昇格させるゲートにします。

### 詳細

- [Rust-first MVP契約](docs/RUST_FIRST_MVP.md)
- [Repository layoutとlegacy境界](docs/REPOSITORY_LAYOUT.md)
- [Rust-first MCP gateway](mcp-nekocode-server/README_RUST_FIRST.md)
- [Canonical workspace README](nekocode-workspace/README.md)
