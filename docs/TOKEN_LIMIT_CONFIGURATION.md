# 🎯 Token Limit Configuration Guide

**NekoCode MCP Server Token Management** - 大容量AST出力の安全な制御

最終更新: 2025-08-13 | Version: 1.0 | 🆕 NEW!

---

## 🚨 背景：トークン制限問題

### 問題の発生
- **大規模プロジェクトのAST出力**: 85,161トークン（TypeScriptコンパイラ等）
- **Claude Code MCP制限**: 通常25,000トークン程度
- **結果**: 3.4倍超過でMCP通信失敗、Claude Codeで表示不可

### 解決アプローチ
1. **設定ファイルによる柔軟な制限値設定**
2. **段階的警告システム**
3. **強制出力オプション**
4. **代替手段の自動提案**

---

## ⚙️ 設定ファイル構成

### nekocode_config.json の配置場所
```bash
# NekoCodeバイナリと同じディレクトリに配置
./target/release/nekocode_config.json
./target/debug/nekocode_config.json
./bin/nekocode_config.json  # Legacy C++版
```

### 完全な設定例
```json
{
  "memory": {
    "edit_history": {
      "max_size_mb": 10,
      "min_files_keep": 10
    },
    "edit_previews": {
      "max_size_mb": 5
    }
  },
  "token_limits": {
    "ast_dump_max": 8000,
    "summary_threshold": 1000,
    "allow_force_output": true
  }
}
```

### token_limits 設定詳細

| 設定項目 | デフォルト値 | 説明 |
|----------|-------------|------|
| `ast_dump_max` | 8000 | AST出力のトークン制限（解析アプリとして実用的な値） |
| `summary_threshold` | 1000 | サイズ情報表示開始の閾値 |
| `allow_force_output` | true | 強制全出力の許可設定 |

---

## 🚀 使用方法

### 1. 通常使用（制限内）
```python
# 8000トークン以内の場合 → 通常表示
mcp__nekocode__ast_dump(session_id="12345678")
```

### 2. 制限超過時の警告
```python
# 8000トークン超過時 → 警告 + 代替案表示
🚨 **AST Dump トークン制限超過** (設定: 8,000 tokens)

📊 **出力サイズ分析:**
• 推定トークン数: **21,543 tokens**
• 設定制限: 8,000 tokens  
• 超過率: **2.7x**

🚀 **選択肢:**
1. **ast_stats**: 統計サマリーのみ（推奨）
2. **強制出力**: `mcp__nekocode__ast_dump(session_id="12345678", force=True)`
3. **行数制限**: limit=50 等で部分表示
4. **設定変更**: nekocode_config.json で制限値調整
```

### 3. 強制出力
```python
# 制限を無視して全出力
mcp__nekocode__ast_dump(session_id="12345678", force=True)
```

### 4. 部分表示
```python
# 最初50行のみ表示
mcp__nekocode__ast_dump(session_id="12345678", limit=50)
```

---

## 🎛️ 制限値の調整

### 用途別推奨設定

#### 統計重視（軽量）
```json
{
  "token_limits": {
    "ast_dump_max": 3000,
    "summary_threshold": 500,
    "allow_force_output": true
  }
}
```

#### バランス重視（推奨）
```json
{
  "token_limits": {
    "ast_dump_max": 8000,
    "summary_threshold": 1000,
    "allow_force_output": true
  }
}
```

#### デバッグ重視（大容量）
```json
{
  "token_limits": {
    "ast_dump_max": 15000,
    "summary_threshold": 2000,
    "allow_force_output": true
  }
}
```

#### 制限なし（危険）
```json
{
  "token_limits": {
    "ast_dump_max": 999999,
    "summary_threshold": 5000,
    "allow_force_output": true
  }
}
```

---

## 💡 最適な使い方

### 1. まず ast_stats を試す（推奨）
```python
# 統計サマリーで全体把握
mcp__nekocode__ast_stats(session_id="12345678")

# 必要に応じてAST詳細を確認
mcp__nekocode__ast_dump(session_id="12345678", limit=30)
```

### 2. 段階的なアプローチ
```bash
# Step 1: セッション作成
session_id = mcp__nekocode__session_create(path="large-project/")

# Step 2: 統計で概要把握
mcp__nekocode__ast_stats(session_id=session_id)

# Step 3: 必要に応じて部分表示
mcp__nekocode__ast_dump(session_id=session_id, limit=50)

# Step 4: 本当に必要な場合のみ全出力
mcp__nekocode__ast_dump(session_id=session_id, force=True)
```

### 3. プロジェクト規模別戦略

| プロジェクト規模 | 推奨アプローチ | 設定 |
|----------------|---------------|------|
| 小規模（<10ファイル） | AST直接出力OK | `ast_dump_max: 15000` |
| 中規模（10-100ファイル） | 統計 → 部分出力 | `ast_dump_max: 8000` |
| 大規模（100+ファイル） | 統計のみ推奨 | `ast_dump_max: 3000` |

---

## 🔧 トラブルシューティング

### Q: 設定ファイルが読み込まれない
**A**: 設定ファイルの配置場所を確認
```bash
# 設定ファイル確認
ls -la target/release/nekocode_config.json
ls -la target/debug/nekocode_config.json

# MCPサーバーログで設定読み込み確認
# 「📋 Config loaded from: /path/to/nekocode_config.json」が出力される
```

### Q: 強制出力が効かない
**A**: 設定で無効化されていないか確認
```json
{
  "token_limits": {
    "allow_force_output": true  // これがfalseだと強制出力不可
  }
}
```

### Q: 制限値を変更したのに反映されない
**A**: MCPサーバーの再起動が必要
```bash
# Claude Codeを再起動してMCPサーバーを再読み込み
```

### Q: JSON形式エラー
**A**: JSON構文の確認
```bash
# JSON構文チェック
python3 -m json.tool nekocode_config.json
```

---

## 📊 性能への影響

### トークン数推定の計算コスト
- **推定方法**: 文字数 ÷ 4 = 近似トークン数
- **計算時間**: <1ms（無視可能）
- **メモリ影響**: 無し

### 制限チェックのオーバーヘッド
- **追加時間**: 2-3ms
- **メモリ使用**: 出力サイズに比例
- **全体への影響**: <1%

---

## 🎯 ベストプラクティス

### 開発フローでの活用
1. **初期調査**: `ast_stats` で全体把握
2. **詳細確認**: `limit=30` で要点確認
3. **必要時のみ**: `force=True` で全出力
4. **設定調整**: プロジェクト規模に応じた制限値設定

### チーム設定の統一
```bash
# チーム共通設定をリポジトリに含める
cp nekocode_config.json.template nekocode_config.json
git add nekocode_config.json
```

### CI/CD統合
```bash
# GitHub Actionsでトークン制限無効化
export NEKOCODE_CONFIG='{"token_limits":{"ast_dump_max":999999}}'
```

---

**🎉 この設定により、NekoCodeは大規模プロジェクトでも安全かつ柔軟に動作します！**