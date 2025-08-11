# 🐱 NekoCode MCP Server - セッション不要化アップデート

## 📅 更新日: 2025-01-07

## 🔥 重要な変更点

### セッション不要コマンド（直接実行可能）

以下のコマンドは**セッションID不要**で直接実行できるようになりました：

1. **replace_preview** - 置換プレビュー
   - 旧: `session_id`, `file_path`, `pattern`, `replacement` が必要
   - 新: `file_path`, `pattern`, `replacement` のみ

2. **replace_confirm** - 置換実行
   - 旧: `session_id`, `preview_id` が必要
   - 新: `preview_id` のみ

3. **insert_preview** - 挿入プレビュー
   - 旧: `session_id`, `file_path`, `position`, `content` が必要
   - 新: `file_path`, `position`, `content` のみ

4. **insert_confirm** - 挿入実行
   - 旧: `session_id`, `preview_id` が必要
   - 新: `preview_id` のみ

5. **edit_history** - 編集履歴表示
   - 旧: `session_id` が必要
   - 新: 引数不要（グローバル履歴を表示）

## 💡 理由

- 編集操作はグローバルに`memory/edit_history/`に保存される
- セッションに依存しない直接編集コマンドが既に実装済み
- MCPインターフェースを簡潔にし、使いやすさを向上

## 📝 使用例

### Before (セッション必要)
```python
# セッション作成が必要
session = mcp__nekocode__session_create(path="src")
mcp__nekocode__replace_preview(
    session_id=session["session_id"],
    file_path="test.js",
    pattern="old",
    replacement="new"
)
```

### After (セッション不要)
```python
# 直接実行可能！
mcp__nekocode__replace_preview(
    file_path="test.js",
    pattern="old",
    replacement="new"
)
```

## ⚠️ 注意事項

- セッションベースのコマンド（stats, complexity等）は引き続きセッションIDが必要
- 編集履歴は全てのコマンド（セッション有無に関わらず）で共通保存される
- MCPサーバーの再起動が必要

## 🚀 適用方法

1. このディレクトリの`mcp_server_real.py`を使用
2. MCPサーバーを再起動
3. Claude Codeを再起動（必要に応じて）

---
**実装者**: Claude + User  
**テスト済み**: ✅ 全機能正常動作確認