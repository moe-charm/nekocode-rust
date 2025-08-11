# 🐱 NekoCode Rust - 超高速コード解析エンジン | C++版より16倍高速 | Tree-sitter搭載

> 🚀 **革命的Rust実装** C++版より**16倍高速**なパフォーマンスを実現！  
> 🤖 **Claude Code最適化**: AI支援開発ワークフローに最適  
> 📊 **8言語対応**: JavaScript, TypeScript, C++, C, Python, C#, Go, Rust  
> 🎯 **超軽量**: わずか9MBのリポジトリ（他社200MB+との比較）！

[![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tree-sitter](https://img.shields.io/badge/Tree--sitter-green.svg)](https://tree-sitter.github.io/tree-sitter/)
[![AI Compatible](https://img.shields.io/badge/AI-対応-purple.svg)](https://github.com/moe-charm/nekocode-rust)
[![Multi Language](https://img.shields.io/badge/多言語対応-orange.svg)](https://github.com/moe-charm/nekocode-rust)
[![Build Status](https://img.shields.io/badge/ビルド-成功-brightgreen.svg)](https://github.com/moe-charm/nekocode-rust)
[![License: MIT](https://img.shields.io/badge/ライセンス-MIT-yellow.svg)](https://github.com/moe-charm/nekocode-rust/blob/main/LICENSE)

日本語 | [🇬🇧 English](README.md)

**作者**: CharmPic
- GitHub: [@moe-charm](https://github.com/moe-charm)
- Twitter: [@CharmNexusCore](https://x.com/CharmNexusCore)
- サポート: [☕ コーヒーを奢る](https://coff.ee/moecharmde6)

## 🚀 なぜNekoCode Rust？

### ⚡ **圧倒的パフォーマンス**
```bash
# TypeScriptコンパイラ（68ファイル）性能比較：
┌──────────────────┬────────────┬─────────────┐
│ パーサー         │ 時間       │ 速度        │
├──────────────────┼────────────┼─────────────┤
│ Rust Tree-sitter │    1.2秒   │ 🚀 16.38倍  │
│ C++ (PEGTL)      │   19.5秒   │ 1.00倍      │
│ Rust (PEST)      │   60.7秒   │ 0.32倍      │
└──────────────────┴────────────┴─────────────┘
```

### 🎯 **優れた検出精度**
```bash
# 検出比較（中規模JSファイル）：
┌──────────────────┬───────────┬──────────┬────────┐
│ パーサー         │ 関数      │ クラス   │ 合計   │
├──────────────────┼───────────┼──────────┼────────┤
│ Rust Tree-sitter │    20     │    2     │   22   │
│ Rust (PEST)      │    13     │    1     │   14   │
│ C++ (PEGTL)      │     4     │    2     │    6   │
└──────────────────┴───────────┴──────────┴────────┘
```

### 🛠️ **ビルド地獄なし**
```bash
# Rust版（天国 ✨）
cargo build --release  # 3秒で完了！

# vs C++版（地獄 💀）
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j16  # テンプレートエラー、依存地獄、5時間以上のデバッグ...
```

## 🔧 インストール

### 前提条件
- [Rust](https://rustup.rs/) （最新安定版）

### オプション1: ビルド済みバイナリを使用（即座に利用可能！）
```bash
git clone https://github.com/moe-charm/nekocode-rust.git
cd nekocode-rust/
./bin/nekocode_ai --help  # すぐ使える！
```

### オプション2: ソースからビルド（3秒！）
```bash
git clone https://github.com/moe-charm/nekocode-rust.git
cd nekocode-rust/
cargo build --release

# バイナリの場所: ./target/release/nekocode-rust
```

## 🚀 クイックスタート

### 基本的な解析
```bash
# JavaScript/TypeScriptプロジェクトを解析
./bin/nekocode_ai analyze src/

# またはRustバイナリを使用
./target/release/nekocode-rust analyze src/ --parser tree-sitter

# パーサー比較（PEST vs Tree-sitter）
./target/release/nekocode-rust analyze src/ --benchmark

# 特定言語の解析
./target/release/nekocode-rust analyze myfile.py --parser tree-sitter
./target/release/nekocode-rust analyze myfile.cpp --parser tree-sitter
```

### 高度な機能
```bash
# セッションベース解析
./bin/nekocode_ai session-create src/
# セッションID: 12345678

# AST解析
./bin/nekocode_ai session-command 12345678 ast-stats
./bin/nekocode_ai session-command 12345678 ast-query "MyClass::myMethod"

# コード編集（MCP統合）
./bin/nekocode_ai replace-preview file.js "oldCode" "newCode"
./bin/nekocode_ai moveclass-preview 12345678 MyClass target.js
```

## 🌟 主な機能

### 🚀 **超高速パフォーマンス**
- **Tree-sitter統合**: GitHubの最先端パーサー技術
- **並列処理**: 安全なRust並行処理で最大速度
- **増分解析**: 変更部分のみ再解析
- **メモリ効率**: Rustのゼロコスト抽象化

### 🎯 **多言語サポート**
```
🟨 JavaScript (.js, .mjs, .jsx, .cjs)
🔷 TypeScript (.ts, .tsx)  
🔵 C++ (.cpp, .cxx, .cc, .hpp, .hxx, .hh)
🔵 C (.c, .h)
🐍 Python (.py, .pyw, .pyi)
🟦 C# (.cs)
🐹 Go (.go)
🦀 Rust (.rs)
```

### 🧠 **AI最適化解析**
- **関数検出**: アロー関数、非同期関数を含む
- **クラス解析**: 継承、メソッド、プロパティ
- **依存関係マッピング**: インポート、エクスポート、モジュール関係
- **複雑度メトリクス**: 循環的複雑度、ネスト深度
- **AST操作**: クエリ、スコープ解析、構造ダンプ

### 🔧 **開発者フレンドリー**
- **セッション管理**: 永続的な解析セッション
- **コード編集**: プレビュー付きの置換、挿入、移動操作
- **メモリシステム**: 解析結果の保存/読み込み
- **MCP統合**: Claude Codeサーバーサポート
- **設定**: 柔軟な設定管理

## 📊 ベンチマーク

### 実世界のパフォーマンス
```bash
# TypeScriptコンパイラ（Microsoft）
# 68ファイル、合計約200KB
Rust Tree-sitter: 1.189秒 ⚡
C++ PEGTL:       19.477秒
Rust PEST:       60.733秒

# 検出精度: 
# 検出された関数: 1,000+（Tree-sitter） vs 200+（PEGTL）
```

## 🤖 Claude Code統合

NekoCode Rust EditionはAI支援開発に最適化されています：

```bash
# MCPサーバー統合
./bin/nekocode_ai session-create large-project/
# Claude Codeと使用してインテリジェントなコード解析

# 直接編集操作  
./bin/nekocode_ai replace-preview src/main.js "oldPattern" "newPattern"
./bin/nekocode_ai moveclass-preview session123 UserClass src/models/user.js
```

## 📚 コマンドリファレンス

### 解析コマンド
```bash
analyze <path>              # ファイル/ディレクトリを解析
languages                   # サポート言語一覧  
```

### セッション管理
```bash
session-create <path>       # 解析セッション作成
session-command <id> <cmd>  # セッションコマンド実行
```

### コード編集（MCP）
```bash
replace-preview <file> <pattern> <replacement>  # 置換プレビュー
replace-confirm <preview_id>                    # 置換確認
insert-preview <file> <line> <content>          # 挿入プレビュー
moveclass-preview <session> <class> <target>    # クラス移動プレビュー
```

### AST操作
```bash
ast-stats <session>         # AST統計
ast-query <session> <path>  # ASTノードクエリ
scope-analysis <session> <line>  # 行でのスコープ解析
ast-dump <session> [format] # AST構造ダンプ
```

## 🏆 なぜNekoCode Rustを選ぶのか？

### ✅ **パフォーマンスチャンピオン**
- C++実装より16倍高速
- 優れた検出精度
- Tree-sitterの最先端技術
- 並列処理の安全性

### ✅ **開発者体験**
- ワンコマンドビルド: `cargo build --release`
- 依存地獄なし、テンプレートエラーなし
- クロスプラットフォームコンパイル
- モダンなツールとパッケージング

### ✅ **将来性**
- Tree-sitter: GitHub、Neovim、Atomで使用
- Rust: 成長するエコシステム、メモリ安全性
- 活発な開発とモダンな機能
- AIファーストの設計思想

## 🗂️ リポジトリ構造

```
nekocode-rust/
├── src/
│   ├── analyzers/          # 言語別アナライザー
│   │   ├── javascript/     # JS/TS（Tree-sitter + PEST）
│   │   ├── python/         # Pythonアナライザー
│   │   ├── cpp/           # C++アナライザー  
│   │   └── ...            # その他の言語
│   ├── core/              # コア機能
│   │   ├── session.rs     # セッション管理
│   │   ├── memory.rs      # メモリシステム
│   │   └── ast.rs         # AST操作
│   └── main.rs            # CLIインターフェース
├── bin/
│   └── nekocode_ai        # ビルド済みバイナリ（6.6MB）
├── docs/                  # ドキュメント
└── mcp-nekocode-server/   # MCPサーバー統合
```

## 🤝 貢献

貢献を歓迎します！Rust版が現在の主要開発ターゲットです。

## 📄 ライセンス

MITライセンス - 詳細は[LICENSE](LICENSE)ファイルを参照してください。

## 👤 作者

**CharmPic**
- GitHub: [@moe-charm](https://github.com/moe-charm)
- プロジェクト: [github.com/moe-charm/nekocode-rust](https://github.com/moe-charm/nekocode-rust)
- Twitter: [@CharmNexusCore](https://x.com/CharmNexusCore)
- サポート: [☕ コーヒーを奢る](https://coff.ee/moecharmde6)

---

**🔥 16倍高速なコード解析を体験する準備はできましたか？**

```bash
# この超軽量リポジトリをクローン（9MB！）
git clone https://github.com/moe-charm/nekocode-rust.git
cd nekocode-rust/

# または、ビルド済みバイナリを使用（即座に利用可能！）
./bin/nekocode_ai analyze your-project/

# または、ソースからビルド（3秒！）
cargo build --release
./target/release/nekocode-rust analyze your-project/ --parser tree-sitter
```

**もうビルド地獄はありません。待ち時間もありません。ただ爆速の解析があるだけです。** 🚀🦀

---

**NekoCodeチームによって🐱で作られました**

*「革新的なコード解析を、光速で提供！」*

*「AI開発者に『な、なんだこれは！！』と言わせたツール」* 🔥