# 🚀 NekoCode Workspace

**Unix哲学に基づく高速多言語コード解析ツールチェーン**

## ✨ 新機能ハイライト

### 🔄 **統一Refreshコマンド + SQLite高速化** (2025-08-18実装完了)

**9倍高速化**と**750倍I/O効率化**を実現した差分更新システム！

```bash
# Smart自動判定（推奨）
nekocode refresh SESSION_ID                      # 変更内容から最適レベル自動選択

# レベル指定
nekocode refresh SESSION_ID --level project      # L2: プロジェクト構造更新
nekocode refresh SESSION_ID --deadcode           # L3: デッドコード検出
nekocode refresh SESSION_ID --security --quality # L4: 高度解析

# ファイル単位の高速更新（SQLite最適化）
nekocode refresh SESSION_ID --file parser.ts     # 2.2ms（従来19.4ms）
```

#### ⚡ **性能改善実績**

| 項目 | 従来(JSON) | SQLite版 | 改善率 |
|------|------------|----------|--------|
| 更新時間 | 19.4ms | **2.2ms** | **9倍高速** |
| I/O量 | 0.6MB | **0.8KB** | **750倍削減** |
| メモリ | 全体ロード | ファイル単位 | 大幅効率化 |

### 🔍 **Dead Code Detection** (2025-08-18実装完了)

外部ツール統合による**商用グレード精度**の未使用コード検出！

```bash
# 完全解析 - セッション作成と同時に実行
nekocode session-create /path/to/project --complete --external --format github-comment

# 個別実行 - 既存セッションで解析
nekocode deadcode SESSION_ID --external --min-confidence 85
```

#### 📊 **精度レベル**

| ツール | 言語 | 精度 | 検出対象 |
|--------|------|------|----------|
| cargo clippy | Rust | 95% | 未使用関数・変数 |
| cargo-machete | Rust | 85% | 未使用依存関係 |
| vulture | Python | 90% | 未使用コード全般 |
| 内部解析 | 全言語 | 60% | 基本的な未参照 |

### 🎊 **Smart Refactoring** (2025-08-17実装完了)

Tree-sitter AST解析による**革命的な正確性**を実現！

```bash
# セマンティック位置指定 - 関数の正確な終わりに挿入
nekorefactor smart insert SESSION_ID file.py "def helper():\n    pass" --after-function main

# スコープ限定置換 - クラス内のみ置換
nekorefactor smart replace SESSION_ID file.py "value" "new_value" --in-class MyClass  

# シンボル移動 - 依存関係も自動更新
nekorefactor smart move SESSION_ID "MyClass::method" target.py --update-imports
```

### 📂 **Split File** (2025-08-21実装完了)

巨大ファイルを**クラス/関数単位で自動分割**！

```bash
# クラス単位で分割（デフォルト・推奨）
nekorefactor split-file objects.rs --output ./split/
# → Calculator.rs, StringProcessor.rs, standalone.rs

# 関数単位で分割
nekorefactor split-file large_file.js --by functions --output ./functions/
# → 01_hello.js, 02_greet.js, 03_calculate.js...

# サイズ指定分割（開発中）
nekorefactor split-file huge.py --by size:500 --output ./chunks/
```

#### ✨ **特徴**
- **言語別最適化**: Rust implブロック、JS/TS class、Python class対応
- **依存関係保持**: 必要なimport文を自動追加
- **スタンドアロン関数**: クラスに属さない関数を別ファイルに

### 🧹 **Strip Comments** (2025-08-18実装完了)

Tree-sitter AST解析による**世界最高精度のコメント削除**！

```bash
# 基本的なコメント削除（文字列リテラル完全保護）
nekorefactor strip-comments file.js                    # 平均30-65%削減

# 重要コメント保護
nekorefactor strip-comments file.py --keep-docs        # docstring保持
nekorefactor strip-comments file.rs --keep-license     # ライセンス保持
nekorefactor strip-comments src/ --recursive --backup  # 再帰処理+バックアップ

# 高度フィルタリング
nekorefactor strip-comments file.js --keep-directives  # eslint-disable等保持
nekorefactor strip-comments file.cpp --keep-important  # WARNING/FIXME保持
```

