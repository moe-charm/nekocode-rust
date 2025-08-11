# 📐 Phase 5: Universal Symbol Native Generation 設計書

**作成日**: 2025-08-08  
**目的**: アナライザーが直接Universal Symbolsを生成する設計への移行

---

## 🎯 **設計目標**

### **現在のアーキテクチャ（Phase 4）**
```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐     ┌──────────────┐
│  Analyzer   │ --> │AnalysisResult│ --> │  Converter  │ --> │Universal     │
│             │     │(classes,     │     │             │     │Symbols       │
│             │     │ functions)   │     │             │     │              │
└─────────────┘     └──────────────┘     └─────────────┘     └──────────────┘
```

### **目標アーキテクチャ（Phase 5）**
```
┌─────────────┐     ┌──────────────────────────┐
│  Analyzer   │ --> │AnalysisResult             │
│             │     │(universal_symbols直接生成)│
└─────────────┘     └──────────────────────────┘
```

---

## 🔧 **実装方針**

### **1. アナライザー改修の共通パターン**

```cpp
// Before (Phase 4)
AnalysisResult JavaScriptAnalyzer::analyze() {
    AnalysisResult result;
    
    // 従来のclasses/functions配列に格納
    result.classes.push_back(classInfo);
    result.functions.push_back(functionInfo);
    
    return result;
}

// After (Phase 5)
AnalysisResult JavaScriptAnalyzer::analyze() {
    AnalysisResult result;
    
    // Universal Symbolsを直接生成
    auto symbol_table = std::make_shared<SymbolTable>();
    
    // クラスをUniversal Symbolとして追加
    UniversalSymbolInfo class_symbol;
    class_symbol.id = generate_unique_id("class", class_name);
    class_symbol.symbol_type = SymbolType::CLASS;
    class_symbol.name = class_name;
    // ... その他のフィールド設定
    symbol_table->add_symbol(std::move(class_symbol));
    
    result.universal_symbols = symbol_table;
    
    // 後方互換性のため従来フィールドも維持（オプション）
    // result.classes.push_back(classInfo);
    
    return result;
}
```

---

## 📋 **実装手順**

### **Phase 5.1: 基盤準備**
1. **共通ユーティリティ作成**
   - ID生成関数の共通化
   - Symbol変換ヘルパー関数
   - テストユーティリティ

### **Phase 5.2: 言語別改修**

#### **JavaScript/TypeScript (最初のターゲット)**
- ファイル: `src/analyzers/javascript/javascript_pegtl_analyzer.cpp`
- 変更内容:
  1. SymbolTable生成ロジック追加
  2. classes/functions解析時に同時にsymbols生成
  3. 階層構造の構築（parent-child関係）

#### **Python**
- ファイル: `src/analyzers/python/python_analyzer.cpp`
- 特有の考慮点:
  - インデントベースの階層構造
  - デコレータのメタデータ化

#### **C++**
- ファイル: `src/analyzers/cpp/cpp_pegtl_analyzer.cpp`
- 特有の考慮点:
  - namespace階層
  - template情報の保持
  - access modifiers

#### **C#**
- ファイル: `src/analyzers/csharp/csharp_pegtl_analyzer.cpp`
- 特有の考慮点:
  - interface実装
  - property vs field
  - ジェネリクス

#### **Go**
- ファイル: `src/analyzers/go/go_analyzer.cpp`
- 特有の考慮点:
  - package構造
  - receiver付きメソッド
  - interface型

#### **Rust**
- ファイル: `src/analyzers/rust/rust_analyzer.cpp`
- 特有の考慮点:
  - impl分離構造
  - trait実装
  - lifetime情報

---

## 🗑️ **削除対象**

### **Phase 5.3: Converter層の削除**
削除予定ファイル:
- `src/converters/js_symbol_converter.cpp/.hpp`
- `src/converters/python_symbol_converter.cpp/.hpp`
- `src/converters/cpp_symbol_converter.cpp/.hpp`
- `src/converters/csharp_symbol_converter.cpp/.hpp`
- `src/converters/go_symbol_converter.cpp/.hpp`
- `src/converters/rust_symbol_converter.cpp/.hpp`

main_ai.cppからの削除:
- 各言語のConverter呼び出し部分
- Converterヘッダーのinclude

---

## 🎯 **成功基準**

### **機能要件**
- ✅ 全言語でUniversal Symbols直接生成
- ✅ 既存のJSON出力と完全互換
- ✅ パフォーマンス向上（変換オーバーヘッド削除）

### **品質要件**
- ✅ 単体テストのパス
- ✅ 統合テストのパス
- ✅ メモリリーク無し
- ✅ 後方互換性の維持（オプション）

---

## 📊 **期待される効果**

### **パフォーマンス**
- 変換処理の削除: **約20-30%高速化**
- メモリ使用量削減: **約15%削減**
- コード複雑度低下: **保守性向上**

### **コードメトリクス**
- 削除行数: 約2,500行（Converters）
- 追加行数: 約1,000行（アナライザー改修）
- 正味削減: **約1,500行**

---

## 🚦 **リスクと対策**

### **リスク**
1. **既存機能の破壊**: 慎重なテストで対応
2. **後方互換性**: 移行期間中は両方式サポート
3. **複雑な階層構造**: 段階的な実装

### **対策**
1. **Feature Flag導入**
   ```cpp
   #ifdef NATIVE_UNIVERSAL_SYMBOLS
   // 新実装
   #else
   // 従来実装
   #endif
   ```

2. **段階的移行**
   - Phase 5.2.1: JavaScript先行実装
   - Phase 5.2.2: 他言語順次対応
   - Phase 5.3: 完全移行後にConverter削除

---

## 📝 **テスト計画**

### **単体テスト**
```cpp
TEST(UniversalSymbolNative, JavaScriptDirectGeneration) {
    JavaScriptAnalyzer analyzer;
    auto result = analyzer.analyze(test_code);
    
    ASSERT_TRUE(result.universal_symbols != nullptr);
    ASSERT_EQ(result.universal_symbols->size(), expected_count);
}
```

### **統合テスト**
- 全言語でのシンボル数確認
- JSON出力の互換性確認
- パフォーマンスベンチマーク

---

## 🎯 **次のステップ**

1. **この設計書のレビュー**
2. **JavaScript/TypeScriptから実装開始**
3. **段階的に各言語対応**
4. **最終的にConverter層削除**

---

**設計承認**: ✅ 実装開始準備完了