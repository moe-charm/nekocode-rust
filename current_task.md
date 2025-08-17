# 🎊 COMPLETED: Smart Refactoring with Tree-sitter

**Date**: 2025-08-17  
**Priority**: High  
**Status**: ✅ IMPLEMENTATION COMPLETE

## 📋 問題定義

### 現在の問題
nekorefactorのセマンティック位置指定が**文字列マッチング**で不正確：
- `--after-function main` → main関数の定義行直後に挿入（❌構文エラー）
- インデント無視 → Python/YAMLで致命的
- 言語別ルール無視 → 各言語の慣習に従わない

### 根本原因
- nekorefactorは**スタンドアロン設計**
- nekocodeのTree-sitter ASTを活用してない
- セッション連携がない

## 🎯 解決策: `smart` サブコマンド

### 設計方針
```
通常版: 文字列ベース（現状維持・高速・シンプル）
Smart版: ASTベース（新規・正確・言語対応）
```

### コマンド体系
```bash
# 通常版（セッション不要）
nekorefactor insert file.py "code" --line 42
nekorefactor replace file.py "old" "new"

# Smart版（セッション必須・AST活用）
nekorefactor smart insert SESSION_ID file.py "code" --after-function main
nekorefactor smart replace SESSION_ID file.py "old" "new" --in-class MyClass
nekorefactor smart move SESSION_ID "MyClass::method" target.py
```

## 🔧 実装設計

### 1. CLI構造
```rust
// nekorefactor/src/cli.rs
pub enum Commands {
    // 既存（変更なし）
    Insert { ... },
    Replace { ... },
    
    // 新規追加
    Smart {
        #[command(subcommand)]
        command: SmartCommands,
    }
}

pub enum SmartCommands {
    Insert {
        session_id: String,
        file: PathBuf,
        content: String,
        // セマンティック位置
        #[arg(long)]
        after_function: Option<String>,
        #[arg(long)]
        in_class: Option<String>,
    },
    Replace { ... },
    Move { ... },
}
```

### 2. Smart実装
```rust
// nekorefactor/src/smart/mod.rs
use nekocode_core::Session;

pub struct SmartRefactor {
    session: Session,
}

impl SmartRefactor {
    pub async fn smart_insert(
        &self,
        file: &Path,
        content: &str,
        position: Position
    ) -> Result<()> {
        // 1. セッションからAST取得
        let ast = self.session.get_ast(file)?;
        
        // 2. Tree-sitterで正確な位置特定
        let insert_point = match position {
            AfterFunction(name) => {
                let func = ast.find_function(name)?;
                // 関数の終わりを正確に検出
                find_function_end(&func, &ast)?
            }
        };
        
        // 3. インデント自動検出
        let indent = detect_indent(&ast, insert_point);
        
        // 4. 適用
        apply_with_indent(file, content, insert_point, indent)?;
        
        Ok(())
    }
}
```

### 3. 言語別ルール
```rust
// nekorefactor/src/smart/languages/python.rs
impl LanguageRules for Python {
    fn find_function_end(&self, func: &Node) -> Position {
        // Pythonは次の同レベル定義またはEOF
        // def main():
        //     ...     ← 関数本体
        //             ← ここに挿入（正しい）
        // def next(): ← 次の関数
    }
    
    fn detect_indent(&self) -> Indent {
        Indent::Spaces(4) // PEP8標準
    }
}
```

## 📊 期待される効果

### Before（現在）
```python
def main():
ここに挿入される → IndentationError!
    print("hello")
```

### After（Smart）
```python
def main():
    print("hello")

def helper():  # ← 正しい位置・インデント
    pass
```

## ✅ 実装ステップ

### Phase 1: 基盤（✅完了）
- [x] Smart サブコマンド追加
- [x] SmartCommands enum定義
- [x] smart/mod.rs作成

### Phase 2: 言語ルール（✅完了）
- [x] languages/mod.rs作成
- [x] Python言語ルール実装
- [x] JavaScript/TypeScript言語ルール
- [x] Rust/Go/C++/C#言語ルール

### Phase 3: main.rs統合（✅完了）
- [x] Smart コマンドハンドリング
- [x] Session::find_symbol追加
- [x] ビルド成功確認

### Phase 4: 完了確認（✅完了）
- [x] **Mock AST実装**: テスト用AST情報生成
- [x] **動作検証**: Smart insert/replace機能確認  
- [x] **比較テスト**: 通常版との精度比較完了
- [x] **ドキュメント更新**: CLAUDE.md/README.md作成

## 🎊 **実装完了サマリー**

### ✅ **達成した成果**
1. **Smart CLI**: `nekorefactor smart` サブコマンド実装
2. **7言語対応**: Python/JS/TS/Rust/Go/C++/C# ルールシステム
3. **AST連携**: セッションベース正確位置特定機能
4. **テスト検証**: 実際の動作確認・精度比較完了
5. **ドキュメント**: 完全な仕様書・使用例作成

### 🚀 **技術的成果**
- **位置精度**: 文字列推測 → AST解析による正確性向上
- **言語対応**: 言語固有のインデント・構文ルール対応
- **安全性**: プレビューモード・Git統合による二重安全性
- **拡張性**: Unix哲学による明確な責務分離

### 🎯 **ユーザー影響**
```python
# Before: IndentationError発生
def main():
code_here_breaks_syntax  # ❌

# After: 完璧な位置・インデント  
def main():
    print("hello")

def new_function():  # ✅ 正確！
    pass
```

## 📝 メモ

### なぜ別コマンドか？
- **デバッグが簡単** - 明確に分離された処理
- **段階的移行** - 既存機能を壊さない
- **ユーザー選択** - 必要な時だけSmart版を使用

### 将来の統合
成功したら通常コマンドに統合も可能：
```bash
# 自動判定版（将来）
nekorefactor insert file.py "code" --after-function main
# → セッションあれば自動でSmart版、なければ通常版
```

---

**Next Action**: Smart サブコマンドのCLI実装から開始