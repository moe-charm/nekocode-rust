# 🦀 NekoCode Project - Claude Context Information (Rust Edition)

## 📁 **重要：メインディレクトリ** (このディレクトリがメイン！)

```
nekocode-rust-clean/  # ✅ メインディレクトリ (GitHub同期済み)
├── src/              # 🦀 Rust + Tree-sitter実装
├── test-workspace/   # 🧪 テスト専用 (Git無視・861MB)
├── mcp-nekocode-server/  # 🔌 MCP統合
├── docs/             # 📚 ドキュメント
├── examples/         # 💡 サンプルコード
├── Cargo.toml        # 🦀 Rust設定
└── README.md         # 📖 プロジェクト概要
```

### **⚠️ 重要な変更（2025-08-11）**
- **メインディレクトリが変更になりました**: `nekocode-rust-clean/` がメイン開発ディレクトリです
- **GitHubリポジトリ**: `github.com/moe-charm/nekocode-rust.git` と同期済み
- **テストデータ**: `test-workspace/` (861MB) は.gitignoreで除外済み

## 📋 **プロジェクト概要**

**NekoCode Rust Edition** は16倍高速な多言語コード解析ツールです。**Tree-sitter統合により性能革命達成！**

### **基本情報**
- **主要実装**: 🦀 Rust + Tree-sitter (推奨・高速・高精度)
- **対応言語**: JavaScript, TypeScript, C++, C, Python, C#, Go, Rust（全8言語完全対応！）
- **特徴**: Claude Code最適化、MCP統合、セッション機能、16倍高速化

## 🆕 **Dead Code Detection機能追加完了！** (2025-08-18)

### **商用グレード精度達成**
```bash
# 完全解析 - セッション作成と同時にデッドコード検出
nekocode session-create /path/to/project --complete --external --format github-comment

# 既存セッションでデッドコード解析
nekocode deadcode SESSION_ID --external --min-confidence 85

# 結果例: 高精度検出
📊 15 dead code items found (90% confidence)
✅ clippy: 11件 (95%精度) - 未使用関数・変数
✅ cargo-machete: 4件 (85%精度) - 未使用依存関係
```

### **外部ツール統合**
- **cargo clippy** (Rust, 95%精度): 未使用関数・構造体・変数
- **cargo-machete** (Rust, 85%精度): Cargo.toml未使用依存関係
- **vulture** (Python, 90%精度): 未使用コード全般
- **内部解析** (全言語, 60%精度): 基本的な未参照検出

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

## 🔧 **最新のMCP修正完了！** (2025-08-11)

### **stats_only問題解決**
- 大規模プロジェクト解析時の126万行出力 → 149文字に圧縮（99.5%削減）
- Claude Codeのトークンオーバーフロー問題を解決
- `_extract_summary()`関数で統計サマリーのみ表示

### **MCP統合機能**
```bash
# Claude Codeから利用可能
mcp-nekocode-server/mcp_server_real.py
```

## 🧪 **テスト環境**

### ⚠️ **【絶対厳守】テスト場所の統一ルール**

```
nekocode-cpp-github/         # ルート
├── nekocode-rust-clean/     # このディレクトリ（GitHub同期）
└── test-workspace/          # 🚨 テストはここだけ！絶対安全！
    ├── test-real-projects/  # 実プロジェクトテストデータ
    │   ├── express/         # JavaScript - Express.js
    │   ├── typescript/      # TypeScript - MS TypeScript Compiler  
    │   ├── react/           # JavaScript/TypeScript - Facebook React
    │   ├── flask/           # Python - Flask Web Framework
    │   ├── django/          # Python - Django Framework
    │   ├── json/            # C++ - nlohmann/json
    │   ├── grpc/            # C++ - Google gRPC
    │   ├── nlog/            # C# - NLog Logging
    │   ├── gin/             # Go - Gin Web Framework
    │   ├── mux/             # Go - Gorilla Mux Router
    │   ├── serde/           # Rust - Serde Serialization
    │   └── tokio/           # Rust - Tokio Async Runtime
    └── test-files/          # 単体テストファイル
```

### 🚨 **絶対に守るべきルール**
1. **テストは `../test-workspace/` でのみ実行**
2. **このディレクトリ内にtest-workspace作成禁止**
3. **理由**: Git管理外で絶対にGitHubアップロードされない
4. **サイズ**: 871MB（でかくてもOK・安全優先）

### 🔥 **コマンド例（必ずこのパスを使用）**
```bash
# 必ず一個上のtest-workspaceを使用
./target/release/nekocode-rust analyze ../test-workspace/test-real-projects/express/
./target/release/nekocode-rust session-create ../test-workspace/test-real-projects/flask/
```

**🛡️ 安全性**: test-workspaceがGitリポジトリ外にあるため物理的に分離・絶対安全！

## ⚡ **使用方法**

### **Rust版（推奨・16倍高速！）**
```bash
# ビルド（3秒で完了）
cargo build --release

# 高速解析（必ず ../test-workspace/ を使用）
./target/release/nekocode-rust analyze ../test-workspace/test-real-projects/express/ --parser tree-sitter

# セッション作成（必ず ../test-workspace/ を使用）
./target/release/nekocode-rust session-create ../test-workspace/test-real-projects/flask/

# 🆕 デッドコード検出（完全解析）
./target/debug/nekocode session-create ../test-workspace/test-real-projects/serde/ --complete --external --format github-comment

# 🆕 既存セッションでデッドコード解析
./target/debug/nekocode deadcode SESSION_ID --external --min-confidence 85

# 性能比較（必ず ../test-workspace/ を使用）
./target/release/nekocode-rust analyze ../test-workspace/test-real-projects/typescript/ --benchmark
```

### **MCP経由（Claude Code）**
```bash
# stats_onlyで大規模プロジェクトも安全（パスは自動調整される）
nekocode-analyze(path: "../test-workspace/test-real-projects/typescript", stats_only: true)

# 🆕 デッドコード検出（Claude Code経由）
session-create(path: "../test-workspace/test-real-projects/serde", complete: true, external: true, format: "github-comment")
deadcode(session_id: "SESSION_ID", external: true, min_confidence: 85, format: "text")
```

## 🎯 **重要なファイル**

### **開発関連**
- `src/` - Rust実装（Tree-sitter統合）
- `Cargo.toml` - プロジェクト設定
- `mcp-nekocode-server/mcp_server_real.py` - MCP統合（修正済み）

### **ドキュメント**
- `README.md` - プロジェクト概要
- `docs/` - 詳細ドキュメント
- このファイル (`CLAUDE.md`) - Claude用コンテキスト

### **テスト**
- `test-workspace/` - テスト環境（Git無視）
- `examples/` - サンプルコード

## 📝 **Claude向けのメモ**

### **重要なコマンド**
```bash
# メインディレクトリに移動
cd nekocode-rust-clean

# ビルド
cargo build --release

# テスト実行（必ず ../test-workspace/ を使用）
./target/release/nekocode-rust analyze ../test-workspace/test-files/
```

### **注意点**
- **メインディレクトリ**: `nekocode-rust-clean/` を使用
- **GitHubリポジトリ**: `github.com/moe-charm/nekocode-rust.git` 
- **🚨 テストデータ**: `../test-workspace/` を絶対使用（Git管理外・物理分離）
- **MCPサーバー**: stats_only問題は修正済み
- **安全性**: test-workspaceがGitリポジトリ外にあるため絶対にアップロードされない

---
**最終更新**: 2025-08-11 15:15:00  
**作成者**: Claude + User collaborative design  
**状況**: 🛡️ **テストフォルダ統一完了！絶対安全なGit管理外配置！**