# 🎯 NekoCode MCP コマンド整理・改善案

## 😵 現状の問題：42個のコマンドは多すぎる！

### 現在のMCPコマンド一覧（42個）

#### 📊 基本解析系（4個）
- `analyze` - プロジェクト解析
- `session_create` - セッション作成
- `session_stats` - セッション統計
- `session_update` - セッション更新

#### 🌳 AST系（4個）
- `ast_stats` - AST統計
- `ast_query` - ASTクエリ
- `ast_dump` - ASTダンプ
- `scope_analysis` - スコープ解析

#### ✏️ リファクタリング系（14個）
- `replace_preview` - 置換プレビュー
- `replace_confirm` - 置換実行
- `insert_preview` - 挿入プレビュー
- `insert_confirm` - 挿入実行
- `movelines_preview` - 行移動プレビュー
- `movelines_confirm` - 行移動実行
- `moveclass_preview` - クラス移動プレビュー
- `moveclass_confirm` - クラス移動実行
- `smart_insert` - スマート挿入
- `smart_replace` - スマート置換
- `smart_move` - スマート移動
- `create_file` - ファイル作成
- `strip_comments` - コメント削除
- `edit_rollback` - 編集ロールバック

#### 📝 履歴・編集系（3個）
- `edit_history` - 編集履歴
- `edit_show` - 編集詳細
- `edit_stats` - 編集統計

#### 🔍 C++専用系（2個）
- `include_cycles` - 循環依存検出
- `include_graph` - 依存グラフ

#### 👁️ 監視系（5個）
- `watch_start` - 監視開始
- `watch_status` - 監視状態
- `watch_stop` - 監視停止
- `watch_stop_all` - 全監視停止
- `watch_config` - 監視設定

#### 💾 メモリ系（4個）
- `memory_save` - メモリ保存
- `memory_load` - メモリ読込
- `memory_list` - メモリ一覧
- `memory_timeline` - メモリタイムライン

#### ⚙️ 設定系（2個）
- `config_show` - 設定表示
- `config_set` - 設定変更

#### 🆕 新機能系（3個）
- `refresh` - リフレッシュ
- `deadcode` - デッドコード検出
- `list_languages` - 言語一覧

## 🤔 問題点の深掘り

### 1. **選択肢が多すぎる**
```
Claude Code君「えーっと、解析したいから...」
「analyze？session_create？ast_stats？どれ使えばいいの？」
```

### 2. **preview/confirm の2段階が面倒**
```
「replace_previewして、それからreplace_confirm？」
「面倒だな...直接やっちゃダメなの？」
```

### 3. **機能の重複**
```
「smart_insertとinsert_preview、どっち使えばいい？」
「session_createとanalyze、何が違うの？」
```

### 4. **セッションIDの管理が大変**
```
「session_createしてIDもらって、それを他のコマンドに...」
「ID忘れた！もう一回作る？」
```

## 💡 改善案：3つのシンプルコマンドに統合

### 🌟 **新・統一コマンド体系**

#### **1. `nekocode` - メインコマンド（90%はこれだけ）**
```python
mcp__nekocode(
    path: str,                    # 必須：解析対象
    action: str = "analyze",      # オプション：アクション
    options: dict = {}            # オプション：詳細設定
)
```

**アクション例：**
- `analyze`（デフォルト）- 解析・統計表示
- `refactor` - リファクタリング準備
- `watch` - 監視モード
- `clean` - クリーンアップ

#### **2. `nekocode_edit` - 編集専用（上級者向け）**
```python
mcp__nekocode_edit(
    file: str,                    # 対象ファイル
    operation: str,               # 操作種別
    params: dict                  # パラメータ
)
```

**操作例：**
- `replace` - 置換（プレビュー自動）
- `insert` - 挿入（位置指定可能）
- `move` - 移動（行/クラス）

#### **3. `nekocode_info` - 情報取得（読み取り専用）**
```python
mcp__nekocode_info(
    query: str,                   # クエリ種別
    params: dict = {}             # パラメータ
)
```

