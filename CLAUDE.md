# 🦀 NekoCode Project - Claude Context Information (Rust Edition)

## 📁 **重要：フォルダ構造と使い分け** (必読！)

```
nekocode-cpp-github/  (このディレクトリ)
├── nekocode-rust-clean/   # ✅ GitHub用 (11MB) - ここからプッシュ！
│   └── ⚠️ GitHubリポジトリと同期中 (github.com/moe-charm/nekocode-rust)
├── nekocode-rust/         # 🔧 開発用 (586MB) - 開発はここで！
│   └── target/ (ビルド結果)
├── test-workspace/        # 🧪 テスト専用 (871MB) - Git無視
│   ├── test-real-projects/  # 実プロジェクト性能テスト
│   └── test-clone/          # クローンテスト
└── nyash/                 # 🐱 別プロジェクト (827MB)
```

### **⚠️ 作業場所の使い分け**
- **開発作業**: `nekocode-rust/` で開発・テスト
- **GitHubプッシュ**: `nekocode-rust-clean/` からのみ！
- **性能テスト**: `test-workspace/` のプロジェクトで

## 📋 **プロジェクト概要**

**NekoCode Rust Edition** は16倍高速な多言語コード解析ツールです。**Tree-sitter統合により性能革命達成！**

### **基本情報**
- **主要実装**: 🦀 Rust + Tree-sitter (推奨・高速・高精度)
- **レガシー**: C++17, PEGTL, CMake (参考実装のみ)
- **対応言語**: JavaScript, TypeScript, C++, C, Python, C#, Go, Rust（全8言語完全対応！）
- **特徴**: Claude Code最適化、MCP統合、セッション機能、16倍高速化

## 🚀 **Rust Edition完全移行完了！** (2025-08-11)

### **性能革命達成**
```bash
# TypeScript Compiler (68 files) 性能比較:
┌──────────────────┬────────────┬─────────────┐
│ Parser           │ Time       │ Speed       │
├──────────────────┼────────────┼─────────────┤
│ 🦀 Rust Tree-sitter │    1.2s    │ 🚀 16.38x   │
│ C++ (PEGTL)      │   19.5s    │ 1.00x       │
│ Rust (PEST)      │   60.7s    │ 0.32x       │
└──────────────────┴────────────┴─────────────┘
```

### **検出精度向上**
- Rust Tree-sitter: 20関数, 2クラス検出
- Rust PEST: 13関数, 1クラス検出  
- C++ PEGTL: 4関数, 2クラス検出

### **開発効率革命**
- **ビルド**: `cargo build --release` (3秒) vs C++ make地獄 (5時間デバッグ)
- **依存管理**: Cargo.toml vs CMake/vcpkg地獄
- **エラー**: Rustの親切なエラーメッセージ vs C++テンプレートエラー地獄

## ✅ **Universal AST 本来設計の復活完了！** (2025-08-07)

### **本来の設計（復活完了）**
Universal AST Adapterは**既存の成熟したPEGTLアナライザーを呼び出し**、その結果を統一フォーマットに変換する設計です。

```cpp
// 全言語で統一された実装パターン
class LanguageUniversalAdapter {
    std::unique_ptr<LanguagePEGTLAnalyzer> legacy_analyzer;
    
    AnalysisResult analyze() {
        // Step 1: 成熟したPEGTL解析を使用
        AnalysisResult result = legacy_analyzer->analyze();
        // Step 2: 統一AST構築
        build_unified_ast_from_legacy_result(result);
        return result;
    }
};
```

### **修正完了状況**
- ✅ JavaScript: 元から正しく実装済み（PEGTLを呼んでいる）
- ✅ Python: 修正完了（PythonPEGTLAnalyzer呼び出しに変更）
- ✅ C++: 修正完了（CppPEGTLAnalyzer呼び出しに変更）
- ✅ C#: 修正完了（CSharpPEGTLAnalyzer呼び出しに変更）
- ✅ Go: 修正完了（GoAnalyzer呼び出しに変更）
- ✅ Rust: 修正完了（RustAnalyzer呼び出しに変更）

**環境変数無効化**: `NEKOCODE_USE_UNIVERSAL_AST`チェックを削除し、Universal ASTがデフォルトになりました

## ⚠️ **重要：役割分担の明確化**

### **通常解析（基本機能）**
- **Universal AST Adapters**（デフォルト）: 言語統一アーキテクチャ
  - 内部でPEGTL Analyzersを呼び出して高精度解析
  - JavaScriptPEGTLAnalyzer、PythonPEGTLAnalyzer等を活用
  - スタンドアロンで動作、外部ツール不要
  - **デッドコード検出はしない**（通常の解析のみ）

### **デッドコード検出（--complete、オマケ機能）**
- **universal_deadcode_analyzer.py**: 外部ツール呼び出し
  - `src/tools/universal_deadcode_analyzer.py`
  - clang-tidy、vulture等の外部ツール必須
  - 通常解析とは完全に独立

**注意**: "Universal AST"と"universal_deadcode_analyzer"は名前が似ているが全く別物！

## 🏗️ **実装済みコンポーネント**

### **Universal Framework** 
- `src/universal/` - 統一アーキテクチャ実装済み
  - UniversalTreeBuilder<LanguageTraits>
  - UniversalCodeAnalyzer<Grammar, Adapter>
  - Language Traits Pattern

### **言語アダプター** 
- `src/adapters/` - 各言語の固有処理
  1. JavaScript/TypeScript (AST完全対応)
  2. Python (統一済み)
  3. C++ (統一済み)
  4. C# (統一済み)
  5. Go, Rust (統一済み)

## 📊 **既存コード再利用マップ**

