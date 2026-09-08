# 🦀 NekoCode

Coverage and freshness: JSON now includes `coverage` explanations and
`freshness.verification` input-hash comparisons; `--format summary` displays them
for humans. [Interpretation and limits](docs/coverage-freshness-v1.md).
指定条件・未確認範囲と、入力ハッシュの一致／変更／未観測を分けて表示します。

Optional text comparison: add `--text-candidates` to a live investigation to
see `rg` matches absent from semantic references as **unconfirmed candidates**.
Comments, strings and other symbols may match; these are not verified callers.
[Scope and limits](docs/text-candidates-v1.md).
`rg`との差分を未確認候補として表示できます。参照件数とは別扱いです。

Explicit repeated investigations: `nekocode context PATH --session` keeps a
foreground backend between JSON-line requests while tracked inputs and settings
remain unchanged. See [session usage and limits](docs/symbol-session-v1.md).
CLIセッションで連続調査時のbackendを再利用できます。編集後は再起動します。
既存MCP呼出しの自動再利用は、まだ対応していません。

## Rust code investigation for AI / AIのためのRustコード調査

[English](#english) · [日本語](#日本語)

**2026-09-08 redesign:** NekoCode supports per-function investigation and
saved follow-up reads through an external rust-analyzer LSP backend. The first
implementation follows [Symbol context v1](docs/symbol-context-v1.md). Existing
snapshot and Git context remain supported; the new mode has its own contract.

**再設計:** 関数やコード位置を指定し、定義・利用箇所・型・テスト候補を
根拠付きで取得し、必要な続きを読めるツールです。初版は調査と
追加取得まで対応します。既存snapshot/Git差分機能も維持しています。

NekoCode helps AI and human developers investigate Rust code. It packages
rust-analyzer definitions, references, contracts and test candidates with source
excerpts, then lets callers expand saved results. Cargo metadata, Git changes
and optional `cargo check`/Clippy diagnostics remain available. Both workflows
return bounded JSON or a human-readable summary of the same evidence.

The Rust-first binary release procedure is documented in
[docs/release.md](docs/release.md). The CLI/core product version and the MCP
adapter version are independent; the versioned `snapshot-v1`, `context-v1`, `symbol-context-v1` and `symbol-delta-v1`
artifacts are the compatibility boundary.

NekoCode does not replace `rustc`, Cargo, or rust-analyzer. Its value is the
selection of relevant evidence, saved follow-up reads, reproducible snapshots,
Git/diagnostic comparisons and explicit budget handling around those tools.

## English

### Current contract

- **Rust-first:** the supported MVP target is a Cargo workspace.
- **Two canonical commands:** `snapshot` and `context`.
- **Versioned artifacts:** `snapshot-v1`, Git `context-v1`, `symbol-context-v1`, and `symbol-delta-v1`.
- **Read-only:** no source editing, hidden database, or automatic commit/push.
- **No unmeasured accuracy claims:** NekoCode does not claim independent
  dead-code, reference, type, or breaking-change accuracy.
- The former multi-language/session/refactoring implementation is archived
  outside `main`; it is not part of this contract.

### Investigate a function and read more

Install the external backend once with `rustup component add rust-analyzer
rust-src`, or select an installed executable with
`NEKOCODE_RUST_ANALYZER_PATH`. From `nekocode-workspace`:

```bash
cargo run --locked -q -p nekocode -- context . --symbol build_context \
  --save-packet /tmp/nekocode-investigation.json

# If ambiguous, first rerun with a returned --at FILE:LINE:COLUMN.
# Then use an item ID / next_cursor from the selected investigation.
cargo run --locked -q -p nekocode -- context . \
  --packet /tmp/nekocode-investigation.json --item ITEM_ID
cargo run --locked -q -p nekocode -- context . \
  --packet /tmp/nekocode-investigation.json --cursor CURSOR
```

An ambiguous name returns concrete candidates; choose one with
`--at path/to/file.rs:42:5` (1-based Unicode positions). A line-only position
selects a containing declaration using the backend outline. Add `--format summary` for terminal reading. Symbol modes return
`symbol-context-v1`; ordinary Git requests still return `context-v1`.

Initial investigation starts rust-analyzer and may run Cargo metadata.
Build scripts/proc macros are disabled unless `--allow-build-scripts` is
explicitly supplied for a trusted workspace. The compiled CLI replays saved
results without Cargo or rust-analyzer, and reports changed inputs as stale.
The development `cargo run` wrapper (and MCP Cargo fallback) still needs Cargo;
use `cargo install --path nekocode-workspace/nekocode --locked` from the repo
root and invoke `nekocode` directly for backend-free replay. See
[the complete mode contract](docs/symbol-context-v1.md) for query status,
backend synchronization limits, budgets and continuation.

### Compare references before and after a change

Save a selected investigation before the change and another after it, then run:

```sh
nekocode context --packet /tmp/after.json --compare-packet /tmp/before.json
nekocode context --packet /tmp/after.json --compare-packet /tmp/before.json --format summary
```

This returns `symbol-delta-v1`: added, removed, matched and unresolved reference
observations, with evidence and handles into the original packets. It compares
full captures even when their first pages were small. Neither a watcher nor
Git/Cargo/rust-analyzer is needed for the comparison itself. Different capture
conditions return `not_comparable`, never an invented deletion count. Duplicate
or edited anchors require review; this is not a complete impact or rename analysis.
See [the comparison contract](docs/symbol-delta-v1.md) for scope and paging.

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
  --diagnostics --all-features --excerpt-lines 8
```

`compare_ref` describes a Git change set; it does not recreate the compiler
result of an older commit. A diagnostic delta is reported only when the saved
and current toolchain, features, and targets are compatible. The result keeps
`added`, `resolved`, and `persisting` diagnostics separate from Git changes.
The JSON delta preserves repeated error/warning observations as a multiset;
the human summary condenses identical fingerprints and labels its counts as
unique. Auxiliary rustc note/help messages remain in the diagnostic run but do
not inflate the delta. External baseline paths are redacted in public output.
In Git mode, `--all-features` is accepted only together with `--diagnostics`;
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

Snapshot/Git responses report their contract version, `evidence`, execution policy,
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

`nekocode_snapshot` and `nekocode_context` are the only public tools in this gateway. It uses
argument-vector execution and keeps logs out of stdout. Snapshot/Git responses
redact path metadata; symbol responses retain the workspace and packet paths
needed for navigation. Source excerpts remain verbatim. See
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
python3 -m pip install -r requirements-dev.txt
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

NekoCodeは、AIと人間がRustコードを調べるためのツールです。関数を指定すると、
rust-analyzer由来の定義・参照・型情報・テスト候補をコード付きで返し、
保存した結果から必要な続きを取得できます。Cargo構造・Git差分・compiler診断を
まとめる既存機能も使えます。出力は根拠付きJSONまたは人向けサマリーです。

Rustの意味解析を独自に再実装するものではありません。正しさの一次情報は
Cargo、`rustc`、`cargo check`、rust-analyzer、Gitです。NekoCode固有の役割は、
調査に必要な根拠を選び、予算内で返し、必要な続きを保存結果から読めるようにすることです。

現行MVPの契約は次の通りです。

- 対象はRust/Cargo workspaceを優先する。
- 正規CLIは`snapshot`と`context`の2コマンド。
- 外部契約は`snapshot-v1`、Git用`context-v1`、関数調査用`symbol-context-v1`、参照比較用`symbol-delta-v1`。
- ソース編集、隠しDB、自動commit/pushは行わない。
- dead code・参照・型・breaking changeの独自精度や、未測定の精度パーセントは主張しない。
- 旧多言語・session・refactor等の実装はmainからarchive済みで、現行契約外。

### 関数を調べて、必要な続きを読む

外部backendの準備は `rustup component add rust-analyzer rust-src` で行えます。
別の実行ファイルを使う場合は `NEKOCODE_RUST_ANALYZER_PATH` で指定します。
`nekocode-workspace` から次のように使います。

```bash
cargo run --locked -q -p nekocode -- context . --symbol build_context \
  --save-packet /tmp/nekocode-investigation.json

# 同名候補が返ったら、まず候補の --at FILE:LINE:COLUMN で調べ直す
# 選択後の応答に含まれる項目IDで、その項目のコードを展開
cargo run --locked -q -p nekocode -- context . \
  --packet /tmp/nekocode-investigation.json --item ITEM_ID

# 応答のnext_cursorで残りの項目を取得
cargo run --locked -q -p nekocode -- context . \
  --packet /tmp/nekocode-investigation.json --cursor CURSOR
```

同名候補や再export候補が返った場合は、候補の位置を`--at FILE:LINE:COLUMN`で
指定します。行番号だけなら `--at src/example.rs:42` で囲む宣言を選べます。
`--format summary` で人向け表示へ切り替えられます。初回は定義・参照箇所・
関連テスト候補などをコード付きで取得し、保存後はbackendを起動せず追加取得します。
編集で入力が変わった場合は古い結果として示します。上の開発用`cargo run`はCargoを
起動します。Cargoなしで続きを読むには、repo rootで
`cargo install --path nekocode-workspace/nekocode --locked`して`nekocode`を直接使います。
build script/proc macroを
準備する場合だけtrusted workspaceで `--allow-build-scripts` を指定します。

このmodeは新しい `symbol-context-v1` を返します。既存のGit差分modeは維持します。
完全な呼出しグラフや削除安全性の保証は行いません。

### 変更前後で、見つかった参照を比較する

変更前と変更後に対象を調べ、それぞれ保存してから比較します。常駐は不要です。

```sh
nekocode context --packet /tmp/after.json --compare-packet /tmp/before.json --format summary
```

追加・消失・一致・要確認の参照を、コードと元の調査結果への項目ID付きで返します。
単なる行番号のずれでは増減を作らず、対応が曖昧なら要確認にします。調査条件の違いや
不完全な結果は比較不能として理由を返します。比較時にCargoやrust-analyzerは起動しません。
これは保存時点どうしの比較で、現在のソースに対する安全性や参照の網羅性の保証ではありません。
[詳しい仕様](docs/symbol-delta-v1.md)に制限と続きを読む方法を記載しています。

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
  --baseline /tmp/nekocode-baseline.json --diagnostics --all-features
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
絶対パスは公開出力でredactします。Git modeの`context --all-features`は`--diagnostics`との
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
Rustの意味解析は外部backendが担います。shellを経由せずにCLIを呼び出し、
snapshot/Gitのパス情報はredactします。関数調査は追加取得に必要なworkspace・保存先の
パスを保持し、ソース抜粋は原文のまま返します。詳細は
[`mcp-nekocode-server/README_RUST_FIRST.md`](mcp-nekocode-server/README_RUST_FIRST.md)
を参照してください。

ローカルCodex向けの最初のSkillは
[`skills/nekocode-rust-context/SKILL.md`](skills/nekocode-rust-context/SKILL.md)です。
呼び出し順序・停止条件・根拠の提示だけを定義し、解析器や別の実行経路は追加しません。

### 開発・テスト

```bash
python3 -m pip install -r requirements-dev.txt
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

Large Rust workspaces can opt into `context PATH --at FILE:LINE --scan-profile large`
(or JSON `scan_profile: "large"`). Input scan limits/reasons and session reuse
reports are documented in [large-workspace scan diagnostics](docs/large-workspace-scan-v1.md).
Incomplete freshness never enables forced backend reuse.

[Symlink input scope](docs/symlink-input-scope-v1.md) explains which links are
verified, outside the Rust/Cargo input boundary, or unresolved. Git tracking
status and `.venv` directory names do not decide exclusion.
