# 📊 SessionData構造ガイド

**SessionData関連の内部構造リファレンス** - move-class実装用

---

## 🏗️ SessionData構造

### **主要フィールド**
```cpp
struct SessionData {
    std::string session_id;                  // セッションID
    std::filesystem::path target_path;       // 解析対象パス
    
    // 解析データ（どちらか一方）
    AnalysisResult single_file_result;       // 単一ファイルの場合
    DirectoryAnalysis directory_result;      // ディレクトリの場合
    bool is_directory = false;               // ディレクトリ判定フラグ
    
    nlohmann::json quick_stats;             // クイック統計
    nlohmann::json dead_code_info;          // デッドコード情報
}
```

---

## 📁 AnalysisResult構造（単一ファイル）

### **クラス情報格納場所**
```cpp
struct AnalysisResult {
    FileInfo file_info;                     // ファイル基本情報
    Language language;                       // 言語種別
    
    // ⭐ クラス情報はここ！
    std::vector<ClassInfo> classes;         // クラス一覧
    std::vector<FunctionInfo> functions;    // 関数一覧
    
    CodeStats stats;                        // 統計情報
    ComplexityInfo complexity;              // 複雑度情報
}
```

---

## 🎯 ClassInfo構造（move-classで使用）

### **クラス位置情報**
```cpp
struct ClassInfo {
    std::string name;                       // クラス名
    std::string parent_class;               // 親クラス
    LineNumber start_line = 0;              // ⭐ 開始行
    LineNumber end_line = 0;                // ⭐ 終了行
    
    std::vector<FunctionInfo> methods;      // メソッド一覧
    std::vector<MemberVariable> member_variables;  // メンバ変数
}
```

---

## 📂 DirectoryAnalysis構造（ディレクトリ）

### **複数ファイルの場合**
```cpp
struct DirectoryAnalysis {
    FilePath directory_path;                // ディレクトリパス
    std::vector<AnalysisResult> files;      // ⭐ 各ファイルの解析結果
    
    struct Summary {
        std::uint32_t total_files;
        std::uint32_t total_classes;        // クラス総数
        std::uint32_t total_functions;      // 関数総数
    } summary;
}
```

---

## 🔧 move-class実装でのアクセス方法

### **単一ファイルセッションの場合**
```cpp
// SessionDataからクラス情報取得
const auto& classes = session.single_file_result.classes;
for (const auto& cls : classes) {
    if (cls.name == target_class_name) {
        // cls.start_line と cls.end_line を使用
        size_t line_count = cls.end_line - cls.start_line + 1;
    }
}
```

### **ディレクトリセッションの場合**
```cpp
// 特定ファイルを探す
for (const auto& file : session.directory_result.files) {
    if (file.file_info.path == target_file) {
        // そのファイルのクラス情報を検索
        for (const auto& cls : file.classes) {
            if (cls.name == target_class_name) {
                // 見つかった！
            }
        }
    }
}
```

---

## 🎮 既存movelines機能の呼び出し

### **DirectEdit名前空間の関数**
```cpp
// src/core/commands/direct_edit/direct_movelines.cpp
namespace nekocode::DirectEdit {
    nlohmann::json movelines_preview(
        const std::string& srcfile,      // ソースファイル
        int start_line,                  // 開始行（1ベース）
        int line_count,                  // 行数
        const std::string& dstfile,      // 宛先ファイル
        int insert_line                  // 挿入位置（1ベース）
    );
}
```

### **内部呼び出し例**
```cpp
// move-classから内部でmovelines_previewを呼ぶ
auto preview = DirectEdit::movelines_preview(
    src_file,
    class_info.start_line,
    class_info.end_line - class_info.start_line + 1,
    dst_file,
    std::numeric_limits<int>::max()  // 末尾に挿入
);
```

---

## 📋 SessionCommandsクラス構造

### **コマンド追加場所**
```cpp
// include/nekocode/session_commands.hpp
class SessionCommands {
public:
    // 既存のコマンド群
    nlohmann::json cmd_stats(const SessionData& session) const;
    nlohmann::json cmd_structure(const SessionData& session) const;
    
    // ⭐ ここに追加！
    nlohmann::json cmd_move_class(
        const SessionData& session,
        const std::vector<std::string>& args
    ) const;
};
```

