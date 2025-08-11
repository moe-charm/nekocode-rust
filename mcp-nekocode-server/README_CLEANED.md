# 🐱 NekoCode MCP Server - SESSION中心構造

**多言語コード解析ツールのMCP統合版** - Claude Codeで便利に使えます！

## 🎮 SESSION（メイン機能）

### `mcp__nekocode__session_create` 
**🎮 セッション作成（すべての起点）**

セッション作成後、以下のコマンドが利用可能:

#### 📊 基本分析:
- `stats` - 統計情報
- `complexity` - 複雑度ランキング  
- `structure` - 構造解析
- `calls` - 関数呼び出し解析
- `files` - ファイル一覧

#### 🔍 高度分析:
- `find <term>` - シンボル検索
- `analyze --complete` - **完全解析（デッドコード検出）**
- `large-files` - 大きなファイル検出
- `todo` - TODO/FIXME検出

#### 🔧 C++専用:
- `include-cycles` - 循環依存検出
- `include-graph` - 依存関係グラフ
- `include-unused` - 不要include検出
- `include-optimize` - 最適化提案

#### 🌳 AST革命:
- `ast-query <path>` - AST検索
- `ast-stats` - AST統計
- `scope-analysis <line>` - スコープ解析
- `ast-dump [format]` - AST構造ダンプ

**使用例:**
```bash
1. mcp__nekocode__session_create project/
2. セッション内でコマンド実行
```

## 🚀 STANDALONE（補助機能）

### `mcp__nekocode__analyze`
**🚀 単発解析（セッション不要）**
軽量な一回限りの解析用。継続的な分析にはsession_createを推奨。

## 🧠 MEMORY SYSTEM

### `mcp__nekocode__memory`
**🧠 Memory System（時間軸Memory革命）**

統合Memory管理。使用可能操作:
- `save {type} {name} [content]` - 保存
- `load {type} {name}` - 読み込み  
- `list [type]` - 一覧表示
- `search {text}` - 検索
- `stats` - 統計
- `timeline [type] [days]` - 時系列表示

Memory種類: `auto`🤖 `memo`📝 `api`🌐 `cache`💾

## 🛠️ UTILS

### `mcp__nekocode__list_languages`
**🌍 サポート言語一覧**

## 🎯 劇的改善点

### Before（混乱）:
```
❌ 15個のフラット命令
- mcp__nekocode__analyze
- mcp__nekocode__session_create  
- mcp__nekocode__session_stats
- mcp__nekocode__session_complexity
- mcp__nekocode__include_cycles
- mcp__nekocode__include_graph
- mcp__nekocode__memory_save
- mcp__nekocode__memory_load
- ...（何から始める？）
```

### After（美しい）:
```
✅ 4個の整理された命令群
🎮 SESSION（メイン）
   └─ session_create → 25個のセッションコマンド利用可能

🚀 STANDALONE（補助）
   └─ analyze

🧠 MEMORY（統合）
   └─ memory → 6個の操作統合

🛠️ UTILS
   └─ list_languages
```

## 📦 インストール

### Claude Code設定
`claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "nekocode": {
      "command": "python3", 
      "args": ["/path/to/mcp_server_nekocode.py"]
    }
  }
}
```

**これで15個→4個に整理完了にゃ！🎯**