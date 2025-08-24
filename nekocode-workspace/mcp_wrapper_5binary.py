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
        releases_dir = os.path.join(os.path.dirname(workspace_dir), 'releases')
        # Prefer prebuilt releases binaries if available; fallback to local target builds
        def pick(bin_name: str) -> str:
            rel = os.path.join(releases_dir, bin_name)
            tgt = os.path.join(workspace_dir, 'target/release', bin_name)
            return rel if os.path.exists(rel) else tgt
        self.binaries = {
            'nekocode': pick('nekocode'),
            'nekorefactor': pick('nekorefactor'),
            'nekoimpact': pick('nekoimpact'),
            'nekoinc': pick('nekoinc'),
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
        
        # Keep last preview params for confirm step
        if not hasattr(self, 'last_preview_params'):
            self.last_preview_params = None
        
        # コマンドマッピング
        if tool == 'replace_preview':
            # Use --preview flag on replace
            file_path = args['file_path']
            pattern = args['pattern']
            replacement = args['replacement']
            cmd.extend(['replace', file_path, pattern, replacement, '--preview'])
            # Save preview params for confirm
            self.last_preview_params = ('replace', file_path, pattern, replacement)
        elif tool == 'replace_confirm':
            # Re-run last preview without --preview
            if not self.last_preview_params or self.last_preview_params[0] != 'replace':
                return {'error': 'No replace preview to confirm'}
            _, file_path, pattern, replacement = self.last_preview_params
            cmd.extend(['replace', file_path, pattern, replacement])
        elif tool == 'insert_preview':
            file_path = args['file_path']
            content = args['content']
            # position can be 'start', 'end', or a line number
            position = args.get('position')
            cmd.extend(['insert', file_path, content])
            if position:
                # If numeric string, treat as line number
                cmd.append(str(position))
            cmd.append('--preview')
            self.last_preview_params = ('insert', file_path, content, position)
        elif tool == 'insert_confirm':
            if not self.last_preview_params or self.last_preview_params[0] != 'insert':
                return {'error': 'No insert preview to confirm'}
            _, file_path, content, position = self.last_preview_params
            cmd.extend(['insert', file_path, content])
            if position:
                cmd.append(str(position))
        elif tool == 'create_file':
            # 新機能！AIが喜ぶ
            cmd.extend(['create-file', args['path']])
            if 'template' in args:
                cmd.extend(['--template', args['template']])
        elif tool == 'movelines_preview':
            # move-lines SOURCE START COUNT DEST INSERT --preview
            source = args['source']
            start = str(args['start_line'])
            count = str(args['line_count'])
            destination = args['destination']
            insert_line = str(args['insert_line'])
            cmd.extend(['move-lines', source, start, count, destination, insert_line, '--preview'])
            self.last_preview_params = ('move-lines', source, start, count, destination, insert_line)
        elif tool == 'movelines_confirm':
            if not self.last_preview_params or self.last_preview_params[0] != 'move-lines':
                return {'error': 'No movelines preview to confirm'}
            _, source, start, count, destination, insert_line = self.last_preview_params
            cmd.extend(['move-lines', source, start, count, destination, insert_line])
        elif tool == 'moveclass_preview':
            # move-class SESSION_ID SYMBOL_ID TARGET [--update-imports] --preview
            sid = args['session_id']
            symbol = args['symbol_id']
            target = args['target']
            cmd.extend(['move-class', sid, symbol, target, '--preview'])
            if args.get('update_imports'):
                cmd.append('--update-imports')
            self.last_preview_params = ('move-class', sid, symbol, target, bool(args.get('update_imports')))
        elif tool == 'moveclass_confirm':
            if not self.last_preview_params or self.last_preview_params[0] != 'move-class':
                return {'error': 'No moveclass preview to confirm'}
            _, sid, symbol, target, update_imports = self.last_preview_params
            cmd.extend(['move-class', sid, symbol, target])
            if update_imports:
                cmd.append('--update-imports')
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
        
        def run(cmd_list):
            return subprocess.run(cmd_list, capture_output=True, text=True)
        
        def needs_legacy(err_out: str) -> bool:
            if not err_out:
                return False
            # Detect old CLI that expects positional SESSION_ID
            return (
                "unexpected argument '--session-id'" in err_out
                or 'Usage: nekocode ast-query <SESSION_ID>' in err_out
                or 'Usage: nekocode ast-dump <SESSION_ID>' in err_out
                or 'Usage: nekocode ast-stats <SESSION_ID>' in err_out
            )
        
        if tool == 'session_create':
            cmd.extend(['session-create', args.get('path', '.')])
        elif tool == 'session_update':
            cmd.extend(['session-update', args['session_id']])
            if args.get('verbose'):
                cmd.append('--verbose')
        elif tool == 'session_list':
            cmd.extend(['session-list'])
            if args.get('detailed'):
                cmd.append('--detailed')
        elif tool == 'session_info':
            cmd.extend(['session-info', args['session_id']])
        elif tool == 'refresh':
            cmd.extend(['refresh', args['session_id']])
            if args.get('deps'):
                cmd.append('--deps')
            if args.get('deadcode'):
                cmd.append('--deadcode')
            if args.get('security'):
                cmd.append('--security')
            if args.get('quality'):
                cmd.append('--quality')
            if f := args.get('file'):
                cmd.extend(['--file', f])
            if args.get('external'):
                cmd.append('--external')
            if fmt := args.get('format'):
                cmd.extend(['--format', fmt])
        elif tool == 'deadcode':
            cmd.append('deadcode')
            if sid := args.get('session_id'):
                cmd.extend(['--session-id', sid])
            if args.get('external'):
                cmd.append('--external')
            if fmt := args.get('format'):
                cmd.extend(['--format', fmt])
            if mc := args.get('min_confidence'):
                cmd.extend(['--min-confidence', str(mc)])
            if out := args.get('output'):
                cmd.extend(['--output', out])
        elif tool == 'ast_stats':
            # New style: optional session via --session-id (auto memory otherwise)
            cmd_new = cmd + ['ast-stats']
            if sid := args.get('session_id'):
                cmd_new += ['--session-id', sid]
            result = run(cmd_new)
            if needs_legacy(result.stderr):
                # Legacy fallback: positional session_id required
                cmd_legacy = [self.binaries['nekocode'], 'ast-stats']
                if sid := args.get('session_id'):
                    cmd_legacy.append(sid)
                result = run(cmd_legacy)
            return {'content': [{'type': 'text', 'text': result.stdout or result.stderr}]}
        elif tool == 'ast_dump':
            # New style first
            cmd_new = cmd + ['ast-dump']
            if sid := args.get('session_id'):
                cmd_new += ['--session-id', sid]
            if 'format' in args:
                cmd_new += ['--format', args['format']]
            result = run(cmd_new)
            if needs_legacy(result.stderr):
                # Legacy fallback expects positional session_id then optional format
                cmd_legacy = [self.binaries['nekocode'], 'ast-dump']
                if sid := args.get('session_id'):
                    cmd_legacy.append(sid)
                if 'format' in args:
                    cmd_legacy += ['--format', args['format']]
                result = run(cmd_legacy)
            return {'content': [{'type': 'text', 'text': result.stdout or result.stderr}]}
        elif tool == 'ast_query':
            # New style: ast-query <PATH> [--session-id SID]
            path = args['path']
            cmd_new = cmd + ['ast-query', path]
            if sid := args.get('session_id'):
                cmd_new += ['--session-id', sid]
            result = run(cmd_new)
            if needs_legacy(result.stderr):
                # Legacy fallback: ast-query <SESSION_ID> <PATH>
                cmd_legacy = [self.binaries['nekocode'], 'ast-query']
                if sid := args.get('session_id'):
                    cmd_legacy += [sid, path]
                else:
                    # No session provided and legacy required → return helpful error
                    return {'error': "No session specified and legacy CLI requires <SESSION_ID>. Run 'session_create' first."}
                result = run(cmd_legacy)
            return {'content': [{'type': 'text', 'text': result.stdout or result.stderr}]}
        elif tool == 'analyze':
            # Deprecated: emulate by creating a session then returning stats or info
            create = run([self.binaries['nekocode'], 'session-create', args.get('path', '.')])
            sid = None
            for line in (create.stdout or '').split('\n'):
                if 'Created session:' in line:
                    sid = line.split(':', 1)[1].strip()
                    break
            if not sid:
                return {'content': [{'type': 'text', 'text': create.stdout or create.stderr}]}
            if args.get('stats_only'):
                # Use new style first, fallback to legacy
                result = run([self.binaries['nekocode'], 'ast-stats', '--session-id', sid])
                if needs_legacy(result.stderr):
                    result = run([self.binaries['nekocode'], 'ast-stats', sid])
            else:
                result = run([self.binaries['nekocode'], 'session-info', sid])
            return {'content': [{'type': 'text', 'text': result.stdout or result.stderr}]}
        
        # Default execution path for other commands
        result = run(cmd)
        
        # セッションIDを抽出して保存
        if tool == 'session_create':
            for line in (result.stdout or '').split('\n'):
                if 'Created session:' in line:
                    session_id = line.split(':', 1)[1].strip()
                    self.sessions['last'] = session_id
                    break
        
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
                # nekocode (解析エンジン) - セッション起点に統一
                {'name': 'session_create', 'description': '🎮 セッション作成'},
                {'name': 'session_update', 'description': '🔄 セッション更新'},
                {'name': 'session_list', 'description': '📋 セッション一覧'},
                {'name': 'session_info', 'description': 'ℹ️ セッション情報'},
                {'name': 'refresh', 'description': '🔁 セッション更新(スマート)'},
                {'name': 'deadcode', 'description': '🧹 デッドコード検出'},
                {'name': 'ast_stats', 'description': '📊 AST統計'},
                {'name': 'ast_dump', 'description': '🌳 AST出力'},
                {'name': 'ast_query', 'description': '🔍 AST検索'},
                
                # nekorefactor (リファクタリング)
                {'name': 'replace_preview', 'description': '📝 置換プレビュー'},
                {'name': 'replace_confirm', 'description': '✅ 置換実行'},
                {'name': 'insert_preview', 'description': '📝 挿入プレビュー'},
                {'name': 'insert_confirm', 'description': '✅ 挿入実行'},
                {'name': 'movelines_preview', 'description': '📝 行移動プレビュー'},
                {'name': 'movelines_confirm', 'description': '✅ 行移動実行'},
                {'name': 'moveclass_preview', 'description': '📝 クラス移動プレビュー'},
                {'name': 'moveclass_confirm', 'description': '✅ クラス移動実行'},
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