### **実装ファイル**
```cpp
// src/core/cmd/structure_commands.cpp に実装追加
nlohmann::json SessionCommands::cmd_move_class(
    const SessionData& session,
    const std::vector<std::string>& args
) const {
    // 実装
}
```

---

## 🔄 コマンドルーティング

### **session_manager.cpp でのディスパッチ**
```cpp
// execute_command内でルーティング
if (command == "move-class") {
    // argsを分解
    std::vector<std::string> args = split_args(remaining);
    return session_commands_.cmd_move_class(session, args);
}
```

---

## 📝 重要な型定義

```cpp
using LineNumber = std::uint32_t;    // 行番号
using FilePath = std::filesystem::path;
using FileSize = std::uintmax_t;
```

---

## ⚠️ 注意事項

1. **行番号は1ベース** - ユーザー向けは1から、内部配列は0から
2. **SessionData依存** - move-classはセッション必須
3. **言語判定** - LanguageでC++/C#/Java等を判定
4. **ファイルパス** - 相対パスを絶対パスに変換必要

---

---

## 📋 実際のJSON出力構造

### **SessionData JSON形式**
```json
{
  "session_id": "session_20250808_032110",
  "session_type": "ai_optimized", 
  "created_at": "2025-08-08T03:21:10",
  "target_path": "/path/to/file.rs",
  "is_directory": false,
  "single_file_result": {
    "file_info": {
      "name": "rust_test_patterns.rs",
      "path": "/full/path/to/rust_test_patterns.rs",
      "size_bytes": 5398,
      "total_lines": 241,
      "code_lines": 221,
      "comment_lines": 20,
      "empty_lines": 0
    },
    "stats": {
      "class_count": 7,
      "function_count": 32,
      "import_count": 1,
      "export_count": 0,
      "unique_calls": 0,
      "total_calls": 0
    },
    "complexity": {
      "cyclomatic_complexity": 12,
      "max_nesting_depth": 4,
      "rating": "Moderate 🟡"
    },
    "classes": [
      {
        "name": "DatabaseManager",
        "parent_class": "",
        "start_line": 7,
        "end_line": 0,
        "methods": [
          {
            "name": "new",
            "start_line": 16,
            "end_line": 22,
            "complexity": 1,
            "parameters": [],
            "is_async": false,
            "is_arrow_function": false,
            "metadata": {}
          }
        ],
        "member_variables": [
          {
            "name": "host",
            "type": "String",
            "declaration_line": 8,
            "is_static": false,
            "is_const": false,
            "access_modifier": "private"
          }
        ]
      }
    ],
    "functions": [
      {
        "name": "standalone_function",
        "start_line": 221,
        "end_line": 223,
        "complexity": 1,
        "parameters": [],
        "is_async": false,
        "is_arrow_function": false,
        "metadata": {}
      }
    ],
    "function_calls": []
  },
  "quick_stats": {
    "classes": 7,
    "functions": 32,
    "lines": 241,
    "complexity": 12,
    "language": 7,
    "size": 5398,
    "type": "file"
  },
  "command_history": []
}
```

### **🚨 現在の問題点**

#### **1. Rustのimplメソッド分類エラー**
```json
// ❌ 現在: implメソッドがfunctions[]に混在
"functions": [
  {"name": "new", "start_line": 16},        // impl DatabaseManager
  {"name": "connect", "start_line": 30},    // impl DatabaseManager  
  {"name": "standalone_function", "start_line": 221}  // 真のスタンドアロン関数
]

// ✅ 正しい: implメソッドはclasses[].methods[]に分類
"classes": [
  {
    "name": "DatabaseManager",
    "methods": [
      {"name": "new", "start_line": 16},
      {"name": "connect", "start_line": 30}
    ]
  }
],
"functions": [
  {"name": "standalone_function", "start_line": 221}  // スタンドアロンのみ
]
```

