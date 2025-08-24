# 🎯 NekoCode スマートセッション設計

## 🚀 統一エントリーポイント

### **シンプルな使い方**

```python
# これだけ！
nekocode(".")      # カレント解析
nekocode("../src") # 指定パス解析
```

## 📊 セッション自動管理の仕組み

### **1. セッションマッピング**

`.nekocode_sessions/` に保存されている既存セッションを活用：

```json
// .nekocode_config.json に追加
{
  "session_mapping": {
    "/absolute/path/to/project": "session_id_abc123",
    "../relative/path": "session_id_def456"
  },
  "default_session": "session_id_abc123",
  "auto_session": true  // 自動セッション管理ON/OFF
}
```

### **2. 内部フロー**

```
nekocode(path) 呼び出し
    ↓
パス正規化（絶対パス変換）
    ↓
既存セッション検索
    ├─ ある → セッション使用（差分更新のみ）
    └─ ない → 新規セッション作成
    ↓
結果を分かりやすく表示
```

### **3. 監視モードの自動調整**

```python
def smart_watch(session_id, path):
    file_count = count_files(path)
    
    if file_count < 100:
        # 小規模：完全監視
        watch_config = {
            "mode": "full",
            "debounce_ms": 300,
            "include_all": true
        }
    elif file_count < 1000:
        # 中規模：主要ファイルのみ
        watch_config = {
            "mode": "partial",
            "debounce_ms": 500,
            "exclude": ["tests", "docs", "examples"]
        }
    else:
        # 大規模：最小限監視
        watch_config = {
            "mode": "minimal",
            "debounce_ms": 1000,
            "include_only": ["src", "lib"],
            "skip_analysis": ["imports", "comments"]
        }
```

## 🎨 分かりやすい表示フォーマット

### **初回実行時**
```
📊 Project Analysis: /my/project
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📁 Files: 190 (12.3 MB)
🔤 Languages: Rust 70% | Python 20% | JS 10%
🔧 Functions: 1,234
📦 Classes: 56
⚡ Session: abc123 [Created]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
💡 Monitoring: Full mode (< 100 files)
```

### **2回目以降（差分更新）**
```
⚡ Quick Update: /my/project
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📝 Changed: 3 files (0.2s)
  ├ src/main.rs (+5 functions)
  ├ src/lib.rs (-2 functions)
  └ tests/test.rs (modified)
📊 Total: 1,239 functions (+5)
⚡ Session: abc123 [Cached]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### **大規模プロジェクト時**
```
⚠️ Large Project: /huge/monorepo
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📁 Files: 5,234 (Too many!)
🚀 Mode: Quick stats only
⏭️ Skipping: tests/, docs/, examples/
📊 Core files: 890
⚡ Session: xyz789 [Optimized]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
💡 Tip: Use nekocode("src/") for specific analysis
```

## 🔧 実装詳細

### **MCPサーバー側（Python）**

```python
class SmartSessionManager:
    def __init__(self):
        self.config = self.load_config()
        self.sessions = self.load_sessions()
        self.path_cache = {}  # パス→セッションIDキャッシュ
    
    def get_or_create_session(self, path):
        """パスに対応するセッション取得/作成"""
        abs_path = os.path.abspath(path)
        
        # キャッシュ確認
        if abs_path in self.path_cache:
            return self.path_cache[abs_path]
        
        # 既存セッション検索
        for session in self.sessions:
            if session['path'] == abs_path:
                self.path_cache[abs_path] = session['id']
                return session['id']
        
        # 新規作成
        session_id = self.create_new_session(abs_path)
        self.path_cache[abs_path] = session_id
        return session_id
    
    def analyze_smart(self, path):
        """スマート解析"""
        session_id = self.get_or_create_session(path)
        
        # セッション情報取得
        session_info = self.get_session_info(session_id)
        
        if session_info['is_new']:
            # 初回：フル解析
            result = self.full_analyze(session_id)
            display_mode = "full"
        else:
            # 2回目以降：差分のみ
            result = self.incremental_update(session_id)
            display_mode = "diff"
        
        # 分かりやすく整形
        return self.format_result(result, display_mode)
```

### **nekocodeバイナリ側（Rust）**

```rust
// セッション自動検出機能追加
impl SessionManager {
    pub fn find_session_for_path(&self, path: &Path) -> Option<String> {
        // .nekocode_sessions/ から該当パスのセッション検索
        for entry in fs::read_dir(".nekocode_sessions")? {
            let session = self.load_session(entry)?;
            if session.path == path {
                return Some(session.id);
            }
        }
        None
    }
    
    pub fn smart_analyze(&mut self, path: &Path) -> Result<AnalysisResult> {
        // セッション自動解決
        let session_id = self.find_session_for_path(path)
            .unwrap_or_else(|| self.create_session(path));
        
        // ファイル数で解析モード決定
        let file_count = count_files(path);
        let mode = if file_count > 1000 {
            AnalysisMode::StatsOnly
        } else if file_count > 100 {
            AnalysisMode::Partial
        } else {
            AnalysisMode::Full
        };
        
        // 解析実行
        self.analyze_with_mode(session_id, mode)
    }
}
```

## 📋 設定ファイル統一

### **.nekocode_config.json**

```json
{
  "general": {
    "threads": 8,
    "auto_session": true
  },
  "session": {
    "default_path": ".",
    "cache_sessions": true,
    "max_sessions": 100
  },
  "watch": {
    "auto_start": false,
    "debounce_ms": 500,
    "exclude": [".git", "node_modules", "target"],
    "mode": "auto"  // auto/full/partial/minimal
  },
  "display": {
    "format": "rich",  // rich/simple/json
    "colors": true,
    "show_tips": true
  }
}
```

## 🚀 期待される効果

1. **導線シンプル化**: `nekocode(path)` だけ
2. **高速化**: 2回目以降は差分のみ（0.2秒）
3. **メモリ効率**: 大規模プロジェクトは自動的に軽量モード
4. **使いやすさ**: セッションID意識不要
5. **視認性**: 絵文字とプログレスバーで分かりやすい

## 📝 移行計画

### **Phase 1**: MCPサーバーに統一関数追加
```python
mcp__nekocode(path, action="analyze")  # 新API
```

### **Phase 2**: 既存関数をラップ
```python
# 内部で既存関数を呼び分け
if action == "analyze":
    return self.smart_analyze(path)
elif action == "refactor":
    return self.prepare_refactor(path)
```

### **Phase 3**: ドキュメント更新
- 新しい使い方を最前面に
- 詳細関数は「上級者向け」として残す

---

これでClaude Code君も迷わず使える！🐱