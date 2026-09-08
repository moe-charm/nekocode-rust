# Hakorune担当への依頼：進行中の作業でNekoCodeを一度試す

## 目的と優先順位

現在のHakorune実装を優先し、その作業に必要な関数調査を一件だけNekoCodeで
試してください。知りたいのは、通常の検索・コード読みに比べて追加検索や
見落としが減るかです。ツールを高評価にすることは目的ではありません。

評価のために現在の作業を中断したり、不要なコード変更を加えたりする必要は
ありません。次の自然な編集の区切りで実施してください。別の担当も同じ
workspaceを編集している場合、安定した観測を取れなければ今回は見送って構いません。

## 作業中の変更を守る

- 現在のbranchや作業ツリーで、評価目的のcheckout/reset/stash/cleanをしない。
- 変更前の状態に戻さない。すでに編集済みなら、現在から次の予定変更までを対象にする。
- 今回必要な作業に含まれないrename・削除・依存変更はしない。
- 保存結果・ログ・評価メモはworkspace外の一時ディレクトリに置く。
- 本体のcommitやpushの時期・単位を、この評価のために変えない。
- NekoCode初回調査はrust-analyzerとCargo metadataを起動し得る。ソースを
  意図的に編集する機能はないが、Cargo.lockやキャッシュ等は生成され得る。
  build script/proc macroの準備は既定で無効のまま試す。

## 準備

NekoCodeのこの指示書と同じcommitのCLIを使い、調査中は同じ実行ファイルを使って
ください。既にビルド済みならそのCLIをコピーすれば足ります。ソースから準備する
場合の例です。パスは実際のcheckoutと**Cargo workspace root**へ置き換えてください。

```sh
nekocode_checkout=/absolute/path/to/nekocode-rust
hakorune_workspace=/absolute/path/to/hakorune-cargo-workspace
nekocode_eval_dir=$(mktemp -d /tmp/hakorune-nekocode.XXXXXX)

cargo build --manifest-path "$nekocode_checkout/nekocode-workspace/Cargo.toml" \
  --locked --offline -p nekocode
cp "$nekocode_checkout/nekocode-workspace/target/debug/nekocode" "$nekocode_eval_dir/nekocode"
nekocode_eval_bin="$nekocode_eval_dir/nekocode"
git -C "$nekocode_checkout" rev-parse HEAD > "$nekocode_eval_dir/nekocode-commit.txt"
```

別のtarget directory設定がある場合は、実際に生成されたCLIのパスを使います。
rust-analyzerが未導入・起動不能の場合、環境修復を長く続けず理由を記録して
本来の実装へ戻ってください。この評価のためだけのtoolchain更新は不要です。

## 1. 今の実装で確認したい対象を選ぶ

現在扱っている関数を一つ選び、「何を確認したいか」を一文で残してください。
例：戻り値の利用箇所、旧経路を呼んでいる場所、確認したいテスト候補。
`LegacyCallV0`は候補例にすぎず、現在の作業に無関係なら探す必要はありません。

まず初回調査だけで役立つか試します。位置はworkspace相対、行・列は1始まりです。
以下の`src/example.rs:42:8`は実際の対象に置き換えてください。

```sh
"$nekocode_eval_bin" context "$hakorune_workspace" \
  --at src/example.rs:42:8 \
  --save-packet "$nekocode_eval_dir/before.packet.json" \
  --output "$nekocode_eval_dir/before.response.json" \
  --max-items 4 --budget 4000 --timeout-seconds 60
```

名前しか分からなければ`--at`の代わりに`--symbol NAME`を使います。同名や
再exportの候補が返った場合は、候補の位置を選んで調べ直してください。

先にstatus・queries・freshness・omissionsを確認し、コードを読んでください。
取得失敗や省略を「参照なし」と扱わないでください。返ったcursorやitem IDで
必要な根拠だけ追加取得できます。`ITEM_ID`は応答の実値に置き換えます。

```sh
"$nekocode_eval_bin" context \
  --packet "$nekocode_eval_dir/before.packet.json" --item ITEM_ID
```

## 2. 予定している変更の次の区切りで再調査する

同じworkspaceで、本来予定していた変更を進めてください。その変更の区切りで、
同じ定義を選び、`after.packet.json`と`after.response.json`へ保存します。
行が動いた場合は現在の位置を指定します。feature設定やbackendを途中で変えないで
ください。現在の作業に適した前後の区切りがなければ、初回調査の評価だけで十分です。