#### **2. UniversalFunctionInfo metadata活用不足**
```json
// 🆕 提案: metadataで言語固有情報を表現
"functions": [
  {
    "name": "connect",
    "metadata": {
      "parent_struct": "DatabaseManager",    // 所属struct
      "impl_type": "inherent",               // inherent/trait
      "trait_name": "",                      // trait実装の場合のtrait名
      "access_modifier": "pub",              // アクセス修飾子
      "is_unsafe": "false"                   // Rust固有
    }
  }
]
```

---

---

## 🎯 Phase 4提案: SessionData構造統一設計

### **現在の問題点: 重複設計**

```cpp
// 現在の冗長な設計 ❌
struct SessionData {
    AnalysisResult single_file_result;    // 単一ファイル用
    DirectoryAnalysis directory_result;   // ディレクトリ用  
    bool is_directory = false;           // 判定フラグ
    
    // 処理コードも分岐だらけ
    if (is_directory) {
        for (auto& file : directory_result.files) { ... }
    } else {
        auto& file = single_file_result; { ... }
    }
}
```

### **統一後設計: シンプル・美しい**

```cpp
// 統一設計 ✅  
struct UnifiedSessionData {
    std::string session_id;
    std::filesystem::path target_path;
    
    // 🎯 統一結果格納（単一ファイルもfiles[0]に格納）
    DirectoryAnalysis analysis_result;
    
    // 🌟 Universal Symbol革命
    std::vector<UniversalSymbolInfo> universal_symbols;
    bool has_universal_symbols = false;
    
    // 既存互換性
    nlohmann::json quick_stats;
    std::vector<CommandHistory> command_history;
};
```

### **統一設計の天才的利点**

#### **1. コード簡素化（50%削減）**
```cpp
// 現在: 分岐地獄 ❌
if (session.is_directory) {
    for (auto& file : session.directory_result.files) {
        for (auto& cls : file.classes) { ... }
    }
} else {
    for (auto& cls : session.single_file_result.classes) { ... }
}

// 統一後: 美しい一本化 ✅
for (auto& file : session.analysis_result.files) {
    for (auto& cls : file.classes) { ... }
}
```

#### **2. move-class実装の劇的簡素化**
```cpp
// 現在: 複雑な分岐 ❌
nlohmann::json cmd_move_class(const SessionData& session, ...) {
    if (session.is_directory) {
        // ディレクトリ用ロジック
        for (auto& file : session.directory_result.files) {
            if (file.file_info.path == source_file) {
                // クラス検索...
            }
        }
    } else {
        // 単一ファイル用ロジック  
        for (auto& cls : session.single_file_result.classes) {
            // クラス検索...
        }
    }
}

// 統一後: 超シンプル ✅
nlohmann::json cmd_move_class(const UnifiedSessionData& session, ...) {
    // 単一処理パスでファイル種別関係なし
    auto* target_file = find_file_by_name(session.analysis_result.files, source_file);
    auto* target_class = find_class_by_name(target_file->classes, class_name);
    // 移動処理...
}
```

#### **3. API統一による開発効率向上**
```cpp
// 統一後: 全機能で同じパターン
template<typename T>
std::vector<T*> find_symbols_in_session(
    const UnifiedSessionData& session,
    const std::string& symbol_name
) {
    std::vector<T*> results;
    for (auto& file : session.analysis_result.files) {
        auto symbols = find_symbols<T>(file, symbol_name);
        results.insert(results.end(), symbols.begin(), symbols.end());
    }
    return results;
}

// あらゆる検索が統一API
auto classes = find_symbols_in_session<ClassInfo>(session, "DatabaseManager");
auto functions = find_symbols_in_session<FunctionInfo>(session, "connect");
```

### **🌟 UniversalSymbolInfo: 次世代シンボル管理**

#### **現在の問題: 型別分散管理**
```cpp
// 現在: バラバラ ❌
std::vector<ClassInfo> classes;      // クラス情報
std::vector<FunctionInfo> functions; // 関数情報
std::vector<MemberVariable> vars;    // 変数情報
// → 検索時に3つの配列を別々に処理
```

