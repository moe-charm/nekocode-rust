# 🧪 NekoCode MCP Server テスト方法

## 🔧 セットアップ手順

### 1. **NekoCodeビルド確認**
```bash
# NekoCodeがビルド済みか確認
ls -la ../build/nekocode_ai

# なければビルド
cd ..
mkdir -p build && cd build
cmake .. && make -j
```

### 2. **MCPサーバー権限設定**
```bash
chmod +x mcp_server_real.py
```

### 3. **Claude Code設定**

`~/.config/claude-desktop/config.json` (Linux)  
`~/Library/Application Support/Claude/claude_desktop_config.json` (Mac)  
に以下を追加:

```json
{
  "mcpServers": {
    "nekocode": {
      "command": "python3",
      "args": ["/絶対パス/nekocode-cpp-github/mcp-nekocode-server/mcp_server_real.py"],
      "env": {
        "NEKOCODE_BINARY_PATH": "/絶対パス/nekocode-cpp-github/build/nekocode_ai"
      }
    }
  }
}
```

**⚠️ 重要**: パスは絶対パスで指定してください！

## 🧪 動作テスト

### **手動テスト** (MCPプロトコル直接)
```bash
# 1. サーバー起動
python3 mcp_server_real.py

# 2. 初期化メッセージ送信
echo '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}' | python3 mcp_server_real.py

# 3. ツール一覧取得
echo '{"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}' | python3 mcp_server_real.py
```

### **Claude Codeでテスト**
1. Claude Code再起動
2. 新しいチャットで確認:
```
利用可能なツール一覧を教えて
```

### **実際のツール実行テスト**
```
# プロジェクト解析テスト
mcp__nekocode__analyze でこのプロジェクトを解析してください

# セッション作成テスト  
mcp__nekocode__session_create でセッションを作成してください

# 言語一覧テスト
mcp__nekocode__list_languages で対応言語を確認してください
```

## 🔍 トラブルシューティング

### **よくある問題**

#### 1. **ツールが見つからない**
```
Error: Tool not found
```
**解決法**: Claude Codeを完全に再起動してください

#### 2. **NekoCodeバイナリが見つからない**
```
Error: NekoCodeバイナリが見つかりません
```
**解決法**: 
- `../build/nekocode_ai` が存在するか確認
- 実行権限があるか確認: `chmod +x ../build/nekocode_ai`

#### 3. **パーミッションエラー**
```
Permission denied
```
**解決法**:
```bash
chmod +x mcp_server_real.py
chmod +x ../build/nekocode_ai
```

#### 4. **JSONエラー**
```
JSON decode error
```
**解決法**: NekoCodeの出力形式を確認:
```bash
# 手動でNekoCodeを実行してみる
../build/nekocode_ai --help
../build/nekocode_ai --list-languages
```

### **ログ確認**
MCPサーバーのログは `stderr` に出力されます:
```bash
# Claude Codeのログを確認
tail -f ~/.config/claude-desktop/logs/claude-desktop.log
```

## 🎯 期待される結果

### **成功時**
```
利用可能なツール:
- mcp__nekocode__analyze
- mcp__nekocode__session_create  
- mcp__nekocode__session_stats
- mcp__nekocode__include_cycles
- mcp__nekocode__include_graph
- mcp__nekocode__list_languages
```

### **解析結果例**
```json
{
  "analysis_type": "directory",
  "directory_path": "/path/to/project",
  "summary": {
    "total_files": 30,
    "total_lines": 9776,
    "total_classes": 0,
    "total_functions": 0
  },
  "performance": {
    "analysis_time_ms": 38,
    "files_per_second": 789.47
  }
}
```

## 🚀 次のステップ

1. ✅ **基本動作確認**: MCPツールが利用可能
2. ✅ **機能テスト**: 各ツールが正常動作
3. 🔄 **パフォーマンステスト**: 実際の解析速度測定
4. 🔄 **エラーハンドリング**: 異常系の動作確認

---
🐱 実際のMCPプロトコルでNekoCodeの力を解放！