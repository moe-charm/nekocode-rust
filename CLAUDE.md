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

### **test-workspace/ (861MB, Git無視)**
```
test-workspace/
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
└── test-files/       # 単体テストファイル
```

**重要**: これらは性能テスト・検証用の重要データです。削除厳禁！

## ⚡ **使用方法**

### **Rust版（推奨・16倍高速！）**
```bash
# ビルド（3秒で完了）
cargo build --release

# 高速解析
./target/release/nekocode-rust analyze test-workspace/test-real-projects/express/ --parser tree-sitter

# セッション作成  
./target/release/nekocode-rust session-create test-workspace/test-real-projects/flask/

# 性能比較
./target/release/nekocode-rust analyze test-workspace/test-real-projects/typescript/ --benchmark
```

### **MCP経由（Claude Code）**
```bash
# stats_onlyで大規模プロジェクトも安全
nekocode-analyze(path: "test-workspace/test-real-projects/typescript", stats_only: true)
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

# テスト実行（小さいプロジェクトで）
./target/release/nekocode-rust analyze examples/
```

### **注意点**
- **メインディレクトリ**: `nekocode-rust-clean/` を使用
- **GitHubリポジトリ**: `github.com/moe-charm/nekocode-rust.git` 
- **大容量テストデータ**: `test-workspace/` はGit追跡対象外
- **MCPサーバー**: stats_only問題は修正済み

---
**最終更新**: 2025-08-11 14:45:00  
**作成者**: Claude + User collaborative design  
**状況**: 🎉 **メインディレクトリ移行完了！MCP修正も適用済み！**