#### **統一設計: 階層的シンボル管理**
```cpp
// UniversalSymbolInfo: 全シンボル統一 ✅
enum class SymbolType {
    // 構造要素
    CLASS, STRUCT, INTERFACE, ENUM, TRAIT,
    // 関数要素  
    FUNCTION, METHOD, CONSTRUCTOR, DESTRUCTOR,
    // 変数要素
    VARIABLE, MEMBER_VAR, PARAMETER, PROPERTY,
    // 組織要素
    NAMESPACE, MODULE, IMPL_BLOCK
};

struct UniversalSymbolInfo {
    SymbolType symbol_type;
    std::string name;
    std::string parent_context;
    LineNumber start_line, end_line;
    
    // 🌳 階層構造（子要素）
    std::vector<UniversalSymbolInfo> children;
    
    // 🎨 言語固有情報
    std::unordered_map<std::string, std::string> metadata;
    
    // ⚡ 機能固有情報
    std::vector<std::string> parameters;  // 関数用
    ComplexityInfo complexity;             // 関数用
    bool is_async = false;                // 関数用
};
```

#### **統一JSON出力の美しさ**
```json
{
  "session_id": "session_xxx", 
  "analysis_result": {
    "files": [
      {
        "file_info": {"name": "rust_file.rs"},
        "symbols": [
          {
            "symbol_type": "struct",
            "name": "DatabaseManager",
            "start_line": 7,
            "metadata": {"language": "rust", "visibility": "pub"},
            "children": [
              {
                "symbol_type": "method",
                "name": "new", 
                "start_line": 16,
                "parent_context": "DatabaseManager",
                "metadata": {"impl_type": "inherent"}
              },
              {
                "symbol_type": "member_var",
                "name": "host",
                "start_line": 8,
                "parent_context": "DatabaseManager"
              }
            ]
          },
          {
            "symbol_type": "function",
            "name": "standalone_function",
            "start_line": 221,
            "parent_context": ""
          }
        ]
      }
    ]
  }
}
```

### **移行戦略: 段階的・安全**

#### **Phase 1: 互換ラッパー導入**
```cpp
// 既存コード保護
class SessionDataCompat {
    UnifiedSessionData unified_data;
    
public:
    // 既存API互換性維持
    const AnalysisResult& single_file_result() const {
        if (unified_data.analysis_result.files.empty()) {
            throw std::runtime_error("No files in session");
        }
        return unified_data.analysis_result.files[0];  // 最初のファイル
    }
    
    const DirectoryAnalysis& directory_result() const {
        return unified_data.analysis_result;
    }
    
    bool is_directory() const {
        return unified_data.analysis_result.files.size() > 1;
    }
};
```

#### **Phase 2: 新API段階移行**
```cpp
// 新機能は新APIで実装
nlohmann::json cmd_universal_search(const UnifiedSessionData& session, ...) {
    // UniversalSymbolInfo活用
}

// 既存機能は互換ラッパー経由
nlohmann::json cmd_move_class(const SessionData& old_session, ...) {
    SessionDataCompat compat(old_session);
    // 既存ロジック継続
}
```

#### **Phase 3: 完全移行**
```cpp
// 全機能が統一API使用
// 互換ラッパー削除
// 構造統一完成
```

### **期待効果: 革命的改善**

| 項目 | 現在 | 統一後 | 改善率 |
|------|------|--------|--------|
| コード量 | 100% | 50% | **50%削減** |
| 新機能追加工数 | 100% | 30% | **70%削減** |
| バグ混入リスク | 高 | 低 | **大幅削減** |
| 保守性 | 複雑 | シンプル | **劇的改善** |
| API一貫性 | 分散 | 統一 | **完全統一** |

### **実装優先度**

1. **Phase 1-3**: 既存システム改善（現在進行中）
2. **Phase 4**: SessionData構造統一（最終仕上げ）
3. **UniversalSymbolInfo**: 次世代シンボル管理（Phase 3で実装）

---

**最終更新**: 2025-08-08  
**用途**: move-class機能実装リファレンス + JSON構造ドキュメント + 構造統一設計書