調査中の入力変更は`changed_during_observation`等で示されます。都合のよい結果が
出るまで繰り返さず、自然な安定区間で再取得できなければ未評価としてください。

## 3. 保存結果を比較する

```sh
"$nekocode_eval_bin" context \
  --packet "$nekocode_eval_dir/after.packet.json" \
  --compare-packet "$nekocode_eval_dir/before.packet.json" \
  --output "$nekocode_eval_dir/delta.json"

"$nekocode_eval_bin" context \
  --packet "$nekocode_eval_dir/after.packet.json" \
  --compare-packet "$nekocode_eval_dir/before.packet.json" --format summary
```

この比較はGit・Cargo・rust-analyzerを起動せず、保存時点どうしを比較します。
現在のディスクと変更前packetが異なること自体は問題ありません。ただし古いpacketを
現在のコードの証拠として使う場合は、再調査が必要です。

- `added` / `removed`：保存した参照観測の追加・消失。安全な削除の保証ではない。
- `matched`：ソースに基づいて対応づけられた参照。単純な行ずれでは増減させない。
- `unresolved`：対応が曖昧な観測。自分で確認し、断定しない。
- `not_comparable`：条件違い・不完全な取得・対象の同一性不明等。null件数は0件ではない。

現在の比較は同じworkspace rootを要求し、別worktree間の比較には対応していません。
対象の宣言行や名前が変わると比較不能になり得ます。これは失敗を隠さず記録する対象で、
本来の実装をその制約に合わせて変更する必要はありません。参照の取りこぼしや未観測の
target/環境条件もあり得ます。必要な通常の検索・レビュー・テストは引き続き行ってください。

## 返してほしい短いメモ

次の形式で、普段の実装報告に添えてください。別担当への送信は既存の連絡方法に従い、
ツールのためだけに新しい通知経路を設ける必要はありません。

```text
NekoCode評価
- 対象と、確認したかったこと：
- NekoCodeのcommit／backend版／観測条件：
- 結果：役立った／同程度／邪魔だった／評価不能
- 実際に得られた根拠と、その項目ID・場所：
- 追加で必要だった検索・コード読み・テスト：
- 見つけ損ねたもの／誤解しそうだったもの：
- 比較結果と、手元の変更に合っていたか（実施した場合）：
- おおよその実行・確認時間（通常手順を測っていなければ未測定）：
- 次に一つだけ改善するとしたら：
- ローカル保存先：
```

「productionかcompatibilityかを毎回調べ直した」「小さな関数なので普通に読んだ方が
速かった」「途中編集で比較できなかった」も有用な結果です。未測定の短縮率や精度を
作らず、コードの中身を含むpacketを公開issue等へそのまま貼り付けないでください。

仕様：[関数調査](symbol-context-v1.md)・[保存した参照の比較](symbol-delta-v1.md)。

## 大規模workspaceでの再試行と確認済みの制約（2026-09-08）

更新版は `--timeout-seconds 300` を受け付けます（範囲1〜600秒、既定60秒）。
元のeff8d17バイナリは120秒が上限なので、更新版CLIを使ってください。
MCPの場合はgatewayも更新が必要です。gatewayの子プロセス制限は660秒ですが、
MCPクライアント自身の制限が短い場合があります。

Hakorune担当から、300秒指定で約117秒後に定義・型情報・参照5件・テスト候補1件を
取得し、backend再起動なしで保存packetの続きを読めたとの報告がありました。
一方、通常検索で見つかる `assert!` 内の呼出し1件は参照一覧に出ませんでした。
この報告は解析の動作確認であり、参照の網羅性や検索時間の削減を示すものではありません。

**削除前・caller-zero確認では必ず `rg` と併用し、macro内の呼出しや間接呼出し、
Rust→C境界を確認してください。** 検索結果も単独で安全性を保証しません。
テスト候補は実行済みテストではないため、通常のプロジェクト検証も続けてください。
タイムアウトや部分結果を参照ゼロとして扱わないでください。

次の要望（解析再利用・不足範囲・鮮度確認）は、優先順と確認条件を
[次期開発項目](hakorune-follow-up.md)に記録しています。現時点では未実装です。
