#!/usr/bin/env python3
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
            for line in result.stdout.split('\n'):
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
            for line in result.stdout.split('\n'):
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
                    sys.stdout.write(f'Content-Length: {len(response_str)}\r\n\r\n')
                    sys.stdout.write(response_str)
                    sys.stdout.flush()
            except Exception as e:
                sys.stderr.write(f'Error: {e}\n')

if __name__ == '__main__':
    server = NekoCode5BinaryMCP()
    server.run()
