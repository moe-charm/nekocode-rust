# 🐱 NekoCode MCP Server

**多言語コード解析ツールのMCP統合版** - Claude Codeで便利に使えます！

📋 **[完全コマンドリファレンスはこちら](../docs/CLI_MCP_REFERENCE.md)** - CLIとMCPの対応表、使用例、新機能説明

## 🚀 まずはクイックガイド（最小セット）

Claudeからは、次の最小ツールだけ覚えればOKです（session_idは省略で自動使用）:

- セッション作成: `mcp__nekocode__session_create(path=".")`
- 構造アウトライン: `mcp__nekocode__ast_outline(limit=200)`
- AST検索: `mcp__nekocode__ast_query(path="MyClass::myMethod")`
- AST統計/ダンプ: `mcp__nekocode__ast_stats()` / `mcp__nekocode__ast_dump(format="tree", limit=1000)`
- 変更（直接実行）: `mcp__nekocode__replace` / `insert` / `movelines` / `moveclass`

ポイント:
- Bash直叩きよりMCPツール優先で。セッションIDは省略可（直近セッションを自動使用）
- 旧CLIに当たっても、MCP側で互換フォールバックします（--session-id → 位置引数）

---

## 🚀 特徴

- **🎮 セッション機能**: 一度解析すれば、その後の操作は超高速（3ms）！
- **🌳 AST Revolution**: リアルタイム構文解析（JavaScript/TypeScript）- ゼロコストAST構築
- **高速解析**: 効率的なコード解析エンジン
- **C++特化機能**: 循環依存検出、依存グラフ、最適化提案
- **多言語対応**: JavaScript, TypeScript, C++, C, Python, C#, Go, Rust（全8言語対応！）
- **日本語対応**: 日本語でも利用可能
- **Claude Code統合**: `mcp__nekocode__*` ツールとして利用

## 📦 インストール

### 1. 前提条件
- Python 3.8+
- NekoCode バイナリ (`nekocode_ai`) がビルド済み

### 2. 依存関係確認
```bash
# 標準ライブラリのみ使用 - 特別なインストール不要！
python3 --version  # Python 3.8+ 必要
```

### 3. Claude Code設定
`claude_desktop_config.json` に追加（推奨は実装完了版の `mcp_server_real.py` を使用）:
```json
{
  "mcpServers": {
    "nekocode": {
      "command": "python3",
      "args": ["/絶対パス/nekocode-rust-clean/mcp-nekocode-server/mcp_server_real.py"],
      "env": {
        "NEKOCODE_BINARY_PATH": "/絶対パス/nekocode-rust-clean/nekocode-workspace/target/release/nekocode",
        "NEKOREFACTOR_BINARY_PATH": "/絶対パス/nekocode-rust-clean/nekocode-workspace/target/release/nekorefactor"
      }
    }
  }
}
```
**注意**: 
- スクリプト名: `mcp_server_real.py`（MCP実装・ツール群・トークン制限最適化入り）
- バイナリ名: `nekocode`（五分割ワークスペース版）。旧`nekocode_ai`はレガシー互換。

**詳細な設定方法は `TEST_SETUP.md` を参照してください**

参考（オプション: シンプル版サーバーを使う場合）
```json
{
  "mcpServers": {
    "nekocode": {
      "command": "python3",
      "args": ["/絶対パス/nekocode-rust-clean/mcp-nekocode-server/mcp_server_nekocode.py"],
      "env": {
        "NEKOCODE_BINARY_PATH": "/絶対パス/nekocode-rust-clean/nekocode-workspace/target/debug/nekocode"
      }
    }
  }
}
```

### 4. 設定ファイル（オプション） - **NEW! 統合設定システム**
NekoCodeバイナリと同じディレクトリに `nekocode_config.json` を配置することで動作をカスタマイズ可能：

