# 📋 Current Task - NekoCode スマートセッション実装

**最終更新**: 2025-08-24 15:09

## ✅ 完了：統一エントリーポイント実装

### **達成内容**
Claude Code君が迷わず使える、シンプルな導線を実装完了！

### **解決した問題** ✅
- 選択肢の統一（`_tool_nekocode()`一つで自動判定）
- 引数のシンプル化（path必須、action/optionsはオプション）
- エラー対策（パス正規化、セッション自動管理、--threads復活）

### **実装済み機能**
```python
# シンプルな使い方を実現！
_tool_nekocode(path=".")           # カレント解析（セッション自動）
_tool_nekocode(path="../src")      # 指定パス解析（差分更新）
_tool_nekocode(path=".", action="watch")  # 監視モード
_tool_nekocode(path=".", action="check")  # デッドコード検出
```

## ✅ 完了したタスク

### 1. **--threadsエラー修正** ✅
- 5分割版nekocodeに`--threads`オプション追加
- tokioワーカースレッド制御実装
- デフォルト8スレッド並列処理

### 2. **MCPバイナリパス修正** ✅
- 確実に5分割版を使用するようパス更新
- `../nekocode-rust-clean/nekocode-workspace/target/debug/nekocode`

### 3. **設定ファイル調査** ✅
- `.nekocode_config.json`（ローカル設定）
- `.nekocode_sessions/`（セッション保存）
- `~/.nekocode/memory/`（メモリ管理）

### 4. **監視モード調査** ✅
除外パターン:
- `.git`, `node_modules`, `target`
- `*.tmp`, `*.log`
- `.nekocode_sessions`（自己参照防止）

## 🚨 新たな問題発覚：MCPコマンド42個は多すぎる！

### **深刻な問題**
- 現在42個のMCPコマンドが存在
- preview/confirm の2段階が面倒
- 似た機能が重複（smart_insert vs insert_preview）
- セッションID管理が大変

### **解決策：3つのシンプルコマンドに統合**

## 🎯 実装予定（優先順位順）

### **Phase 1: スマート再解析コマンド実装**（最優先）
```python
# シンプルな統一コマンド
mcp__nekocode(path, action="analyze")
```

**機能：**
- 既存セッションあれば差分更新（高速）
- なければ新規作成
- セッションID自動管理（ユーザー意識不要）

**実装手順：**
1. MCPサーバーに`_tool_nekocode()`追加
2. パス→セッションマッピング実装
3. 差分検出ロジック実装
4. 分かりやすい結果表示

### **Phase 2: コマンド統合**（その後）
```python
# 42個→3個への段階的統合
mcp__nekocode(path, action, options)  # メイン
mcp__nekocode_edit(file, op, params)  # 編集
mcp__nekocode_info(query, params)     # 情報
```

**移行計画：**
- 既存42コマンドは残す（後方互換）
- 新コマンドをデフォルトに
- ドキュメント更新

### **Phase 3: 性能最適化**（安定後）
- タイムスタンプキャッシュ
- ハッシュ比較高速化
- `.nekocode_cache`活用
- 並列処理強化

### **Phase X: 常駐監視**（将来的に）
- オプション機能として提供
- 最小限モードのみ
- 明示的に有効化が必要

## 📝 設計ドキュメント

- [MCP_COMMAND_REORGANIZATION.md](MCP_COMMAND_REORGANIZATION.md) - **NEW!** コマンド統合案
- [SMART_SESSION_DESIGN.md](SMART_SESSION_DESIGN.md) - スマートセッション設計
- [MCP_USAGE_GUIDE.md](MCP_USAGE_GUIDE.md) - 使い方ガイド

## 📌 今すぐやること

### **作業1: `_tool_nekocode()` 実装**
```python
# mcp_server_real.py に追加
async def _tool_nekocode(self, args: Dict) -> Dict:
    """スマート統一エントリーポイント"""
    path = args["path"]
    action = args.get("action", "analyze")
    
    # パス正規化
    abs_path = self.normalize_path(path)
    
    # セッション自動解決
    session_id = self.get_or_create_session(abs_path)
    
    if action == "analyze":
        # 差分があれば更新、なければキャッシュ返却
        return await self.smart_analyze(session_id, abs_path)
    
    # 他のアクションは後で追加
```

### **作業2: セッションマッピング実装**
```python
def get_or_create_session(self, path):
    # .nekocode_sessions/ から既存セッション検索
    # なければ新規作成
    # パス→セッションIDをキャッシュ
```

### **作業3: 動作テスト**
```python
# Claude Code経由でテスト
mcp__nekocode(path=".")
mcp__nekocode(path="../src")
```

## 📊 実装成果まとめ

### **Phase 1 完了！** ✅
統一エントリーポイント`_tool_nekocode()`の実装が完了しました：

1. **自動セッション管理** - パスからセッションID自動解決
2. **スマート差分更新** - 2回目以降は変更分のみ高速解析
3. **パス正規化** - 相対パス/絶対パス自動変換
4. **ファイル数判定** - 1000ファイル超えで自動的に軽量モード
5. **分かりやすい表示** - 絵文字と整形された出力

### **次のステップ（Phase 2）**
必要に応じて実装：
- 既存42コマンドの段階的統合
- `nekocode_edit()`と`nekocode_info()`の追加
- ドキュメント更新

---

**ステータス**: 🎊 **Phase 1 完了 - 統一エントリーポイント実装成功！**