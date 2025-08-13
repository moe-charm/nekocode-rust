# 🚀 GitHub Actions 爆速セットアップツール

**面倒なGitHub Actions設定を1コマンドで完了！**

## 📍 ツールの場所

```bash
nekocode-rust-clean/bin/gh-actions-setup.sh
```

## 🎯 対応パターン

### ✅ **完成済み**
- **nekocode**: PR分析ワークフロー (nyashで実証済み)

### 🚧 **開発予定**
- **test-ci**: CI/CDテストワークフロー
- **deploy**: デプロイワークフロー  
- **all**: 全部まとめて設定

## 🛠️ 使用方法

### **基本コマンド**
```bash
# NekoCode PR分析を設定
./nekocode-rust-clean/bin/gh-actions-setup.sh nekocode
```

### **実行例**
```bash
# 任意のGitリポジトリで
cd your-awesome-project/
/path/to/nekocode-rust-clean/bin/gh-actions-setup.sh nekocode

# 結果
🚀 GitHub Actions Setup Tool
   面倒な設定を1コマンドで！

ℹ️  リポジトリ: your-awesome-project  
ℹ️  デフォルトブランチ: main
🚀 NekoCode PR分析ワークフローを設定中...
✅ ディレクトリ作成: .github/workflows/
✅ ワークフローファイル作成: nekocode-analysis.yml
✅ Gitコミット完了

🎉 NekoCode分析ワークフロー設定完了！

ℹ️  次回PRを作成すると、自動でコード分析が実行されます
ℹ️  PRコメントに詳細な分析レポートが投稿されます
```

## 📊 設定される内容

### **生成されるファイル**
```
.github/workflows/nekocode-analysis.yml
```

### **ワークフロー機能**
- ✅ **自動実行**: PR作成・更新時
- ✅ **多言語対応**: JavaScript, TypeScript, Rust, Python, C++, C#, Go
- ✅ **詳細レポート**: ファイル数、分析時間、言語検出
- ✅ **エラー処理**: 失敗時の詳細表示
- ✅ **PRコメント**: 自動でレポート投稿・更新

### **レポート例**
```markdown
## 🦀 NekoCode 分析レポート

### 📊 コードの影響の概要

| メトリック | 価値 |
|-----------|------|
| **分析対象** | your-project ディレクトリ全体 |
| **分析されたファイル** | 127 ファイル |
| **分析時間** | 1.2s |
| **検出された言語** | 4言語 |

### ✅ 分析ステータス
- **コード品質**: NekoCode分析が正常完了
- **パフォーマンス**: 分析時間 1.2s  
- **互換性**: 検出されたファイルがエラーなしで処理完了
```

## 🔧 前提条件

- Gitリポジトリ内で実行
- GitHub リポジトリ（GitHub Actionsが利用可能）
- インターネット接続（NekoCodeバイナリダウンロード用）

## 💡 使用場面

### **こんな時に便利**
- ✅ 新しいプロジェクトでPR分析を導入したい
- ✅ 他のプロジェクトでもnyashと同じ分析を使いたい
- ✅ GitHub Actions設定が面倒すぎる
- ✅ YAMLの構文を覚えるのが面倒

### **実際のユースケース**
1. **新プロジェクト**: `git init` → `gh-actions-setup.sh nekocode` → 完了
2. **既存プロジェクト**: PR分析をサクッと追加
3. **チーム導入**: 全リポジトリに統一設定を一括展開

## ⚠️ 注意点

- 既存の `.github/workflows/nekocode-analysis.yml` は上書きされます
- 自動でGitコミットが作成されます
- PRにコメント投稿権限が必要です（通常は自動設定）

## 🚀 今後の拡張予定

### **Phase 2: CI/CDサポート**
```bash
./gh-actions-setup.sh test-ci rust     # Rustプロジェクト用CI
./gh-actions-setup.sh test-ci node     # Node.jsプロジェクト用CI
./gh-actions-setup.sh test-ci python   # Pythonプロジェクト用CI
```

### **Phase 3: デプロイサポート**
```bash
./gh-actions-setup.sh deploy pages     # GitHub Pages
./gh-actions-setup.sh deploy vercel    # Vercel
./gh-actions-setup.sh deploy docker    # Docker Hub
```

### **Phase 4: 統合セットアップ**
```bash
./gh-actions-setup.sh all              # 全機能まとめて
```

## 📝 トラブルシューティング

### **Q: 権限エラーが出る**
```bash
chmod +x nekocode-rust-clean/bin/gh-actions-setup.sh
```

### **Q: Gitリポジトリではない**
```bash
git init
git remote add origin https://github.com/user/repo.git
```

### **Q: ワークフローが実行されない**
- GitHub リポジトリの Settings → Actions → Allow all actions を確認
- ブランチ保護ルールの確認

## 🎉 まとめ

**たった1コマンドで、nyashレベルの高機能PR分析が任意のリポジトリに追加できます！**

```bash
# これだけで完了！
./nekocode-rust-clean/bin/gh-actions-setup.sh nekocode
```

面倒な設定作業から解放されて、開発に集中できますにゃ！ 🐱