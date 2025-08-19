#!/usr/bin/env python3
"""
🐱 NekoCode 5-Binary Split Edition セットアップ
Unix Philosophy: Do One Thing and Do It Well
"""
import os
import sys
import subprocess

def main():
    # カラー定義
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    BOLD = '\033[1m'
    RESET = '\033[0m'
    
    # ディレクトリ構造を確認
    current_dir = os.path.dirname(os.path.abspath(__file__))
    workspace_dir = current_dir  # nekocode-workspace/
    project_root = os.path.dirname(workspace_dir)  # nekocode-rust-clean/
    
    print(f"""
{CYAN}{BOLD}🐱 NekoCode 5-Binary Split Edition セットアップ{RESET}
{CYAN}{'='*60}{RESET}

{BOLD}📦 Unix哲学に基づく5分割アーキテクチャ:{RESET}
  • {GREEN}nekocode{RESET}     - コア解析エンジン (AST解析・セッション管理)
  • {GREEN}nekorefactor{RESET} - リファクタリング専用ツール
  • {GREEN}nekoimpact{RESET}   - 変更影響分析ツール
  • {GREEN}nekoinc{RESET}      - インクリメンタル解析・ファイル監視
  • {GREEN}nekomcp{RESET}      - MCP統合ゲートウェイ
""")
    
    # バイナリの存在確認
    print(f"{BOLD}🔍 バイナリチェック中...{RESET}")
    
    binaries = {
        'nekocode': os.path.join(workspace_dir, 'target', 'release', 'nekocode'),
        'nekorefactor': os.path.join(workspace_dir, 'target', 'release', 'nekorefactor'),
        'nekoimpact': os.path.join(workspace_dir, 'target', 'release', 'nekoimpact'),
        'nekoinc': os.path.join(workspace_dir, 'target', 'release', 'nekoinc'),
        'nekomcp': os.path.join(workspace_dir, 'target', 'release', 'nekomcp'),
    }
    
    missing = []
    found = []
    for name, path in binaries.items():
        if os.path.exists(path):
            # ファイルサイズ取得
            size_mb = os.path.getsize(path) / (1024 * 1024)
            found.append(f"  ✅ {name:<12} ({size_mb:.1f}MB)")
        else:
            missing.append(name)
    
    if found:
        print("\n".join(found))
    
    if missing:
        print(f"""
{RED}⚠️  以下のバイナリが見つかりません: {', '.join(missing)}{RESET}

{YELLOW}ビルドが必要です。以下を実行してください:{RESET}

  cd {workspace_dir}
  cargo build --release

{CYAN}💡 ヒント:{RESET}
  • ビルドには約3-5分かかります
  • 初回は依存関係のダウンロードで時間がかかります
  • 合計サイズは約25-30MBになります
""")
        sys.exit(1)
    
    # MCPラッパーの作成
    mcp_wrapper = os.path.join(workspace_dir, 'mcp_wrapper_5binary.py')
    
    print(f"\n{BOLD}🔧 MCPラッパー作成中...{RESET}")
    
    wrapper_content = '''#!/usr/bin/env python3
"""
5-Binary Split MCP Wrapper for Claude Code
自動生成されたファイル - 手動で編集しないでください
"""
import json
import sys
import subprocess
import os
from typing import Dict, Any

class NekoCode5BinaryMCP:
    def __init__(self):
        workspace_dir = os.path.dirname(os.path.abspath(__file__))
        self.binaries = {
            'nekocode': os.path.join(workspace_dir, 'target/release/nekocode'),
            'nekorefactor': os.path.join(workspace_dir, 'target/release/nekorefactor'),
            'nekoimpact': os.path.join(workspace_dir, 'target/release/nekoimpact'),
            'nekoinc': os.path.join(workspace_dir, 'target/release/nekoinc'),
        }
        self.sessions = {}
        self.last_preview_id = None
        
    def handle_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """MCPリクエストを処理"""
        method = request.get('method', '')
        params = request.get('params', {})
        
        if method == 'tools/call':
            tool_name = params.get('name', '')
            args = params.get('arguments', {})
            
            # リファクタリング系コマンド
            if tool_name in ['replace_preview', 'replace_confirm', 'insert_preview', 
                            'insert_confirm', 'movelines_preview', 'movelines_confirm',
                            'moveclass_preview', 'moveclass_confirm', 'create_file',
                            'extract_function', 'split_file']:
                return self._call_nekorefactor(tool_name, args)
            
            # 影響分析系コマンド
            elif tool_name in ['analyze_impact', 'compare_ref', 'risk_assessment']:
                return self._call_nekoimpact(tool_name, args)
            
            # インクリメンタル系コマンド
            elif tool_name in ['watch_start', 'watch_stop', 'incremental_update',
                             'track_changes', 'export_changes']:
                return self._call_nekoinc(tool_name, args)
            
            # デフォルトは解析エンジン
            else:
                return self._call_nekocode(tool_name, args)
        
        elif method == 'initialize':
            return self._handle_initialize()
        elif method == 'tools/list':
            return self._handle_list_tools()
        else:
            return {'error': f'Unknown method: {method}'}
    
    def _call_nekorefactor(self, tool: str, args: Dict) -> Dict:
        """nekorefactorを呼び出し"""
        cmd = [self.binaries['nekorefactor']]
        
        # コマンドマッピング
        if tool == 'replace_preview':
            cmd.extend(['replace-preview', args['file_path'], 
                       args['pattern'], args['replacement']])
        elif tool == 'replace_confirm':
            preview_id = args.get('preview_id', self.last_preview_id)
            cmd.extend(['replace-confirm', preview_id])
        elif tool == 'insert_preview':
            cmd.extend(['insert-preview', args['file_path'], 
                       args['position'], args['content']])
        elif tool == 'insert_confirm':
            preview_id = args.get('preview_id', self.last_preview_id)
            cmd.extend(['insert-confirm', preview_id])
        elif tool == 'create_file':
            # 新機能！AIが喜ぶ
            cmd.extend(['create-file', args['path']])
            if 'template' in args:
                cmd.extend(['--template', args['template']])
        elif tool == 'extract_function':
            cmd.extend(['extract-function', args['session_id'],
                       args['function'], args['target']])
        elif tool == 'split_file':
            cmd.extend(['split-file', args['file']])
            if 'by' in args:
                cmd.extend(['--by', args['by']])
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        
        # プレビューIDを抽出して保存
        if 'preview' in tool and 'Preview ID:' in result.stdout:
            for line in result.stdout.split('\\n'):
                if 'Preview ID:' in line:
                    self.last_preview_id = line.split('Preview ID:')[1].strip()
        
        return {'content': [{'type': 'text', 'text': result.stdout or result.stderr}]}
    
    def _call_nekoimpact(self, tool: str, args: Dict) -> Dict:
        """nekoimpactを呼び出し"""
        cmd = [self.binaries['nekoimpact'], 'analyze']
        
        if 'compare_ref' in args:
            cmd.extend(['--compare-ref', args['compare_ref']])
        if 'format' in args:
            cmd.extend(['--format', args['format']])
        
        cmd.append(args.get('path', '.'))
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        return {'content': [{'type': 'text', 'text': result.stdout or result.stderr}]}
    
    def _call_nekoinc(self, tool: str, args: Dict) -> Dict:
        """nekoincを呼び出し"""
        cmd = [self.binaries['nekoinc']]
        
        if tool == 'watch_start':
            cmd.extend(['watch', 'start', args.get('path', '.')])
            if 'session_id' in args:
                cmd.extend(['--session', args['session_id']])
        elif tool == 'watch_stop':
            cmd.extend(['watch', 'stop'])
        elif tool == 'incremental_update':
            cmd.extend(['update', args.get('session_id')])
        elif tool == 'track_changes':
            cmd.extend(['track', args.get('path', '.')])
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        return {'content': [{'type': 'text', 'text': result.stdout or result.stderr}]}
    
    def _call_nekocode(self, tool: str, args: Dict) -> Dict:
        """nekocodeを呼び出し"""
        cmd = [self.binaries['nekocode']]
        
        if tool == 'analyze':
            cmd.extend(['analyze', args.get('path', '.')])
            if args.get('stats_only'):
                cmd.append('--stats-only')
            if 'language' in args:
                cmd.extend(['--language', args['language']])
        elif tool == 'session_create':
            cmd.extend(['session-create', args.get('path', '.')])
        elif tool == 'session_update':
            cmd.extend(['session-update', args['session_id']])
            if args.get('verbose'):
                cmd.append('--verbose')
        elif tool == 'session_stats':
            cmd.extend(['session-stats', args['session_id']])
        elif tool == 'ast_dump':
            cmd.extend(['ast-dump', args['session_id']])
            if 'format' in args:
                cmd.extend(['--format', args['format']])
        elif tool == 'ast_query':
            cmd.extend(['ast-query', args['session_id'], args['path']])
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        
        # セッションIDを抽出して保存
        if tool == 'session_create' and 'Session ID:' in result.stdout:
            for line in result.stdout.split('\\n'):
                if 'Session ID:' in line:
                    session_id = line.split('Session ID:')[1].strip()
                    self.sessions['last'] = session_id
        
        return {'content': [{'type': 'text', 'text': result.stdout or result.stderr}]}
    
    def _handle_initialize(self) -> Dict:
        """初期化レスポンス"""
        return {
            'protocolVersion': '2024-11-05',
            'capabilities': {
                'tools': {},
                'resources': {}
            },
            'serverInfo': {
                'name': 'nekocode-5binary',
                'version': '0.2.0'
            }
        }
    
    def _handle_list_tools(self) -> Dict:
        """利用可能なツールリスト"""
        return {
            'tools': [
                # nekocode (解析エンジン)
                {'name': 'analyze', 'description': '🚀 プロジェクト解析'},
                {'name': 'session_create', 'description': '🎮 セッション作成'},
                {'name': 'session_update', 'description': '🔄 セッション更新'},
                {'name': 'session_stats', 'description': '📊 セッション統計'},
                {'name': 'ast_dump', 'description': '🌳 AST出力'},
                {'name': 'ast_query', 'description': '🔍 AST検索'},
                
                # nekorefactor (リファクタリング)
                {'name': 'replace_preview', 'description': '📝 置換プレビュー'},
                {'name': 'replace_confirm', 'description': '✅ 置換実行'},
                {'name': 'insert_preview', 'description': '📝 挿入プレビュー'},
                {'name': 'insert_confirm', 'description': '✅ 挿入実行'},
                {'name': 'create_file', 'description': '📄 ファイル作成 (新機能!)'},
                {'name': 'extract_function', 'description': '🔧 関数抽出'},
                {'name': 'split_file', 'description': '✂️ ファイル分割'},
                
                # nekoimpact (影響分析)
                {'name': 'analyze_impact', 'description': '💥 影響分析'},
                
                # nekoinc (インクリメンタル)
                {'name': 'watch_start', 'description': '👀 監視開始'},
                {'name': 'incremental_update', 'description': '⚡ 差分更新'},
            ]
        }
    
    def run(self):
        """stdio MCPサーバーとして実行"""
        while True:
            try:
                line = sys.stdin.readline()
                if not line:
                    break
                
                # Content-Lengthヘッダーをパース
                if line.startswith('Content-Length:'):
                    length = int(line.split(':')[1].strip())
                    sys.stdin.readline()  # 空行をスキップ
                    content = sys.stdin.read(length)
                    request = json.loads(content)
                    
                    # リクエスト処理
                    response = self.handle_request(request)
                    response['jsonrpc'] = '2.0'
                    response['id'] = request.get('id')
                    
                    # レスポンス送信
                    response_str = json.dumps(response)
                    sys.stdout.write(f'Content-Length: {len(response_str)}\\r\\n\\r\\n')
                    sys.stdout.write(response_str)
                    sys.stdout.flush()
            except Exception as e:
                sys.stderr.write(f'Error: {e}\\n')

if __name__ == '__main__':
    server = NekoCode5BinaryMCP()
    server.run()
'''
    
    # ラッパー作成
    with open(mcp_wrapper, 'w') as f:
        f.write(wrapper_content)
    os.chmod(mcp_wrapper, 0o755)
    print(f"  ✅ MCPラッパー作成完了")
    
    # 絶対パスを取得
    mcp_wrapper_abs = os.path.abspath(mcp_wrapper)
    
    print(f"""
{CYAN}{'='*60}{RESET}
{GREEN}{BOLD}✨ セットアップ完了！{RESET}

{BOLD}📋 Claude Code設定方法:{RESET}

{YELLOW}方法1: コマンドラインから追加 (推奨){RESET}
  
  1. {CYAN}あなたのプロジェクトに移動:{RESET}
     cd ~/your-awesome-project
  
  2. {CYAN}以下のコマンドを実行:{RESET}
     
     claude mcp add nekocode-5binary \\
       -- python3 {mcp_wrapper_abs}

{YELLOW}方法2: 設定ファイルを直接編集{RESET}

  {CYAN}Linux:{RESET} ~/.config/claude-desktop/config.json
  {CYAN}Mac:{RESET}   ~/Library/Application Support/Claude/claude_desktop_config.json
  
  以下を追加:
  {{
    "mcpServers": {{
      "nekocode-5binary": {{
        "command": "python3",
        "args": ["{mcp_wrapper_abs}"]
      }}
    }}
  }}

{BOLD}3. Claude Codeを再起動{RESET}

{CYAN}{'='*60}{RESET}

{BOLD}🎯 新機能 (AIが喜ぶ！):{RESET}
  • {GREEN}create_file{RESET}    - 新規ファイル作成 (解決済み！)
  • {YELLOW}--force{RESET}        - プレビュー省略 (開発中)
  • {YELLOW}--after-function{RESET} - セマンティック位置指定 (開発中)

{BOLD}💡 使用例:{RESET}
  # 新規ファイル作成 (AIが喜ぶ！)
  nekorefactor create-file todo.py --template python-cli
  
  # プレビューなしで直接実行 (開発中)
  nekorefactor replace "old" "new" --force
  
  # セマンティック位置指定 (開発中)
  nekorefactor insert file.py --after-function main "def helper():"

{CYAN}{'='*60}{RESET}
{BOLD}問題が発生した場合:{RESET}
  • バイナリが見つからない → cargo build --release
  • MCPが動かない → Claude Code再起動
  • それでもダメ → GitHub Issueで報告

{GREEN}{BOLD}Happy Coding with NekoCode! 🐱{RESET}
""")

if __name__ == '__main__':
    main()