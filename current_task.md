# 🚀 NekoCode エコシステム分離計画

## 📋 現在の状況 (2025-08-16)

### 🎯 解決すべき問題
- **NekoCodeが機能詰め込みすぎ**でモノリシック化
- リファクタリング機能追加でさらに複雑化の懸念
- バイナリサイズが大きく起動が遅い（15MB）
- 各機能が密結合で独立して進化できない

### 💡 解決策：Unix哲学に基づく分離
「1つのことをうまくやる」ツール群に分割

## 🎯 実行ファイル構成（5種類）

### 1. **nekocode** - コア解析エンジン
- **役割**: プロジェクト解析とセッション管理
- **主要機能**:
  - `session-create`: プロジェクト全体を解析してセッション作成
  - `session-update`: インクリメンタル更新
  - `analyze`: 単発解析（セッションなし）
  - `ast-dump`, `ast-query`, `ast-stats`: AST操作
- **依存**: Tree-sitter全言語パーサー（重い）
- **サイズ目標**: 15MB

### 2. **nekorefactor** - リファクタリング専用
- **役割**: コードの構造的な変更
- **主要機能**:
  - `move-function`: 関数を別ファイルに移動
  - `move-struct`: 構造体と実装を移動
  - `extract-module`: モジュール抽出
  - `split-file`: ファイル分割
- **依存**: nekocodeのセッションを読むだけ（軽い）
- **サイズ目標**: 3MB

### 3. **nekoimpact** - 影響分析専用
- **役割**: 変更の影響範囲分析
- **主要機能**:
  - `analyze`: 影響分析実行
  - `--compare-ref`: Git履歴との比較
  - `--format`: 出力形式（plain/json/github-comment）
  - 循環依存検出、複雑度変化測定
- **依存**: Git連携、セッション読み込み
- **サイズ目標**: 2MB

### 4. **nekowatch** - ファイル監視専用
- **役割**: リアルタイム変更検出
- **主要機能**:
  - `start`: 監視開始
  - `stop`: 監視停止
  - `--trigger-update`: 変更時にsession-update実行
- **依存**: notify-rs（ファイルシステム監視）
- **サイズ目標**: 1MB

### 5. **nekomcp** - MCP統合ゲートウェイ
- **役割**: Claude Code統合
- **主要機能**:
  - 全ツールへの統一インターフェース
  - MCPプロトコル実装
  - 各バイナリをサブプロセスで実行
- **依存**: 各ツールをコマンドラインで呼び出し
- **サイズ目標**: 1MB

## 📁 ディレクトリ構造

```
nekocode-rust/
├── Cargo.toml                 # ワークスペース定義
├── nekocode-core/             # 共通ライブラリ
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── session.rs         # セッション管理
│       ├── ast.rs             # AST構造体定義
│       ├── types.rs           # 共通型定義
│       └── config.rs          # 設定管理
│
├── nekocode/                  # 解析エンジン
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── analyzers/         # 各言語アナライザー
│       │   ├── javascript/
│       │   ├── rust/
│       │   ├── python/
│       │   └── ...
│       ├── commands/
│       └── core/
│
├── nekorefactor/              # リファクタリング
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── move_function.rs
│       ├── move_struct.rs
│       ├── extract_module.rs
│       └── build_verify.rs   # ビルド確認
│
├── nekoimpact/                # 影響分析
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── git_integration.rs
│       ├── impact_analyzer.rs
│       └── risk_assessment.rs
│
├── nekowatch/                 # ファイル監視
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── watcher.rs
│
└── nekomcp/                   # MCP統合
    ├── Cargo.toml
    └── src/
        ├── main.rs
        └── server.rs
```

## 🔄 実装計画

### Phase 1: 基盤構築（✅ 完了 2025-08-16）
- [x] 詳細設計完了（Task先生による分析）
- [x] nekocode-workspace/ディレクトリ作成
- [x] Workspace Cargo.toml作成
- [x] nekocode-core/Cargo.toml作成
- [x] nekocode-core/src/lib.rs基本構造
- [x] nekocode-core/src/types.rs（共通型定義）
- [x] nekocode-core/src/error.rs（エラー型統一）
- [x] nekocode-core/src/session.rs（セッション管理）
- [x] nekocode-core/src/config.rs（設定管理）
- [x] nekocode-core/src/io.rs（ファイルI/O）
- [x] nekocode-core/src/memory.rs（メモリ管理）
- [x] nekocode-core/src/traits.rs（共通トレイト）
- [x] 全バイナリのスタブ作成とビルド成功

