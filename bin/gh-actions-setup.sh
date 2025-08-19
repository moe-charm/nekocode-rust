#!/bin/bash
set -e

# 🚀 GitHub Actions Quick Setup Tool v1.0.0
# 1コマンドでGitHub Actionsワークフローを爆速セットアップ！

# カラー定義
RED='\033[0;31m'
GREEN='\033[0;32m' 
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m'

# ログ関数
log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }
log_header() { echo -e "${PURPLE}🚀 $1${NC}"; }

# バナー表示
show_banner() {
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════╗"
    echo "║      🚀 GitHub Actions Setup Tool       ║"  
    echo "║         面倒な設定を1コマンドで！          ║"
    echo "╚══════════════════════════════════════════╝"
    echo -e "${NC}"
}

# 使用方法表示
show_usage() {
    echo "使用方法:"
    echo "  $0 nekocode      # NekoCode PR分析設定"
    echo "  $0 test-ci       # CI/CDテスト設定"  
    echo "  $0 deploy        # デプロイ設定"
    echo "  $0 all           # 全部まとめて設定"
    echo ""
    echo "例:"
    echo "  $0 nekocode      # nyashリポジトリみたいなPR分析"
}

# リポジトリ情報取得
get_repo_info() {
    REPO_NAME=$(basename "$(git rev-parse --show-toplevel)" 2>/dev/null || echo "unknown")
    REPO_REMOTE=$(git remote get-url origin 2>/dev/null || echo "")
    DEFAULT_BRANCH=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo "main")
    
    log_info "リポジトリ: $REPO_NAME"
    log_info "デフォルトブランチ: $DEFAULT_BRANCH"
}

