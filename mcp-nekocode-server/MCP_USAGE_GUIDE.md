# 🚀 NekoCode MCP 統一フロー使用ガイド

## 📌 **新しい統一導線（2025-08-24実装）**

### **すべては `analyze_start` から始まる！**

```python
# Step 1: 必ず最初にこれを実行
mcp__nekocode__analyze_start("/path/to/project")
→ セッション作成＆記憶

# Step 2: あとはセッションID不要！
mcp__nekocode__stats()           # 統計表示
mcp__nekocode__deadcode(limit=10)  # デッドコード検出
mcp__nekocode__ast_stats()         # AST統計
mcp__nekocode__ast_query("MyClass") # AST検索
```

## 🎯 **重要な変更点**

### **Before（従来の混乱した導線）**
```python
# ファイルとディレクトリで違うコマンド
mcp__nekocode__analyze("file.py")        # ファイル用
mcp__nekocode__session_create("project/") # ディレクトリ用

# セッションIDの管理が必要
session = create_session(...)
stats(session.id)  # IDを毎回指定
```

### **After（新しい統一導線）**
```python
# すべて analyze_start で統一！
mcp__nekocode__analyze_start("anything")  # ファイルもディレクトリもOK

# セッションIDは自動管理
mcp__nekocode__stats()  # ID不要！自動で使用
```

## 📋 **利用可能なコマンド一覧**

### **1. 開始コマンド（必須）**
- `analyze_start(path)` - 🚀 すべての開始点

### **2. 解析コマンド（セッションID自動）**
- `stats()` - 📊 統計情報
- `deadcode(limit=20)` - 🔍 デッドコード検出
- `ast_stats()` - 🌳 AST統計
- `ast_query(path)` - 🔍 AST検索
- `ast_dump(format="tree", limit=50)` - 📋 AST構造表示

### **3. その他のコマンド**
- `list_languages()` - 🌍 対応言語一覧
- `memory(operation, type, name, content)` - 🧠 メモリシステム

## 💡 **実装の特徴**

### **自動セッション記憶**
- 設定ファイル: `~/.nekocode/mcp_session.json`
- 最後のセッションIDを自動保存
- 次回から自動で使用

### **トークン保護**
- deadcodeはデフォルト20件制限
- 大きすぎる出力は自動切り詰め
- 親切なメッセージで誘導

### **エラー改善**
- セッションがない場合: "まず analyze_start を実行してください"
- 使用中のセッションを明示: "📌 セッション abc123 を使用"

## 🧪 **テスト方法**

```python
# test_unified_flow.py
import asyncio
from mcp_server_nekocode import NekoCodeMCPServer

async def test():
    server = NekoCodeMCPServer()
    
    # 1. 開始
    await server.analyze_start(".")
    
    # 2. セッションID省略で各種操作
    await server.show_stats()
    await server.detect_deadcode(limit=5)
    await server.show_ast_stats()

asyncio.run(test())
```

## 🎊 **これで解決した問題**

1. **導線の混乱** → 1本に統一
2. **トークン爆死** → 自動制限
3. **セッションID管理** → 自動化
4. **エラーメッセージ** → 親切に

---

**作成日**: 2025-08-24  
**作成者**: Claude + User collaborative engineering  
**状態**: ✅ **統一導線実装完了！**