# 🎯 Current Task: Smart Refactoring with Tree-sitter

**Date**: 2025-08-17  
**Priority**: High  
**Status**: 🚀 Design Phase

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

### Phase 1: 基盤（1日）
- [ ] Smart サブコマンド追加
- [ ] SmartCommands enum定義
- [ ] smart/mod.rs作成

### Phase 2: セッション連携（1日）
- [ ] nekocode-coreからSession import
- [ ] セッション読み込み実装
- [ ] AST取得インターフェース

### Phase 3: Smart Insert（2日）
- [ ] smart_insert関数実装
- [ ] find_function_end実装
- [ ] detect_indent実装

### Phase 4: 言語対応（2日）
- [ ] Python言語ルール
- [ ] TypeScript言語ルール
- [ ] Rust言語ルール

### Phase 5: テスト（1日）
- [ ] 単体テスト
- [ ] 統合テスト
- [ ] 実プロジェクトでの検証

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