```json
{
  "memory": {
    "edit_history": {
      "max_size_mb": 10,
      "min_files_keep": 10
    },
    "edit_previews": {
      "max_size_mb": 5
    }
  },
  "token_limits": {
    "ast_dump_max": 8000,
    "summary_threshold": 1000,
    "allow_force_output": true
  },
  "file_watching": {
    "debounce_ms": 500,
    "max_events_per_second": 1000,
    "exclude_patterns": [".git", "node_modules", "target"],
    "include_extensions": ["js", "ts", "py", "rs"],
    "include_important_files": ["Makefile", "Dockerfile", "LICENSE"]
  },
  "analysis": {
    "included_extensions": [".js", ".ts", ".py", ".rs"],
    "excluded_patterns": ["node_modules", ".git", "target"],
    "include_test_files": false
  }
}
```

**🎯 統合設定システム:**
- `token_limits`: AST出力のトークン制限、MCP最適化
- `file_watching`: **NEW!** ファイル監視設定（拡張子なしファイル対応）
- `analysis`: 解析対象設定（拡張子・パターン）
- `memory`: Memory System設定（サイズ制限・履歴管理）

## 🛠️ 利用可能なツール

### 🚀 **必ず最初に実行するコマンド**
- `mcp__nekocode__analyze_start` - **⭐ すべてはここから始まる！** ファイルもディレクトリも同じコマンドで開始

### 🎮 セッション機能（analyze_start後に使用）
- `mcp__nekocode__stats` - 📊 統計情報（セッションID自動使用）
- `mcp__nekocode__deadcode` - 🔍 デッドコード検出（limit指定推奨）
- `mcp__nekocode__ast_stats` - 🌳 AST統計情報
- `mcp__nekocode__ast_query` - 🔍 AST検索
- `mcp__nekocode__ast_dump` - 📋 AST構造表示

### 📝 後方互換（旧コマンド）
- `mcp__nekocode__session_create` - セッション作成（analyze_startを推奨）
- `mcp__nekocode__session_stats` - 統計情報（statsを推奨）
- `mcp__nekocode__session_update` - インクリメンタル更新
- `mcp__nekocode__find_files` - ファイル検索

### 🆕 **デッドコード検出機能** - ⚠️ **--external必須！**

**🚨 重要: 精度の違い**
- **外部ツール使用時**: 90%+ の高精度 ✅
- **外部ツールなし**: 60% の低精度（誤検出多発） ❌

**正しい使い方（CLI）:**
```bash
# ✅ 推奨: 外部ツールで高精度解析
./nekocode session-create project/ --complete --external
./nekocode deadcode SESSION_ID --external --min-confidence 85

# ツールが自動でガイド表示:
💡 Tip: External tools detected! Use --external flag for better accuracy
```

**MCPでの使い方:**
```python
# 新しい統一導線（推奨）
mcp__nekocode__analyze_start("project/")  # まずこれを実行
mcp__nekocode__deadcode(limit=20)  # セッションID自動使用

# 旧方式（後方互換）
mcp__nekocode__session_create(path="project/", complete=True, external=True)
```

**必要な外部ツール:**
- **Rust**: cargo clippy（標準搭載）
- **Python**: `pip install vulture`
- **Go**: `go install honnef.co/go/tools/cmd/staticcheck@latest`

### 🔍 File Watching System（NEW!）
**リアルタイムファイル監視 - 編集と同時に解析自動更新**
- **自動ファイル監視**: コードファイル・設定ファイル・重要ファイルを自動検出
- **スマート検出**: `Makefile`, `Dockerfile`, `LICENSE`等の拡張子なしファイル対応
- **デバウンス処理**: 500ms遅延で連続変更をまとめて処理
- **設定カスタマイズ**: `nekocode_config.json`で拡張子・パターン調整可能

**MCPツール:**
- `mcp__nekocode__watch_start` - 🔍 ファイル監視開始（リアルタイム解析）
- `mcp__nekocode__watch_status` - 📊 ファイル監視状態確認
- `mcp__nekocode__watch_stop` - 🛑 ファイル監視停止
- `mcp__nekocode__watch_stop_all` - 🛑 全ファイル監視停止
- `mcp__nekocode__watch_config` - ⚙️ ファイル監視設定表示

**使用方法（CLI）:**
```bash
# ファイル監視開始
./nekocode watch-start <session_id>

# 監視状態確認  
./nekocode watch-status

# 監視停止
./nekocode watch-stop <session_id>
```

### ✏️ コード編集機能（NEW！セッション不要の直接モードも対応！）

