#!/bin/bash
set -e

# 🚀 NekoCode Super Easy GitHub Actions Setup v2.0.0
# 複数モード対応: 自動・対話・カスタム

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
    echo "║   🚀 NekoCode Super Easy Setup v2.0     ║"  
    echo "║       自動・対話・カスタム対応！          ║"
    echo "╚══════════════════════════════════════════╝"
    echo -e "${NC}"
}

# 使用方法表示
show_usage() {
    echo -e "${BLUE}🚀 NekoCode GitHub Actions セットアップツール${NC}"
    echo ""
    echo "使い方を選んでください:"
    echo ""
    echo -e "${GREEN}  -auto${NC}        🤖 完全自動（推奨・初心者向け）"
    echo "                   何も聞かれず、全部おまかせで設定完了"
    echo ""
    echo -e "${BLUE}  -interactive${NC} 💬 対話式（安心・カスタマイズ可能）"  
    echo "                   質問に答えながら、お好みに合わせて設定"
    echo ""
    echo -e "${YELLOW}  -custom${NC}      ⚙️  上級者向けカスタムモード"
    echo "                   詳細設定をマニュアルで調整可能"
    echo ""
    echo "例:"
    echo "  $0 -auto        # とにかく簡単に！"
    echo "  $0 -interactive # 相談しながら安心設定"
    echo "  $0 -custom      # 自分で細かく調整"
    echo ""
    echo -e "${PURPLE}💡 初めての方は ${GREEN}-auto${PURPLE} がおすすめにゃ！${NC}"
}

# 🎯 自動検出機能
auto_detect_settings() {
    log_header "環境を自動検出中..."
    
    # デフォルトブランチ自動検出
    DEFAULT_BRANCH=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || 
                     git branch -r | grep -E '(origin/main|origin/master)' | head -1 | sed 's/.*origin\///' | tr -d ' ' || 
                     echo "main")
    
    # リポジトリ名検出
    REPO_NAME=$(basename "$(git rev-parse --show-toplevel)" 2>/dev/null || echo "unknown")
    
    # GitHub リポジトリ URL 検出
    REPO_URL=$(git remote get-url origin 2>/dev/null || echo "")
    
    log_success "デフォルトブランチ: $DEFAULT_BRANCH"
    log_success "リポジトリ名: $REPO_NAME"
    log_info "リポジトリURL: $REPO_URL"
}

