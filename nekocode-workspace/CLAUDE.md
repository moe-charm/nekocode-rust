# 🚀 NekoCode Workspace - 5分割Unix哲学ツールチェーン

## 📁 **アーキテクチャ概要** 

```
nekocode-workspace/           # 🎯 メインワークスペース
├── nekocode-core/           # 📦 共通ライブラリ基盤 (10.1MB)
├── nekocode/                # 🔍 解析エンジン (67.8MB) 
├── nekorefactor/            # 🔧 リファクタリング (51.4MB) ⭐NEW!
├── nekoimpact/              # 📊 影響度解析 (51.2MB)
├── nekoinc/                 # ⚡ インクリメンタル (57.8MB)
└── Cargo.toml               # 🦀 Rustワークスペース設定
```

## 🎯 **5分割Unix哲学ツールチェーン**

### **2. nekorefactor** - 🔧 **革新的リファクタリング** ⭐NEW! 

#### **🌟 Smart Refactoring（AST活用・2025-08-17実装完了）**
```bash
# Smart Insert - セマンティック位置指定
./target/debug/nekorefactor smart insert SESSION_ID file.py "def new_func():\n    pass" --after-function main

# Smart Replace - スコープ限定  
./target/debug/nekorefactor smart replace SESSION_ID file.py "old" "new" --in-class MyClass

# Smart Move - シンボル移動
./target/debug/nekorefactor smart move SESSION_ID "MyClass::method" target.py

# プレビューモード
./target/debug/nekorefactor smart insert SESSION_ID file.py "code" --after-function main --preview
```

**🎯 Smart版の革新的精度:**
```python
# Before (通常版・文字列マッチング)
def main():
    print("hello")
code_inserted_here_causes_IndentationError  # ❌ 構文エラー！

# After (Smart版・AST解析)  
def main():
    print("hello")

def new_function():  # ✅ 正確な位置・インデント！
    pass
```

#### **通常版（文字列ベース・高速）**
```bash
# 即適用（デフォルト）
./target/debug/nekorefactor insert file.py "code" --line 42
./target/debug/nekorefactor replace file.js "oldName" "newName"

# プレビューモード
./target/debug/nekorefactor insert file.py "code" --after-function main --preview

# 行移動・クラス移動
./target/debug/nekorefactor move-lines src.js 10 5 dest.js 20
./target/debug/nekorefactor move-class SESSION_ID SYMBOL_ID target.js
```

#### **✅ 検証済み機能（2025-08-17テスト完了）**
- **Smart版**: AST解析による正確な位置特定・7言語対応ルール
- **通常版**: 高速文字列マッチング・即適用デフォルト  
- **二重安全**: Gitとプレビューモードによる安全性確保
- **言語対応**: Python/JS/TS/Rust/Go/C++/C# 言語別インデント

### **1. nekocode** - 🔍 **核心解析エンジン** (67.8MB)
```bash
# セッション管理（Smart機能で必須）
./target/debug/nekocode session-create /path/to/project/
./target/debug/nekocode session-list --detailed

# 基本解析
./target/debug/nekocode analyze /path/to/file.js
```

### **3. nekoimpact** - 📊 **変更影響度解析** (51.2MB)
```bash
# GitHub Actions最適化
./target/debug/nekoimpact analyze SESSION_ID --format github-comment
```

### **4. nekoinc** - ⚡ **高速インクリメンタル解析** (57.8MB)
```bash
# インクリメンタル更新
./target/debug/nekoinc update SESSION_ID --verbose
```

## 🎊 **2025-08-17達成済み成果**

### **Smart Refactoring完全実装**
- ✅ **Smart CLI**: `nekorefactor smart` サブコマンド
- ✅ **7言語ルール**: Python/JS/TS/Rust/Go/C++/C#対応
- ✅ **AST連携**: セッションベース正確位置特定  
- ✅ **テスト検証**: 実動作確認完了

### **設計革命**
- **Unix哲学**: 1機能1ツール・明確責務分離
- **段階的移行**: 既存機能を壊さない安全設計
- **選択の自由**: 必要時のみSmart版・通常は高速版

## 🔧 **開発ガイドライン**

### **ビルド**
```bash
# 全バイナリビルド
cargo build --release

# 個別バイナリビルド
cargo build --bin nekorefactor
```

### **テスト実行**
```bash
# セッション作成
./target/debug/nekocode session-create /tmp/test_project

# Smart機能テスト  
./target/debug/nekorefactor smart insert SESSION_ID file.py "code" --after-function main --preview
```

---

**🎊 NekoCode Workspace**: Unix哲学による**世界最高速クラス多言語解析ツールチェーン**完成！

**最終更新**: 2025-08-17 Smart Refactoring実装完了  
**ステータス**: 🚀 **商用グレード品質達成**