### Phase 2: 個別バイナリ実装（実施中 2025-08-16）
- [x] nekoimpact実装完了（影響分析ツール）
  - impact.rs: 影響分析コア機能
  - analyzer.rs: 分析オプション
  - cli.rs: CLIインターフェース
  - main.rs: メインエントリポイント
  - ビルド成功確認済み
- [x] nekorefactor実装完了（リファクタリングツール）
  - preview.rs: プレビュー管理システム
  - replace.rs: テキスト置換機能
  - moveclass.rs: クラス/関数移動機能
  - cli.rs: 豊富なCLIコマンド
  - main.rs: 統合エントリポイント
  - ビルド成功確認済み
- [x] nekoinc実装完了（インクリメンタル解析・Watch機能）
  - incremental.rs: 変更検出エンジン
  - watch.rs: ファイル監視システム（tokio::sync::Mutex使用）
  - cli.rs: 多機能CLIコマンド
  - main.rs: 統合エントリポイント
  - ビルド成功確認済み
- [ ] nekocode実装（Tree-sitter解析）
- [ ] 依存関係解析
- [ ] use文自動生成
- [ ] ビルド検証機能

### Phase 3: 既存機能の移行（1週間）
- [ ] analyze-impact → nekoimpact
- [ ] watch機能 → nekowatch
- [ ] 各機能のテスト

### Phase 4: MCP統合（3日）
- [ ] nekomcp実装
- [ ] 統一インターフェース
- [ ] Claude Code設定更新

### Phase 5: 最適化（3日）
- [ ] バイナリサイズ最適化
- [ ] 起動速度改善
- [ ] ドキュメント更新

## 🎯 成功基準

### 機能面
- [ ] 各ツールが独立して動作
- [ ] セッション共有が正常に機能
- [ ] nekorefactorでmove-functionが動作
- [ ] ビルド成功を保証

### パフォーマンス
- [ ] nekorefactor起動時間 < 100ms
- [ ] move-function実行 < 1秒
- [ ] バイナリサイズ合計 < 25MB（現在の15MBから分離）

### 開発効率
- [ ] 各ツールを独立してテスト可能
- [ ] 機能追加が他ツールに影響しない
- [ ] 新規開発者が理解しやすい

## 📝 使用例

```bash
# Step 1: 解析してセッション作成
$ nekocode session-create ./my-rust-project
Session created: abc123

# Step 2: リファクタリング実行
$ nekorefactor move-function abc123 process_data src/lib.rs src/processors/data.rs
✅ Moved function 'process_data'
✅ Added necessary imports
✅ Build successful!

# Step 3: 影響分析
$ nekoimpact analyze abc123 --compare-ref main
⚠️ 3 breaking changes detected

# Step 4: 監視開始
$ nekowatch start abc123 --trigger-update
👀 Watching for changes...
```

## 🚀 期待される効果

1. **保守性向上**: 各ツールが単一責任
2. **パフォーマンス改善**: 必要な機能だけロード
3. **開発速度向上**: 並行開発可能
4. **ユーザビリティ**: 必要なツールだけインストール
5. **拡張性**: 新ツール追加が容易

## ⚠️ 注意事項

- セッションフォーマットの後方互換性維持
- 既存のMCP設定との互換性確保
- ドキュメント・READMEの全面更新必要

---
**更新日時**: 2025-08-16 13:15:00  
**現在の焦点**: Phase 2実施中 - 個別バイナリ実装  
**ステータス**: 🚀 **3/5バイナリ完成！残り2つ実装中**

## 🎉 達成事項
- **nekocode-core共通ライブラリ** 完成
- **5つのバイナリ構造** 確立
- **nekoimpact** 完全実装・ビルド成功
  - 影響分析機能
  - セッション比較
  - 3つの出力形式（plain/json/github-comment）
- **nekorefactor** 完全実装・ビルド成功
  - プレビュー管理システム
  - テキスト置換（regex対応）
  - クラス/関数移動
  - 行移動・ファイル分割機能
- **nekoinc** 完全実装・ビルド成功
  - インクリメンタル変更検出
  - ファイルWatch機能（notify使用）
  - 比較・エクスポート機能
  - 非同期処理対応（tokio::sync::Mutex）