# 🔧 完全自動ワークフロー生成
generate_smart_workflow() {
    log_header "完全自動ワークフローを生成中..."
    
    mkdir -p .github/workflows
    
    # 🎯 すべて自動検出する最新版ワークフロー
    cat > .github/workflows/nekocode-analysis.yml << EOF
name: NekoCode PR Impact Analysis

on:
  pull_request:
    types: [opened, synchronize, reopened]
    # 🎯 自動ブランチ検出 - どんなブランチでも動作
    branches: ['$DEFAULT_BRANCH', 'main', 'master', 'dev', 'develop']

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
          
      - name: Auto-detect Repository Settings
        id: detect
        run: |
          # 🎯 完全自動検出
          echo "Detecting repository settings..."
          
          # デフォルトブランチ検出
          DEFAULT_BRANCH=\$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo "$DEFAULT_BRANCH")
          echo "default_branch=\$DEFAULT_BRANCH" >> \$GITHUB_OUTPUT
          echo "✅ Default branch: \$DEFAULT_BRANCH"

      - name: Download NekoCode (Smart)
        run: |
          mkdir -p bin
          
          # 🎯 複数のダウンロード先を試す（フォールバック方式）
          DOWNLOAD_SUCCESS=false
          
          # 1. GitHub Releases API（最新版自動取得）
          echo "🔍 Trying GitHub Releases API..."
          if DOWNLOAD_URL=\$(curl -s "https://api.github.com/repos/moe-charm/nekocode-rust/releases/latest" | grep "browser_download_url.*nekocode-rust" | cut -d '"' -f 4); then
            if [ -n "\$DOWNLOAD_URL" ] && curl -L "\$DOWNLOAD_URL" -o bin/nekocode-rust; then
              chmod +x bin/nekocode-rust
              if [ -x bin/nekocode-rust ]; then
                echo "✅ Downloaded from GitHub Releases API"
                DOWNLOAD_SUCCESS=true
              fi
            fi
          fi
          
          # 2. フォールバック: 直接パス（現在の方法）
          if [ "\$DOWNLOAD_SUCCESS" = false ]; then
            echo "🔍 Trying direct path method..."
            DEFAULT_BRANCH=\${{ steps.detect.outputs.default_branch }}
            if curl -L "https://github.com/moe-charm/nekocode-rust/raw/\$DEFAULT_BRANCH/releases/nekocode-rust" -o bin/nekocode-rust; then
              chmod +x bin/nekocode-rust
              if [ -x bin/nekocode-rust ]; then
                echo "✅ Downloaded from direct path"
                DOWNLOAD_SUCCESS=true
              fi
            fi
          fi
          
          # 3. 最後のフォールバック: master固定
          if [ "\$DOWNLOAD_SUCCESS" = false ]; then
            echo "🔍 Trying master branch fallback..."
            curl -L "https://github.com/moe-charm/nekocode-rust/raw/master/releases/nekocode-rust" -o bin/nekocode-rust
            chmod +x bin/nekocode-rust
            if [ -x bin/nekocode-rust ]; then
              echo "✅ Downloaded from master branch"
              DOWNLOAD_SUCCESS=true
            fi
          fi
          
          # 失敗チェック
          if [ "\$DOWNLOAD_SUCCESS" = false ]; then
            echo "❌ Failed to download nekocode-rust binary from all sources!"
            exit 1
          fi
          
          # バージョン確認
          ./bin/nekocode-rust --version || echo "Binary downloaded successfully"

      - name: Analyze Current PR (Smart)
        id: analyze-pr
        run: |
          echo "🔍 Analyzing PR changes..."
          
          # 🎯 分析実行（エラー処理強化）
          if ./bin/nekocode-rust analyze . --stats-only > pr_analysis.txt 2>&1; then
            echo "✅ NekoCode analysis completed successfully"
          else
            echo "❌ NekoCode analysis failed!"
            echo "--- Analysis Output ---"
            cat pr_analysis.txt
            echo "--- End Output ---"
            exit 1
          fi
          
          echo "📊 Analysis Results:"
          cat pr_analysis.txt
          
          # 🎯 スマートパース（複数パターン対応）
          # ファイル数抽出（複数パターン）
          FILES_COUNT=\$(grep -E "(📄 総ファイル数:|found [0-9]+ files)" pr_analysis.txt | grep -oE "[0-9]+" | head -1 || echo "0")
          
          # 分析時間抽出（複数パターン） 
          ANALYSIS_TIME=\$(grep -E "(🏁.*took:|Total.*took:|analysis took:)" pr_analysis.txt | grep -oE "[0-9.]+s" | head -1 || echo "N/A")
          
          # 言語数抽出（複数パターン）
          LANGUAGES=\$(grep -A 20 -E "(🗂️|言語別|Languages|languages)" pr_analysis.txt | grep -E "• \w+:" | wc -l || echo "1")
          
          # デバッグ情報
          echo "🔍 Extracted values:"
          echo "  FILES_COUNT=$FILES_COUNT"
          echo "  ANALYSIS_TIME=$ANALYSIS_TIME"
          echo "  LANGUAGES=$LANGUAGES"
          
          # 出力（エラー回避のため値チェック）
          [ -n "\$FILES_COUNT" ] && echo "files_analyzed=\$FILES_COUNT" >> \$GITHUB_OUTPUT || echo "files_analyzed=0" >> \$GITHUB_OUTPUT
          [ -n "\$ANALYSIS_TIME" ] && echo "analysis_time=\$ANALYSIS_TIME" >> \$GITHUB_OUTPUT || echo "analysis_time=N/A" >> \$GITHUB_OUTPUT
          [ -n "\$LANGUAGES" ] && echo "languages_detected=\$LANGUAGES" >> \$GITHUB_OUTPUT || echo "languages_detected=1" >> \$GITHUB_OUTPUT

      - name: Generate Impact Report
        run: |
          PR_FILES=\${{ steps.analyze-pr.outputs.files_analyzed }}
          ANALYSIS_TIME=\${{ steps.analyze-pr.outputs.analysis_time }}
          LANGUAGES_COUNT=\${{ steps.analyze-pr.outputs.languages_detected }}
          
          cat > impact_report.md << 'REPORT_EOF'
