#!/usr/bin/env python3
"""
🐱 NekoCode 5-Binary Split MCP セットアップ
Unix Philosophy: Do One Thing and Do It Well
"""
import os
import sys

# ディレクトリ構造を確認
current_dir = os.path.dirname(os.path.abspath(__file__))
workspace_dir = current_dir  # nekocode-workspace/
project_root = os.path.dirname(workspace_dir)  # nekocode-rust-clean/

# 5つのバイナリパスを設定
binaries = {
    'nekocode': os.path.join(workspace_dir, 'target', 'release', 'nekocode'),
    'nekorefactor': os.path.join(workspace_dir, 'target', 'release', 'nekorefactor'),
    'nekoimpact': os.path.join(workspace_dir, 'target', 'release', 'nekoimpact'),
    'nekoinc': os.path.join(workspace_dir, 'target', 'release', 'nekoinc'),
    'nekomcp': os.path.join(workspace_dir, 'target', 'release', 'nekomcp'),
}

# バイナリの存在確認
missing = []
for name, path in binaries.items():
    if not os.path.exists(path):
        missing.append(name)

if missing:
    print(f"""
⚠️  以下のバイナリが見つかりません: {', '.join(missing)}

まず以下を実行してください:
cd {workspace_dir}
cargo build --release

これにより5つのバイナリがビルドされます:
- nekocode: コア解析エンジン
- nekorefactor: リファクタリング専用
- nekoimpact: 影響分析専用
- nekoinc: インクリメンタル解析
- nekomcp: MCP統合ゲートウェイ
""")
    sys.exit(1)

# MCPサーバーラッパーの場所
mcp_wrapper = os.path.join(workspace_dir, 'mcp_wrapper_5binary.py')

