# 🎯 NekoCode 設計思想 - Design Philosophy

## 🚨 **最重要原則: セッションファースト**

### **すべてはセッション作成から始まる**

これがNekoCodeの**絶対的な設計原則**です。

```bash
# ✅ 唯一の正しい導線
nekocode session-create /path/to/project  # すべての始まり
nekocode ast-stats                        # セッションベースで動作
nekocode deadcode                         # セッションベースで動作
```

### **なぜセッションファーストなのか**

#### 1. **パフォーマンス**
- 初回解析: プロジェクト全体をパース（数秒）
- 2回目以降: メモリから即座に取得（3ms）
- 100倍以上の高速化を実現

#### 2. **一貫性**
- すべての操作が同じコンテキストで動作
- ファイル間の依存関係を正確に追跡
- 解析結果の整合性を保証

#### 3. **ユーザー体験**
- 単一の明確な操作フロー
- 迷わない、混乱しない
- Gitのような直感的な使用感

## ❌ **アンチパターン（絶対にやってはいけないこと）**

### 1. **analyzeコマンドの存在**
```bash
# ❌ 絶対NG
nekocode analyze /path/to/file  # 導線がめちゃくちゃになる！
```

**なぜダメなのか：**
- セッションを使わない単発解析は設計思想に反する
- ユーザーが2つの導線で混乱する
- パフォーマンスの利点を活かせない

### 2. **セッションIDの露出**
```bash
# ❌ ユーザーにIDを意識させるな
nekocode session-create /project → "Session: abc123"
nekocode ast-stats abc123  # IDを覚えさせるのはUXの失敗
```

**理想：**
- セッションIDは内部実装の詳細
- ユーザーは存在を知らなくていい

### 3. **複数の導線**
```bash
# ❌ 混乱の元
nekocode analyze ...       # 導線1
nekocode session-create ... # 導線2
nekocode quick-stats ...   # 導線3
```

**正解：**
- 導線は1つだけ: `session-create` → その他すべて

## ✅ **正しい実装方針**

### 1. **セッション自動管理**
```rust
// すべてのコマンドで自動的に最後のセッションを使用
fn get_current_session() -> Session {
    // .nekocode_sessions/last_session から自動取得
}
```

### 2. **MCPサーバーの実装**
```python
# セッションを内部で自動管理
class NekocodeServer:
    def __init__(self):
        self.current_session = None
    
    def session_create(self, path):
        self.current_session = create_session(path)
        return "Session created"  # IDは返さない
    
    def stats(self):
        # current_sessionを自動使用
        return get_stats(self.current_session)
```

### 3. **エラーメッセージ**
```bash
# セッションがない時のエラー
$ nekocode ast-stats
Error: No active session. Please create a session first:
  nekocode session-create /path/to/project
```

## 📊 **導線フロー図**

```
[ユーザー] 
    ↓
[session-create] ← 唯一の入口
    ↓
[セッション作成]
    ↓
┌─────────────────────────┐
│ すべての機能が使用可能   │
├─────────────────────────┤
│ • ast-stats             │
│ • deadcode              │
│ • ast-query             │
│ • refresh               │
│ • その他すべて          │
└─────────────────────────┘
```

## 🔒 **この設計を守るためのルール**

1. **新機能追加時**
   - 必ずセッションベースで実装
   - 単発実行モードは作らない

2. **ドキュメント作成時**
   - 必ず`session-create`から始める
   - analyzeコマンドの存在を書かない

3. **MCPツール作成時**
   - セッション作成を前提とする
   - analyzeのようなショートカットを作らない

4. **エラー処理**
   - セッションがない場合は明確にガイド
   - 「まずsession-createを実行してください」

## 💡 **将来の拡張**

### **セッションID完全隠蔽（理想形）**
```bash
# 現在地から自動でセッション解決
cd /my/project
nekocode                    # session-createを自動実行
nekocode stats              # 自動でコンテキスト認識
```

### **Git統合**
```bash
# .gitディレクトリと連動
nekocode                    # Gitプロジェクトを自動認識
nekocode stats              # プロジェクト全体の統計
```

## 📝 **まとめ**

**NekoCodeの設計思想は「セッションファースト」です。**

- すべてはセッション作成から始まる
- 導線は1つだけ
- セッションIDはユーザーから隠蔽
- analyzeのような単発コマンドは存在しない

この原則を守ることで、高速で一貫性があり、使いやすいツールを実現します。

---
**作成日**: 2025-08-25  
**最終更新**: 2025-08-25  
**ステータス**: 🔒 **確定済み設計思想**