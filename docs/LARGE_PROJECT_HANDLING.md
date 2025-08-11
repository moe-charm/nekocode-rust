# 🏗️ 大規模プロジェクト対応ガイド

NekoCode AIの大規模プロジェクト対応機能の使用方法と最適化手法

## 📊 プロジェクト規模の分類

### 自動分類システム
```
Small:   < 100ファイル    (即座に実行)
Medium:  100-999ファイル  (軽微な警告)
Large:   1,000-9,999ファイル  (確認プロンプト)
Massive: 10,000+ ファイル     (強制確認)
```

### 時間予測アルゴリズム
- **TypeScript基準**: 0.16秒/ファイル
- **推定時間**: ファイル数 × 0.16 ÷ 60 (分)

## 🚀 事前チェック機能

### 基本使用方法
```bash
# 📋 プロジェクト規模確認のみ
nekocode_ai session-create large_project/ --check-only

# 🔍 確認なしで強制実行
nekocode_ai session-create huge_project/ --force

# ⚡ 事前チェックをスキップ（上級者向け）
nekocode_ai session-create auto_script/ --no-check
```

### 出力例
```
🔍 Quick project scan...
📊 Project Analysis:
• Total files: 20,732
• Code files: 18,456
• Scale: Massive
• Estimated time: 55 minutes

⚠️  Large project detected!
This will block Claude Code for ~55 minutes.

Continue? [y/N]:
```

## 🚀 高速処理（Claude Code最適化）

大規模プロジェクトでClaude Codeがブロックされる問題を解決する高速化機能

### 基本的な使い方
```bash
# 🔍 事前にファイル数確認
find project/ -type f | wc -l

# 1,000ファイル未満: 通常セッション（数分）
nekocode_ai session-create project/

# 1,000ファイル以上: 高速統計解析（秒単位）
nekocode_ai analyze project/ --stats-only --io-threads 16
```

### 高速処理のワークフロー

#### ステップ1: 高速統計解析（推奨）
```bash
nekocode_ai analyze large_project/ --stats-only --io-threads 16
```

**即座に出力される情報:**
```json
{
  "analysis_type": "directory",
  "directory_path": "large_project/",
  "summary": {
    "total_classes": 145,
    "total_functions": 892,
    "total_lines": 25637
  },
  "total_files": 1534
}
```

#### ステップ2: 詳細解析が必要な場合
```bash
# セッション作成（詳細な対話解析用）
nekocode_ai session-create large_project/
```

**出力例:**
```json
{
  "session_id": "ai_session_20250729_123456",
  "message": "✅ AI Session created",
  "files_analyzed": 1534,
  "processing_time": "45.2s"
}
```

#### ステップ3: 対話的解析
```bash
# セッション利用
nekocode_ai session-command ai_session_20250729_123456 stats
nekocode_ai session-command ai_session_20250729_123456 "find interface --limit 20"
nekocode_ai session-command ai_session_20250729_123456 complexity
```

### Claude Code最適化のメリット
- ✅ **超高速レスポンス**: --stats-onlyで秒単位の結果取得
- ✅ **並列処理**: --io-threads 16で大幅高速化
- ✅ **完全な互換性**: 従来のsession-commandがそのまま利用可能
- ✅ **メモリ効率**: 統計のみなので軽量処理

## ⚡ パフォーマンス最適化

### ストレージ別最適化
```bash
# 🔥 SSD最適化（並列I/O重視）
nekocode_ai session-create project/ --ssd

# 💿 HDD最適化（シーケンシャル重視）
nekocode_ai session-create project/ --hdd

# 🎯 手動スレッド数指定
nekocode_ai session-create project/ --threads 16
```

### 推奨設定
| プロジェクトサイズ | 推奨オプション | Claude Code対応 |
|-------------------|---------------|----------------|
| < 1,000ファイル   | デフォルト     | ✅ 問題なし |
| 1,000-10,000     | `--stats-only --io-threads 16`   | ✅ 秒単位完了 |
| 10,000+          | `--stats-only --io-threads 16` | ✅ 高速統計 |

## 📊 進捗監視機能

