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

現在の `index` / `context` はCargo構造、package features、rustc/cargoのtoolchain provenance、Git変更を返す。`--diagnostics`指定時は`cargo check --message-format=json`の診断を一次情報として追加する。予算に収まらない診断・変更は`omitted_diagnostics` / `omitted_changed_files`と`evidence: incomplete`で示す。シンボル参照やbreaking-change判定はまだ出力せず、JSONの`limitations`に明示する。

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
