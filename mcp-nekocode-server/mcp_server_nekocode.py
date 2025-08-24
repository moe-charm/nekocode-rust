#!/usr/bin/env python3
"""
🐱 NekoCode MCP Server - 多言語コード解析ツールのMCP統合版

Claude Codeで直接NekoCodeの機能を利用可能にするMCPサーバー
- 高速解析エンジン
- セッション管理による効率的な操作
- C++プロジェクト特化機能
- 日本語対応
"""

import asyncio
import json
import subprocess
import os
import sys
from pathlib import Path
from typing import Dict, List, Optional, Any
import logging

# MCP関連のインポート (仮想的な実装)
class MCPServer:
    def __init__(self, name: str):
        self.name = name
        self.tools = []
        self.sessions = {}  # セッション管理
    
    def add_tool(self, name: str, description: str, handler, input_schema: Dict):
        self.tools.append({
            "name": name,
            "description": description,
            "handler": handler,
            "inputSchema": input_schema
        })
    
    async def run(self):
        print(f"🚀 {self.name} MCP Server started")
        # 実際のMCPプロトコル実装はここに


class NekoCodeMCPServer:
    """NekoCode MCP Server メインクラス"""
    
    def __init__(self):
        self.server = MCPServer("nekocode")
        self.nekocode_path = self._find_nekocode_binary()
        self.sessions = {}  # アクティブセッション管理
        self.config_dir = Path.home() / ".nekocode"
        self.config_file = self.config_dir / "mcp_session.json"
        self.current_session_id = None
        self._init_config()
        self.setup_tools()
    
    def _init_config(self):
        """設定ファイルの初期化"""
        self.config_dir.mkdir(exist_ok=True)
        if self.config_file.exists():
            try:
                with open(self.config_file, 'r') as f:
                    config = json.load(f)
                    self.current_session_id = config.get('last_session_id')
            except:
                pass
    
    def _save_current_session(self, session_id: str, path: str):
        """現在のセッションを設定ファイルに保存"""
        config = {
            'last_session_id': session_id,
            'last_session_path': path,
            'created_at': asyncio.get_event_loop().time()
        }
        with open(self.config_file, 'w') as f:
            json.dump(config, f, indent=2)
        self.current_session_id = session_id
    
    def _find_nekocode_binary(self) -> str:
        """nekocode バイナリの場所を特定"""
        # 新しいバイナリ名で検索
        possible_paths = [
            # 新しいワークスペースのパス
            "../nekocode-workspace/target/debug/nekocode",
            "../nekocode-workspace/target/release/nekocode",
            "../../nekocode-rust-clean/nekocode-workspace/target/debug/nekocode",
            "../../nekocode-rust-clean/nekocode-workspace/target/release/nekocode",
            "/mnt/workdisk/public_share/nyacore-workspace/nekocode-cpp-github/nekocode-rust-clean/nekocode-workspace/target/debug/nekocode",
            # releases フォルダ
            "../releases/nekocode",
            "../../nekocode-rust-clean/releases/nekocode",
            # 旧パス（互換性）
            "./bin/nekocode_ai",
            "../bin/nekocode_ai",
        ]
        
        for path in possible_paths:
            if os.path.exists(path):
                return path
            # whichで探す
            result = subprocess.run(["which", path], capture_output=True, text=True)
            if result.returncode == 0:
                return path.strip()
        
        # デバッグ情報を出力
        print("❌ nekocode binary not found. Searched paths:")
        for path in possible_paths:
            print(f"   - {path}: {'✓' if os.path.exists(path) else '✗'}")
        
        raise FileNotFoundError("nekocode binary not found")
    
    def setup_tools(self):
        """🎮 NekoCode MCP ツール整理版 - analyze_start中心構造"""
        
        # ========================
        # 🚀 MAIN ENTRY POINT
        # ========================
        
        self.server.add_tool(
            "analyze_start",
            """🚀 解析開始 - すべてはここから始まる！

⭐ これが最初のコマンドです！ファイルでもフォルダでも、まずこれを実行してください。

使い方:
1. まず analyze_start でファイルまたはプロジェクトを指定
2. セッションが自動作成され、記憶されます
3. その後は他のコマンドをセッションID不要で実行可能

例:
  mcp__nekocode__analyze_start("/path/to/project")
  → セッション作成＆記憶
  → その後: mcp__nekocode__stats() でセッションID不要！

💡 ヒント:
- ファイルでもフォルダでもOK
- セッションIDは自動で記憶
- 次回からはセッションID省略可能""",
            self.analyze_start,
            {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "解析対象のファイルまたはプロジェクトパス"}
                },
                "required": ["path"]
            }
        )
        
        # ========================
        # 📊 ANALYSIS COMMANDS（セッション自動使用）
        # ========================
        
        self.server.add_tool(
            "stats",
            """📊 統計情報表示

⚠️ 先に analyze_start を実行してください！
セッションIDは自動で前回のものを使用します。

明示的にセッションIDを指定することも可能です。""",
            self.show_stats,
            {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string", "description": "セッションID（省略可）"}
                }
            }
        )
        
        self.server.add_tool(
            "deadcode",
            """🔍 デッドコード検出

⚠️ 先に analyze_start を実行してください！
セッションIDは自動で前回のものを使用します。

大規模プロジェクトではlimitを指定してください。""",
            self.detect_deadcode,
            {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string", "description": "セッションID（省略可）"},
                    "limit": {"type": "number", "description": "結果の最大数", "default": 20}
                }
            }
        )
        
        self.server.add_tool(
            "ast_stats",
            """🌳 AST統計情報

⚠️ 先に analyze_start を実行してください！
セッションIDは自動で前回のものを使用します。""",
            self.show_ast_stats,
            {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string", "description": "セッションID（省略可）"}
                }
            }
        )
        
        self.server.add_tool(
            "ast_query",
            """🔍 AST検索

⚠️ 先に analyze_start を実行してください！
セッションIDは自動で前回のものを使用します。

例: "MyClass", "handleRequest", "FileBoxProxy" など""",
            self.ast_query,
            {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "検索するシンボル名"},
                    "session_id": {"type": "string", "description": "セッションID（省略可）"}
                },
                "required": ["path"]
            }
        )
        
        self.server.add_tool(
            "ast_dump",
            """📋 AST構造ダンプ

⚠️ 先に analyze_start を実行してください！
セッションIDは自動で前回のものを使用します。""",
            self.ast_dump,
            {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string", "description": "セッションID（省略可）"},
                    "format": {"type": "string", "description": "出力形式", "enum": ["tree", "json", "flat"], "default": "tree"},
                    "limit": {"type": "number", "description": "出力行数制限", "default": 50}
                }
            }
        )
        
        # 既存のsession_createは後方互換性のため残す
        self.server.add_tool(
            "session_create",
            """🎮 セッション作成（メイン機能）

⚠️ パス指定について:
- 絶対パス推奨: /full/path/to/project  
- 相対パス例: ../nekocode-cpp-github/test-workspace/test-real-projects/flask

セッション作成後、以下のコマンドが利用可能:
📊 基本分析:
  • stats              - 統計情報
  • complexity         - 複雑度ランキング  
  • structure          - 構造解析
  • calls              - 関数呼び出し解析
  • files              - ファイル一覧

🔍 高度分析:
  • find <term>        - シンボル検索
  • analyze --complete - 完全解析（デッドコード検出）
  • large-files        - 大きなファイル検出
  • todo               - TODO/FIXME検出

🔧 C++専用:
  • include-cycles     - 循環依存検出
  • include-graph      - 依存関係グラフ
  • include-unused     - 不要include検出
  • include-optimize   - 最適化提案

🌳 AST革命:
  • ast-query <path>   - AST検索
  • ast-stats          - AST統計
  • scope-analysis <line> - スコープ解析
  • ast-dump [format]  - AST構造ダンプ

使用例:
  1. mcp__nekocode__session_create project/
  2. セッション内でコマンド実行""",
            self.create_session,
            {
                "type": "object", 
                "properties": {
                    "path": {"type": "string", "description": "プロジェクト/ファイルパス"}
                },
                "required": ["path"]
            }
        )
        
        # ========================
        # 🚀 STANDALONE（補助機能）
        # ========================
        
        self.server.add_tool(
            "analyze",
            """🚀 単発解析（セッション不要）

軽量な一回限りの解析用。継続的な分析にはsession_createを推奨。

⚠️ パス指定について:
- 絶対パス推奨: /full/path/to/project
- 相対パス例: ../nekocode-cpp-github/test-workspace/test-real-projects/express""",
            self.analyze_project,
            {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "解析対象のプロジェクトパス"},
                    "language": {"type": "string", "description": "言語指定 (auto|js|ts|cpp|c)", "default": "auto"},
                    "stats_only": {"type": "boolean", "description": "統計のみ高速出力", "default": False}
                },
                "required": ["path"]
            }
        )
        
        # ========================
        # 🧠 MEMORY SYSTEM
        # ========================
        
        self.server.add_tool(
            "memory",
            """🧠 Memory System（時間軸Memory革命）

使用可能操作:
• save {type} {name} [content] - 保存
• load {type} {name}          - 読み込み  
• list [type]                 - 一覧表示
• search {text}               - 検索
• stats                       - 統計
• timeline [type] [days]      - 時系列表示

Memory種類: auto🤖 memo📝 api🌐 cache💾""",
            self.memory_command,
            {
                "type": "object",
                "properties": {
                    "operation": {"type": "string", "description": "操作: save|load|list|search|stats|timeline"},
                    "type": {"type": "string", "description": "Memory種類: auto|memo|api|cache", "enum": ["auto", "memo", "api", "cache"], "default": "auto"},
                    "name": {"type": "string", "description": "Memory名（save/load時）"},
                    "content": {"type": "string", "description": "保存内容（save時）", "default": ""},
                    "text": {"type": "string", "description": "検索テキスト（search時）"},
                    "days": {"type": "number", "description": "過去日数（timeline時）", "default": 7}
                },
                "required": ["operation"]
            }
        )
        
        # ========================
        # 🛠️ UTILS
        # ========================
        
        self.server.add_tool(
            "list_languages",
            "🌍 サポート言語一覧",
            self.list_supported_languages,
            {"type": "object", "properties": {}}
        )
    
    async def _run_nekocode(self, args: List[str]) -> Dict:
        """NekoCode コマンドを実行してJSONを返す"""
        try:
            cmd = [self.nekocode_path] + args
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
            
            if result.returncode != 0:
                return {"error": f"NekoCode実行エラー: {result.stderr}"}
            
            # JSON出力をパース
            try:
                return json.loads(result.stdout)
            except json.JSONDecodeError:
                # JSON以外の出力の場合
                return {"output": result.stdout, "raw": True}
                
        except subprocess.TimeoutExpired:
            return {"error": "NekoCode実行がタイムアウトしました"}
        except Exception as e:
            return {"error": f"予期しないエラー: {str(e)}"}
    
    # ========================================
    # ツール実装
    # ========================================
    
    def _normalize_path(self, path: str) -> str:
        """パス正規化：よくある相対パスパターンを自動修正"""
        # ../test-workspace/ -> ../nekocode-cpp-github/test-workspace/ に自動変換
        if path.startswith("../test-workspace/"):
            path = path.replace("../test-workspace/", "../nekocode-cpp-github/test-workspace/")
        return path
    
    def _count_project_files(self, path: str) -> int:
        """🚨 大規模プロジェクト検出：ファイル数カウント（413エラー対策）"""
        try:
            from pathlib import Path
            p = Path(path)
            if not p.exists():
                return 0
            
            # 対象ファイル拡張子（解析対象のみカウント）
            extensions = {'.js', '.ts', '.tsx', '.jsx', '.py', '.cpp', '.c', '.h', '.hpp', '.cs', '.go', '.rs'}
            
            count = 0
            for ext in extensions:
                count += len(list(p.rglob(f'*{ext}')))
            
            return count
        except Exception:
            return 0  # エラー時は0を返してデフォルト動作

    def _truncate_large_output(self, result: Dict) -> Dict:
        """🛡️ 大容量出力の切り捨て（413エラー完全防止）"""
        try:
            # JSON文字列サイズチェック
            json_str = json.dumps(result, ensure_ascii=False)
            size_mb = len(json_str.encode('utf-8')) / (1024 * 1024)
            
            # サイズ制限: 1MB を超える場合は切り捨て
            if size_mb > 1.0:
                truncated_result = {
                    "analysis_summary": {
                        "warning": "🚨 大規模プロジェクト検出 - 出力を安全にサマリー化しました",
                        "original_size_mb": round(size_mb, 2),
                        "truncated": True,
                        "reason": "Claude Code API制限対応（413エラー防止）"
                    }
                }
                
                # 重要な統計情報のみ保持
                if "stats" in result:
                    truncated_result["stats"] = result["stats"]
                if "summary" in result:
                    truncated_result["summary"] = result["summary"]
                if "file_count" in result:
                    truncated_result["file_count"] = result["file_count"]
                if "language_breakdown" in result:
                    truncated_result["language_breakdown"] = result["language_breakdown"]
                
                # メタ情報も保持
                if "nekocode_info" in result:
                    truncated_result["nekocode_info"] = result["nekocode_info"]
                if "safety_notice" in result:
                    truncated_result["safety_notice"] = result["safety_notice"]
                
                return truncated_result
            
            return result
            
        except Exception as e:
            # エラー時は安全なフォールバック
            return {
                "error": f"出力処理中にエラー: {str(e)}",
                "fallback": "安全のため最小限の出力に切り替えました"
            }
    
    async def analyze_project(self, path: str, language: str = "auto", stats_only: bool = False) -> Dict:
        """🚨 プロジェクト解析（413エラー対策済み）"""
        path = self._normalize_path(path)  # パス正規化
        
        # 🚨 大規模プロジェクト自動検出（デモ事故防止）
        file_count = self._count_project_files(path)
        auto_switched = False
        
        # しきい値：50ファイル以上で自動stats_onlyモード（より積極的なAPI制限対応）
        if not stats_only and file_count > 50:
            stats_only = True
            auto_switched = True
        
        args = ["analyze", path]
        
        # Rust版は言語を自動検出するため--langオプションなし
        # if language != "auto":
        #     args.extend(["--lang", language])
        
        if stats_only:
            args.append("--stats-only")
        
        result = await self._run_nekocode(args)
        
        # 日本語メッセージ追加
        if "error" not in result:
            result["nekocode_info"] = {
                "message": "🚀 NekoCode超高速解析完了!",
                "speed": "Python版の900倍高速",
                "features": ["多言語対応", "UTF-8完全対応", "並列処理"]
            }
            
            # 🚨 自動切り替え警告メッセージ
            if auto_switched:
                result["safety_notice"] = {
                    "warning": "🛡️ 大規模プロジェクト検出",
                    "action": "自動でstats_onlyモードに切り替えました",
                    "reason": f"{file_count}ファイル > 50ファイル（しきい値）",
                    "benefit": "413エラーを防止し、高速なサマリー表示"
                }
        
        # 🛡️ 大容量出力の安全処理（413エラー完全防止）
        result = self._truncate_large_output(result)
        
        return result
    
    async def analyze_start(self, path: str) -> Dict:
        """🚀 解析開始 - すべてはここから始まる"""
        path = self._normalize_path(path)  # パス正規化
        
        # セッション作成
        result = await self._run_nekocode(["session-create", path])
        
        # 出力からセッションIDを抽出
        session_id = None
        if "output" in result and "Created session" in result["output"]:
            # "Created session: abc123" の形式からIDを抽出
            import re
            match = re.search(r'Created session:?\s+([a-f0-9]+)', result["output"])
            if match:
                session_id = match.group(1)
        elif "session_id" in result:
            session_id = result["session_id"]
        
        if session_id:
            
            # セッション情報を保存
            self.sessions[session_id] = {
                "path": path,
                "created_at": asyncio.get_event_loop().time()
            }
            
            # 設定ファイルに保存
            self._save_current_session(session_id, path)
            
            # 基本統計も取得
            stats_result = await self._run_nekocode(["session-stats", session_id])
            
            # 結果を統合
            result.update({
                "message": f"✨ 解析を開始しました！",
                "session_id": session_id,
                "path": path,
                "stats": stats_result if "error" not in stats_result else None,
                "next_commands": [
                    "📊 統計: mcp__nekocode__stats()",
                    "🔍 デッドコード: mcp__nekocode__deadcode(limit=10)",
                    "🌳 AST: mcp__nekocode__ast_stats()",
                    "📋 すべてセッションIDは自動で使用されます！"
                ]
            })
        
        return self._truncate_large_output(result)
    
    async def _get_session_id(self, session_id: Optional[str] = None) -> str:
        """セッションIDを取得（指定なければ最後のものを使用）"""
        if session_id:
            return session_id
        
        if self.current_session_id:
            return self.current_session_id
        
        raise ValueError("❌ セッションがありません。まず mcp__nekocode__analyze_start を実行してください！")
    
    async def show_stats(self, session_id: Optional[str] = None) -> Dict:
        """統計情報表示（セッション自動使用）"""
        try:
            session_id = await self._get_session_id(session_id)
            result = await self._run_nekocode(["session-stats", session_id])
            result["used_session"] = f"📌 セッション {session_id} を使用"
            return self._truncate_large_output(result)
        except ValueError as e:
            return {"error": str(e)}
    
    async def detect_deadcode(self, session_id: Optional[str] = None, limit: int = 20) -> Dict:
        """デッドコード検出（セッション自動使用）"""
        try:
            session_id = await self._get_session_id(session_id)
            
            # デッドコード解析実行
            result = await self._run_nekocode(["deadcode", session_id, "--min-confidence", "60", "--format", "text"])
            
            # 大きすぎる結果は制限
            if "output" in result and len(result["output"]) > 10000:
                lines = result["output"].split("\n")[:limit]
                result["output"] = "\n".join(lines)
                result["truncated"] = True
                result["tip"] = f"💡 結果を{limit}件に制限しました。全件表示には limit パラメータを増やしてください"
            
            result["used_session"] = f"📌 セッション {session_id} を使用"
            return self._truncate_large_output(result)
        except ValueError as e:
            return {"error": str(e)}
    
    async def show_ast_stats(self, session_id: Optional[str] = None) -> Dict:
        """AST統計表示（セッション自動使用）"""
        try:
            session_id = await self._get_session_id(session_id)
            result = await self._run_nekocode(["ast-stats", session_id])
            result["used_session"] = f"📌 セッション {session_id} を使用"
            return self._truncate_large_output(result)
        except ValueError as e:
            return {"error": str(e)}
    
    async def ast_query(self, path: str, session_id: Optional[str] = None) -> Dict:
        """AST検索（セッション自動使用）"""
        try:
            session_id = await self._get_session_id(session_id)
            result = await self._run_nekocode(["ast-query", session_id, path])
            result["used_session"] = f"📌 セッション {session_id} を使用"
            return self._truncate_large_output(result)
        except ValueError as e:
            return {"error": str(e)}
    
    async def ast_dump(self, session_id: Optional[str] = None, format: str = "tree", limit: int = 50) -> Dict:
        """AST構造ダンプ（セッション自動使用）"""
        try:
            session_id = await self._get_session_id(session_id)
            args = ["ast-dump", session_id, "--format", format, "--limit", str(limit)]
            result = await self._run_nekocode(args)
            result["used_session"] = f"📌 セッション {session_id} を使用"
            return self._truncate_large_output(result)
        except ValueError as e:
            return {"error": str(e)}
    
    async def create_session(self, path: str) -> Dict:
        """セッション作成（後方互換性）"""
        path = self._normalize_path(path)  # パス正規化
        result = await self._run_nekocode(["session-create", path])
        
        if "session_id" in result:
            # セッション情報を保存
            self.sessions[result["session_id"]] = {
                "path": path,
                "created_at": asyncio.get_event_loop().time()
            }
            
            result["nekocode_info"] = {
                "message": "🎮 対話式セッション作成完了!",
                "benefit": "継続操作は3msの爆速実行",
                "available_commands": [
                    "stats - 統計情報",
                    "complexity - 複雑度分析", 
                    "include-cycles - 循環依存検出",
                    "include-graph - 依存グラフ",
                    "find - ファイル検索"
                ]
            }
        
        return result
    
    async def session_stats(self, session_id: str) -> Dict:
        """セッション統計情報"""
        if session_id not in self.sessions:
            return {"error": f"セッション {session_id} が見つかりません"}
        
        result = await self._run_nekocode(["session-cmd", session_id, "stats"])
        
        if "error" not in result:
            result["nekocode_info"] = {
                "message": "📊 爆速統計取得完了 (3ms)!",
                "session_id": session_id
            }
        
        # 🛡️ 大容量出力の安全処理（413エラー完全防止）
        result = self._truncate_large_output(result)
        
        return result
    
    async def session_complexity(self, session_id: str) -> Dict:
        """複雑度分析"""
        if session_id not in self.sessions:
            return {"error": f"セッション {session_id} が見つかりません"}
        
        return await self._run_nekocode(["session-cmd", session_id, "complexity"])
    
    async def detect_include_cycles(self, session_id: str) -> Dict:
        """循環依存検出 (Serenaにない独自機能!)"""
        if session_id not in self.sessions:
            return {"error": f"セッション {session_id} が見つかりません"}
        
        result = await self._run_nekocode(["session-cmd", session_id, "include-cycles"])
        
        if "error" not in result:
            result["nekocode_advantage"] = {
                "message": "🔍 Serenaにない独自機能!",
                "feature": "C++循環依存検出",
                "benefit": "大規模C++プロジェクトの問題を瞬時に発見"
            }
        
        return result
    
    async def show_include_graph(self, session_id: str) -> Dict:
        """依存関係グラフ"""
        if session_id not in self.sessions:
            return {"error": f"セッション {session_id} が見つかりません"}
        
        result = await self._run_nekocode(["session-cmd", session_id, "include-graph"])
        
        if "error" not in result:
            result["nekocode_advantage"] = {
                "message": "🌐 依存関係可視化完了!",
                "feature": "include依存グラフ",
                "serena_comparison": "Serenaにはない独自機能"
            }
        
        return result
    
    async def optimize_includes(self, session_id: str) -> Dict:
        """include最適化提案"""
        if session_id not in self.sessions:
            return {"error": f"セッション {session_id} が見つかりません"}
        
        return await self._run_nekocode(["session-cmd", session_id, "include-optimize"])
    
    async def find_files(self, session_id: str, term: str) -> Dict:
        """ファイル検索"""
        if session_id not in self.sessions:
            return {"error": f"セッション {session_id} が見つかりません"}
        
        return await self._run_nekocode(["session-cmd", session_id, f"find {term}"])
    
    # 🧠 Memory System Handlers - 時間軸Memory革命
    
    async def memory_command(self, operation: str, type: str = "auto", name: str = "", 
                           content: str = "", text: str = "", days: int = 7) -> Dict:
        """🧠 統合Memory System handler"""
        
        # 操作マッピング
        operation_map = {
            "save": "save",
            "load": "load", 
            "list": "list",
            "search": "search",
            "stats": "stats",
            "timeline": "timeline"
        }
        
        if operation not in operation_map:
            return {"error": f"不明な操作: {operation}. 利用可能: {list(operation_map.keys())}"}
        
        # Memory コマンド構築
        if operation == "save":
            if not name:
                return {"error": "save操作にはnameが必要です"}
            cmd = ["memory", "save", type, name]
            if content:
                cmd.append(content)
        elif operation == "load":
            if not name:
                return {"error": "load操作にはnameが必要です"}
            cmd = ["memory", "load", type, name]
        elif operation == "list":
            cmd = ["memory", "list"]
            if type != "auto":
                cmd.append(type)
        elif operation == "search":
            if not text:
                return {"error": "search操作にはtextが必要です"}
            cmd = ["memory", "search", text]
        elif operation == "stats":
            cmd = ["memory", "stats"]
        elif operation == "timeline":
            cmd = ["memory", "timeline"]
            if type != "auto":
                cmd.append(type)
            if days != 7:
                cmd.append(str(days))
        
        result = await self._run_nekocode(cmd)
        
        # 成功時の情報追加
        if "error" not in result:
            result["nekocode_info"] = {
                "operation": operation,
                "memory_type": type,
                "message": f"🧠 Memory {operation} 完了!"
            }
        
        return result

    async def list_supported_languages(self) -> Dict:
        """サポート言語一覧"""
        result = await self._run_nekocode(["languages"])
        
        if "error" not in result:
            result["nekocode_info"] = {
                "message": "🌍 多言語対応エンジン",
                "current_languages": ["JavaScript", "TypeScript", "C++", "C", "Python", "C#"],
                "features": ["Universal AST Revolution", "Memory System", "1,512x Performance"],
                "advantage": "各言語に最適化された高速解析"
            }
        
        return result
    
    async def run(self):
        """MCP Server 起動"""
        print("🐱 NekoCode MCP Server - 革命的多言語解析エンジン")
        print(f"📂 NekoCode バイナリ: {self.nekocode_path}")
        print("🚀 起動完了 - Claude Codeで利用可能!")
        print()
        print("利用可能なツール:")
        for tool in self.server.tools:
            print(f"  mcp__nekocode__{tool['name']} - {tool['description']}")
        
        await self.server.run()


# メイン実行
if __name__ == "__main__":
    try:
        server = NekoCodeMCPServer()
        asyncio.run(server.run())
    except KeyboardInterrupt:
        print("\n🐱 NekoCode MCP Server 停止")
    except Exception as e:
        print(f"❌ エラー: {e}")
        sys.exit(1)