#### 🆕 セッション不要の直接編集（最速！）
**コマンドラインで直接実行：**
```bash
# セッション作成不要！即座に実行可能！
./nekocode_ai replace main.cpp "oldFunction" "newFunction"          # 即実行
./nekocode_ai replace-preview main.cpp "oldFunction" "newFunction"  # プレビュー
./nekocode_ai movelines src.js 10 5 dest.js 20                     # 行移動
./nekocode_ai insert file.py 42 "# New comment"                    # 挿入
```

#### 🔒 セッションベースの安全編集（2段階実行）
- `mcp__nekocode__replace_preview` - 📝 置換プレビュー（変更前後の確認）
- `mcp__nekocode__replace_confirm` - ✅ 置換実行（プレビューID指定）
- `mcp__nekocode__insert_preview` - 📝 挿入プレビュー（start/end/行番号）
- `mcp__nekocode__insert_confirm` - ✅ 挿入実行（プレビューID指定）

#### 📝 行移動機能（ファイル間でコード移動！）
- `mcp__nekocode__movelines_preview` - 📝 行移動プレビュー
- `mcp__nekocode__movelines_confirm` - ✅ 行移動実行

#### 📋 履歴管理
- `mcp__nekocode__edit_history` - 📋 編集履歴表示（最新20件）
- `mcp__nekocode__edit_show` - 🔍 編集詳細表示（ID指定）

### 🚀 MoveClass機能 - クラス・関数の自動移動（NEW！全言語対応！）
**リファクタリングの革命！クラスや関数を自動でファイル移動 + import/include自動更新**

#### 🌟 対応言語 (全8言語完全テスト済み！)
- **JavaScript/TypeScript** - React Component、関数の移動
- **Python** - クラス・関数の移動、import自動更新  
- **C++** - テンプレートクラス、namespace、#include自動処理
- **C#** - クラス移動、namespace・using自動更新
- **Go** - struct・interface移動、package/import自動処理
- **Rust** - struct・enum移動、mod・use自動更新

#### 📝 MoveClass専用ツール
**大規模テスト完了！1.4GB実プロジェクトで検証済み**
- React（Components.js → NativeComponents.js）✅
- Flask（View/MethodView → base_view.py/method_view.py）✅  
- nlohmann/json（template class → byte_container_impl.hpp）✅
- NLog（NullLogger → NullLoggerImpl.cs）✅
- Gin（LoggerConfig struct → logger_config.go）✅
- Serde（IgnoredAny → ignored_any_type.rs module）✅

### 🌳 AST Revolution - リアルタイム構文解析（NEW！）
**JavaScript/TypeScript向け高度解析機能**
- `mcp__nekocode__session_ast_stats` - 📈 AST基盤統計（ノード数・深度・複雑度）
- `mcp__nekocode__session_ast_query` - 🔍 AST検索（例: MyClass::myMethod）
- `mcp__nekocode__session_scope_analysis` - 🎯 行スコープ解析（変数・関数・クラス）
- `mcp__nekocode__session_ast_dump` - 📋 AST構造ダンプ（可視化・デバッグ）

### C++特化機能（セッション必須）
- `mcp__nekocode__include_cycles` - 🔍 循環依存検出
- `mcp__nekocode__include_graph` - 🌐 依存関係グラフ
- `mcp__nekocode__include_optimize` - ⚡ 最適化提案

### 🧠 Memory System - 時間軸Memory革命（NEW!）
**解析結果・メモの永続化機能 - Serenaにない独自機能！**
- `mcp__nekocode__memory_save` - 💾 Memory保存（auto|memo|api|cache）
- `mcp__nekocode__memory_load` - 📖 Memory読み込み
- `mcp__nekocode__memory_list` - 📋 Memory一覧表示
- `mcp__nekocode__memory_search` - 🔍 Memory検索
- `mcp__nekocode__memory_timeline` - 📅 時系列表示（7日間デフォルト）
- `mcp__nekocode__memory_stats` - 📊 Memory統計情報

**4種類のMemoryタイプ:**
- `auto` - 🤖 解析結果自動保存
- `memo` - 📝 手動メモ・計画
- `api` - 🌐 外部システム連携
- `cache` - 💾 一時保存（わからないやつもここ）