# ラッパーが存在しない場合は作成
if not os.path.exists(mcp_wrapper):
    wrapper_content = '''#!/usr/bin/env python3
"""
5-Binary Split MCP Wrapper for Claude Code
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
        
    def handle_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """MCPリクエストを処理"""
        method = request.get('method', '')
        params = request.get('params', {})
        
        # ツール呼び出しをルーティング
        if method == 'tools/call':
            tool_name = params.get('name', '')
            args = params.get('arguments', {})
            
            # リファクタリング系
            if tool_name in ['replace_preview', 'insert_preview', 'movelines_preview',
                            'moveclass_preview', 'create_file']:
                return self._call_nekorefactor(tool_name, args)
            
            # 影響分析系
            elif tool_name in ['analyze_impact', 'compare_ref']:
                return self._call_nekoimpact(tool_name, args)
            
            # インクリメンタル系
            elif tool_name in ['watch_start', 'incremental_update']:
                return self._call_nekoinc(tool_name, args)
            
            # デフォルトは解析エンジン
            else:
                return self._call_nekocode(tool_name, args)
        
        # その他のMCPメソッド
        elif method == 'initialize':
            return self._handle_initialize()
        elif method == 'tools/list':
            return self._handle_list_tools()
        else:
            return {'error': f'Unknown method: {method}'}
    
    def _call_nekorefactor(self, tool: str, args: Dict) -> Dict:
        """nekorefactorを呼び出し"""
        cmd = [self.binaries['nekorefactor']]
        
        # コマンドライン引数を構築
        if tool == 'replace_preview':
            cmd.extend(['replace-preview', args['file'], args['pattern'], args['replacement']])
        elif tool == 'create_file':
            cmd.extend(['create-file', args['path'], '--template', args.get('template', 'empty')])
        # ... 他のツールも同様
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        return {'content': [{'type': 'text', 'text': result.stdout}]}
    
    def _call_nekoimpact(self, tool: str, args: Dict) -> Dict:
        """nekoimpactを呼び出し"""
        cmd = [self.binaries['nekoimpact'], 'analyze']
        if 'compare_ref' in args:
            cmd.extend(['--compare-ref', args['compare_ref']])
        cmd.append(args.get('path', '.'))
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        return {'content': [{'type': 'text', 'text': result.stdout}]}
    
    def _call_nekoinc(self, tool: str, args: Dict) -> Dict:
        """nekoincを呼び出し"""
        cmd = [self.binaries['nekoinc']]
        if tool == 'watch_start':
            cmd.extend(['watch', '--path', args.get('path', '.')])
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        return {'content': [{'type': 'text', 'text': result.stdout}]}
    
    def _call_nekocode(self, tool: str, args: Dict) -> Dict:
        """nekocodeを呼び出し"""
        cmd = [self.binaries['nekocode']]
        
        if tool == 'session_create':
            cmd.extend(['session-create', args.get('path', '.')])
        elif tool == 'session_list':
            cmd.extend(['session-list'])
            if args.get('detailed'):
                cmd.append('--detailed')
        elif tool == 'session_info':
            cmd.extend(['session-info', args['session_id']])
        elif tool == 'ast_stats':
            cmd.append('ast-stats')
            if sid := args.get('session_id'):
                cmd.extend(['--session-id', sid])
        elif tool == 'ast_dump':
            cmd.extend(['ast-dump', args['session_id']])
            if 'format' in args:
                cmd.extend(['--format', args['format']])
        elif tool == 'ast_query':
            cmd.extend(['ast-query', args['session_id'], args['path']])
        elif tool == 'analyze':
            # 非推奨: セッション作成→ast-statsへルーティング
            create = subprocess.run([self.binaries['nekocode'], 'session-create', args.get('path', '.')], capture_output=True, text=True)
            sid = None
            for line in (create.stdout or '').split('\n'):
                if 'Created session:' in line:
                    sid = line.split(':', 1)[1].strip()
                    break
            if not sid:
                return {'content': [{'type': 'text', 'text': create.stdout or create.stderr}]}
            result = subprocess.run([self.binaries['nekocode'], 'ast-stats', sid], capture_output=True, text=True)
            return {'content': [{'type': 'text', 'text': result.stdout}]}
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        return {'content': [{'type': 'text', 'text': result.stdout}]}
    
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
                'version': '0.1.0'
            }
        }
    
    def _handle_list_tools(self) -> Dict:
        """利用可能なツールリスト"""
        return {
            'tools': [
                # nekocode tools
                {'name': 'session_create', 'description': 'Create session'},
                {'name': 'session_list', 'description': 'List sessions'},
                {'name': 'session_info', 'description': 'Session info'},
                {'name': 'ast_stats', 'description': 'AST statistics'},
                {'name': 'ast_dump', 'description': 'AST dump'},
                {'name': 'ast_query', 'description': 'AST query'},
                # nekorefactor tools
                {'name': 'replace_preview', 'description': 'Preview replacement'},
                {'name': 'create_file', 'description': 'Create new file'},
                # nekoimpact tools
                {'name': 'analyze_impact', 'description': 'Analyze impact'},
                # nekoinc tools
                {'name': 'watch_start', 'description': 'Start watching'},
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
                    sys.stdout.write(f'Content-Length: {len(response_str)}\r\n\r\n')
                    sys.stdout.write(response_str)
                    sys.stdout.flush()
            except Exception as e:
                sys.stderr.write(f'Error: {e}\n')

if __name__ == '__main__':
    server = NekoCode5BinaryMCP()
    server.run()
'''
    with open(mcp_wrapper, 'w') as f:
        f.write(wrapper_content)
    os.chmod(mcp_wrapper, 0o755)
    print(f"✅ MCPラッパーを作成しました: {mcp_wrapper}")

print(f"""
🚀 NekoCode 5-Binary Split MCP セットアップ
================================================

✅ すべてのバイナリが見つかりました！

Unix Philosophy: Do One Thing and Do It Well
- nekocode: 解析エンジン
- nekorefactor: リファクタリング
- nekoimpact: 影響分析
- nekoinc: インクリメンタル
- nekomcp: MCP統合

📋 Claude Code設定方法:

1. あなたのプロジェクトに移動:
   cd ~/your-project

2. MCPサーバーを追加:
   claude mcp add nekocode-5binary \\
     -- python3 {mcp_wrapper}

または手動で設定ファイルに追加:
~/.config/claude-desktop/config.json (Linux)
~/Library/Application Support/Claude/claude_desktop_config.json (Mac)

{{
  "mcpServers": {{
    "nekocode-5binary": {{
      "command": "python3",
      "args": ["{mcp_wrapper}"]
    }}
  }}
}}

3. Claude Codeを再起動

================================================
🎯 新機能:
- create-file: 新規ファイル作成（AIが喜ぶ！）
- セマンティック位置指定（開発中）
- --force オプション（開発中）
================================================
""")