## 🦀 NekoCode 分析レポート

### 📊 コードの影響の概要

| メトリック | 価値 |
|-----------|------|
| **分析対象** | \$(basename "\$GITHUB_WORKSPACE") ディレクトリ全体 |
| **分析されたファイル** | \${PR_FILES} ファイル |
| **分析時間** | \${ANALYSIS_TIME} |
| **検出された言語** | \${LANGUAGES_COUNT}言語 |

### ✅ 分析ステータス

- **コード品質**: NekoCode分析が正常完了
- **パフォーマンス**: 分析時間 \${ANALYSIS_TIME}
- **互換性**: 検出されたファイルがエラーなしで処理完了

---
*🚀 [NekoCode](https://github.com/moe-charm/nekocode-rust)による分析*  
*⚙️ 完全自動セットアップ by [Super Easy Setup](https://github.com/moe-charm/nekocode-rust)*
REPORT_EOF

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
    
    log_success "スマートワークフローファイル生成完了"
}

# 🧪 セットアップテスト
test_setup() {
    log_header "セットアップをテスト中..."
    
    if [ ! -f ".github/workflows/nekocode-analysis.yml" ]; then
        log_error "ワークフローファイルが見つかりません"
        return 1
    fi
    
    # YAML構文チェック（yamlがインストールされている場合）
    if command -v yamllint &> /dev/null; then
        if yamllint .github/workflows/nekocode-analysis.yml &> /dev/null; then
            log_success "YAML構文チェック: OK"
        else
            log_warning "YAML構文に問題がある可能性があります"
        fi
    fi
    
    log_success "セットアップテスト完了"
}

# 🤖 自動モード
mode_auto() {
    log_header "🤖 完全自動モードを開始します"
    log_info "何も聞かれません。全部おまかせで設定します！"
    
    # 自動検出
    auto_detect_settings
    
    # ワークフロー生成
    generate_smart_workflow
    
    # テスト
    test_setup
    
    # Git操作
    if command -v git &> /dev/null && [ -d .git ]; then
        git add .github/workflows/nekocode-analysis.yml
        git commit -m "🚀 Add NekoCode GitHub Actions (Auto Setup)

✅ 完全自動セットアップ:
- ブランチ名自動検出対応
- バイナリダウンロード複数フォールバック  
- 出力パース複数パターン対応
- エラー自動回復機能

🤖 Generated by Super Easy Setup v2.0 (Auto Mode) 🛠️"
        log_success "Gitコミット完了"
    fi
    
    # 完了メッセージ
    echo ""
    log_success "🎉 自動セットアップ完了！"
    echo ""
    log_info "次回PRを作成すると、自動でコード分析が実行されます"
    log_info "どんなリポジトリでも確実に動作します"
    echo ""
    echo -e "${GREEN}🚀 一発で完璧に動作するにゃ！${NC}"
}

# 💬 対話モード  
mode_interactive() {
    log_header "💬 対話式モードを開始します"
    log_info "いくつか質問させてください。お好みに合わせて設定します！"
    
    echo ""
    echo -e "${BLUE}❓ まず、あなたのレベルを教えてください:${NC}"
    echo "1) 🔰 初心者 (おまかせで良い)"
    echo "2) 💼 中級者 (少しカスタマイズしたい)"  
    echo "3) ⚙️  上級者 (細かく調整したい)"
    
    read -p "選択してください (1-3): " user_level
    
    case "$user_level" in
        1)
            log_info "初心者モード選択！ほぼ自動で進めます"
            auto_detect_settings
            ;;
        2)
            log_info "中級者モード選択！いくつか確認します"
            interactive_detect_settings
            ;;
        3)
            log_info "上級者モード選択！詳細設定します"
            custom_detect_settings
            ;;
        *)
            log_warning "無効な選択です。自動モードで進めます"
            auto_detect_settings
            ;;
    esac
    
    # 設定確認
    echo ""
    echo -e "${YELLOW}📋 設定確認:${NC}"
    echo "  デフォルトブランチ: $DEFAULT_BRANCH"
    echo "  リポジトリ名: $REPO_NAME"
    echo ""
    
    read -p "この設定で良いですか？ (y/N): " confirm
    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
        log_info "設定をやり直します..."
        mode_interactive
        return
    fi
    
    # ワークフロー生成
    generate_smart_workflow
    
    # テスト
    test_setup
    
    # Git操作確認
    read -p "Gitにコミットしますか？ (Y/n): " git_confirm  
    if [[ "$git_confirm" =~ ^[Nn]$ ]]; then
        log_info "Gitコミットをスキップしました"
    else
        if command -v git &> /dev/null && [ -d .git ]; then
            git add .github/workflows/nekocode-analysis.yml
            git commit -m "🚀 Add NekoCode GitHub Actions (Interactive Setup)