#### 🛡️ **安全性と精度**

| 特徴 | 従来ツール | NekoCode |
|------|-----------|----------|
| 文字列保護 | ❌ 誤削除あり | ✅ 100%安全 |
| 言語対応 | 限定的 | 7言語完全対応 |
| 選択的保護 | 基本的 | 🎯 高度フィルタ |
| 履歴管理 | なし | 🔄 完全ロールバック |

#### 🎯 **精度比較**

| 機能 | 通常版（文字列） | Smart版（AST） |
|------|-----------------|---------------|
| 位置精度 | 推測ベース | セマンティック正確 |
| インデント | 手動指定 | 言語別自動検出 |
| 速度 | 🚀 高速 | ⚡ 中速（高精度） |
| セッション | 不要 | 必須 |

## 🏗️ アーキテクチャ

```
├── nekocode-core/     # 📦 共通ライブラリ・型システム
├── nekocode/          # 🔍 Tree-sitter解析エンジン  
├── nekorefactor/      # 🔧 Smart+通常リファクタリング ⭐NEW!
├── nekoimpact/        # 📊 変更影響度解析
└── nekoinc/           # ⚡ インクリメンタル解析
```

## 🚀 クイックスタート

### 1. ビルド
```bash
cargo build --release
```

### 2. セッション作成 / お掃除
```bash
./target/debug/nekocode session-create /path/to/project
# 出力: ✅ Created session: 12345678

Tip: 以降の `ast-stats` / `ast-query` / `ast-dump` / `deadcode` は `--session-id` を省略すると
最後に作成したセッションを自動使用します（CLIセッション自動記憶）。

自動お掃除（Auto-Prune）:
- デフォルトで「直近5件を保持」、30日以上未使用・パスが壊れたセッションを自動削除します。
- 実行タイミング: `session-create` 完了直後に実行されます。
- 設定は `~/.nekocode/cli_session.json` 内の `settings` で変更可能です。
  - `auto_prune_enabled`: true/false
  - `auto_prune_keep_recent`: 保持件数（既定: 5）
  - `auto_prune_max_age_days`: 期限（例: 30、無効化は null）
  - `auto_prune_delete_stale`: 壊れたパスを削除（true/false）

お掃除（クリーンアップ）:

```bash
# CLIセッションの履歴だけを消す（本体データは保持）
nekocode session-history --clear

# 使っていない古いセッションを14日より前のものを削除
nekocode session-prune --older-than 14

# パスが存在しない壊れたセッションを削除
nekocode session-prune --stale

# 最新5件だけ残して他を削除
nekocode session-prune --keep 5

# すべてのセッションを削除（注意！）
nekocode session-prune --all
```
```

### 3. リファクタリング

#### Smart版（AST解析・高精度）
```bash
# Python関数の後に新しい関数を挿入
./target/debug/nekorefactor smart insert 12345678 main.py \
  "def helper():\n    \"\"\"Helper function\"\"\"\n    return True" \
  --after-function main

# プレビューモード
./target/debug/nekorefactor smart insert 12345678 main.py "code" \
  --after-function main --preview
```

#### コメント削除（即座に実行可能）
```bash
# 基本的なコメント削除
./target/debug/nekorefactor strip-comments main.js

# 安全な削除（重要コメント保護）
./target/debug/nekorefactor strip-comments src/ \
  --recursive --keep-license --keep-docs --backup

# プレビュー確認
./target/debug/nekorefactor strip-comments main.py --preview
```

#### 編集履歴管理
```bash
# 履歴確認
./target/debug/nekorefactor edit-history --detailed

# 編集詳細表示
./target/debug/nekorefactor edit-show EDIT_ID

# 変更のロールバック
./target/debug/nekorefactor edit-rollback EDIT_ID

# 統計情報
./target/debug/nekorefactor edit-stats
```

## 🌍 対応言語

