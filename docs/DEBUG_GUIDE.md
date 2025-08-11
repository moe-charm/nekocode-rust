# 🐛 NekoCode Debug Guide - デバッグフラグ使用方法

## 概要

NekoCode v2.0 では、find コマンドに `--debug` フラグが追加されました。このフラグを使用することで、検索処理の詳細な情報を取得でき、問題の診断やデバッグに役立てることができます。

## --debug フラグの機能

### 基本機能
- **検索処理の詳細表示**: ファイル処理の進行状況をリアルタイム表示
- **ファイルアクセス情報**: 各ファイルの存在確認、パーミッション、サイズ情報
- **エラー診断**: ファイルが開けない理由の詳細表示
- **パフォーマンス分析**: 検索速度やマッチング効率の分析

### 出力内容
デバッグモードでは以下の情報が `stderr` に出力されます：

```
[DEBUG SymbolFinder::find] Starting search for: std::cout
[DEBUG SymbolFinder::find] Files count: 847
[DEBUG findInFiles] Target files count: 847
[DEBUG findInFiles] Processing file: src/main/main_ai.cpp
[DEBUG findInFiles] File path string: 'src/main/main_ai.cpp'
[DEBUG findInFiles] File exists: 1
[DEBUG findInFiles] Is regular file: 1
[DEBUG findInFiles] File permissions readable: 1
[DEBUG findInFiles] File content size: 20461 bytes
[DEBUG findInFiles] First 100 chars: //=============================================================================
// 🤖 NekoCode AI Tool - Claude Code最
[DEBUG findInFile] Searching for 'std::cout' in src/main/main_ai.cpp
[DEBUG findInFile] Content size: 20461 bytes
[DEBUG findInFiles] Found 2 matches in this file
```

## 使用方法

### 基本的な使い方

```bash
# セッション作成
nekocode_ai session-create myproject/

# デバッグ情報付きで検索
nekocode_ai session-command <session_id> "find std::cout --debug"
```

### コマンドオプション組み合わせ

```bash
# デバッグ + 表示制限
nekocode_ai session-command <session_id> "find MyClass --debug --limit 10"

# デバッグ + 関数のみ検索
nekocode_ai session-command <session_id> "find handleClick --debug --function"

# デバッグ + 特定パス指定
nekocode_ai session-command <session_id> "find error --debug src/"

# デバッグ + ファイル出力
nekocode_ai session-command <session_id> "find namespace --debug --output debug_results.txt"
```

## トラブルシューティング

### よくある問題と解決策

#### 1. ファイルが見つからない
```
[DEBUG findInFiles] Failed to open file: src/nonexistent.cpp
[DEBUG findInFiles] Current working directory: /project/root
[DEBUG findInFiles] Absolute path: /project/root/src/nonexistent.cpp
```

**解決策**: ファイルパスやセッションの作成対象を確認してください。

#### 2. ファイルが読み取れない
```
[DEBUG findInFiles] File exists: 1
[DEBUG findInFiles] Is regular file: 1
[DEBUG findInFiles] File permissions readable: 0
```

**解決策**: ファイルの読み取り権限を確認してください。

#### 3. 検索対象ファイル数が0
```
[DEBUG findInFiles] Target files count: 0
```

**解決策**: 検索パス指定やセッション作成時の対象ディレクトリを確認してください。

#### 4. 大量の出力でパフォーマンス低下
```bash
# ファイル数が多い場合は出力リダイレクトを推奨
nekocode_ai session-command <session_id> "find term --debug" 2>debug.log
```

## Claude Code との連携

### JSON出力の保護
`--debug` フラグは `stderr` に出力するため、メインの JSON 出力（`stdout`）には影響しません：

```bash
# Claude Code でも安全に使用可能
nekocode_ai session-command <session_id> "find MyClass --debug" > results.json 2>debug.log
```

### AI 分析での活用
- デバッグ情報を使って検索性能を最適化
- ファイルアクセスパターンの分析
- 大規模プロジェクトでの問題箇所特定

## パフォーマンス影響

### デバッグモードのオーバーヘッド
- **CPU**: 約 5-10% の追加負荷
- **メモリ**: 文字列処理による若干の増加
- **I/O**: デバッグ出力による軽微な影響

### 推奨使用場面
- ✅ **開発・デバッグ時**: 問題診断や動作確認
- ✅ **パフォーマンス分析**: 検索効率の調査
- ✅ **大規模プロジェクト**: ファイル処理状況の把握
- ❌ **本番環境**: 通常の検索では不要

## 技術詳細

### 実装場所
- **FindOptions構造体**: `include/nekocode/symbol_finder.hpp:63`
- **コマンド解析**: `src/commands/find_command.cpp:89`
- **デバッグ出力**: `src/finders/symbol_finder.cpp:24-147`
- **セッション管理**: `src/core/session_manager.cpp:95-98`

### 出力フォーマット
```
[DEBUG <関数名>] <メッセージ内容>
```

すべてのデバッグ出力は `std::cerr` に送信され、JSON出力に影響しません。

## 更新履歴

- **v2.0** (2025-07-28): `--debug` フラグ追加
  - ランタイムデバッグ制御の実装
  - ファイルアクセス詳細情報の追加
  - Claude Code互換性の確保

---

**関連ドキュメント**:
- [USAGE_jp.md](USAGE_jp.md) - 基本的な使用方法
- [PERFORMANCE_GUIDE.md](PERFORMANCE_GUIDE.md) - パフォーマンス最適化

**🐱 Happy Debugging! にゃー**