### 基本機能
- `mcp__nekocode__analyze` - 🚀 高速プロジェクト解析（単発実行）
- `mcp__nekocode__list_languages` - 🌍 サポート言語一覧

## 🎯 使用例

### 📍 推奨: セッション作成から始める

**NekoCodeの最大の特徴は「セッション機能」です！** 最初にセッションを作成することで、その後の解析が超高速（3ms）で実行できます。

```python  
# 1. 最初に必ずセッション作成（これが肝心！）
session = await mcp__nekocode__session_create("/path/to/project")
# → 初回解析は時間がかかりますが、結果はメモリに保持されます

# 2. その後の操作は超高速（3ms）！
stats = await mcp__nekocode__session_stats(session["session_id"])
complexity = await mcp__nekocode__session_complexity(session["session_id"])
files = await mcp__nekocode__find_files(session["session_id"], "*.ts")

# 3. C++特化機能も高速実行
cycles = await mcp__nekocode__include_cycles(session["session_id"])
graph = await mcp__nekocode__include_graph(session["session_id"])
```

### 基本解析（単発実行）
```python
# セッションを使わない単発解析
result = await mcp__nekocode__analyze("/path/to/project")
# → 毎回フル解析するため遅い

# 高速統計のみ取得（stats_only=True）
result = await mcp__nekocode__analyze("/path/to/project", stats_only=True)
# → 複雑度解析をスキップして高速化
```

### ✏️ コード編集機能 - 2つのモード（NEW!）

#### 🆕 Mode 1: セッション不要の直接実行（最速！）
```bash
# コマンドラインから直接実行（セッション作成不要！）
./nekocode_ai replace main.cpp "oldFunction" "newFunction"          # 即実行
./nekocode_ai replace-preview main.cpp "oldFunction" "newFunction"  # プレビュー
./nekocode_ai replace-confirm preview_123                           # 確定

./nekocode_ai movelines src.js 10 5 dest.js 20                     # 行移動
./nekocode_ai insert file.py 42 "# New comment"                    # 挿入
```

#### 🔒 Mode 2: セッションベースの安全実行
```python
# 1. 置換プレビュー（実際には変更しない）
preview = await mcp__nekocode__replace_preview(
    session_id, "src/main.cpp", "old_function", "new_function"
)
# → preview_id: "PRV_001", before/after表示

# 2. 置換実行（プレビューID指定）
result = await mcp__nekocode__replace_confirm(session_id, "PRV_001")
# → 実際にファイルを変更

# 3. 挿入プレビュー（様々な位置指定）
preview = await mcp__nekocode__insert_preview(
    session_id, "src/main.cpp", "start", "// New header comment"
)
preview = await mcp__nekocode__insert_preview(
    session_id, "src/main.cpp", "42", "// Insert at line 42"
)
preview = await mcp__nekocode__insert_preview(
    session_id, "src/main.cpp", "end", "// End of file comment"
)

# 4. 挿入実行
result = await mcp__nekocode__insert_confirm(session_id, "INS_001")

# 5. 編集履歴確認
history = await mcp__nekocode__edit_history(session_id)
# → 最新20件の編集操作履歴

# 6. 編集詳細表示
details = await mcp__nekocode__edit_show(session_id, "ED_001")
# → 特定の編集操作の詳細情報
```

### 🚀 MoveClass機能 - 実戦的なリファクタリング例（NEW!）

#### JavaScript/TypeScript例
```bash
# Reactコンポーネントの分離（実際のテスト結果）
./test_js_moveclass.sh
# → NativeClass: Components.js (651B) → NativeComponents.js (129B)
# → import文自動更新: import { NativeClass } from './NativeComponents'
```

#### Python例
```python
# Flaskクラスの分離（実際のテスト結果）
./test_python_moveclass.py
# → View: views.py → base_view.py (132行)
# → MethodView: views.py → method_view.py (67行)  
# → import自動追加: from .base_view import View
```

#### C++例  
```bash
# テンプレートクラスの移動（nlohmann/json実テスト）
./test_cpp_moveclass.sh
# → byte_container_with_subtype: 82行のテンプレートクラス移動
# → namespace・#include自動処理、include guard生成
```

