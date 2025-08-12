# 🦀 NekoCode - Ultra-fast Multi-language Code Analyzer

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tree-sitter](https://img.shields.io/badge/Tree--sitter-20232A?style=for-the-badge&logo=tree-sitter&logoColor=white)](https://tree-sitter.github.io/)
[![GitHub Actions](https://img.shields.io/badge/GitHub_Actions-2088FF?style=for-the-badge&logo=github-actions&logoColor=white)](https://github.com/features/actions)

> **16x faster than traditional parsers** • **8 languages supported** • **GitHub PR automation ready**

## 🚀 What NekoCode Does

- **⚡ Lightning-fast analysis**: Analyze 1000+ files in seconds using Tree-sitter
- **🔍 PR Impact Detection**: Automatically detect breaking changes in Pull Requests  
- **🤖 GitHub Actions Integration**: Auto-comment PR analysis results
- **🌐 Multi-language**: JavaScript, TypeScript, Python, C++, C#, Go, Rust, C
- **🔧 Advanced Features**: Sessions, AST queries, Claude Code integration

## 📦 Quick Start

### Installation
```bash
# Linux/macOS
curl -L https://github.com/moe-charm/nekocode-rust/releases/latest/download/nekocode-rust > nekocode
chmod +x nekocode

# Or build from source
cargo build --release
```

### Basic Usage
```bash
# Analyze a directory
./nekocode analyze src/

# Get detailed analysis
./nekocode analyze src/ --output json

# Analyze specific languages
./nekocode analyze . --type js
```

## 🎯 Core Features

### 1. **Code Analysis** (Core Feature)

**Supported Languages:**
- **JavaScript/TypeScript** - Functions, classes, imports/exports
- **Python** - Functions, classes, imports, decorators  
- **C/C++** - Functions, classes, includes, namespaces
- **C#** - Methods, classes, using statements, properties
- **Go** - Functions, structs, imports, interfaces
- **Rust** - Functions, structs, traits, modules

**What it detects:**
```bash
✅ Functions and methods with parameters
✅ Classes and structs with inheritance  
✅ Import/export dependencies
✅ Complexity metrics and line counts
✅ Cross-file references and calls
```

**Example Output:**
```json
{
  "functions": [
    {
      "name": "getUserById", 
      "line": 25,
      "parameters": ["id", "includeMetadata"],
      "complexity": 3
    }
  ],
  "references": [
    {"file": "api.js", "line": 15, "type": "call"}
  ]
}
```

### 2. **PR Impact Analysis** (GitHub Integration)

**Automatically detect breaking changes in Pull Requests:**

```bash
# Compare branches for breaking changes
./nekocode analyze-impact src/ --compare-ref master --format github-comment
```

**What it catches:**
- ❌ **Deleted functions** with existing references
- ⚠️ **Signature changes** that may break calls
- ✅ **New functions** (safe additions)
- 🔄 **Renamed functions** needing updates

**GitHub Actions Setup:**
```yaml
# .github/workflows/pr-analysis.yml
name: PR Impact Analysis
on: [pull_request]
jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Run NekoCode Analysis
      run: |
        ./nekocode analyze-impact src/ --compare-ref origin/${{ github.base_ref }} --format github-comment
```

**Auto-generated PR Comments:**
```markdown
🔍 **Impact Analysis Results**

⚠️ **BREAKING CHANGES DETECTED**
- `getUser()` function deleted (3 references found)
- `src/api.js:25` - calls getUser() ❌
- `src/order.js:18` - calls getUser() ❌

**Risk Level:** 🔴 High - Manual fixes required before merge
```

## 🔧 Advanced Features

### Session Management
```bash
# Create persistent analysis session
./nekocode session-create src/
./nekocode session-command <id> stats
./nekocode session-command <id> ast-query "MyClass::myMethod"
```

### AST Queries  
```bash
# Deep syntax tree analysis
./nekocode session-command <id> ast-stats
./nekocode session-command <id> scope-analysis 42
```

### Claude Code Integration
```bash
# MCP server for Claude Code
python mcp-nekocode-server/mcp_server_real.py
```

## 📊 Performance Comparison

| Parser | Time (TypeScript 68 files) | Speed vs PEGTL |
|--------|----------------------------|-----------------|
| 🦀 **NekoCode (Tree-sitter)** | **1.2s** | **16.38x faster** |
| C++ PEGTL | 19.5s | 1.00x baseline |
| Rust PEST | 60.7s | 0.32x slower |

## 🎮 Examples & Use Cases

### Use Case 1: Daily Development
```bash
# Before committing - check what changed
./nekocode analyze src/ --output json | jq '.functions | length'
# "Added 3 new functions, modified 2 existing"
```

### Use Case 2: PR Reviews
```bash
# Automated in GitHub Actions
# Reviewer sees: "⚠️ Breaking change: getUserData() deleted, 5 references found"
```

### Use Case 3: Refactoring Safety
```bash
# Before large refactor - baseline analysis
./nekocode analyze . > baseline.json

# After refactor - compare
./nekocode analyze-impact . --compare-ref baseline-commit
# Shows exactly what broke and needs fixing
```

## 🛠️ Installation & Setup

### Requirements
- **Rust 1.70+** (for building from source)
- **Git** (for PR analysis features)  
- **GitHub CLI** (optional, for GitHub Actions)

### Build from Source
```bash
git clone https://github.com/moe-charm/nekocode-rust.git
cd nekocode-rust
cargo build --release
./target/release/nekocode-rust --help
```

### GitHub Actions Integration
1. **Copy binary to your repository**
2. **Create `.github/workflows/pr-analysis.yml`** (see example above)
3. **Set repository permissions**: Settings → Actions → Read and write permissions

## 🤝 Contributing

1. **Report issues**: Especially for language parsing edge cases
2. **Test new languages**: Add grammar files for additional languages  
3. **Improve accuracy**: Help enhance PR impact detection
4. **Add integrations**: VS Code extensions, CI/CD plugins

## 👤 Author & Support

**Created by CharmPic** 🐱

- 🐙 **GitHub**: [@moe-charm](https://github.com/moe-charm)
- 🐦 **Twitter**: [@CharmNexusCore](https://x.com/CharmNexusCore)
- ☕ **Support**: [Buy me a coffee](https://buymeacoffee.com/moecharmde6)

*If NekoCode helps your development workflow, consider supporting the project!*

## 📄 License

MIT License - feel free to use in commercial projects.

---

## 🌏 日本語 (Japanese)

<details>
<summary>🎌 日本語版README (クリックして展開)</summary>

# 🦀 NekoCode - 超高速多言語コード解析ツール

> **従来パーサーの16倍高速** • **8言語対応** • **GitHub PR自動化対応**

## 🚀 NekoCodeができること

- **⚡ 超高速解析**: Tree-sitterで1000+ファイルを秒単位で解析
- **🔍 PR影響検出**: プルリクエストの破壊的変更を自動検出
- **🤖 GitHub Actions統合**: PRに分析結果を自動コメント投稿
- **🌐 多言語対応**: JavaScript、TypeScript、Python、C++、C#、Go、Rust、C
- **🔧 高度機能**: セッション、AST、Claude Code統合

## 📦 クイックスタート

### インストール
```bash
# Linux/macOS
curl -L https://github.com/moe-charm/nekocode-rust/releases/latest/download/nekocode-rust > nekocode
chmod +x nekocode

# またはソースからビルド
cargo build --release
```

### 基本的な使用方法
```bash
# ディレクトリを解析
./nekocode analyze src/

# 詳細な解析結果
./nekocode analyze src/ --output json

# 特定言語のみ解析
./nekocode analyze . --type js
```

## 🎯 主要機能

### 1. **コード解析** (コア機能)

**対応言語:**
- **JavaScript/TypeScript** - 関数、クラス、import/export
- **Python** - 関数、クラス、import、デコレータ
- **C/C++** - 関数、クラス、include、namespace
- **C#** - メソッド、クラス、using、プロパティ
- **Go** - 関数、構造体、import、interface
- **Rust** - 関数、構造体、trait、モジュール

### 2. **PR影響分析** (GitHub統合)

**プルリクエストの破壊的変更を自動検出:**

```bash
# ブランチ間の破壊的変更を比較
./nekocode analyze-impact src/ --compare-ref master --format github-comment
```

**検出する内容:**
- ❌ **削除された関数** (既存の参照あり)
- ⚠️ **シグネチャ変更** (呼び出しが壊れる可能性)
- ✅ **新規関数** (安全な追加)
- 🔄 **関数名変更** (更新が必要)

### GitHub Actions設定例
```yaml
# .github/workflows/pr-analysis.yml
name: PR Impact Analysis
on: [pull_request]
jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: NekoCode解析実行
      run: |
        ./nekocode analyze-impact src/ --compare-ref origin/${{ github.base_ref }} --format github-comment
```

**自動生成されるPRコメント:**
```markdown
🔍 **影響分析結果**

⚠️ **破壊的変更を検出**
- `getUser()` 関数が削除されました (3箇所で参照)
- `src/api.js:25` - getUser()を呼び出し ❌
- `src/order.js:18` - getUser()を呼び出し ❌

**リスクレベル:** 🔴 高 - マージ前に手動修正が必要
```

## 🔧 高度機能

### セッション管理
```bash
# 永続的な解析セッション作成
./nekocode session-create src/
./nekocode session-command <id> stats
./nekocode session-command <id> ast-query "MyClass::myMethod"
```

### ASTクエリ
```bash
# 構文木の詳細分析
./nekocode session-command <id> ast-stats
./nekocode session-command <id> scope-analysis 42
```

### Claude Code統合
```bash
# Claude Code用MCPサーバー
python mcp-nekocode-server/mcp_server_real.py
```

## 📊 性能比較

| パーサー | 時間 (TypeScript 68ファイル) | PEGTL比 |
|---------|----------------------------|---------|
| 🦀 **NekoCode (Tree-sitter)** | **1.2秒** | **16.38倍高速** |
| C++ PEGTL | 19.5秒 | 1.00倍 |
| Rust PEST | 60.7秒 | 0.32倍 |

## 👤 作者・サポート

**作者: CharmPic** 🐱

- 🐙 **GitHub**: [@moe-charm](https://github.com/moe-charm)
- 🐦 **Twitter**: [@CharmNexusCore](https://x.com/CharmNexusCore)  
- ☕ **サポート**: [Buy me a coffee](https://buymeacoffee.com/moecharmde6)

*NekoCodeがあなたの開発を助けているなら、プロジェクトのサポートをご検討ください！*

</details>

---

**Made with 🦀 Rust and ❤️ for developers worldwide**