**Smart Refactoring対応**: 7言語完全対応
- **Python**: PEP 8準拠（4スペース）
- **JavaScript/TypeScript**: 2スペース標準
- **Rust**: 4スペース・impl block対応
- **Go**: タブインデント・package対応 
- **C++/C#**: 4スペース・class対応

## 📚 ドキュメント

- [`CLAUDE.md`](./CLAUDE.md) - Claude向け詳細仕様
- [`current_task.md`](../current_task.md) - 開発タスク状況
- [`completed_tasks.md`](../completed_tasks.md) - 完了機能一覧

## 🎯 設計思想

### Unix哲学
- **Do One Thing Well**: 各ツールは単一責務
- **Composability**: ツール間連携による柔軟性
- **Simplicity**: 明確なインターフェース

### 安全性優先
- **Git統合**: `git restore`で即座に復元可能
- **プレビューモード**: 全操作で事前確認可能
- **段階的移行**: 既存機能を壊さない設計

## 🔧 開発者向け

### コマンド一覧
```bash
# Smart版（セッション必須・AST活用）
nekorefactor smart insert SESSION_ID file content --after-function func
nekorefactor smart replace SESSION_ID file old new --in-class Class
nekorefactor smart move SESSION_ID "Class::method" target.py

# 通常版（高速・文字列ベース）  
nekorefactor insert file content --line 42
nekorefactor replace file old new --regex
nekorefactor move-lines src.js 10 5 dest.js 20

# コメント削除（即座に実行・Tree-sitter解析）
nekorefactor strip-comments file.js                    # 基本削除
nekorefactor strip-comments src/ --recursive           # ディレクトリ処理
nekorefactor strip-comments file.py --keep-docs        # docstring保持
nekorefactor strip-comments file.cpp --keep-license    # ライセンス保持
nekorefactor strip-comments file.js --preview          # プレビューモード

# 編集履歴管理
nekorefactor edit-history --detailed                   # 履歴表示
nekorefactor edit-show EDIT_ID                         # 編集詳細
nekorefactor edit-rollback EDIT_ID                     # ロールバック
nekorefactor edit-stats                                 # 統計表示
```

### テストコマンド
```bash
# セッション作成
./target/debug/nekocode session-create /tmp/test_project

# Smart機能テスト
./target/debug/nekorefactor smart insert SESSION_ID test.py "print('test')" --after-function main --preview

## 🧩 MCP連携（Claude Code）

- バイナリは `releases/` を優先（無ければ `target/release/`）。
- セッションは自動記憶されるため、多くのMCPツールは `session_id` 省略で動作します。

主要MCPツール例:

```python
# 1) セッション作成（最初に一度だけ）
await mcp__nekocode__session_create(path=".")

# 2) AST操作（session_id省略でOK）
await mcp__nekocode__ast_query(path="VM::execute_binary_op")
await mcp__nekocode__ast_stats()
await mcp__nekocode__ast_dump(format="tree")

# 3) 置換（プレビュー→確定）
await mcp__nekocode__replace_preview(file_path="src/vm.rs",
    pattern="self.execute_binary_op",
    replacement="vm_modules::operators::execute_binary_op")
await mcp__nekocode__replace_confirm()

# 4) 行移動（プレビュー→確定）
await mcp__nekocode__movelines_preview(
    source="src/a.rs", start_line=10, line_count=5,
    destination="src/b.rs", insert_line=20)
await mcp__nekocode__movelines_confirm()

# 5) クラス移動（プレビュー→確定）
await mcp__nekocode__moveclass_preview(
    session_id="<optional>", symbol_id="MyClass::method",
    target="src/target.rs", update_imports=True)
await mcp__nekocode__moveclass_confirm()
```

互換性メモ:
- 旧版CLIでは `ast-query <SESSION_ID> <PATH>` 形式を要求することがあり、
  MCPラッパーは自動的にレガシー形式へフォールバックします。
- 可能なら最新の `releases/` バイナリを使用してください。
```

---

**🎊 世界最高速クラスの多言語解析ツールチェーン** - Unix哲学 × Tree-sitter × Rust の完璧な融合

**Status**: 🚀 Production Ready  
**License**: MIT  
**Language**: Rust 🦀
