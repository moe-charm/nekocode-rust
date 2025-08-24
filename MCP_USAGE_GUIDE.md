# 🐱 NekoCode MCP Server 使い方ガイド

## 🚀 クイックスタート

NekoCode MCPサーバーは、Claude Codeから直接高速コード解析を実行できます。

### 基本的な使い方

```python
# プロジェクト解析（統計のみ・高速）
mcp__nekocode__analyze(path: "/path/to/project", stats_only: true)

# セッション作成（詳細解析）
mcp__nekocode__session_create(path: "/path/to/project")

# セッション統計
mcp__nekocode__session_stats(session_id: "SESSION_ID")
```

## ⚠️ パスの指定方法（重要）

### ✅ 推奨：絶対パスを使用

```python
# 絶対パスなら確実に動作
mcp__nekocode__analyze(
    path: "/mnt/workdisk/public_share/nyacore-workspace/nekocode-cpp-github/src",
    stats_only: true
)
```

### 📁 相対パスを使う場合

MCPサーバーは `nekocode-rust-clean/mcp-nekocode-server/` から実行されるため：

```python
# ✅ 正しい相対パス（2つ上に戻る）
mcp__nekocode__session_create(path: "../../test-workspace/test-files")

# ❌ 間違い（1つしか戻らない）
mcp__nekocode__session_create(path: "../test-workspace/test-files")
```

## 🎯 主要機能

### 1. 高速統計解析

大規模プロジェクトでも瞬時に統計情報を取得：

```python
# 1000ファイル以上でも数秒で完了
mcp__nekocode__analyze(
    path: "/large/project",
    stats_only: true  # 統計のみ（超高速）
)
```

### 2. セッション管理

プロジェクト全体を解析してセッションとして保存：

```python
# セッション作成
session_id = mcp__nekocode__session_create(
    path: "/my/project"
)

# セッション統計
mcp__nekocode__session_stats(session_id: session_id)

# AST解析
mcp__nekocode__ast_stats(session_id: session_id)
mcp__nekocode__ast_query(session_id: session_id, path: "MyClass::myMethod")
```

### 3. リファクタリング支援

```python
# プレビュー
preview_id = mcp__nekocode__replace_preview(
    file_path: "/path/to/file.js",
    pattern: "oldName",
    replacement: "newName"
)

# 実行
mcp__nekocode__replace_confirm(preview_id: preview_id)
```

### 4. インクリメンタル解析

```python
# セッション更新（変更分のみ）
mcp__nekocode__session_update(
    session_id: session_id,
    verbose: true
)
```

## 🔧 トラブルシューティング

### "Path does not exist" エラー

**原因**: 相対パスの解決位置が異なる

**解決方法**:
1. 絶対パスを使用（推奨）
2. 正しい相対パス（`../../`から始める）

### "Session not found" エラー

**原因**: セッションIDが存在しないか、異なるバイナリで作成された

**解決方法**:
```python
# セッション一覧を確認
mcp__nekocode__list_sessions()

# 新しいセッションを作成
new_session = mcp__nekocode__session_create(path: "/path")
```

### タイムアウトエラー

**原因**: 大規模プロジェクトの完全解析

**解決方法**:
```python
# stats_onlyオプションを使用
mcp__nekocode__analyze(path: "/huge/project", stats_only: true)
```

## 🚀 パフォーマンスTips

### 1. 並列処理の活用

5分割版nekocodeは`--threads`オプションで並列処理可能：
- デフォルト8スレッド
- 大規模プロジェクトで特に効果的

### 2. stats_onlyの使い分け

```python
# 初回探索：stats_onlyで高速確認
mcp__nekocode__analyze(path: path, stats_only: true)

# 詳細分析：セッション作成
mcp__nekocode__session_create(path: path)
```

### 3. セッションの再利用

```python
# 一度作成したセッションは保存される
session_id = "abc12345"

# 後から統計や解析を実行可能
mcp__nekocode__session_stats(session_id: session_id)
mcp__nekocode__ast_dump(session_id: session_id, format: "tree")
```

## 📊 対応言語

- JavaScript / TypeScript
- Python
- Rust
- C++ / C
- Go
- C#

## 🎨 活用例

### プロジェクト構造の把握

```python
# 1. 全体統計
result = mcp__nekocode__analyze(
    path: "/project",
    stats_only: true
)

# 2. 詳細セッション作成
session = mcp__nekocode__session_create(path: "/project")

# 3. AST解析で構造把握
mcp__nekocode__ast_stats(session_id: session)
```

### リファクタリング前の影響調査

```python
# 変更前に影響範囲を確認
preview = mcp__nekocode__replace_preview(
    file_path: "/src/core.js",
    pattern: "legacyFunction",
    replacement: "modernFunction"  
)

# プレビューで確認してから実行
mcp__nekocode__replace_confirm(preview_id: preview)
```

## 🔗 関連ドキュメント

- [CLAUDE.md](CLAUDE.md) - プロジェクト全体の概要
- [nekocode-workspace/README.md](nekocode-workspace/README.md) - 5分割アーキテクチャ詳細
- [mcp-nekocode-server/README.md](mcp-nekocode-server/README.md) - MCPサーバー技術詳細

---

**最終更新**: 2025-08-24
**バージョン**: 5分割版 v1.2.0（並列処理対応）