#### C#例
```python
# NLogクラスの移動（実テスト結果）
./test_csharp_moveclass.py
# → NullLogger: NullLogger.cs → NullLoggerImpl.cs
# → namespace・using文完全保持
```

#### Go例
```python
# Gin構造体の移動（実テスト結果）
./test_go_moveclass.py  
# → LoggerConfig: logger.go → logger_config.go (33行)
# → 関連型（Skipper, LogFormatter）も同時移動
```

#### Rust例
```python
# Serde構造体の移動（実テスト結果）
./test_rust_moveclass.py
# → IgnoredAny: ignored_any.rs → ignored_any_type.rs (105行)
# → mod宣言・pub use re-export自動生成
```

**💡 全言語共通の特徴:**
- ✅ **自動バックアップ作成** (.bak)
- ✅ **import/include文自動更新**
- ✅ **プレビュー機能** (変更前後確認)
- ✅ **言語固有構文対応** (namespace, template, etc.)

### 🧠 Memory System - 解析結果の永続化（NEW!）
```python
# 解析結果の自動保存
await mcp__nekocode__memory_save("auto", "project_analysis_jan15", "")
# → 自動的に現在の解析結果を保存

# 手動メモの保存
await mcp__nekocode__memory_save("memo", "refactor_plan_phase2", "リファクタリング計画：core.cpp分割")

# 保存されたMemory一覧表示
result = await mcp__nekocode__memory_list("auto")  # 解析結果のみ
result = await mcp__nekocode__memory_list("memo")  # メモのみ

# Memory検索
matches = await mcp__nekocode__memory_search("complexity")
# → "complexity"を含むMemoryを検索

# 時系列表示（過去7日間の変化）
timeline = await mcp__nekocode__memory_timeline("auto", 7)

# Memory統計情報
stats = await mcp__nekocode__memory_stats()
# → 各タイプのMemory数、使用状況を表示
```

**💡 ヒント**: 複数回の操作を行う場合は、必ず最初に `analyze_start` を使ってください！セッションが自動作成・記憶されます。

## ⚙️ コマンドラインオプション

NekoCode AIが内部で使用する主なオプション：

- `--stats-only` - 高速統計のみ（複雑度解析スキップ）
- `--io-threads <N>` - 並列読み込み数（推奨:16）  
- `--cpu-threads <N>` - 解析スレッド数（デフォルト:CPU数）
- `--progress` - 進捗表示
- `--debug` - 詳細ログ表示
- `--performance` - パフォーマンス統計表示
- `--no-check` - 大規模プロジェクトの事前チェックスキップ
- `--force` - 確認なしで強制実行  
- `--check-only` - サイズチェックのみ（解析しない）

## 🔧 他のツールとの使い分け

NekoCodeは**高速解析**に特化したツールです。

- **NekoCode**: プロジェクト全体の分析、統計取得、C++依存関係解析
- **他のツール**: コード編集、詳細なシンボル検索など

**併用することで最適な開発環境が構築できます**

## 🛣️ ロードマップ

### Phase 1 (完了)
- [x] 基本MCP統合
- [x] 全機能ツール化
- [x] セッション管理
- [x] 🌳 **AST Revolution** - リアルタイム構文解析（ゼロコストAST構築）

### Phase 2 (完了！)
- [x] **🚀 MoveClass機能実装** - 全8言語対応完了！
- [x] **C#, Go, Rust完全対応** - Universal AST対応済み
- [x] **大規模リファクタリングテスト** - 1.4GB実プロジェクトで検証
- [x] **コード編集機能拡張** - プレビュー→確認パターン実装

### Phase 3 (予定)  
- [ ] セキュリティ解析機能
- [ ] AI最適化リコメンド機能  
- [ ] 自動テスト生成機能
- [ ] プロジェクト間依存関係分析

## 🐱 開発情報

- **プロジェクト**: NekoCode C++
- **作者**: NyaCore Team
- **ライセンス**: MIT
- **目標**: 使いやすく高機能な解析ツール

---
**🚀 多言語解析エンジン - 高速で便利なコード分析ツール!**