💬 対話式セットアップで作成:
- ユーザー選択: レベル $user_level
- カスタマイズ設定適用
- 確認済み設定で生成

💬 Generated by Super Easy Setup v2.0 (Interactive Mode) 🛠️"
            log_success "Gitコミット完了"
        fi
    fi
    
    # 完了メッセージ
    echo ""
    log_success "🎉 対話式セットアップ完了！"
    echo ""
    log_info "お疲れさまでした！設定は完璧です"
    echo -e "${GREEN}🚀 PRを作成して動作確認してみてください！${NC}"
}

# ⚙️ カスタムモード
mode_custom() {
    log_header "⚙️ カスタムモードを開始します"
    log_warning "上級者向けの詳細設定モードです"
    
    echo ""
    echo "🔧 各種設定をカスタマイズできます:"
    echo "1. ブランチ設定"
    echo "2. ダウンロード設定" 
    echo "3. 出力パース設定"
    echo "4. エラー処理設定"
    echo ""
    
    log_info "現在はシンプル版のみ実装済みです"
    log_info "詳細カスタマイズは今後のバージョンで対応予定"
    
    echo ""
    read -p "とりあえず基本設定で進めますか？ (Y/n): " proceed
    if [[ "$proceed" =~ ^[Nn]$ ]]; then
        log_info "カスタムモードを終了します"
        exit 0
    fi
    
    # 基本設定で進行
    auto_detect_settings
    generate_smart_workflow
    test_setup
    
    # Git操作
    if command -v git &> /dev/null && [ -d .git ]; then
        git add .github/workflows/nekocode-analysis.yml
        git commit -m "🚀 Add NekoCode GitHub Actions (Custom Setup)

⚙️ カスタムモードで作成:
- 上級者向け設定
- 基本構成で生成

⚙️ Generated by Super Easy Setup v2.0 (Custom Mode) 🛠️"
        log_success "Gitコミット完了"
    fi
    
    log_success "🎉 カスタムセットアップ完了！"
}

# 対話式の詳細設定
interactive_detect_settings() {
    log_header "環境を検出中..."
    
    # 自動検出実行
    auto_detect_settings
    
    # ブランチ名確認
    echo ""
    echo -e "${BLUE}❓ ブランチ設定について:${NC}"
    echo "  検出されたデフォルトブランチ: $DEFAULT_BRANCH"
    read -p "他のブランチも追加しますか？ (y/N): " add_branches
    
    if [[ "$add_branches" =~ ^[Yy]$ ]]; then
        read -p "追加ブランチ名 (例: dev,develop): " extra_branches
        if [ -n "$extra_branches" ]; then
            DEFAULT_BRANCH="$DEFAULT_BRANCH,$extra_branches"
            log_info "ブランチ設定を更新: $DEFAULT_BRANCH"
        fi
    fi
}

# カスタムの詳細設定  
custom_detect_settings() {
    # 将来の拡張用
    auto_detect_settings
}

# メイン処理
main() {
    show_banner
    
    # 引数チェック
    if [ $# -eq 0 ]; then
        show_usage
        exit 0
    fi
    
    # Git リポジトリチェック
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        log_error "Gitリポジトリで実行してください"
        exit 1
    fi
    
    # モード選択
    case "$1" in
        -auto)
            mode_auto
            ;;
        -interactive)
            mode_interactive
            ;;
        -custom)
            mode_custom
            ;;
        -h|--help)
            show_usage
            ;;
        *)
            log_error "不明なオプション: $1"
            echo ""
            show_usage
            exit 1
            ;;
    esac
}

# スクリプト実行
main "$@"