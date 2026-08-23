# 📌 Current Task — Rust-first context layer（2026-08-23）

> この節が現在の正規方針です。以下に残る2025年の計画は履歴であり、矛盾する項目（analyze復活、多言語同時拡張、未検証の精度宣言など）は実行しません。

## 製品定義

NekoCodeは、Rust公式ツールの結果とGit差分を、比較可能・予算制限付き・根拠付きのコードコンテキストへ変換する。Rust解析器、IDEバックエンド、意味インデックスとは呼ばない。

設計の正本は次の文書です。

- [`docs/product-boundary.md`](../docs/product-boundary.md)
- [`docs/execution-trust.md`](../docs/execution-trust.md)
- [`docs/artifact-contract.md`](../docs/artifact-contract.md)
- [`docs/legacy-retirement.md`](../docs/legacy-retirement.md)

## 決定事項

- 意味的な本体・SSOTは小さなRustライブラリ`nekocode-core`とする。
- 正規実行入口はCLI `nekocode`とする。公開use caseは`snapshot`と`context`の2つ。
- MCPは同じcore payloadを運ぶ薄いgateway、Skillは作業順序と停止条件、Pluginは配布単位とする。
- Rustを唯一のTier 1対象とし、他言語はRust昇格ゲート後のexperimental候補とする。
- 未測定の60%/90%/95%精度、「完全対応」「商用グレード」は使わない。
- 独自dead-code、symbol/reference index、refactor、split、strip-comments、watch、security/quality/impactは凍結する。
- `metadata-only`を既定とし、`cargo-check`はtrusted workspaceで明示opt-inにする。sandbox済みとは表現しない。

## 正規コマンド

```bash
cd nekocode-workspace
nekocode snapshot PATH
nekocode snapshot PATH --analysis cargo-check --output baseline.json
nekocode context PATH --baseline baseline.json --diagnostics
```

既存スクリプトのためCLIに限り`index` aliasを短期間残すが、READMEとMCPでは`snapshot`だけを公開する。`analyze`は作らない。

## Artifact契約

- 外部artifactは`snapshot-v1`と`context-v1`。
- snapshotは明示pathだけに保存し、hidden latestや自動履歴を作らない。
- baseline diagnosticsが無い場合は`baseline_missing`であり、空deltaにしない。
- toolchain・target・package・features・compiler config・analysis profileが不一致なら`not_comparable`と理由を返す。
- diagnostic deltaはexact/multiset比較のみ。fuzzy line matchingはしない。
- budgetはbytes/items/linesをhard limitとし、全省略にreason/count/priorityを付ける。

## 実行順序

1. docsとschemaをSSOTとして固定する。
2. coreの共通request/response型、snapshot/context、comparability、budget、provenanceを整える。
3. CLIを`snapshot`正規化し、MCPも同名二toolへ揃える。
4. golden fixture、schema validation、CLI/MCP parity、execution-trust fixtureを通す。
5. 条件成立後にlegacy final tagとread-only archiveを作り、mainから物理削除する。
6. Rust昇格ゲート後にのみ追加backend/言語を検討する。

## 作業状態

- Phase 0（製品境界・信頼モデル・artifact・legacy退役の文書化）: 完了
- Phase 1（core契約・snapshot改名・schema・parity）: 実装済み（CLI snapshot、index alias、MCP二tool、schema/omission/comparabilityを追加）
- Phase 2（診断delta・budget・安全性fixture）: 実装済み（cargo-checkはtrusted workspaceで明示opt-in、timeout/process group、output cap、offline、環境allowlist、専用target、symlink jailを追加）。OS sandboxは未実装
- Phase 3（Skill v0）: schema安定後
- Phase 4（MCP hardening・legacy archive）: parityと安全性ゲート後

検証メモ: workspace test、core/CLI check、snapshot/context smoke、schemaチェック、Rust-first MCP smoke、実行安全性fixtureは通過済み。legacy crateのwarning-freeは現行契約ではない。

アーカイブ状態: Stage A（論理アーカイブ・canonical固定）完了。物理archiveは[`docs/legacy-retirement.md`](../docs/legacy-retirement.md)の全条件を満たすまで実施しない。

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
