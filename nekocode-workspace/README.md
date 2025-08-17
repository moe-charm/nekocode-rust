# 🚀 NekoCode Workspace

**Unix哲学に基づく高速多言語コード解析ツールチェーン**

## ✨ 新機能ハイライト

### 🎊 **Smart Refactoring** (2025-08-17実装完了)

Tree-sitter AST解析による**革命的な正確性**を実現！

```bash
# セマンティック位置指定 - 関数の正確な終わりに挿入
nekorefactor smart insert SESSION_ID file.py "def helper():\n    pass" --after-function main

# スコープ限定置換 - クラス内のみ置換
nekorefactor smart replace SESSION_ID file.py "value" "new_value" --in-class MyClass  

# シンボル移動 - 依存関係も自動更新
nekorefactor smart move SESSION_ID "MyClass::method" target.py --update-imports
```

#### 🎯 **精度比較**

| 機能 | 通常版（文字列） | Smart版（AST） |
|------|-----------------|---------------|
| 位置精度 | 推測ベース | セマンティック正確 |
| インデント | 手動指定 | 言語別自動検出 |
| 速度 | 🚀 高速 | ⚡ 中速（高精度） |
| セッション | 不要 | 必須 |

## 🏗️ アーキテクチャ

```
├── nekocode-core/     # 📦 共通ライブラリ・型システム
├── nekocode/          # 🔍 Tree-sitter解析エンジン  
├── nekorefactor/      # 🔧 Smart+通常リファクタリング ⭐NEW!
├── nekoimpact/        # 📊 変更影響度解析
└── nekoinc/           # ⚡ インクリメンタル解析
```

## 🚀 クイックスタート

### 1. ビルド
```bash
cargo build --release
```

### 2. セッション作成
```bash
./target/debug/nekocode session-create /path/to/project
# 出力: ✅ Created session: 12345678
```

### 3. Smart リファクタリング
```bash
# Python関数の後に新しい関数を挿入
./target/debug/nekorefactor smart insert 12345678 main.py \
  "def helper():\n    \"\"\"Helper function\"\"\"\n    return True" \
  --after-function main

# プレビューモード
./target/debug/nekorefactor smart insert 12345678 main.py "code" \
  --after-function main --preview
```

## 🌍 対応言語

**Smart Refactoring対応**: 7言語完全対応
- **Python**: PEP 8準拠（4スペース）
- **JavaScript/TypeScript**: 2スペース標準
- **Rust**: 4スペース・impl block対応
- **Go**: タブインデント・package対応 
- **C++/C#**: 4スペース・class対応

## 📚 ドキュメント

- [`CLAUDE.md`](./CLAUDE.md) - Claude向け詳細仕様
- [`current_task.md`](../current_task.md) - 開発タスク状況
- [`completed_tasks.md`](../completed_tasks.md) - 完了機能一覧

## 🎯 設計思想

### Unix哲学
- **Do One Thing Well**: 各ツールは単一責務
- **Composability**: ツール間連携による柔軟性
- **Simplicity**: 明確なインターフェース

### 安全性優先
- **Git統合**: `git restore`で即座に復元可能
- **プレビューモード**: 全操作で事前確認可能
- **段階的移行**: 既存機能を壊さない設計

## 🔧 開発者向け

### コマンド一覧
```bash
# Smart版（セッション必須・AST活用）
nekorefactor smart insert SESSION_ID file content --after-function func
nekorefactor smart replace SESSION_ID file old new --in-class Class
nekorefactor smart move SESSION_ID "Class::method" target.py

# 通常版（高速・文字列ベース）  
nekorefactor insert file content --line 42
nekorefactor replace file old new --regex
nekorefactor move-lines src.js 10 5 dest.js 20
```

### テストコマンド
```bash
# セッション作成
./target/debug/nekocode session-create /tmp/test_project

# Smart機能テスト
./target/debug/nekorefactor smart insert SESSION_ID test.py "print('test')" --after-function main --preview
```

---

**🎊 世界最高速クラスの多言語解析ツールチェーン** - Unix哲学 × Tree-sitter × Rust の完璧な融合

**Status**: 🚀 Production Ready  
**License**: MIT  
**Language**: Rust 🦀