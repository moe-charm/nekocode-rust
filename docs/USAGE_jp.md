# 🐱 NekoCode使い方ガイド

## 📖 目次

1. [はじめに](#はじめに)
2. [インストール](#インストール)
3. [基本的な使い方](#基本的な使い方)
4. [高度な機能](#高度な機能)
5. [AI開発者向けガイド](#ai開発者向けガイド)
6. [トラブルシューティング](#トラブルシューティング)

## はじめに

NekoCode C++は、超高速なコード解析ツールです。特にAI開発者（Claude Code、GitHub Copilot等）との相性が抜群です！

## インストール

### 必要なもの
- C++17対応コンパイラ（GCC 7+、Clang 5+、MSVC 2017+）
- CMake 3.10以上
- Git

### ビルド手順

```bash
# 1. リポジトリをクローン
git clone https://github.com/moe-charm/nekocode.git
cd nekocode

# 2. ビルドディレクトリを作成
mkdir build && cd build

# 3. CMakeでビルド設定
cmake ..

# 4. ビルド実行（並列ビルド推奨）
make -j$(nproc)

# 5. 動作確認
./bin/nekocode_ai --help
```

## 基本的な使い方

### 単一ファイル解析

```bash
# C++ファイルを解析
./bin/nekocode_ai main.cpp

# JavaScriptファイルを解析
./bin/nekocode_ai app.js

# 詳細な統計情報付き
./bin/nekocode_ai --performance main.cpp

# 🔧 デバッグログ付き解析（詳細情報表示）
./bin/nekocode_ai --debug main.cpp
```

### ディレクトリ全体の解析

```bash
# srcディレクトリ全体を解析
./bin/nekocode_ai src/

# 特定の言語のみ解析
./bin/nekocode_ai --lang cpp src/

# コンパクトなJSON出力
./bin/nekocode_ai --compact src/
```

## 高度な機能

### 💬 コメント抽出・解析機能（最新機能！v2.1）

コメントアウトされたコードを自動検出・分析する革新的機能！

```bash
# 📝 単一ファイルのコメント解析
./bin/nekocode_ai analyze src/legacy_module.py --io-threads 8

# JSON出力例:
{
  "commented_lines": [
    {
      "line_start": 42,
      "line_end": 42,
      "type": "single_line",
      "content": "# old_function(data)",
      "looks_like_code": true  # ← AIがコードと判定！
    },
    {
      "line_start": 50,
      "line_end": 55,
      "type": "multi_line",
      "content": "/* class LegacyProcessor:\n     def process(self):\n         return self.data */",
      "looks_like_code": true
    }
  ],
  "statistics": {
    "commented_lines_count": 120  # 総コメント行数
  }
}

# 📊 プロジェクト全体のコメント統計
./bin/nekocode_ai analyze project/ --stats-only --io-threads 16
# → summary.total_commented_lines で全体把握
```

#### コメント抽出の活用例
- **🔍 レガシーコード発見**: 古い実装や代替案を発見
- **📈 コード品質評価**: コメント化されたコードの割合を分析
- **🧹 リファクタリング**: 不要なコメントを整理
- **📚 開発履歴理解**: コメントから開発の経緯を理解

#### 対応言語とコメント形式
- **JavaScript/TypeScript**: `//` と `/* */`
- **C/C++**: `//` と `/* */`
- **Python**: `#`
- **C#**: `//` と `/* */` と `///`

### ⚡ パフォーマンス最適化

NekoCodeは超高速なストレージ最適化機能を搭載！

```bash
# 🔥 SSDモード - 並列処理で最高速
./bin/nekocode_ai analyze large-project/ --ssd --performance
# CPUコア数フル活用、NVMe/SSDで威力発揮

# 🛡️ HDDモード - 安全なシーケンシャル処理
./bin/nekocode_ai analyze large-project/ --hdd --performance  
# 1スレッドでHDDに優しい処理

# 📊 プログレス表示 - 大規模プロジェクト監視
./bin/nekocode_ai session-create large-project/ --progress
# リアルタイム進捗: "🚀 Starting analysis: 38,021 files"
# プログレスファイル: sessions/SESSION_ID_progress.txt
```

**Claude Code攻略法**: 30,000ファイル以上のプロジェクトでは必ず `--progress` で進捗監視！

### インタラクティブセッション

最も強力な機能の1つです！

```bash
# 1. プログレス監視付きセッション作成
./bin/nekocode_ai session-create /path/to/your/project --progress
# 出力例: Session created! Session ID: ai_session_20250727_180532

# 2. セッションIDを使って様々な解析を実行
SESSION_ID=ai_session_20250727_180532

# プロジェクト統計
./bin/nekocode_ai session-command $SESSION_ID stats

# 複雑度ランキング（最重要！）
./bin/nekocode_ai session-command $SESSION_ID complexity

# ファイル検索
./bin/nekocode_ai session-command $SESSION_ID "find manager"

# 関数構造解析
./bin/nekocode_ai session-command $SESSION_ID structure
```

### C++専用の高度な解析

#### インクルード依存関係

```bash
# 依存関係グラフを生成
./bin/nekocode_ai session-command $SESSION_ID include-graph

# 循環依存を検出（重要！）
./bin/nekocode_ai session-command $SESSION_ID include-cycles

# 不要なincludeを検出
./bin/nekocode_ai session-command $SESSION_ID include-unused
```

#### テンプレート・マクロ解析

```bash
# テンプレート特殊化を検出
./bin/nekocode_ai session-command $SESSION_ID template-analysis

# マクロ展開を追跡
./bin/nekocode_ai session-command $SESSION_ID macro-analysis

# メタプログラミングパターンを検出
./bin/nekocode_ai session-command $SESSION_ID metaprogramming
```

#### 🎯 メンバ変数検出機能（新機能！）

NekoCodeの革新的なメンバ変数検出機能により、全ての言語でクラスの内部構造を詳細に解析できます。

```bash
# 基本的なメンバ変数解析
./bin/nekocode_ai analyze src/MyClass.cpp
# 出力: 型、アクセス修飾子、行番号付きでメンバ変数を表示

# 言語別メンバ変数検出
./bin/nekocode_ai analyze src/Component.js    # JavaScript: this.property, static変数
./bin/nekocode_ai analyze src/Service.ts      # TypeScript: 型付きメンバ, interface
./bin/nekocode_ai analyze src/Manager.cpp     # C++: private/public/protected
./bin/nekocode_ai analyze src/Model.py        # Python: self.vars, クラス変数, 型ヒント
./bin/nekocode_ai analyze src/Entity.cs       # C#: フィールド, プロパティ, static
./bin/nekocode_ai analyze src/struct.rs       # Rust: pub/private, ジェネリック, enum
./bin/nekocode_ai analyze Assets/PlayerController.cs  # Unity: SerializeField, MonoBehaviour
```

**検出される情報:**
- 📝 **変数名**: 正確な変数名
- 🏷️ **型情報**: `std::vector<T>`, `List<string>`, `Optional[int]` など
- 🔒 **アクセス修飾子**: public, private, protected, internal
- ⚡ **修飾子**: static, const, readonly, mutable
- 📍 **行番号**: ソースコード内の正確な位置

**対応言語別の特徴:**

| 言語 | 検出内容例 |
|------|------------|
| **C++** | `private: std::map<string, int> data;` |
| **C#** | `public static List<User> Users { get; set; }` |
| **JavaScript** | `this.config = {}`, `static count = 0` |
| **TypeScript** | `private name?: string`, `readonly id: number` |
| **Python** | `self._private: Optional[str]`, `class_var: int = 0` |
| **Rust** | `pub name: String`, `data: Arc<Mutex<T>>` |
| **Unity C#** | `[SerializeField] private float speed`, `public GameObject target` |

```bash
# 詳細なクラス構造解析
./bin/nekocode_ai session-command $SESSION_ID "analyze --detailed MyClass.cpp"

# メンバ変数統計
./bin/nekocode_ai session-command $SESSION_ID "stats --member-variables"
```

## AI開発者向けガイド

### Claude Codeでの使い方

1. **プロジェクトにNekoCodeを配置**
   ```bash
   cd your-project
   git clone https://github.com/moe-charm/nekocode.git tools/nekocode
   ```

2. **Claudeに伝える魔法の言葉**
   ```
   「tools/nekocodeにコード解析ツールがあるから使って」
   「このプロジェクトの複雑度を測定して」
   「循環依存をチェックして」
   ```

3. **Claudeが自動的に実行**
   - ビルド
   - セッション作成
   - 解析実行
   - 結果の解釈

### 実践例：リファクタリング

```bash
# 1. 現在の複雑度を測定
./bin/nekocode_ai session-command $SESSION_ID complexity

# 出力例:
# FileA.cpp: Complexity 156 (Very Complex)
# FileB.cpp: Complexity 89 (Complex)

# 2. リファクタリング実施

# 3. 改善を確認
./nekocode_ai session-command $SESSION_ID complexity
# FileA.cpp: Complexity 23 (Simple)  ← 85%削減！
```

### 🛠️ デバッグ機能

NekoCodeには強力なデバッグ機能が搭載されています！

```bash
# 🔧 基本デバッグ - 詳細な処理状況を表示
./bin/nekocode_ai --debug your_file.js

# 🔍 大規模ファイル用デバッグ - 戦略切り替えを可視化
./bin/nekocode_ai --debug large_project.ts
# 出力例:
# 🔧 デバッグ: use_high_speed_mode=1
# 🔧 デバッグ: 40000以上か? 1
# ⚡ 高速モード: 基本検出のみ（JavaScript戦略移植・Geminiスキップ）

# 📊 セッション用デバッグ - 解析戦略の詳細確認
./bin/nekocode_ai session-create --debug project/
./bin/nekocode_ai session-command $SESSION_ID "find function --debug"
```

**デバッグ機能の活用法**:
- **性能問題調査**: どの処理が重いかを特定
- **戦略確認**: ファイルサイズに応じた最適化モードを確認  
- **開発・検証**: 新機能のテスト時に内部動作を監視

## トラブルシューティング

### ビルドエラー

**Q: CMakeがC++17をサポートしていないと言われる**
```bash
# GCCのバージョンを確認
g++ --version

# 古い場合は新しいコンパイラを指定
cmake -DCMAKE_CXX_COMPILER=g++-9 ..
```

**Q: Tree-sitterが必要ですか？**
```text
不要です。本プロジェクトではTree-sitterはプレースホルダーとして統合されており、
インストール無しでビルド・実行が可能です。切替用のCMakeフラグは提供していません。
PEGTLが主要なパーサーとして使用されます。
```

### 使用時の問題

**Q: セッションが見つからない**
```bash
# セッション一覧を確認
ls sessions/

# 新しいセッションを作成
./bin/nekocode_ai session-create .
```

**Q: メモリ不足**
```bash
# スレッド数を制限
./bin/nekocode_ai --threads 2 large-project/

# ファイル数を制限
./bin/nekocode_ai --stats-only large-project/
```

## 💡 Pro Tips

1. **複雑度優先**: まず`complexity`コマンドで問題のあるファイルを特定
2. **セッション活用**: 何度も解析する場合は必ずセッションを使用（180倍高速！）
3. **並列ビルド**: `make -j$(nproc)`で全コアを使用してビルド
4. **JSON出力**: 他のツールと連携する場合は`--compact`オプション

---

詳しい情報は[公式ドキュメント](https://github.com/moe-charm/nekocode)をご覧ください！

*Happy Analyzing! 🐱*