# NekoCodeワークフロー設定
setup_nekocode_workflow() {
    log_header "NekoCode PR分析ワークフローを設定中..."
    
    # ディレクトリ作成
    mkdir -p .github/workflows
    log_success "ディレクトリ作成: .github/workflows/"
    
    # ワークフローファイル生成
    cat > .github/workflows/nekocode-analysis.yml << 'EOF'
name: NekoCode PR Impact Analysis

on:
  pull_request:
    types: [opened, synchronize, reopened]
    branches: [main, dev]

jobs:
  nekocode-analysis:
    runs-on: ubuntu-latest
    name: Code Impact Analysis
    
    permissions:
      contents: read
      pull-requests: write

    steps:
      - name: Checkout PR
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
          
      - name: Download NekoCode
        run: |
          mkdir -p bin
          curl -L https://github.com/moe-charm/nekocode-rust/raw/main/releases/nekocode-rust -o bin/nekocode-rust
          chmod +x bin/nekocode-rust

      - name: Analyze Current PR
        id: analyze-pr
        run: |
          echo "🔍 Analyzing PR changes..."
          
          # 🎯 全ディレクトリを分析対象に
          ./bin/nekocode-rust analyze . --stats-only > pr_analysis.txt 2>&1
          
          # 🎯 エラー処理
          if [ $? -ne 0 ]; then
            echo "❌ NekoCode analysis failed!"
            cat pr_analysis.txt
            exit 1
          fi
          
          echo "📊 Analysis Results:"
          cat pr_analysis.txt
          
          # 🎯 結果抽出
          FILES_COUNT=$(grep -E "found [0-9]+ files" pr_analysis.txt | grep -oE "[0-9]+" || echo "0")
          ANALYSIS_TIME=$(grep -E "Total.*took: [0-9.]+s" pr_analysis.txt | grep -oE "[0-9.]+s" || echo "N/A")
          LANGUAGES=$(grep -A 10 "言語別:" pr_analysis.txt | grep -E "• \w+:" | wc -l || echo "1")
          
          echo "files_analyzed=$FILES_COUNT" >> $GITHUB_OUTPUT
          echo "analysis_time=$ANALYSIS_TIME" >> $GITHUB_OUTPUT
          echo "languages_detected=$LANGUAGES" >> $GITHUB_OUTPUT

      - name: Generate Impact Report
        run: |
          PR_FILES=${{ steps.analyze-pr.outputs.files_analyzed }}
          ANALYSIS_TIME=${{ steps.analyze-pr.outputs.analysis_time }}
          LANGUAGES_COUNT=${{ steps.analyze-pr.outputs.languages_detected }}
          
          cat > impact_report.md << EOF
          ## 🦀 NekoCode 分析レポート
          
          ### 📊 コードの影響の概要
          
          | メトリック | 価値 |
          |-----------|------|
          | **分析対象** | $(basename "$GITHUB_WORKSPACE") ディレクトリ全体 |
          | **分析されたファイル** | ${PR_FILES} ファイル |
          | **分析時間** | ${ANALYSIS_TIME} |
          | **検出された言語** | ${LANGUAGES_COUNT}言語 |
          
          ### ✅ 分析ステータス
          
          - **コード品質**: NekoCode分析が正常完了
          - **パフォーマンス**: 分析時間 ${ANALYSIS_TIME}
          - **互換性**: 検出されたファイルがエラーなしで処理完了
          
          ---
          *🚀 [NekoCode](https://github.com/moe-charm/nekocode-rust)による分析*
          EOF

      - name: Comment PR
        uses: actions/github-script@v7
        with:
          script: |
            const fs = require('fs');
            const report = fs.readFileSync('impact_report.md', 'utf8');
            
            const comments = await github.rest.issues.listComments({
              owner: context.repo.owner,
              repo: context.repo.repo,
              issue_number: context.issue.number,
            });
            
            const nekocodeComment = comments.data.find(comment => 
              comment.body.includes('🦀 NekoCode 分析レポート')
            );
            
            if (nekocodeComment) {
              await github.rest.issues.updateComment({
                owner: context.repo.owner,
                repo: context.repo.repo,
                comment_id: nekocodeComment.id,
                body: report
              });
            } else {
              await github.rest.issues.createComment({
                owner: context.repo.owner,
                repo: context.repo.repo,
                issue_number: context.issue.number,
                body: report
              });
            }
EOF
    
    log_success "ワークフローファイル作成: nekocode-analysis.yml"
    
    # Git操作
    if command -v git &> /dev/null && [ -d .git ]; then
        git add .github/workflows/nekocode-analysis.yml
        git commit -m "🚀 Add NekoCode PR analysis workflow

- Automatic code analysis on pull requests
- Multi-language detection and reporting  
- Performance metrics and impact summary

Generated by GitHub Actions Setup Tool 🛠️"
        log_success "Gitコミット完了"
    fi
    
    # 完了メッセージ
    echo ""
    log_success "🎉 NekoCode分析ワークフロー設定完了！"
    echo ""
    log_info "次回PRを作成すると、自動でコード分析が実行されます"
    log_info "PRコメントに詳細な分析レポートが投稿されます"
    echo ""
}

# テストCIワークフロー設定（プレースホルダー）
setup_test_ci_workflow() {
    log_warning "test-ci機能は開発中です。近日公開予定！"
}

# デプロイワークフロー設定（プレースホルダー）  
setup_deploy_workflow() {
    log_warning "deploy機能は開発中です。近日公開予定！"
}

# 全設定（プレースホルダー）
setup_all_workflows() {
    log_warning "all機能は開発中です。近日公開予定！"
}

# メイン処理
main() {
    show_banner
    
    if [ $# -eq 0 ]; then
        show_usage
        exit 1
    fi
    
    # Git リポジトリチェック
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        log_error "Gitリポジトリで実行してください"
        exit 1
    fi
    
    get_repo_info
    
    case "$1" in
        nekocode)
            setup_nekocode_workflow
            ;;
        test-ci)
            setup_test_ci_workflow "$2"
            ;;
        deploy)
            setup_deploy_workflow "$2"
            ;;
        all)
            setup_all_workflows
            ;;
        *)
            log_error "不明なオプション: $1"
            show_usage
            exit 1
            ;;
    esac
}

# スクリプト実行
main "$@"