### プログレス表示
```bash
# 🔄 リアルタイム進捗表示
nekocode_ai session-create large_project/ --progress

# 出力例：
🚀 Starting analysis: 5,432 files in large_project/
Processing 540/5432 (9.9%) | Rate: 12.3/sec | ETA: 6m 32s
Current: src/components/complex-component.tsx (45.2KB)
```

### 進捗ファイル監視
```bash
# 📁 進捗ファイル確認
tail -f sessions/ai_session_20250729_123456_progress.txt

# 出力例：
[2025-07-29 12:34:56] START: 5432 files | Target: large_project/
[2025-07-29 12:35:12] PROCESSING: 154/5432 (2.8%) | file.ts (23.1KB) | OK | 16.2s
[2025-07-29 12:40:33] COMPLETE: 5432/5432 (100%) | Total: 5m 37s | Success: 5401 | Errors: 31 | Skipped: 0
```

## 🎯 実践的ワークフロー

### 1. 初回調査フェーズ
```bash
# ステップ1: 規模確認
nekocode_ai session-create project/ --check-only

# ステップ2: サブディレクトリ選択
nekocode_ai session-create project/src/core/ --check-only
nekocode_ai session-create project/src/components/ --check-only
```

### 2. 段階的解析フェーズ
```bash
# 重要部分から先に解析
nekocode_ai session-create project/src/core/ --progress
nekocode_ai session-create project/src/api/ --progress

# 全体解析（時間がある時）
nekocode_ai session-create project/ --progress --ssd
```

### 3. Claude Code連携フェーズ
```bash
# セッション作成後の対話的解析
nekocode_ai session-command ai_session_20250729_123456 stats
nekocode_ai session-command ai_session_20250729_123456 "find interface --limit 20"
nekocode_ai session-command ai_session_20250729_123456 complexity
```

## 🚨 トラブルシューティング

### Claude Codeブロック対策
| 問題 | 原因 | 解決策 |
|------|------|--------|
| 長時間無応答 | 大規模プロジェクト実行中 | `--check-only`で事前確認 |
| メモリ不足 | 大量ファイル同時処理 | `--hdd`で逐次処理 |
| 解析中断 | プロセス強制終了 | 進捗ファイルで状況確認 |

### 緊急時対応
```bash
# 🚨 解析プロセス確認
ps aux | grep nekocode_ai

# 📄 最新進捗確認
ls -t sessions/*_progress.txt | head -1 | xargs tail

# 🔄 セッション復旧
nekocode_ai session-command [session_id] stats
```

## 📈 パフォーマンス統計

### 実測データ（参考値）
| プロジェクト | ファイル数 | 処理時間 | スループット |
|-------------|-----------|---------|-------------|
| TypeScript Compiler | 735 | 1.9分 | 6.4 files/sec |
| .NET Runtime | 12,000+ | 32分 | 6.2 files/sec |
| Large React App | 3,400 | 8.5分 | 6.7 files/sec |

### ハードウェア要件
- **CPU**: 8コア以上推奨（--ssdモード）
- **RAM**: 8GB以上（10,000ファイル以上）
- **Storage**: SSD推奨（高速I/O）

## 🎪 高度な活用法

### 継続的解析
```bash
# 🔄 定期実行スクリプト例
#!/bin/bash
SESSION_ID=$(nekocode_ai session-create . --force --progress | jq -r '.session_id')
echo "Session created: $SESSION_ID"
echo "Progress file: sessions/${SESSION_ID}_progress.txt"
```

### Claude Code最適化
```bash
# 📊 統計のみ高速取得
nekocode_ai analyze project/ --stats-only

# 🎯 特定言語のみ解析
nekocode_ai analyze project/ --lang typescript

# 📦 コンパクト出力でネットワーク最適化
nekocode_ai analyze project/ --compact
```

## 🌟 まとめ

大規模プロジェクトでNekoCode AIを効率的に使用するコツ：

1. **事前確認必須**: `--check-only`で規模把握
2. **段階的解析**: 重要部分から先に解析  
3. **進捗監視**: `--progress`で状況把握
4. **ハードウェア最適化**: `--ssd`/`--hdd`で性能調整
5. **Claude Code配慮**: 長時間ブロックを避ける

大規模プロジェクトも怖くないにゃ！ 🚀✨