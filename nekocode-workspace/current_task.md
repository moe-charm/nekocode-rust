# 📌 Current Task — Rust-first 再構築（2026-08-23）

> この節が現在の正規方針です。以下に残る2025年の計画は履歴であり、矛盾する項目（analyze復活、多言語同時拡張、未検証の精度宣言など）は実行しません。

## 目的

Rustの公式・定番ツールを正しさの情報源として使い、NekoCodeは永続スナップショット、Git差分、根拠付きAI/MCPコンテキストの提供に集中する。

## 決定事項

- Rustを唯一のTier 1対応言語とする。他言語はgolden fixture整備後のexperimental backend。
- 正規MVPは `index PATH`、`context PATH --compare-ref REF --budget N`。`query SYMBOL`はsemantic backend後に追加する。
- 旧単一バイナリ版と5バイナリ版を同時に正規実装として扱わない。
- 未測定の60%/90%/95%精度や「完全対応」「商用グレード」表記を使わない。
- 独自dead-code判定、smart refactor、split、strip-comments、watch、security/quality/circularは凍結する。
- 編集機能は、意味解析・atomic patch・`cargo check/test`・rollbackが揃うまで読み取り専用とする。

## 実行順序

1. 現在の実装をlegacyブランチ/タグとして固定する。
2. 正規workspace、README、CI、Makefile、Releaseを一本化する。
3. Rust fixtureと回帰テストを作る（trait、impl、macro、cfg、feature、workspace、tests/examples、同名シンボル）。
4. Rust MVPを実装し、test/fmt/Clippy/CLI smoke testを通す。
5. Rustの昇格ゲート通過後、PythonまたはJavaScriptを一言語ずつ追加する。

## 物理アーカイブ計画

- 現在は論理アーカイブ段階。旧root package・旧5バイナリを移動/削除せず、canonicalをこのworkspaceに限定する。
- Rust MVPのJSON schema、golden fixture、CLI smoke testが安定したら、作業ツリーをcommitし`legacy-2026-pre-rust-first`タグ（または専用branch）を作る。
- release/MCP/setup scriptの旧パス参照を検査し、依存がないことを確認してから旧実装を`archive/legacy`または別branchへ移す。
- 移動後もclean checkoutで`cargo test`・`cargo check`・CLI smokeを実行し、問題があればタグから復旧する。

物理移動の完了までは、旧実装を「保守対象」ではなく「復旧用legacy」として扱う。

## 証拠レベル

出力には `tool-confirmed`、`semantic-resolved`、`syntax-only`、`incomplete` と、toolchain/features/target/workspace/失敗情報を記録する。

## 作業状態

- Phase 0（方針とcurrent_taskの更新）: 完了
- Phase 1（リポジトリ一本化とRust評価基盤）: 完了（Cargo/Gitコンテキスト基盤、統合テスト、Rust-first CIを追加済み）
- Phase 2（Rust MVP）: index/contextの最小入口とCargo features/toolchain provenanceを実装済み。semantic backendは未着手
- Phase 3（追加言語）: Rust昇格ゲート通過後

検証メモ: workspace test、core/CLI check、index/context smokeは通過。workspace全体のfmt/`clippy -D warnings`はlegacyコードの既存違反で未達のため、Rust昇格ゲートまでに分離・整理する。

アーカイブ状態: Stage A（論理アーカイブ・canonical固定）完了。Stage B（tag/clean checkout/dependency audit）とStage C（物理移動）はRust MVPゲート後。

監査状態: root旧package、5-binary配布、MCPの依存監査を完了。golden fixtureを追加済み。物理移動前に旧導線の切替が必要。

---

# 🚨 Legacy Task History - NekoCode 緊急修正タスク - 2025-08-25

## 📋 発見された重大な問題点

### 1. ✅ **analyzeコマンドは作らない（設計思想）**
**決定**: analyzeコマンドは設計思想に反するため**作らない**
```bash
# ❌ 絶対NG - analyzeは存在しない
nekocode analyze /path/to/file  # 導線がめちゃくちゃになる！

# ✅ 正しい導線 - 必ずセッション作成から
nekocode session-create /path/to/file
```
**対応**: MCPサーバー側を修正して`analyze`を削除
**ステータス**: ✅ 方針決定済み

### 2. ❌ **セッションIDの存在自体がUXのバグ**
**問題**: ユーザーがセッションIDを覚えて毎回指定する必要がある
```bash
# 現状の面倒な流れ
nekocode session-create /project → "Session: abc123"
nekocode ast-stats abc123  # ID必須
nekocode deadcode abc123   # ID必須
```
**理想**: Gitのように自然に使える（ID意識不要）
**影響**: 使いにくさの根本原因
**優先度**: 🔴 最高

### 3. ❌ **list_languagesの言語リスト不完全**
**問題**: MCPが返す「JS/TS/C++/C/Python/C#」にRust/Goが含まれていない
**実際**: nekocodeは8言語対応（Rust/Go含む）
**影響**: Rust/Goユーザーが「非対応」と誤解
**優先度**: 🟡 中