**クエリ例：**
- `sessions` - セッション一覧
- `history` - 履歴
- `config` - 設定確認

## 🎨 使用例（Before/After）

### **Before（現在）：複雑**
```python
# セッション作成して...
session_id = mcp__nekocode__session_create(path="../src")

# 統計見て...  
mcp__nekocode__session_stats(session_id=session_id)

# AST解析して...
mcp__nekocode__ast_stats(session_id=session_id)

# リファクタリングプレビューして...
preview_id = mcp__nekocode__replace_preview(
    file_path="../src/main.rs",
    pattern="old",
    replacement="new"
)

# 確認して実行...
mcp__nekocode__replace_confirm(preview_id=preview_id)
```

### **After（改善後）：シンプル**
```python
# 解析（これだけ！）
mcp__nekocode(path="../src")

# リファクタリング（自動プレビュー付き）
mcp__nekocode_edit(
    file="../src/main.rs",
    operation="replace",
    params={"from": "old", "to": "new"}
)

# 情報確認
mcp__nekocode_info(query="sessions")
```

## 📊 統合マッピング

### **`nekocode` に統合されるコマンド**
```
analyze + session_create + session_stats + session_update
→ nekocode(path, action="analyze")

watch_start + watch_status + watch_stop
→ nekocode(path, action="watch")

refresh + deadcode
→ nekocode(path, action="check")
```

### **`nekocode_edit` に統合されるコマンド**
```
replace_preview + replace_confirm
→ nekocode_edit(file, operation="replace")

insert_preview + insert_confirm + smart_insert
→ nekocode_edit(file, operation="insert")

movelines_preview + movelines_confirm + moveclass_preview + moveclass_confirm
→ nekocode_edit(file, operation="move")
```

### **`nekocode_info` に統合されるコマンド**
```
edit_history + edit_show + edit_stats
→ nekocode_info(query="history")

config_show + config_set
→ nekocode_info(query="config") / nekocode(action="config", options={...})

memory_list + memory_timeline
→ nekocode_info(query="memory")
```

## 🚀 実装計画

### **Phase 1: 統合関数の追加**
```python
# 既存コマンドを残しつつ、新しい統合コマンドを追加
async def _tool_nekocode(self, args):
    # 内部で既存関数を呼び分け
    
async def _tool_nekocode_edit(self, args):
    # 編集系を統合
    
async def _tool_nekocode_info(self, args):
    # 情報系を統合
```

### **Phase 2: 自動セッション管理**
```python
class SmartSessionManager:
    def get_or_create_session(self, path):
        # パスからセッション自動解決
        # ユーザーはセッションIDを意識しない
```

### **Phase 3: プレビュー自動化**
```python
def smart_edit(self, operation, params):
    # 自動的にプレビュー生成
    # ユーザー確認後に実行
    # または、--auto フラグで即実行
```

## 📈 期待効果

### **Before（現在）**
- 42個のコマンドから選ぶ必要
- セッションID管理が必要
- preview/confirm の2段階
- 似た機能の使い分けが不明

### **After（改善後）**
- 3個のコマンドでカバー
- セッション自動管理
- プレビュー自動化
- 直感的な操作

## 🎯 最終的な使い心地

```python
# 90%のケースはこれだけ！
nekocode(".")           # 解析
nekocode(".", "watch")  # 監視

# たまに使う編集
nekocode_edit("main.rs", "replace", {"from": "old", "to": "new"})

# 情報確認したい時
nekocode_info("config")
```

## 📋 移行戦略

1. **互換性維持**: 既存42コマンドは残す（deprecatedマーク）
2. **段階的移行**: 新コマンドを徐々に推奨
3. **ドキュメント**: 新コマンドを前面に、旧コマンドは「上級者向け」
4. **自動変換**: 旧コマンド使用時に新コマンドを提案

---

**結論**: 42個→3個への統合で、Claude Code君も迷わない！🐱