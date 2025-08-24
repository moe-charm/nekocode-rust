# 🚀 NekoCode MCP クイックスタート

## 📌 **新しい統一導線（2025-08-24）**

### **1分で始める！**

```python
# Step 1: 必ずこれから始める（ファイルもディレクトリもOK）
mcp__nekocode__analyze_start("/path/to/anything")

# Step 2: あとは全部自動！セッションID不要！
mcp__nekocode__stats()           # 統計表示
mcp__nekocode__deadcode(limit=10) # デッドコード検出
mcp__nekocode__ast_stats()        # AST統計
mcp__nekocode__ast_query("MyClass") # シンボル検索
```

## 🎯 **重要なポイント**

1. **analyze_start が全ての始まり**
   - ファイルもディレクトリも同じコマンド
   - セッションIDは自動で記憶される
   - 次回から自動で使用

2. **セッションID管理不要**
   - `~/.nekocode/mcp_session.json` に自動保存
   - 明示的に指定することも可能

3. **トークン保護済み**
   - deadcodeはデフォルト20件制限
   - 大きすぎる結果は自動切り詰め

## 📝 **よくある使い方**

### **プロジェクト解析**
```python
# 1. プロジェクト全体を解析開始
mcp__nekocode__analyze_start("~/my_project/")

# 2. 統計を見る
mcp__nekocode__stats()

# 3. デッドコード検出（少なめに）
mcp__nekocode__deadcode(limit=10)
```

### **単一ファイル解析**
```python
# 1. ファイルを解析開始
mcp__nekocode__analyze_start("main.py")

# 2. AST構造を見る
mcp__nekocode__ast_dump(limit=30)

# 3. 特定シンボルを検索
mcp__nekocode__ast_query("handle_request")
```

## ⚠️ **注意事項**

- **最初に必ず analyze_start を実行**
- セッションがない状態で他のコマンドを実行するとエラー
- エラーメッセージ: "まず analyze_start を実行してください"

## 📚 **詳細ドキュメント**

- [MCP_USAGE_GUIDE.md](MCP_USAGE_GUIDE.md) - 詳細な使用方法
- [README.md](README.md) - 全機能一覧
- [TEST_SETUP.md](TEST_SETUP.md) - セットアップ手順