### 4. ❌ **MCP引数マッピングのバグ**
**問題**: MCPが位置引数を名前付き引数に変換してしまう
```bash
# MCPが生成する間違ったコマンド
nekocode ast-query --session-id 657ba664 --path VM

# 正しいコマンド
nekocode ast-query 657ba664 VM
```
**影響**: ast-query等が使えない
**優先度**: 🟡 中

### 6. ❌ **AST統計の不完全な情報**
**問題**: ノード数と深度が0になるバグ
```
Total functions: 46  # ✅ 正常
Total classes: 2     # ✅ 正常
Total nodes: 0       # ❌ おかしい！
Max AST depth: 0     # ❌ おかしい！
```
**原因**: AST統計の計算ロジックが未実装または壊れている
**影響**: AST分析機能が不完全
**優先度**: 🟡 中

### 7. ❌ **セッション対象の混乱**
**問題**: セッション作成時と異なるファイルの結果が出る
```bash
# vm.rsでセッション作成
nekocode session-create vm.rs
# → デッドコード検出でoptimizer.rsの結果が出る？！
```
**原因**: セッション管理のスコープが不明確
**影響**: 予期しない解析結果
**優先度**: 🔴 高

### 5. ❌ **メモリー・コンフィグコマンドのMCP未対応**
**問題**: 実装したmemory/configコマンドがMCPから呼べない
```bash
# 実装済みだが呼べない
nekocode memory-save
nekocode config-show
```
**影響**: MCPエラー続出
**優先度**: 🟡 中

## 🔧 修正計画

### **Phase 1: 即座に修正（今すぐ）**

#### 1.1 analyzeコマンドの復活
```rust
// nekocode/src/cli.rs に追加
/// Analyze file or directory (single-shot analysis)
Analyze {
    /// Path to analyze
    path: PathBuf,
    
    /// Stats only mode (faster)
    #[arg(long)]
    stats_only: bool,
}
```

```rust
// nekocode/src/main.rs に追加
Commands::Analyze { path, stats_only } => {
    // 内部的にはsession-createを呼ぶが、セッションIDは表示しない
    let session = create_temp_session(&path)?;
    if stats_only {
        print_stats(&session);
    } else {
        print_full_analysis(&session);
    }
    // セッションは自動削除
}
```

#### 1.2 セッションID省略対応（最小限）
```rust
// すべてのコマンドでsession_idをOptional<String>に変更
AstStats {
    /// Session ID (optional - uses last session if not provided)
    session_id: Option<String>,
}
```

### **Phase 2: セッションID完全隠蔽（根本解決）**

#### 2.1 新しいCLI設計
```bash
# 新方式（セッションID不要）
nekocode /project       # 解析開始（セッション自動管理）
nekocode stats          # 自動でコンテキスト認識
nekocode deadcode       # IDなんて知らない！

# ファイル単体も同じ
nekocode main.rs        # main.rsを解析
nekocode stats          # 自動でそのコンテキスト
```

#### 2.2 内部実装
```rust
// グローバルコンテキスト管理
struct GlobalContext {
    current_path: PathBuf,
    session: Option<Session>,
}

impl GlobalContext {
    fn auto_resolve() -> Self {
        // 1. カレントディレクトリをチェック
        // 2. .nekocode/context ファイルを読む
        // 3. セッションを自動作成/取得
        // → ユーザーはIDを一切知らない
    }
}
```

#### 2.3 MCPサーバー側の修正
```python
# 新しいMCP実装
class NekocodeServer:
    def __init__(self):
        self.current_context = None  # セッションID隠蔽
    
    def analyze(self, path, stats_only=False):
        # セッションは内部で自動管理
        self.current_context = path
        return run_command(["nekocode", path])
    
    def stats(self):
        # current_contextから自動解決
        return run_command(["nekocode", "stats"])
```

### **Phase 3: 完全な統合**

#### 3.1 すべてのコマンドをコンテキスト対応に
- `nekocode` - 現在のディレクトリを解析
- `nekocode stats` - 統計表示
- `nekocode deadcode` - デッドコード検出
- `nekocode edit` - 編集機能
- すべてID不要！

#### 3.2 後方互換性
```rust
// 旧方式もサポート（非推奨警告付き）
if args.contains("session-create") {
    eprintln!("⚠️ Deprecated: session-create is no longer needed");
    eprintln!("  Just use: nekocode /path/to/project");
}
```

## 📈 期待される効果

1. **使いやすさ10倍向上** - セッションID管理から解放
2. **Claude Code体験改善** - 自然な対話フロー
3. **初心者に優しい** - 複雑な概念を隠蔽
4. **Gitライクな直感的UX** - みんな知ってる操作感

## 🎯 実装順序

1. ✅ メモリー・コンフィグコマンド追加（完了）
2. 🔧 analyzeコマンド復活（最優先）
3. 🔧 セッションID省略対応（次優先）
4. 🔧 MCPサーバー修正（並行作業）
5. 🎯 セッションID完全隠蔽（最終目標）

## 📝 メモ

- セッションは「実装の詳細」であり、ユーザーが意識すべきものではない
- Gitのように「今どこで作業してるか」を自動認識すべき
- MCPとCLIの一貫性が最重要
- 後方互換性を保ちつつ段階的に移行

---
**作成**: 2025-08-25  
**更新**: 2025-08-25  
**ステータス**: 🚨 緊急対応中
