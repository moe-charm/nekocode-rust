# 🦀 NekoCode

## Rust-first code context layer / Rust-first コードコンテキスト層

[English](#english) · [日本語](#日本語)

NekoCode is a read-only context layer for Rust workspaces. It collects
Cargo metadata, Git changes, and optional `cargo check`/Clippy diagnostics, then
returns bounded, provenance-aware JSON for AI/MCP clients or a deterministic
human-readable summary of the same evidence.

The Rust-first binary release procedure is documented in
[docs/release.md](docs/release.md). The CLI/core product version and the MCP
adapter version are independent; the versioned `snapshot-v1` and `context-v1`
artifacts are the compatibility boundary.

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
- The former multi-language/session/refactoring implementation is archived
  outside `main`; it is not part of this contract.

### Quick start

```bash
cd nekocode-workspace

# Cargo workspace/package/target structure
cargo run -q -p nekocode -- snapshot .

# Bounded Git context for an AI or review workflow
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --budget 8000 --diagnostics

# Read the same evidence as a concise terminal summary
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --format summary

# Include staged, unstaged, and untracked working-tree markers
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --working-tree

# Read untracked file contents only when explicitly requested
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --working-tree --include-untracked-content
```

`PATH` may be the Cargo workspace/package root, a nested directory such as
`src`, or an existing source file. NekoCode searches upward for the nearest
`Cargo.toml`, asks Cargo for the canonical workspace root, and uses that root
consistently for Cargo and Git evidence.

### Human-readable change summary

`context` returns the versioned `context-v1` JSON artifact by default. Add
`--format summary` when a human wants a quick review of the same collected
evidence. The summary shows changed files and hunks, visible patch line counts,
compiler-diagnostic state and delta, comparability, budget use, omissions, and
limitations. `--output` writes whichever format was selected.

With `--working-tree`, NekoCode keeps staged, unstaged, and untracked changes
separate. When `--compare-ref` is present, committed revision changes are a
fourth scope. Git numstat totals are independent from retained patch text, so
the summary still reports counted `+/-` lines when a large patch is omitted.
Binary and marker-only untracked files are reported as unknown, not zero-line
changes.

The summary is deterministic presentation, not a second analysis path. It does
not invent a semantic explanation, resolve symbols, or declare breaking
impact. JSON remains the machine contract used by MCP and durable artifacts.
If the byte budget removes the whole patch body, the summary says it was
omitted instead of displaying `+0/-0` visible lines.

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
The JSON delta preserves repeated error/warning observations as a multiset;
the human summary condenses identical fingerprints and labels its counts as
unique. Auxiliary rustc note/help messages remain in the diagnostic run but do
not inflate the delta. External baseline paths are redacted in public output.
`--all-features` is accepted by `context` only together with `--diagnostics`;
otherwise the CLI returns a configuration error instead of ignoring it.

Clippy is an explicit alternative producer rather than a cargo-check alias:

```bash
# Optional Clippy snapshot (trusted workspace; default lints are observed as-is)
cargo run -q -p nekocode -- snapshot . --analysis clippy

# Optional Clippy diagnostics in a bounded context
cargo run -q -p nekocode -- context . \
  --diagnostics --diagnostic-producer clippy --budget 8000
```

Diagnostic JSON records `producer`, `profile`, and `producer_version`. Exact
deltas are computed only within the same producer/profile and compatible
toolchain/feature/target conditions; a cargo-check baseline is never silently
compared with Clippy.

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
`--include-untracked-content`. Git filenames are collected with NUL-delimited
output so UTF-8 names are preserved without Git's octal quoting. An operational
Cargo failure, timeout, or output limit produces `evidence: incomplete`, never
`tool-confirmed`. Explanatory `limitations` alone do not downgrade evidence:
the default marker-only handling of untracked files remains `tool-confirmed`
when the requested Git observation completed without omissions.

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
cargo test --locked
cargo check --locked --all-targets
cd ..
python3 -m unittest discover -s mcp-nekocode-server/tests -p 'test_*.py'
```

The Cargo workspace contains only the canonical core and CLI. The core
dependency graph contains no async runtime, SQLite, session storage, or
language parser.
Rust-first fixtures, schema,
safety fixtures, and CLI/MCP smoke tests are the promotion gate for future
semantic backends.

### Repository boundaries

The canonical implementation and completed archive are documented in
[`docs/RUST_FIRST_MVP.md`](docs/RUST_FIRST_MVP.md) and
[`docs/REPOSITORY_LAYOUT.md`](docs/REPOSITORY_LAYOUT.md). The root Cargo
Historical source is absent from `main` and recoverable from the tag and
archive branch named in the retirement decision.

## 日本語

### 現在の位置づけ

NekoCodeは、RustのCargo workspaceを対象に、Cargo metadata・Git差分・
必要に応じた`cargo check`/Clippy診断を読み取り、AI/MCP向けの根拠付きJSONまたは
同じ根拠の人向けサマリーへまとめる読み取り専用のコンテキスト層です。

Rustの意味解析を独自に再実装するものではありません。正しさの一次情報は
Cargo、`rustc`、`cargo check`、rust-analyzer、Gitです。NekoCode固有の役割は、
それらの結果を再現可能なsnapshot、差分、診断delta、予算制限付きの形で返すことです。

現行MVPの契約は次の通りです。

- 対象はRust/Cargo workspaceを優先する。
- 正規CLIは`snapshot`と`context`の2コマンド。
- 外部artifact契約は`snapshot-v1`と`context-v1`で、provenance・evidence・制限情報を含む。
- ソース編集、隠しDB、自動commit/pushは行わない。
- dead code・参照・型・breaking changeの独自精度や、未測定の精度パーセントは主張しない。
- 旧多言語・session・refactor等の実装はmainからarchive済みで、現行契約外。

### 最短手順

```bash
cd nekocode-workspace

# Cargo workspaceの構造を取得
cargo run -q -p nekocode -- snapshot .

# Git差分とcompiler診断を含む、予算制限付きコンテキスト
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --budget 8000 --diagnostics

# 同じ根拠を、人がすぐ読める短いサマリーで表示
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --format summary

# 未追跡ファイルは既定でmarkerだけ返す。内容を読む場合だけ明示する。
cargo run -q -p nekocode -- context . \
  --compare-ref HEAD~1 --working-tree --include-untracked-content

# 明示的なbaselineを保存し、後でdiagnostic deltaを比較
cargo run -q -p nekocode -- snapshot . \
  --analysis cargo-check --output /tmp/nekocode-baseline.json --all-features
cargo run -q -p nekocode -- context . \
  --baseline /tmp/nekocode-baseline.json --diagnostics
```

`PATH`にはCargo workspace/packageのrootだけでなく、`src`のような配下directoryや
既存source fileも指定できます。最も近い`Cargo.toml`を上方向へ探し、Cargoが返した
workspace rootをCargo/Git根拠の共通境界として使います。

`context`の既定出力は、versioned contractである`context-v1` JSONです。
人が差分を素早く確認するときだけ`--format summary`を指定します。サマリーには
変更ファイル・hunk・表示できたpatchの増減行数・compiler診断とdelta・比較可能性・
budget・省略・制限を表示します。`--output`には選択した形式を書き込みます。

`--working-tree`ではstaged・unstaged・untrackedを別々のscopeとして保持し、
`--compare-ref`指定時はcommit済みrevision差分も分離します。Git numstatの集計は
保持されたpatch本文に依存しないため、大きなpatchが省略されても数えられた`+/-`行を
表示できます。binaryやmarkerのみの未追跡fileを0行とは扱わず、未知として明示します。

このサマリーは同じ根拠の決定論的な表示であり、別の解析器ではありません。
意味を推測した説明、symbol解決、breaking impact判定は追加しません。MCPと保存用の
機械契約は引き続きJSONです。byte budgetによりpatch本文が全て省略された場合は、
`+0/-0`ではなくpatchが省略されたことを明示します。

`compare_ref`はGitの変更範囲を指定するだけで、過去commitのcompiler結果を
再現しません。diagnostic deltaは、保存したbaselineと現在のtoolchain・features・
targetsが互換する場合だけ比較し、`added`・`resolved`・`persisting`を返します。
JSONのdeltaは同じerror/warningが複数回観測された回数を保持し、人向けサマリーでは
同じfingerprintを1件へまとめて`unique`と明示します。rustcの補足note/helpは元の
diagnostic runへ保持しますが、delta件数には混ぜません。workspace外のbaseline
絶対パスは公開出力でredactします。`context --all-features`は`--diagnostics`との
同時指定だけを許可し、単独指定は黙って無視せずconfiguration errorにします。
予算超過時は省略数と`limitations`をJSONへ残します。source excerptはGit hunk周辺の
表示補助であり、symbol/reference解決ではありません。cargo-checkはtrusted
workspaceでのみ明示的に実行し、応答の`execution_policy`にoffline・環境allowlist・
専用target・未実装のOS network isolationを記録します。Git pathはNUL区切りで取得し、
日本語などのUTF-8 filenameを8進escapeへ変えず保持します。Cargoの実行失敗・timeout・
出力上限到達時は`evidence: incomplete`とし、`tool-confirmed`にはしません。説明用の
`limitations`が存在するだけではevidenceを格下げしません。未追跡fileをmarkerだけで
返す既定modeも、Git観測が省略なく完了すれば`tool-confirmed`です。

Clippyはcargo-checkとは別の明示producerです。必要なときだけ次のように指定します。

```bash
# default lintをそのまま観測するClippy snapshot
cargo run -q -p nekocode -- snapshot . --analysis clippy

# Clippy診断を含むcontext
cargo run -q -p nekocode -- context . \
  --diagnostics --diagnostic-producer clippy --budget 8000
```

診断JSONには`producer`・`profile`・`producer_version`を記録します。
deltaは同じproducer/profileかつtoolchain・feature・target条件が互換の場合だけ
exact multisetとして計算し、cargo-check baselineとClippyを黙って比較しません。

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
cargo test --locked
cargo check --locked --all-targets
cd ..
python3 -m unittest discover -s mcp-nekocode-server/tests -p 'test_*.py'
```

Cargo workspaceは正規coreとCLIの2 memberだけです。正規coreはasync runtime、
SQLite、session storage、言語parserへ依存しません。
Rust fixture、schema、実行安全性fixture、CLI/MCP smoke testを、今後のsemantic
backendを昇格させるゲートにします。

### 詳細

- [Rust-first MVP契約](docs/RUST_FIRST_MVP.md)
- [Repository layoutとlegacy境界](docs/REPOSITORY_LAYOUT.md)
- [Rust-first MCP gateway](mcp-nekocode-server/README_RUST_FIRST.md)
- [Canonical workspace README](nekocode-workspace/README.md)