### 🟢 **完全再利用 (33% - 10ファイル)**
- `src/core/` - Session管理、統計処理 → **変更なし**
- `src/utils/` - ファイル処理、UTF8処理 → **変更なし**  
- `src/formatters/` - 出力フォーマット → **変更なし**

### 🟡 **部分再利用 (50% - 15ファイル)**
- 各言語analyzer - パターンマッチング部分抽出
- base_analyzer.hpp - インターフェース活用

### ✅ **統一完了 (100% - 全ファイル)**
- src/universal/ - 統一アーキテクチャ実装済み
- src/adapters/ - 言語別アダプター実装済み

## 🎯 **重要なファイル**

### **現在のAST実装** (参考用)
- `src/analyzers/javascript/javascript_pegtl_analyzer.hpp` - 既存AST実装
- `include/nekocode/types.hpp` - ASTNode定義済み

### **進捗管理**
- `current_task.md` - 現在のタスク詳細
- このファイル (CLAUDE.md) - プロジェクト全体把握

### **統一システム**
- `src/universal/` - 実装済み統一システム
- `src/adapters/` - 言語別アダプター

## 💡 **技術的ポイント**

### **AST Revolution の核心**
```cpp
// 既存: 言語別に重複実装
JavaScriptAnalyzer::extract_functions_from_line()
PythonAnalyzer::extract_functions()
CppAnalyzer::extract_functions()

// 新設計: 99%共通化
template<typename Lang>
UniversalAnalyzer<Lang>::analyze() {
    // 共通処理 + 言語固有アダプター
}
```

### **現在利用可能なAST機能** (JavaScript/TS)
```bash
# 既に動作中のAST機能（JS/TS専用）
./nekocode_ai session-command <id> ast-stats
./nekocode_ai session-command <id> ast-query <path>
./nekocode_ai session-command <id> scope-analysis <line>
./nekocode_ai session-command <id> ast-dump [format]
```

## 🔄 **進捗状況**

### **完了済み**
- [x] AST Revolution機能実装（全言語対応）
- [x] Universal AST Revolution完了
- [x] MCP統合完了
- [x] ドキュメント更新完了
- [x] 大規模リファクタリング完了
- [x] 統一アーキテクチャ実装完了
- [x] 全言語アダプター実装完了
- [x] **🚀 MoveClass機能完了！** - 全8言語対応
- [x] **🧪 大規模テスト完了** - 1.4GB実プロジェクトで検証

### **現在の状況** 
- **✅ 全機能完成！** MoveClass, AST Revolution, MCP統合すべて動作確認済み
- **🎯 本格運用開始** - Claude Codeでの日常的な利用に最適化済み
- **📚 ドキュメント完備** - READMEとMCPドキュメント最新化完了

---

## 📝 **Claude向けのメモ**

### **重要なコマンド**
```bash
# プロジェクトのビルド
cd build && make -j$(nproc)

# テスト実行  
./bin/nekocode_ai session-create test.js
./bin/nekocode_ai session-command <id> ast-stats

# 進捗確認
cat current_task.md
```

### **注意点**
- ビルドエラーが出たら即座に報告
- current_task.md を定期的に更新
- MCP統合は完了済み

---

## ⚠️ **重要：テストフォルダについて（削除厳禁！）**

### **🧪 test-workspace/ フォルダ (871MB, Git無視)**
**📂 現在の場所**: `test-workspace/` に統合済み！

```
test-workspace/  # すべてのテストを集約
├── test-real-projects/  # 実プロジェクトテストデータ
│   ├── express/      # JavaScript - Express.js
│   ├── typescript/   # TypeScript - MS TypeScript Compiler  
│   ├── react/        # JavaScript/TypeScript - Facebook React
│   ├── flask/        # Python - Flask Web Framework
│   ├── django/       # Python - Django Framework
│   ├── json/         # C++ - nlohmann/json
│   ├── grpc/         # C++ - Google gRPC
│   ├── nlog/         # C# - NLog Logging
│   ├── gin/          # Go - Gin Web Framework
│   ├── mux/          # Go - Gorilla Mux Router
│   ├── serde/        # Rust - Serde Serialization
│   └── tokio/        # Rust - Tokio Async Runtime
├── test-clone/       # クローンテスト用
└── test-files/       # 単体テストファイル
```

**⚠️ 絶対に削除しないでください！理由：**
- 大規模パフォーマンステスト用の重要データ
- C++ vs Rust性能比較のベンチマーク環境
- 各言語の実際のプロジェクトでAST機能検証
- .gitignoreに登録済み（git追跡対象外）
- 各プロジェクトは`--depth=1`でクローン済み（最小サイズ）

**📂 現在の場所**: `test-workspace/test-real-projects/` に移動済み

**推奨使用方法：**
```bash
# 🦀 Rust版（推奨・16倍高速！）
cd nekocode-rust
cargo build --release  # 3秒でビルド完了！

# 高速解析 (test-workspaceのプロジェクトを解析)
./target/release/nekocode-rust analyze ../test-workspace/test-real-projects/express/ --parser tree-sitter

# セッション作成  
./target/release/nekocode-rust session-create ../test-workspace/test-real-projects/flask/

# 性能比較
./target/release/nekocode-rust analyze ../test-workspace/test-real-projects/typescript/ --benchmark

# C++版テスト（レガシー・参考のみ）  
cd ..
./bin/nekocode_ai analyze test-workspace/test-real-projects/express/
```

---
**最終更新**: 2025-08-11 09:00:00  
**作成者**: Claude + User collaborative design  
**状況**: 🚀 **Rust Edition完全移行完了！Tree-sitter統合で16倍高速化達成！**