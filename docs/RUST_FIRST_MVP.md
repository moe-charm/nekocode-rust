# Rust-first MVP

このリポジトリは、旧来の「多言語・多機能コード解析ツール」から、Rustの公式ツールを束ねるコード・コンテキスト層へ段階的に移行する。

## 正規の責務

NekoCodeはRustの意味解析を再実装しない。正しさの情報源は次の通り。

- Cargo metadata: workspace/package/target構造
- rustc・cargo check・Clippy: compiler/lint diagnostics
- rust-analyzer: symbol/reference/semantic operations（後続backend）
- Git: baseline/headの変更集合

NekoCode固有の価値は、これらをスナップショット化し、差分・履歴・根拠・AI向け予算に合わせて返すことにある。

## MVPコマンド

```bash
cd nekocode-workspace

# Cargo workspace構造をJSONで取得
cargo run -q -p nekocode -- index .

# Git差分を含む、制限付きAIコンテキストを取得
cargo run -q -p nekocode -- context . --compare-ref origin/main --budget 8000

# 必要なときだけworkspace全targetのcargo check診断を含める
cargo run -q -p nekocode -- context . --compare-ref origin/main --diagnostics

# 後で診断差分に使う明示JSON snapshotを保存
cargo run -q -p nekocode -- index . --snapshot /tmp/rust-baseline.json --diagnostics --all-features

# hunk周辺のsource excerptと保存済み診断のdeltaを取得
cargo run -q -p nekocode -- context . --compare-ref origin/main \
  --excerpt-lines 8 --baseline /tmp/rust-baseline.json --diagnostics --all-features
```

## Canonical release staging

旧5バイナリの`Makefile`や`update_releases.sh`とは分離し、Rust-first CLIだけを
`releases/nekocode`へ更新する。

```bash
scripts/update_rust_first_release.sh
# 既存のrelease binaryを別ディレクトリへ検査する場合
scripts/update_rust_first_release.sh --skip-build --output /tmp/nekocode-release-check
```

このスクリプトは`nekocode`だけをコピーし、`nekorefactor`等のlegacy artifactを
削除・上書きしない。自動commit/pushもしないため、成果物の確認後に明示的に
コミットする。

ルートの`make`、`nekocode-workspace/build.sh`、`build-and-deploy.sh`もこの
Rust-first stagingを既定にする。旧5-binaryのbuild/copyが必要な復旧時だけ
`make legacy-release`または各scriptの`--legacy`を明示する。`releases/setup.py
--install`と`--direct-json`もRust-first CLI/MCPを登録し、旧登録は
`--install-legacy` / `--direct-json-legacy`で分離している。

現在の `index` / `context` はCargo構造、package targets/features、入力ファイルdigest、rustc/cargoのtoolchain provenance、Git変更hunk/patchを返す。`--diagnostics`指定時は`cargo check --message-format=json`の診断を一次情報として追加する。予算に収まらないdiff・診断・変更は`omitted_*`、実測bytes/tokens、`budget_exceeded`、`evidence: incomplete`で示す。シンボル参照やbreaking-change判定はまだ出力せず、JSONの`limitations`に明示する。

## 現行契約（Phase 2.1）

Phase 2.1は、意味解析を再実装せず、同じRust-first JSON契約の上に実装済みである。

### Snapshot

`index --snapshot FILE` は、Cargo workspace snapshotと任意の診断実行結果を、再利用可能なJSONとして保存する。これは当面「永続DB」ではなく、明示的なファイルsnapshotである。schema versionは3で、snapshotには次を含める。

- schema version、workspace/package/target情報
- toolchain、Cargo.toml/Cargo.lock/rust-toolchainのdigest
- 実行コマンド、cwd、tool version、exit code
- `--diagnostics`を指定した場合のcargo check結果

snapshot ID、常駐DB、過去commitの自動再解析はまだ実装しない。書き込みは指定されたsnapshot pathだけに限定し、atomic replaceを使う。

### Source excerpts

`context` は変更hunkの前後を、明示した行数だけworkspace-relativeに抜粋する。

```json
{
  "path": "src/lib.rs",
  "start_line": 10,
  "end_line": 24,
  "content": "...",
  "source": "git-diff-hunk",
  "truncated": false
}
```

抜粋はsyntax-onlyの表示補助であり、symbol/reference解決を意味しない。budgetを超える場合は抜粋単位で省略し、`omitted_excerpts`へ記録する。

### Diagnostic delta

`context --baseline SNAPSHOT --diagnostics` は、同じtoolchain/features/targets条件で保存されたsnapshotと現在の`cargo check`結果を比較する。Gitの変更差分と診断差分は混同しない。

- `added`: 現在だけにある診断
- `resolved`: baselineだけにある診断
- `persisting`: 両方にある診断
- fingerprint: code、workspace-relative path、line、正規化message
- baseline/currentのtool provenanceと実行条件

baseline条件が異なる、診断が保存されていない、または実行に失敗した場合はdeltaを断定せず、`incomplete`と理由を返す。

`index`/`context`のJSONは、MCP gatewayからも同じ引数で利用できる。`--snapshot`、
`--baseline`はcallerが指定するパスだけを読み書きし、gatewayは応答中の絶対パスを
`<path>`へredactする。

## 証拠レベル

出力の`evidence`は信頼度の代用品ではなく、根拠の種類を表す。

- `tool-confirmed`: Cargo/Git等の外部ツールが直接返した情報
- `semantic-resolved`: 意味解析backendで解決した情報
- `syntax-only`: 構文解析だけで得た情報
- `incomplete`: 予算超過またはbackend未接続などで不完全な情報

未測定の「90%」「95%」のような数字は使わない。

## Rust昇格ゲート

新しいRust backendを追加する前に、次をfixtureと回帰テストで固定する。

- trait / impl / macro / cfg / feature
- workspace / 複数crate / tests / examples
- 同名シンボル、可視性、span、解析エラー
- false positive / false negativeと実行条件

このゲートを通過するまでは、他言語は`experimental`扱いとし、READMEの「完全対応」には含めない。

## 旧実装との境界

旧単一バイナリと旧5バイナリの機能はlegacyとして保守対象を限定する。独自dead-code判定、smart refactor、split、strip-comments、watch、security/quality/circular解析をMVPの正規導線に戻さない。

リポジトリ上のcanonical/legacy境界と、アーカイブを保留している理由は [`REPOSITORY_LAYOUT.md`](REPOSITORY_LAYOUT.md) に記載する。
