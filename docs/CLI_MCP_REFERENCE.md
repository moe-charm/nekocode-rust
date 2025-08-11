# 🐱 NekoCode 統合コマンドリファレンス

**CLI と MCP Server の完全ガイド** - 最新のDirect Mode対応版

最終更新: 2025-08-07 | Version: 2.2 | 🆕 move-class機能追加

---

## 📋 目次

1. [クイックリファレンス表](#クイックリファレンス表)
2. [基本解析機能](#基本解析機能)
3. [Direct Mode編集機能](#direct-mode編集機能)
4. [セッション機能](#セッション機能)  
5. [move-class機能](#move-class機能)
6. [Memory System](#memory-system)
7. [実用例](#実用例)
8. [設定とトラブルシューティング](#設定とトラブルシューティング)

---

## 🚀 クイックリファレンス表

### 基本コマンド対応表

| 機能 | CLIコマンド | MCPツール | 説明 |
|------|-------------|-----------|------|
| **解析** | `nekocode_ai analyze <path>` | `mcp__nekocode__analyze` | プロジェクト/ファイル解析 |
| **セッション作成** | `nekocode_ai session-create <path>` | `mcp__nekocode__session_create` | 対話式セッション開始 |
| **統計情報** | `nekocode_ai session-command <id> stats` | `mcp__nekocode__session_stats` | セッション統計表示 |
| **言語一覧** | `nekocode_ai languages` | `mcp__nekocode__list_languages` | サポート言語表示 |

### 🆕 Direct Mode編集コマンド（セッション不要！）

| 機能 | CLIコマンド | MCPツール | 説明 |
|------|-------------|-----------|------|
| **置換プレビュー** | `replace-preview <file> <pattern> <replacement>` | `mcp__nekocode__replace_preview` | 変更確認 |
| **置換実行** | `replace-confirm <preview_id>` | `mcp__nekocode__replace_confirm` | プレビュー適用 |
| **挿入プレビュー** | `insert-preview <file> <position> <content>` | `mcp__nekocode__insert_preview` | 挿入確認 |
| **挿入実行** | `insert-confirm <preview_id>` | `mcp__nekocode__insert_confirm` | 挿入適用 |
| **行移動プレビュー** | `movelines-preview <src> <start> <count> <dst> <pos>` | `mcp__nekocode__movelines_preview` | 移動確認 |
| **行移動実行** | `movelines-confirm <preview_id>` | `mcp__nekocode__movelines_confirm` | 移動適用 |
| **編集履歴** | `edit-history` | `mcp__nekocode__edit_history` | 最新20件表示 |

---

## 📊 基本解析機能

### CLI使用例

```bash
# 高速統計モード（推奨）
./bin/nekocode_ai analyze src/ --stats-only --io-threads 16

# 単一ファイル解析
./bin/nekocode_ai analyze main.cpp

# プログレス表示付き（大規模プロジェクト用）
./bin/nekocode_ai session-create large-project/ --progress
```

### MCP使用例（Claude Code内）

```python
# プロジェクト解析
mcp__nekocode__analyze(
    path="src/",
    stats_only=True,
    language="auto"
)

# セッション作成（詳細解析）
result = mcp__nekocode__session_create(
    path="large-project/"
)
session_id = result["session_id"]
```

### 引数説明

| 引数 | CLI形式 | MCP形式 | 説明 | デフォルト |
|------|---------|---------|------|------------|
| パス | 位置引数 | `path` | 解析対象 | 必須 |
| 統計のみ | `--stats-only` | `stats_only` | 基本統計のみ | false |
| 言語指定 | `--lang <lang>` | `language` | 言語フィルター | "auto" |
| スレッド数 | `--io-threads <n>` | - | 並列度 | CPU数 |

---

## ✏️ Direct Mode編集機能

### 特徴
- 🚀 **セッション不要** - 即座に実行可能
- 🔒 **2段階確認** - preview → confirm で安全
- 📋 **履歴管理** - 10MB容量制限（最低10ファイル保持）
- 📁 **プレビュー管理** - 5MB容量制限（自動クリーンアップ）

### 置換操作

#### CLI使用例
```bash
# プレビュー生成
./bin/nekocode_ai replace-preview main.cpp "oldFunction" "newFunction"
# 出力: {"preview_id": "preview_20250807_140016", "total_matches": 3, ...}

# プレビュー確認後、実行
./bin/nekocode_ai replace-confirm preview_20250807_140016
```

#### MCP使用例
```python
# プレビュー生成
preview = mcp__nekocode__replace_preview(
    file_path="main.cpp",
    pattern="oldFunction",
    replacement="newFunction"
)

# 実行
mcp__nekocode__replace_confirm(
    preview_id=preview["preview_id"]
)
```

### 挿入操作

#### 位置指定オプション
- `start` - ファイル先頭
- `end` - ファイル末尾
- `数値` - 行番号（1ベース）

#### CLI使用例
```bash
# 3行目に挿入
./bin/nekocode_ai insert-preview file.py 3 "# TODO: Implement this"

# ファイル末尾に追加
./bin/nekocode_ai insert-preview file.py end "// EOF marker"
```

#### MCP使用例
```python
preview = mcp__nekocode__insert_preview(
    file_path="file.py",
    position="3",
    content="# TODO: Implement this"
)
```

### 行移動操作

#### CLI使用例
```bash
# src.jsの10行目から5行をdest.jsの20行目に移動
./bin/nekocode_ai movelines-preview src.js 10 5 dest.js 20

# プレビュー確認後、実行
./bin/nekocode_ai movelines-confirm movelines_20250807_101443
```

#### MCP使用例
```python
preview = mcp__nekocode__movelines_preview(
    srcfile="src.js",
    start_line=10,
    line_count=5,
    dstfile="dest.js",
    insert_line=20
)
```

### 編集履歴管理

#### 容量制限（自動管理）
- **編集履歴**: 10MB制限、最低10ファイル保持
- **プレビュー**: 5MB制限、古いものから自動削除

#### CLI使用例
```bash
# 履歴表示（最新20件）
./bin/nekocode_ai edit-history

# 特定の編集詳細表示
./bin/nekocode_ai edit-show edit_20250807_140016
```

---

## 🎮 セッション機能

### セッションの利点
- ⚡ **超高速応答** - 一度解析すれば後は3ms
- 🧠 **状態保持** - 解析結果をメモリに保持
- 🔍 **詳細分析** - AST、複雑度、依存関係

### セッション作成と利用

#### CLI使用例
```bash
# セッション作成
./bin/nekocode_ai session-create large-project/
# 出力: Session created: session_20250807_140000

# セッションコマンド実行
./bin/nekocode_ai session-command session_20250807_140000 stats
./bin/nekocode_ai session-command session_20250807_140000 complexity
./bin/nekocode_ai session-command session_20250807_140000 find MyClass
```

#### MCP使用例
```python
# セッション作成
result = mcp__nekocode__session_create(path="large-project/")
session_id = result["session_id"]

# 統計取得（超高速3ms）
stats = mcp__nekocode__session_stats(session_id=session_id)

# C++特化機能
cycles = mcp__nekocode__include_cycles(session_id=session_id)
graph = mcp__nekocode__include_graph(session_id=session_id)
```

---

## 🔄 move-class機能

### **クラス単位の移動機能（NEW！）**

セッションモードでクラス全体を安全に移動できるにゃ！

### 特徴
- 🎯 **シンボル名指定** - 行番号不要でクラス名だけで移動
- 🔒 **セッション必須** - AST解析済みで高精度
- ⚡ **高速検出** - 事前解析データを活用
- 🛡️ **プレビュー確認** - movelines機能を内部で使用

### CLI使用例

```bash
# セッション作成
./bin/nekocode_ai session-create src/

# クラス移動（プレビュー生成）
./bin/nekocode_ai session-command SESSION_ID \
    move-class IncludeAnalyzer \
    src/core/cmd/include_commands.cpp \
    src/utils/include_analyzer.cpp

# プレビュー確認後、実行
./bin/nekocode_ai movelines-confirm PREVIEW_ID
```

### MCP使用例（Claude Code内）

```json
// まだMCPサーバーには未実装
// セッションコマンドとしてのみ利用可能
```

### 引数仕様

| 引数 | 説明 | 例 |
|------|------|----| 
| `class_name` | 移動するクラス名 | `IncludeAnalyzer` |
| `src_file` | ソースファイル | `src/core/cmd/include_commands.cpp` |
| `dst_file` | 宛先ファイル | `src/utils/include_analyzer.cpp` |

### 対応言語

| 言語 | 対応状況 | 備考 |
|------|----------|------|
| C++ | ✅ 対応 | `class`, `struct` |
| C# | ✅ 対応 | `class`（partial除く） |  
| Java | ✅ 対応 | `class`（interface除く） |
| JavaScript/TypeScript | ⚠️ 制限的 | ES6+ class のみ |
| Python | ❌ 保留 | インデント問題 |

### 制限事項（Phase 1）

- **ネストクラス非対応** - トップレベルクラスのみ
- **テンプレート特殊化非対応** - 基本形のみ
- **依存関係自動修正なし** - include文は手動で更新
- **位置指定固定** - 宛先ファイルの末尾のみ

### エラーメッセージ例

```json
{
  "error": "Class not found: NonExistentClass",
  "details": {
    "class_name": "NonExistentClass", 
    "file": "src/test.cpp",
    "hint": "Make sure the class exists in the specified file"
  },
  "available_classes": ["ExistingClass1", "ExistingClass2"]
}
```

---

## 🧠 Memory System

### 時間軸Memory革命
- 4種類のメモリータイプ（auto/memo/api/cache）
- 時系列管理と自動クリーンアップ
- 10MB容量管理（編集履歴と共通）

### CLI使用例
```bash
# 解析結果を自動保存
./bin/nekocode_ai memory save auto project_analysis_jan15

# メモを手動保存
./bin/nekocode_ai memory save memo refactor_plan "Phase 1: Split large files"

# 読み込み
./bin/nekocode_ai memory load auto project_analysis_jan15

# 時系列表示（過去7日）
./bin/nekocode_ai memory timeline auto 7

# 古いキャッシュ削除（30日以上）
./bin/nekocode_ai memory cleanup cache 30
```

---

## 💡 実用例

### 例1: 大規模リファクタリング

```bash
# 1. プロジェクト解析とセッション作成
./bin/nekocode_ai session-create src/ --progress

# 2. 複雑度の高いファイルを特定
./bin/nekocode_ai session-command SESSION_ID complexity

# 3. 関数名を一括置換（プレビュー）
./bin/nekocode_ai replace-preview src/core.cpp "calculateTotal" "computeSum"

# 4. 変更内容を確認して実行
./bin/nekocode_ai replace-confirm PREVIEW_ID

# 5. 編集履歴を確認
./bin/nekocode_ai edit-history
```

### 例2: Claude Codeでの対話的編集

```python
# MCPツールを使った対話的な編集フロー
# 1. セッション作成
session = mcp__nekocode__session_create(path="project/")

# 2. 統計確認
stats = mcp__nekocode__session_stats(session_id=session["session_id"])

# 3. 編集プレビュー
preview = mcp__nekocode__replace_preview(
    file_path="main.cpp",
    pattern="TODO",
    replacement="DONE"
)

# 4. 確認して実行
if preview["summary"]["risk_level"] == "low":
    mcp__nekocode__replace_confirm(preview_id=preview["preview_id"])
```

### 例3: クラス単位の安全な移動（NEW！）

```bash
# セッション作成
./bin/nekocode_ai session-create src/
# 出力: Session created: session_20250807_160000

# クラス移動（IncludeAnalyzer クラスを別ファイルに移動）
./bin/nekocode_ai session-command session_20250807_160000 \
    move-class IncludeAnalyzer \
    src/core/cmd/include_commands.cpp \
    src/utils/include_analyzer.cpp

# プレビュー確認（クラスの内容と移動先を確認）
# 出力にpreview_idが含まれる

# 問題なければ実行
./bin/nekocode_ai movelines-confirm movelines_20250807_160100
```

### 例4: ファイル間のコード移動（従来方式）

```bash
# utilsからhelpers.jsに関数を移動
./bin/nekocode_ai movelines-preview utils.js 100 50 helpers.js 1

# プレビューで移動内容を確認
cat memory/movelines_previews/movelines_*.json

# 問題なければ実行
./bin/nekocode_ai movelines-confirm PREVIEW_ID
```

---

## ⚙️ 設定とトラブルシューティング

### MCP Server設定

**claude_desktop_config.json**:
```json
{
  "mcpServers": {
    "nekocode": {
      "command": "python3",
      "args": ["/absolute/path/to/mcp_server_real.py"],
      "env": {
        "NEKOCODE_BINARY_PATH": "/absolute/path/to/bin/nekocode_ai"
      }
    }
  }
}
```

### よくある問題と解決策

| 問題 | 原因 | 解決策 |
|------|------|---------|
| `preview_id not found` | プレビューが期限切れ | 再度preview生成 |
| `Session not found` | セッションタイムアウト | session-create再実行 |
| `File not found` | 相対パス問題 | 絶対パスを使用 |
| 履歴が消えた | 10MB制限到達 | 正常動作（古いものから削除） |

### パフォーマンス最適化

```bash
# SSD環境（推奨）
--io-threads 16 --ssd

# HDD環境
--io-threads 1 --hdd

# 大規模プロジェクト（30,000+ファイル）
--progress --io-threads 8
```

---

## 📚 関連ドキュメント

- [USAGE.md](USAGE.md) - 基本的な使い方
- [mcp-nekocode-server/README.md](../mcp-nekocode-server/README.md) - MCP詳細
- [ARCHITECTURE.md](ARCHITECTURE.md) - 内部設計
- [CLAUDE.md](../CLAUDE.md) - プロジェクト概要

---

**🐱 NekoCode - Fast, Safe, and AI-Friendly Code Analysis**

*容量ベース履歴管理により、大規模プロジェクトでも安定動作！*