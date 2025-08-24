# 🔍 Include解析機能ガイド - C++コンパイル高速化の決定版

> 注意: コマンド例の `nekocode_ai` はレガシー表記です。現行は `nekocode` バイナリをご使用ください。

## 📋 目次
1. [概要](#概要)
2. [主要機能](#主要機能)
3. [使い方](#使い方)
4. [実践例](#実践例)
5. [トラブルシューティング](#トラブルシューティング)

## 概要

NekoCode AIのInclude解析機能は、C++プロジェクトの**コンパイル時間を劇的に改善**するための強力なツールです。大規模プロジェクトでは、不適切なinclude管理により、コンパイル時間が**10倍以上**遅くなることもあります。

### 🎯 こんな問題を解決！

- **コンパイルが遅い** → 不要includeを削除してビルド時間を50%短縮
- **ヘッダー変更で全体再コンパイル** → 依存関係を最適化して影響範囲を最小化
- **循環依存でビルドエラー** → 依存サイクルを検出して解決方法を提案
- **どのヘッダーが重い？** → ホットスポット分析で問題のあるヘッダーを特定

## 主要機能

### 1. 📊 include-graph - Include依存グラフ

プロジェクト全体のinclude依存関係を可視化します。

```json
{
  "command": "include-graph",
  "dependency_graph": {
    "src/core.hpp": {
      "direct_includes": ["types.hpp", "utils.hpp"],
      "transitive_includes": ["types.hpp", "utils.hpp", "string", "vector"],
      "include_depth": 3,
      "included_by_count": 42
    }
  },
  "hotspot_headers": [
    {
      "file": "src/types.hpp",
      "included_by_count": 87,
      "impact_score": 95
    }
  ]
}
```

**活用方法：**
- `included_by_count`が多いヘッダーは変更時の影響が大きい
- `include_depth`が深いファイルはコンパイルが遅い原因

### 2. 🔄 include-cycles - 循環依存検出

A→B→C→Aのような循環依存を検出します。

```json
{
  "command": "include-cycles",
  "circular_dependencies": [
    {
      "cycle_path": ["ui/widget.hpp", "ui/manager.hpp", "ui/widget.hpp"],
      "severity": "critical",
      "suggestion": "前方宣言を使用するか、インターフェースを分離してください"
    }
  ]
}
```

**解決方法：**
1. 前方宣言（forward declaration）を使用
2. インターフェースと実装を分離
3. PIMPLイディオムを適用

### 3. 🗑️ include-unused - 不要include検出

実際に使われていないincludeを特定し、削除候補を提示します。

```json
{
  "command": "include-unused",
  "unused_includes": [
    {
      "file": "src/main.cpp",
      "unused_include": "#include <algorithm>",
      "line_number": 15,
      "reason": "このヘッダーの機能は使用されていません"
    }
  ],
  "optimization_potential": {
    "removable_includes": 127,
    "estimated_compile_time_reduction": 23.5
  }
}
```

**効果：**
- 不要なヘッダーを削除してコンパイル時間を短縮
- 依存関係を減らしてビルドの安定性向上

### 4. 💥 include-impact - 変更影響範囲解析

特定のファイルを変更した場合の影響範囲を分析します。

```bash
nekocode_ai session-command <session-id> "include-impact core/types.hpp"
```

```json
{
  "command": "include-impact",
  "target_file": "core/types.hpp",
  "directly_affected": [
    "core/utils.cpp",
    "core/manager.cpp"
  ],
  "transitively_affected": [
    "ui/widget.cpp",
    "network/client.cpp",
    "..."
  ],
  "total_affected_files": 67,
  "recompilation_units": 89
}
```

**活用シーン：**
- リファクタリング前の影響調査
- ヘッダー変更時のビルド時間予測
- 依存関係の複雑さ評価

### 5. 🚀 include-optimize - 最適化提案

コンパイル時間を改善するための具体的な提案を生成します。

```json
{
  "command": "include-optimize",
  "optimizations": [
    {
      "type": "FORWARD_DECLARATION",
      "target_file": "ui/widget.hpp",
      "suggestion": "class Manager; を使用して manager.hpp のincludeを削除",
      "estimated_impact": 85
    },
    {
      "type": "MOVE_TO_IMPLEMENTATION",
      "target_file": "core/utils.hpp",
      "suggestion": "<algorithm> を .cpp ファイルに移動",
      "estimated_impact": 72
    },
    {
      "type": "PIMPL_CANDIDATE",
      "target_file": "network/client.hpp",
      "suggestion": "PIMPLパターンで実装詳細を隠蔽",
      "estimated_impact": 90
    }
  ]
}
```

## 使い方

### ステップ1: セッション作成

```bash
# C++プロジェクトのセッションを作成
nekocode_ai session-create /path/to/cpp/project
```

### ステップ2: 基本分析

```bash
# 依存グラフ全体を確認
nekocode_ai session-command <session-id> include-graph

# 循環依存をチェック
nekocode_ai session-command <session-id> include-cycles
```

### ステップ3: 最適化

```bash
# 不要includeを検出
nekocode_ai session-command <session-id> include-unused

# 最適化提案を取得
nekocode_ai session-command <session-id> include-optimize
```

## 実践例

### 例1: 大規模プロジェクトのコンパイル時間改善

```bash
# 1. まず現状を把握
nekocode_ai session-command mysession include-graph > before.json

# 2. 不要includeを特定・削除
nekocode_ai session-command mysession include-unused

# 3. 循環依存を解決
nekocode_ai session-command mysession include-cycles

# 4. 最適化を適用
nekocode_ai session-command mysession include-optimize
```

**結果例：**
- コンパイル時間: 45分 → 12分（73%削減）
- インクリメンタルビルド: 5分 → 30秒（90%削減）

### 例2: ホットスポットヘッダーの改善

```bash
# types.hpp が87箇所からincludeされている場合
nekocode_ai session-command mysession "include-impact types.hpp"

# 影響範囲を確認して、以下の対策を実施：
# 1. 型定義を複数のヘッダーに分割
# 2. 前方宣言用ヘッダー（types_fwd.hpp）を作成
# 3. テンプレートの実装を.hppから.tppに分離
```

### 例3: CI/CDパイプラインへの統合

```yaml
# .github/workflows/include-check.yml
name: Include Analysis
on: [pull_request]

jobs:
  include-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Run Include Analysis
        run: |
          session_id=$(nekocode_ai session-create . | jq -r '.session_id')
          
          # 循環依存チェック
          cycles=$(nekocode_ai session-command $session_id include-cycles | jq '.circular_dependencies | length')
          if [ $cycles -gt 0 ]; then
            echo "❌ 循環依存が検出されました"
            exit 1
          fi
          
          # 不要include警告
          unused=$(nekocode_ai session-command $session_id include-unused | jq '.unused_includes | length')
          if [ $unused -gt 10 ]; then
            echo "⚠️ ${unused}個の不要includeが検出されました"
          fi
```

## トラブルシューティング

### Q: include-impactが未実装と表示される

A: 現在、一部の機能は開発中です。基本的な機能（graph、cycles、unused）は利用可能です。

### Q: システムヘッダーも解析したい

A: デフォルトではシステムヘッダー（<iostream>など）は除外されます。必要な場合は設定で有効化できます（将来実装予定）。

### Q: 解析が遅い

A: 大規模プロジェクトでは初回解析に時間がかかることがあります。`--io-threads 16`オプションで高速化できます。

### Q: 前方宣言の使い方がわからない

A: 以下の例を参考にしてください：

```cpp
// ❌ 悪い例：widget.hpp
#include "manager.hpp"  // Manager全体が必要？

class Widget {
    Manager* m_manager;  // ポインタだけ
};

// ✅ 良い例：widget.hpp  
class Manager;  // 前方宣言で十分

class Widget {
    Manager* m_manager;  // ポインタなら前方宣言でOK
};
```

## まとめ

Include解析機能を活用することで：

- 🚀 **コンパイル時間を50-90%削減**
- 🔍 **依存関係の可視化**で設計改善
- 🛠️ **自動最適化提案**で作業効率UP
- 📊 **CI/CD統合**で品質維持

C++プロジェクトの生産性を劇的に向上させる強力なツールです。ぜひご活用ください！
