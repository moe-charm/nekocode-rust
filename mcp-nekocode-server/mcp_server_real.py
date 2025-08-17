#!/usr/bin/env python3
"""
🐱 NekoCode MCP Server - 実際のMCP実装版

実際のMCPプロトコル（stdio + JSON-RPC）で実装
"""

import asyncio
import json
import sys
import subprocess
import os
from typing import Dict, List, Any, Optional
import logging

# ログ設定 (stderrに出力、stdioと混同しないように)
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    stream=sys.stderr
)
logger = logging.getLogger(__name__)


class NekoCodeMCPServer:
    """実際のMCPプロトコル実装"""
    
    def __init__(self):
        self.nekocode_path = self._find_nekocode_binary()
        self.sessions = {}
        self.tools = self._define_tools()
        self.config = self._load_config()
    
    def _find_nekocode_binary(self) -> str:
        """nekocode-rust バイナリの場所を特定"""
        # 環境変数から取得を優先
        env_path = os.environ.get('NEKOCODE_BINARY_PATH')
        if env_path and os.path.exists(env_path):
            return os.path.abspath(env_path)
        
        possible_paths = [
            # 🦀 NEW: Rust版のバイナリパスを優先
            "./target/release/nekocode-rust",
            "../target/release/nekocode-rust",
            "./target/debug/nekocode-rust",
            "../target/debug/nekocode-rust",
            # 🚀 GitHub Actions / CI用 releases/ ディレクトリ
            "./releases/nekocode-rust",
            "../releases/nekocode-rust",
            # Legacy C++ paths  
            "./bin/nekocode_ai",
            "../bin/nekocode_ai",
            "./build/nekocode_ai",
            "../build/nekocode_ai", 
            "/usr/local/bin/nekocode_ai",
            "nekocode_ai",
            "nekocode-rust"
        ]
        
        for path in possible_paths:
            if os.path.exists(path):
                return os.path.abspath(path)
        
        # PATHから検索（Rust版を優先）
        import shutil
        rust_binary = shutil.which("nekocode-rust")
        if rust_binary:
            return rust_binary
        
        legacy_binary = shutil.which("nekocode_ai")
        if legacy_binary:
            return legacy_binary
        
        # デフォルト（Rust版を優先）
        return "./target/release/nekocode-rust"
    
    def _load_config(self) -> Dict:
        """nekocode_config.json を読み込み（あれば）"""
        try:
            # nekocode_ai と同じディレクトリの設定ファイルを探す
            config_path = os.path.join(
                os.path.dirname(self.nekocode_path),
                "nekocode_config.json"
            )
            
            if os.path.exists(config_path):
                with open(config_path, 'r', encoding='utf-8') as f:
                    config = json.load(f)
                    logger.info(f"📋 Config loaded from: {config_path}")
                    logger.info(f"   History limit: {config.get('memory', {}).get('edit_history', {}).get('max_size_mb', 10)} MB")
                    return config
            else:
                logger.info("📋 Using default config (no config file found)")
                return {
                    "memory": {
                        "edit_history": {"max_size_mb": 10, "min_files_keep": 10},
                        "edit_previews": {"max_size_mb": 5}
                    },
                    "token_limits": {
                        "ast_dump_max": 8000,
                        "summary_threshold": 1000,
                        "allow_force_output": True
                    }
                }
        except Exception as e:
            logger.warning(f"⚠️ Config load error: {e}, using defaults")
            return {
                "memory": {
                    "edit_history": {"max_size_mb": 10, "min_files_keep": 10},
                    "edit_previews": {"max_size_mb": 5}
                },
                "token_limits": {
                    "ast_dump_max": 8000,
                    "summary_threshold": 1000,
                    "allow_force_output": True
                }
            }
    
    def _define_tools(self) -> List[Dict]:
        """利用可能なツール定義"""
        return [
            {
                "name": "analyze",
                "description": "🚀 高速プロジェクト解析",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "解析対象パス"},
                        "language": {"type": "string", "description": "言語指定", "default": "auto"},
                        "stats_only": {"type": "boolean", "description": "統計のみ", "default": False}
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "session_create",
                "description": "🎮 対話式セッション作成",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "プロジェクトパス"}
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "session_stats",
                "description": "📊 セッション統計情報",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "セッションID"}
                    },
                    "required": ["session_id"]
                }
            },
            {
                "name": "session_update",
                "description": "⚡ インクリメンタル解析 (超高速更新)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "セッションID"},
                        "verbose": {"type": "boolean", "description": "詳細JSON出力", "default": False},
                        "dry_run": {"type": "boolean", "description": "変更プレビューのみ", "default": False}
                    },
                    "required": ["session_id"]
                }
            },
            {
                "name": "include_cycles",
                "description": "🔍 C++循環依存検出",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "セッションID"}
                    },
                    "required": ["session_id"]
                }
            },
            {
                "name": "include_graph",
                "description": "🌐 C++依存関係グラフ",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "セッションID"}
                    },
                    "required": ["session_id"]
                }
            },
            {
                "name": "list_languages",
                "description": "🌍 サポート言語一覧",
                "inputSchema": {"type": "object", "properties": {}}
            },
            {
                "name": "replace_preview",
                "description": "📝 置換プレビュー（セッション不要・直接実行）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {"type": "string", "description": "ファイルパス"},
                        "pattern": {"type": "string", "description": "検索パターン"},
                        "replacement": {"type": "string", "description": "置換文字列"}
                    },
                    "required": ["file_path", "pattern", "replacement"]
                }
            },
            {
                "name": "replace_confirm",
                "description": "✅ 置換実行（セッション不要・プレビューID指定）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "preview_id": {"type": "string", "description": "プレビューID"}
                    },
                    "required": ["preview_id"]
                }
            },
            {
                "name": "create_file",
                "description": "📄 新規ファイル作成（テンプレート対応）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {"type": "string", "description": "作成するファイルパス"},
                        "template": {"type": "string", "description": "テンプレート（python-cli/rust-lib/js-module等）"},
                        "force": {"type": "boolean", "description": "既存ファイルを上書き", "default": False}
                    },
                    "required": ["file_path"]
                }
            },
            {
                "name": "insert_preview",
                "description": "📝 挿入プレビュー（セマンティック位置対応）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {"type": "string", "description": "ファイルパス"},
                        "content": {"type": "string", "description": "挿入内容"},
                        "position": {"type": "string", "description": "挿入位置（start/end/行番号）"},
                        "after_function": {"type": "string", "description": "指定関数の後に挿入"},
                        "before_function": {"type": "string", "description": "指定関数の前に挿入"},
                        "in_imports": {"type": "boolean", "description": "インポートセクションに挿入"},
                        "after_class": {"type": "string", "description": "指定クラスの後に挿入"}
                    },
                    "required": ["file_path", "content"]
                }
            },
            {
                "name": "insert_confirm",
                "description": "✅ 挿入実行（セッション不要・プレビューID指定）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "preview_id": {"type": "string", "description": "プレビューID"}
                    },
                    "required": ["preview_id"]
                }
            },
            {
                "name": "movelines_preview",
                "description": "🔄 行移動プレビュー（セッション不要・ファイル間移動）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "srcfile": {"type": "string", "description": "ソースファイルパス"},
                        "start_line": {"type": "integer", "description": "開始行番号（1ベース）"},
                        "line_count": {"type": "integer", "description": "移動行数"},
                        "dstfile": {"type": "string", "description": "宛先ファイルパス"},
                        "insert_line": {"type": "integer", "description": "挿入行番号（1ベース）"}
                    },
                    "required": ["srcfile", "start_line", "line_count", "dstfile", "insert_line"]
                }
            },
            {
                "name": "movelines_confirm",
                "description": "✅ 行移動実行（セッション不要・プレビューID指定）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "preview_id": {"type": "string", "description": "プレビューID"}
                    },
                    "required": ["preview_id"]
                }
            },
            {
                "name": "edit_history",
                "description": "📋 編集履歴表示（セッション不要・最新20件）",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "edit_show",
                "description": "🔍 編集詳細表示（ID指定）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "セッションID"},
                        "edit_id": {"type": "string", "description": "編集ID"}
                    },
                    "required": ["session_id", "edit_id"]
                }
            },
            {
                "name": "ast_stats",
                "description": "🌳 AST統計情報（セッション）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "セッションID"}
                    },
                    "required": ["session_id"]
                }
            },
            {
                "name": "ast_query",
                "description": "🔍 AST構造クエリ（パス指定）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "セッションID"},
                        "path": {"type": "string", "description": "クエリパス（例: MyClass::myMethod）"}
                    },
                    "required": ["session_id", "path"]
                }
            },
            {
                "name": "scope_analysis",
                "description": "🎯 スコープ解析（行番号指定）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "セッションID"},
                        "line": {"type": "integer", "description": "解析対象行番号"}
                    },
                    "required": ["session_id", "line"]
                }
            },
            {
                "name": "ast_dump",
                "description": "📊 AST構造ダンプ（形式指定・トークン制限対応）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "セッションID"},
                        "format": {"type": "string", "description": "出力形式（tree/json/flat）", "default": "tree"},
                        "force": {"type": "boolean", "description": "強制全出力（トークン制限無視）", "default": False},
                        "limit": {"type": "integer", "description": "出力行数制限（省略可）"}
                    },
                    "required": ["session_id"]
                }
            },
            {
                "name": "moveclass_preview",
                "description": "🔄 クラス移動プレビュー（セッション）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "セッションID"},
                        "symbol_id": {"type": "string", "description": "移動対象シンボルID"},
                        "target": {"type": "string", "description": "移動先ファイルパス"}
                    },
                    "required": ["session_id", "symbol_id", "target"]
                }
            },
            {
                "name": "moveclass_confirm",
                "description": "✅ クラス移動実行（プレビューID指定）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "preview_id": {"type": "string", "description": "プレビューID"}
                    },
                    "required": ["preview_id"]
                }
            },
            {
                "name": "memory_save",
                "description": "💾 メモリ保存（タイプ・名前・内容指定）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "memory_type": {"type": "string", "description": "メモリタイプ（auto/memo/api/cache）"},
                        "name": {"type": "string", "description": "メモリ名"},
                        "content": {"type": "string", "description": "保存内容"}
                    },
                    "required": ["memory_type", "name", "content"]
                }
            },
            {
                "name": "memory_load",
                "description": "📂 メモリ読み込み（タイプ・名前指定）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "memory_type": {"type": "string", "description": "メモリタイプ"},
                        "name": {"type": "string", "description": "メモリ名"}
                    },
                    "required": ["memory_type", "name"]
                }
            },
            {
                "name": "memory_list",
                "description": "📋 メモリ一覧（タイプフィルタ可能）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "memory_type": {"type": "string", "description": "フィルタ用メモリタイプ（省略可）"}
                    }
                }
            },
            {
                "name": "memory_timeline",
                "description": "📅 メモリタイムライン（日数指定）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "memory_type": {"type": "string", "description": "フィルタ用メモリタイプ（省略可）"},
                        "days": {"type": "integer", "description": "表示日数", "default": 7}
                    }
                }
            },
            {
                "name": "config_show",
                "description": "⚙️ 設定表示",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "config_set",
                "description": "⚙️ 設定変更（キー・値指定）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "key": {"type": "string", "description": "設定キー"},
                        "value": {"type": "string", "description": "設定値"}
                    },
                    "required": ["key", "value"]
                }
            },
            {
                "name": "watch_start",
                "description": "🔍 ファイル監視開始（リアルタイム解析）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "監視対象セッションID"}
                    },
                    "required": ["session_id"]
                }
            },
            {
                "name": "watch_status",
                "description": "📊 ファイル監視状態確認",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "セッションID（省略時は全セッション）"}
                    }
                }
            },
            {
                "name": "watch_stop",
                "description": "🛑 ファイル監視停止",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "停止対象セッションID"}
                    },
                    "required": ["session_id"]
                }
            },
            {
                "name": "watch_stop_all",
                "description": "🛑 全ファイル監視停止",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "watch_config",
                "description": "⚙️ ファイル監視設定表示",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    
    def _extract_summary(self, result: Dict) -> str:
        """解析結果から統計サマリーを抽出"""
        try:
            if "error" in result:
                return json.dumps(result, indent=2, ensure_ascii=False)
            
            summary = []
            summary.append("📊 **解析結果サマリー**\n")
            
            # 基本情報
            if "directory_path" in result:
                summary.append(f"📁 パス: {result['directory_path']}")
            
            # ファイル統計
            if "files" in result:
                files = result["files"]
                total_files = len(files)
                summary.append(f"📄 総ファイル数: {total_files}")
                
                # 言語別統計
                lang_counts = {}
                total_functions = 0
                total_classes = 0
                total_lines = 0
                total_code_lines = 0
                
                for file in files:
                    lang = file.get("language", "unknown")
                    lang_counts[lang] = lang_counts.get(lang, 0) + 1
                    
                    if "functions" in file:
                        total_functions += len(file["functions"])
                    if "classes" in file:
                        total_classes += len(file["classes"])
                    if "file_info" in file:
                        info = file["file_info"]
                        total_lines += info.get("total_lines", 0)
                        total_code_lines += info.get("code_lines", 0)
                
                summary.append(f"\n📈 **統計情報:**")
                summary.append(f"  • 総行数: {total_lines:,}")
                summary.append(f"  • コード行数: {total_code_lines:,}")
                summary.append(f"  • 関数数: {total_functions:,}")
                summary.append(f"  • クラス数: {total_classes:,}")
                
                if lang_counts:
                    summary.append(f"\n🗂️ **言語別:**")
                    for lang, count in sorted(lang_counts.items()):
                        summary.append(f"  • {lang}: {count} files")
            
            # 実行時間情報（もしあれば）
            if "output" in result and "Total directory analysis took:" in result.get("output", ""):
                # outputから実行時間を抽出
                output_lines = result["output"].split("\n")
                for line in output_lines:
                    if "Total directory analysis took:" in line:
                        summary.append(f"\n⏱️ {line.strip()}")
                        break
            
            return "\n".join(summary)
            
        except Exception as e:
            logger.error(f"サマリー抽出エラー: {e}")
            # エラー時は少なくとも基本情報を返す
            return f"⚠️ サマリー生成エラー: {str(e)}\n\n元データのキー: {list(result.keys())}"
    
    async def _run_nekocode(self, args: List[str]) -> Dict:
        """NekoCode実行"""
        try:
            cmd = [self.nekocode_path] + args
            logger.info(f"Executing: {' '.join(cmd)}")
            
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
            
            # --helpなどは0以外のreturn codeでも正常
            if result.returncode != 0 and "--help" not in args:
                return {"error": f"NekoCode実行エラー: {result.stderr}"}
            
            # stderrに出力される場合もある（helpなど）
            output = result.stdout if result.stdout.strip() else result.stderr
            
            # Rust版はプログレス情報をJSON前に出力するので、JSON部分だけを抽出
            lines = output.split('\n')
            json_start = -1
            for i, line in enumerate(lines):
                if line.strip().startswith('{'):
                    json_start = i
                    break
            
            if json_start >= 0:
                # JSON部分だけを取り出して解析
                json_text = '\n'.join(lines[json_start:])
                try:
                    return json.loads(json_text)
                except json.JSONDecodeError:
                    # JSONパースに失敗した場合は元の出力を返す
                    return {"output": output, "raw": True}
            else:
                # JSON開始が見つからない場合は、全体をJSONとして解析を試みる
                try:
                    return json.loads(output)
                except json.JSONDecodeError:
                    return {"output": output, "raw": True}
                
        except subprocess.TimeoutExpired:
            return {"error": "実行がタイムアウトしました"}
        except FileNotFoundError:
            return {"error": f"NekoCodeバイナリが見つかりません: {self.nekocode_path}"}
        except Exception as e:
            return {"error": f"予期しないエラー: {str(e)}"}
    
    # ========================================
    # MCPプロトコル実装
    # ========================================
    
    async def handle_initialize(self, params: Dict) -> Dict:
        """初期化ハンドラ"""
        logger.info("MCP Server initializing...")
        return {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {"listChanged": False},
                "resources": {"subscribe": False, "listChanged": False}
            },
            "serverInfo": {
                "name": "nekocode",
                "version": "1.0.0"
            }
        }
    
    async def handle_tools_list(self, params: Dict) -> Dict:
        """ツール一覧ハンドラ"""
        return {"tools": self.tools}
    
    async def handle_resources_list(self, params: Dict) -> Dict:
        """リソース一覧ハンドラ"""
        readme_path = os.path.join(os.path.dirname(__file__), "README.md")
        
        resources = []
        if os.path.exists(readme_path):
            resources.append({
                "uri": "nekocode://readme",
                "name": "NekoCode MCP Server README",
                "description": "🐱 NekoCodeの使い方ガイド - セッション機能を活用した高速解析",
                "mimeType": "text/markdown"
            })
        
        return {"resources": resources}
    
    async def handle_resources_read(self, params: Dict) -> Dict:
        """リソース読み取りハンドラ"""
        uri = params.get("uri", "")
        
        if uri == "nekocode://readme":
            readme_path = os.path.join(os.path.dirname(__file__), "README.md")
            if os.path.exists(readme_path):
                with open(readme_path, "r", encoding="utf-8") as f:
                    contents = f.read()
                
                return {
                    "contents": [{
                        "uri": uri,
                        "mimeType": "text/markdown",
                        "text": contents
                    }]
                }
        
        return {"error": f"Resource not found: {uri}"}
    
    async def handle_tools_call(self, params: Dict) -> Dict:
        """ツール実行ハンドラ"""
        tool_name = params.get("name")
        arguments = params.get("arguments", {})
        
        logger.info(f"Tool call: {tool_name} with args: {arguments}")
        
        try:
            if tool_name == "analyze":
                return await self._tool_analyze(arguments)
            elif tool_name == "session_create":
                return await self._tool_session_create(arguments)
            elif tool_name == "session_stats":
                return await self._tool_session_stats(arguments)
            elif tool_name == "session_update":
                return await self._tool_session_update(arguments)
            elif tool_name == "include_cycles":
                return await self._tool_include_cycles(arguments)
            elif tool_name == "include_graph":
                return await self._tool_include_graph(arguments)
            elif tool_name == "list_languages":
                return await self._tool_list_languages(arguments)
            elif tool_name == "replace_preview":
                return await self._tool_replace_preview(arguments)
            elif tool_name == "replace_confirm":
                return await self._tool_replace_confirm(arguments)
            elif tool_name == "create_file":
                return await self._tool_create_file(arguments)
            elif tool_name == "insert_preview":
                return await self._tool_insert_preview(arguments)
            elif tool_name == "insert_confirm":
                return await self._tool_insert_confirm(arguments)
            elif tool_name == "movelines_preview":
                return await self._tool_movelines_preview(arguments)
            elif tool_name == "movelines_confirm":
                return await self._tool_movelines_confirm(arguments)
            elif tool_name == "edit_history":
                return await self._tool_edit_history(arguments)
            elif tool_name == "edit_show":
                return await self._tool_edit_show(arguments)
            elif tool_name == "ast_stats":
                return await self._tool_ast_stats(arguments)
            elif tool_name == "ast_query":
                return await self._tool_ast_query(arguments)
            elif tool_name == "scope_analysis":
                return await self._tool_scope_analysis(arguments)
            elif tool_name == "ast_dump":
                return await self._tool_ast_dump(arguments)
            elif tool_name == "moveclass_preview":
                return await self._tool_moveclass_preview(arguments)
            elif tool_name == "moveclass_confirm":
                return await self._tool_moveclass_confirm(arguments)
            elif tool_name == "memory_save":
                return await self._tool_memory_save(arguments)
            elif tool_name == "memory_load":
                return await self._tool_memory_load(arguments)
            elif tool_name == "memory_list":
                return await self._tool_memory_list(arguments)
            elif tool_name == "memory_timeline":
                return await self._tool_memory_timeline(arguments)
            elif tool_name == "config_show":
                return await self._tool_config_show(arguments)
            elif tool_name == "config_set":
                return await self._tool_config_set(arguments)
            elif tool_name == "watch_start":
                return await self._tool_watch_start(arguments)
            elif tool_name == "watch_status":
                return await self._tool_watch_status(arguments)
            elif tool_name == "watch_stop":
                return await self._tool_watch_stop(arguments)
            elif tool_name == "watch_stop_all":
                return await self._tool_watch_stop_all(arguments)
            elif tool_name == "watch_config":
                return await self._tool_watch_config(arguments)
            else:
                return {
                    "content": [{"type": "text", "text": f"Unknown tool: {tool_name}"}],
                    "isError": True
                }
        except Exception as e:
            logger.error(f"Tool execution error: {e}")
            return {
                "content": [{"type": "text", "text": f"エラー: {str(e)}"}],
                "isError": True
            }
    
    # ========================================
    # ツール実装
    # ========================================
    
    async def _tool_analyze(self, args: Dict) -> Dict:
        """プロジェクト解析"""
        path = args["path"]
        language = args.get("language", "auto")
        stats_only = args.get("stats_only", False)
        
        cmd_args = ["analyze", path]
        if language != "auto":
            cmd_args.extend(["--lang", language])
        
        # 🚀 NEW: Rust版に--stats-onlyオプションを追加済み！
        if stats_only:
            cmd_args.append("--stats-only")
        
        # Rust版では--io-threads → --threads に変更
        cmd_args.extend(["--threads", "8"])
        
        result = await self._run_nekocode(cmd_args)
        
        # stats_onlyの場合はRust側で既に統計サマリー形式で出力される（plaintext）
        if stats_only:
            if isinstance(result, dict) and "output" in result:
                # plain textが{"output": "..."} 形式で返される場合
                return {
                    "content": [{"type": "text", "text": result["output"]}]
                }
            elif isinstance(result, dict) and "raw" in result:
                # raw出力の場合
                return {
                    "content": [{"type": "text", "text": result["output"]}]
                }
            else:
                # その他の場合はそのまま返す
                return {
                    "content": [{"type": "text", "text": str(result)}]
                }
        
        # 通常のJSONモードの場合
        return {
            "content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_session_create(self, args: Dict) -> Dict:
        """セッション作成"""
        path = args["path"]
        result = await self._run_nekocode(["session-create", path])
        
        # セッションID抽出（JSON形式またはRust版のテキスト出力）
        session_id = None
        if "session_id" in result:
            session_id = result["session_id"]
        elif "output" in result and isinstance(result["output"], str):
            # Rust版の出力: "Session created: XXXXX"
            import re
            match = re.search(r"Session created: ([a-f0-9]+)", result["output"])
            if match:
                session_id = match.group(1)
        
        if session_id:
            self.sessions[session_id] = {"path": path}
            logger.info(f"✅ セッション登録完了: {session_id} -> {path}")
        else:
            logger.warning(f"⚠️ セッションID抽出失敗: {result}")
        
        return {
            "content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_session_stats(self, args: Dict) -> Dict:
        """セッション統計"""
        session_id = args["session_id"]
        
        if session_id not in self.sessions:
            return {
                "content": [{"type": "text", "text": f"セッション {session_id} が見つかりません"}],
                "isError": True
            }
        
        result = await self._run_nekocode(["session-command", session_id, "stats"])
        
        return {
            "content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_session_update(self, args: Dict) -> Dict:
        """⚡ インクリメンタル解析 (超高速更新)"""
        session_id = args["session_id"]
        verbose = args.get("verbose", False)
        dry_run = args.get("dry_run", False)
        
        if session_id not in self.sessions:
            return {
                "content": [{"type": "text", "text": f"セッション {session_id} が見つかりません"}],
                "isError": True
            }
        
        # コマンド引数構築
        cmd_args = ["session-update", session_id]
        if verbose:
            cmd_args.append("--verbose")
        if dry_run:
            cmd_args.append("--dry-run")
        
        result = await self._run_nekocode(cmd_args)
        
        # 結果の解析・フォーマット
        if verbose and isinstance(result, dict) and not result.get("error"):
            # verbose modeの場合、JSONレスポンスが期待される
            content_text = json.dumps(result, indent=2, ensure_ascii=False)
        elif dry_run and isinstance(result, dict) and "output" in result:
            # dry-runの場合、プレーンテキスト出力
            content_text = result["output"]
        elif isinstance(result, dict) and "output" in result:
            # 標準モードの場合
            content_text = result["output"]
        else:
            # その他の場合はJSONとしてフォーマット
            content_text = json.dumps(result, indent=2, ensure_ascii=False)
        
        # 性能情報の抽出・追加表示
        if isinstance(result, dict) and not dry_run and not result.get("error"):
            # 性能数値を抽出してハイライト表示
            lines = content_text.split('\n')
            speedup_info = []
            
            for line in lines:
                if 'speedup' in line.lower() or 'faster' in line.lower():
                    speedup_info.append(line)
            
            if speedup_info:
                content_text += "\n\n🚀 **性能ハイライト:**\n" + "\n".join(f"  • {line}" for line in speedup_info)
        
        return {
            "content": [{"type": "text", "text": content_text}]
        }
    
    async def _tool_include_cycles(self, args: Dict) -> Dict:
        """循環依存検出"""
        session_id = args["session_id"]
        
        if session_id not in self.sessions:
            return {
                "content": [{"type": "text", "text": f"セッション {session_id} が見つかりません"}],
                "isError": True
            }
        
        result = await self._run_nekocode(["session-command", session_id, "include-cycles"])
        
        return {
            "content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_include_graph(self, args: Dict) -> Dict:
        """依存関係グラフ"""
        session_id = args["session_id"]
        
        if session_id not in self.sessions:
            return {
                "content": [{"type": "text", "text": f"セッション {session_id} が見つかりません"}],
                "isError": True
            }
        
        result = await self._run_nekocode(["session-command", session_id, "include-graph"])
        
        return {
            "content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_list_languages(self, args: Dict) -> Dict:
        """言語一覧"""
        # 最新版ではhelpから言語情報を取得
        result = await self._run_nekocode(["--help"])
        
        if "output" in result:
            # LANGUAGES行を抽出
            lines = result["output"].split('\n')
            lang_line = next((line for line in lines if 'LANGUAGES:' in line), "")
            languages = lang_line.replace('LANGUAGES:', '').strip() if lang_line else "JS/TS/C++/C/Python/C#"
            return {"content": [{"type": "text", "text": f"対応言語: {languages}"}]}
        else:
            return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_replace_preview(self, args: Dict) -> Dict:
        """置換プレビュー（セッション不要）"""
        file_path = args["file_path"]
        pattern = args["pattern"]
        replacement = args["replacement"]
        
        # 新コマンド構造: replace --preview
        result = await self._run_nekocode(["replace", file_path, pattern, replacement, "--preview"])
        
        return {
            "content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_replace_confirm(self, args: Dict) -> Dict:
        """置換実行（直接実行）"""
        # 新設計: replace_confirmは使わず、replaceを直接実行
        file_path = args.get("file_path")
        pattern = args.get("pattern") 
        replacement = args.get("replacement")
        
        if not all([file_path, pattern, replacement]):
            # 後方互換性のため preview_id も受け付けるが、実際は直接実行を推奨
            return {
                "content": [{"type": "text", "text": "新設計では replace コマンドを直接実行してください（preview_id は廃止）"}],
                "isError": True
            }
        
        # 直接実行（デフォルト）
        result = await self._run_nekocode(["replace", file_path, pattern, replacement])
        
        return {
            "content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_create_file(self, args: Dict) -> Dict:
        """新規ファイル作成"""
        file_path = args["file_path"]
        cmd_args = ["create-file", file_path]
        
        if "template" in args:
            cmd_args.extend(["--template", args["template"]])
        if args.get("force", False):
            cmd_args.append("--force")
            
        result = await self._run_nekocode(cmd_args)
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_insert_preview(self, args: Dict) -> Dict:
        """挿入プレビュー（セマンティック位置対応）"""
        file_path = args["file_path"]
        content = args["content"]
        
        # Build command with semantic options
        cmd_args = ["insert", file_path, content]
        
        if "after_function" in args:
            cmd_args.extend(["--after-function", args["after_function"]])
        elif "before_function" in args:
            cmd_args.extend(["--before-function", args["before_function"]])
        elif args.get("in_imports", False):
            cmd_args.append("--in-imports")
        elif "after_class" in args:
            cmd_args.extend(["--after-class", args["after_class"]])
        elif "position" in args:
            # Only add position if no semantic option is used
            cmd_args.append(args["position"])
        
        # プレビューモード追加
        cmd_args.append("--preview")
        
        result = await self._run_nekocode(cmd_args)
        
        return {
            "content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_insert_confirm(self, args: Dict) -> Dict:
        """挿入実行（直接実行）"""
        # 新設計: insert_confirmは使わず、insertを直接実行
        file_path = args.get("file_path")
        content = args.get("content")
        
        if not all([file_path, content]):
            # 後方互換性のため preview_id も受け付けるが、実際は直接実行を推奨
            return {
                "content": [{"type": "text", "text": "新設計では insert コマンドを直接実行してください（preview_id は廃止）"}],
                "isError": True
            }
        
        # Build command with semantic options
        cmd_args = ["insert", file_path, content]
        
        if "after_function" in args:
            cmd_args.extend(["--after-function", args["after_function"]])
        elif "before_function" in args:
            cmd_args.extend(["--before-function", args["before_function"]])
        elif args.get("in_imports", False):
            cmd_args.append("--in-imports")
        elif "after_class" in args:
            cmd_args.extend(["--after-class", args["after_class"]])
        elif "position" in args:
            cmd_args.append(args["position"])
        
        # 直接実行（プレビューなし）
        result = await self._run_nekocode(cmd_args)
        
        return {
            "content": [{"type": "text", "text": json.dumps(result.get("output", result), indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_movelines_preview(self, args: Dict) -> Dict:
        """行移動プレビュー（セッション不要）"""
        srcfile = args["srcfile"]
        start_line = str(args["start_line"])
        line_count = str(args["line_count"])
        dstfile = args["dstfile"]
        insert_line = str(args["insert_line"])
        
        # 新コマンド構造: move-lines --preview
        result = await self._run_nekocode([
            "move-lines", srcfile, start_line, line_count, dstfile, insert_line, "--preview"
        ])
        
        return {
            "content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_movelines_confirm(self, args: Dict) -> Dict:
        """行移動実行（直接実行）"""
        # 新設計: movelines_confirmは使わず、move-linesを直接実行
        srcfile = args.get("srcfile")
        start_line = args.get("start_line")
        line_count = args.get("line_count")
        dstfile = args.get("dstfile")
        insert_line = args.get("insert_line")
        
        if not all([srcfile, start_line, line_count, dstfile, insert_line]):
            return {
                "content": [{"type": "text", "text": "新設計では move-lines コマンドを直接実行してください（preview_id は廃止）"}],
                "isError": True
            }
        
        # 直接実行（デフォルト）
        result = await self._run_nekocode([
            "move-lines", srcfile, str(start_line), str(line_count), dstfile, str(insert_line)
        ])
        
        return {
            "content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_edit_history(self, args: Dict) -> Dict:
        """編集履歴表示（セッション不要）"""
        # セッション不要でedit-historyディレクトリから直接読み込み
        try:
            import os
            import glob
            
            history_dir = "memory/edit_history"
            if not os.path.exists(history_dir):
                return {
                    "content": [{"type": "text", "text": json.dumps({"history": [], "total_count": 0, "summary": "編集履歴なし"}, indent=2, ensure_ascii=False)}]
                }
            
            # JSONファイルを取得して最新順でソート
            history_files = glob.glob(f"{history_dir}/*.json")
            history_files.sort(key=os.path.getmtime, reverse=True)
            
            history_list = []
            for file_path in history_files[:20]:  # 最新20件
                try:
                    with open(file_path, 'r', encoding='utf-8') as f:
                        history_data = json.load(f)
                        history_list.append(history_data)
                except Exception as e:
                    logger.warning(f"Failed to load history file {file_path}: {e}")
            
            result = {
                "command": "edit-history",
                "total_count": len(history_files),
                "history": history_list,
                "summary": "最新20件の編集履歴"
            }
            
        except Exception as e:
            logger.error(f"Edit history error: {e}")
            result = {"error": f"編集履歴の取得に失敗: {str(e)}"}
        
        return {
            "content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]
        }
    
    async def _tool_edit_show(self, args: Dict) -> Dict:
        """編集詳細表示"""
        session_id = args["session_id"]
        edit_id = args["edit_id"]
        
        # セッション存在チェック
        if session_id not in self.sessions:
            return {
                "content": [{"type": "text", "text": f"Session not found: {session_id}"}],
                "isError": True
            }
        
        # コマンド実行（引数を個別に渡す）
        result = await self._run_nekocode(["session-command", session_id, "edit-show", edit_id])
        
        return {
            "content": [{"type": "text", "text": json.dumps(result.get("output", result), indent=2, ensure_ascii=False)}]
        }
    
    # ========================================
    # MCPプロトコル通信
    # ========================================
    
    async def send_message(self, message: Dict):
        """メッセージ送信 (stdout)"""
        json.dump(message, sys.stdout, ensure_ascii=False)
        sys.stdout.write('\n')
        sys.stdout.flush()
    
    async def receive_message(self) -> Optional[Dict]:
        """メッセージ受信 (stdin)"""
        try:
            line = sys.stdin.readline()
            if not line:
                return None
            return json.loads(line.strip())
        except json.JSONDecodeError as e:
            logger.error(f"JSON decode error: {e}")
            return None
        except Exception as e:
            logger.error(f"Message receive error: {e}")
            return None
    
    # ========================================
    # 🌳 AST関連ツール
    # ========================================
    
    async def _tool_ast_stats(self, args: Dict) -> Dict:
        """AST統計情報を取得"""
        session_id = args["session_id"]
        result = await self._run_nekocode(["ast-stats", session_id])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_ast_query(self, args: Dict) -> Dict:
        """AST構造をクエリ"""
        session_id = args["session_id"]
        path = args["path"]
        result = await self._run_nekocode(["ast-query", session_id, path])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_scope_analysis(self, args: Dict) -> Dict:
        """スコープ解析を実行"""
        session_id = args["session_id"]
        line = str(args["line"])
        result = await self._run_nekocode(["scope-analysis", session_id, line])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_ast_dump(self, args: Dict) -> Dict:
        """AST構造をダンプ（設定ファイル対応・トークン制限）"""
        session_id = args["session_id"]
        format_type = args.get("format", "tree")
        force = args.get("force", False)
        line_limit = args.get("limit")
        
        # 📋 設定ファイルからトークン制限を取得
        token_config = self.config.get("token_limits", {})
        token_limit = token_config.get("ast_dump_max", 8000)
        summary_threshold = token_config.get("summary_threshold", 1000)
        allow_force = token_config.get("allow_force_output", True)
        
        # 🚨 まずast-statsでサイズ確認
        stats_result = await self._run_nekocode(["ast-stats", session_id])
        
        result = await self._run_nekocode(["ast-dump", session_id, format_type])
        
        # 🔥 トークン制限チェック
        if isinstance(result, dict):
            output_text = json.dumps(result, indent=2, ensure_ascii=False)
        else:
            output_text = str(result)
        
        # 行数制限適用（指定された場合）
        if line_limit and not force:
            lines = output_text.split('\n')
            if len(lines) > line_limit:
                output_text = '\n'.join(lines[:line_limit])
                output_text += f"\n\n... ({len(lines) - line_limit} 行省略) ..."
        
        # トークン数推定（文字数 / 4 = 近似トークン数）
        estimated_tokens = len(output_text) // 4
        
        # force=True または 設定で強制許可されていない場合は制限チェック
        if not force and estimated_tokens > token_limit:
            # 🚨 トークン制限超過 - 警告と代替案提示
            force_cmd = f'mcp__nekocode__ast_dump(session_id="{session_id}", format="{format_type}", force=True)'
            
            warning_msg = f"""🚨 **AST Dump トークン制限超過** (設定: {token_limit:,} tokens)

📊 **出力サイズ分析:**
• 推定トークン数: **{estimated_tokens:,} tokens**
• 設定制限: {token_limit:,} tokens  
• 超過率: **{(estimated_tokens/token_limit):.1f}x**

⚠️ **解析アプリとして使いやすさ重視で8000に設定済み**

🚀 **選択肢:**
1. **ast_stats**: 統計サマリーのみ（推奨）
2. **強制出力**: `{force_cmd}`
3. **行数制限**: limit=50 等で部分表示
4. **設定変更**: nekocode_config.json で制限値調整

📋 **AST統計サマリー:**
{json.dumps(stats_result, indent=2, ensure_ascii=False) if isinstance(stats_result, dict) else str(stats_result)}

---
💡 **設定ファイル例** (nekocode_config.json):
```json
{{
    "token_limits": {{
        "ast_dump_max": 15000,
        "summary_threshold": 2000
    }}
}}
```"""
            
            return {"content": [{"type": "text", "text": warning_msg}]}
        
        # 強制出力または制限内の場合
        if force and not allow_force:
            return {"content": [{"type": "text", "text": "❌ 強制出力は設定で無効化されています"}]}
        
        # サイズ情報を付加（summary_threshold超過時）
        if estimated_tokens > summary_threshold:
            size_info = f"📊 出力サイズ: {estimated_tokens:,} tokens\n\n"
            output_text = size_info + output_text
        
        return {"content": [{"type": "text", "text": output_text}]}
    
    # ========================================
    # 🔄 クラス移動ツール
    # ========================================
    
    async def _tool_moveclass_preview(self, args: Dict) -> Dict:
        """クラス移動のプレビューを生成"""
        session_id = args["session_id"]
        symbol_id = args["symbol_id"]
        target = args["target"]
        # 新コマンド構造: move-class --preview
        result = await self._run_nekocode(["move-class", session_id, symbol_id, target, "--preview"])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_moveclass_confirm(self, args: Dict) -> Dict:
        """クラス移動を実行"""
        # 新設計: moveclass_confirmは使わず、move-classを直接実行
        session_id = args.get("session_id")
        symbol_id = args.get("symbol_id")
        target = args.get("target")
        
        if not all([session_id, symbol_id, target]):
            return {
                "content": [{"type": "text", "text": "新設計では move-class コマンドを直接実行してください（preview_id は廃止）"}],
                "isError": True
            }
        
        # 直接実行（デフォルト）
        result = await self._run_nekocode(["move-class", session_id, symbol_id, target])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    # ========================================
    # 💾 メモリシステムツール
    # ========================================
    
    async def _tool_memory_save(self, args: Dict) -> Dict:
        """メモリに内容を保存"""
        memory_type = args["memory_type"]
        name = args["name"]
        content = args["content"]
        result = await self._run_nekocode(["memory", "save", memory_type, name, content])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_memory_load(self, args: Dict) -> Dict:
        """メモリから内容を読み込み"""
        memory_type = args["memory_type"]
        name = args["name"]
        result = await self._run_nekocode(["memory", "load", memory_type, name])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_memory_list(self, args: Dict) -> Dict:
        """メモリ一覧を取得"""
        cmd_args = ["memory", "list"]
        if "memory_type" in args:
            cmd_args.append(args["memory_type"])
        result = await self._run_nekocode(cmd_args)
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_memory_timeline(self, args: Dict) -> Dict:
        """メモリタイムラインを取得"""
        cmd_args = ["memory", "timeline"]
        if "memory_type" in args:
            cmd_args.append(args["memory_type"])
        if "days" in args:
            cmd_args.extend(["--days", str(args["days"])])
        result = await self._run_nekocode(cmd_args)
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    # ========================================
    # ⚙️ 設定ツール
    # ========================================
    
    async def _tool_config_show(self, args: Dict) -> Dict:
        """現在の設定を表示"""
        result = await self._run_nekocode(["config", "show"])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_config_set(self, args: Dict) -> Dict:
        """設定を変更"""
        key = args["key"]
        value = args["value"]
        result = await self._run_nekocode(["config", "set", key, value])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    # ========================================
    # 🔍 ファイル監視ツール
    # ========================================
    
    async def _tool_watch_start(self, args: Dict) -> Dict:
        """ファイル監視開始"""
        session_id = args["session_id"]
        result = await self._run_nekocode(["watch-start", session_id])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_watch_status(self, args: Dict) -> Dict:
        """ファイル監視状態確認"""
        cmd_args = ["watch-status"]
        if "session_id" in args and args["session_id"]:
            cmd_args.append(args["session_id"])
        result = await self._run_nekocode(cmd_args)
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_watch_stop(self, args: Dict) -> Dict:
        """ファイル監視停止"""
        session_id = args["session_id"]
        result = await self._run_nekocode(["watch-stop", session_id])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_watch_stop_all(self, args: Dict) -> Dict:
        """全ファイル監視停止"""
        result = await self._run_nekocode(["watch-stop-all"])
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2, ensure_ascii=False)}]}
    
    async def _tool_watch_config(self, args: Dict) -> Dict:
        """ファイル監視設定表示"""
        # 設定ファイルの file_watching セクションを直接表示
        try:
            watch_config = self.config.get("file_watching", {})
            config_display = {
                "file_watching": watch_config,
                "説明": {
                    "debounce_ms": "ファイル変更検知の遅延時間（ミリ秒）",
                    "max_events_per_second": "1秒間の最大イベント処理数",
                    "exclude_patterns": "監視除外パターン",
                    "include_extensions": "監視対象ファイル拡張子",
                    "include_important_files": "拡張子に関係なく監視する重要ファイル"
                }
            }
            return {"content": [{"type": "text", "text": json.dumps(config_display, indent=2, ensure_ascii=False)}]}
        except Exception as e:
            return {"content": [{"type": "text", "text": f"設定表示エラー: {str(e)}"}], "isError": True}
    
    async def run(self):
        """MCPサーバー実行"""
        logger.info("🐱 NekoCode MCP Server starting...")
        logger.info(f"📂 NekoCode binary: {self.nekocode_path}")
        logger.info(f"🔧 Config: History {self.config['memory']['edit_history']['max_size_mb']}MB, "
                   f"Preview {self.config['memory']['edit_previews']['max_size_mb']}MB")
        token_config = self.config.get("token_limits", {})
        logger.info(f"🎯 Token Limits: AST Dump {token_config.get('ast_dump_max', 8000)}, "
                   f"Summary {token_config.get('summary_threshold', 1000)}")
        
        while True:
            try:
                # メッセージ受信
                message = await self.receive_message()
                if message is None:
                    break
                
                method = message.get("method")
                params = message.get("params", {})
                message_id = message.get("id")
                
                logger.info(f"Received: {method}")
                
                # ハンドラ呼び出し
                if method == "initialize":
                    result = await self.handle_initialize(params)
                elif method == "tools/list":
                    result = await self.handle_tools_list(params)
                elif method == "tools/call":
                    result = await self.handle_tools_call(params)
                elif method == "resources/list":
                    result = await self.handle_resources_list(params)
                elif method == "resources/read":
                    result = await self.handle_resources_read(params)
                else:
                    result = {"error": f"Unknown method: {method}"}
                
                # レスポンス送信
                if message_id is not None:
                    response = {
                        "jsonrpc": "2.0",
                        "id": message_id,
                        "result": result
                    }
                    await self.send_message(response)
                
            except KeyboardInterrupt:
                break
            except Exception as e:
                logger.error(f"Main loop error: {e}")
                break
        
        logger.info("🐱 NekoCode MCP Server stopped")


if __name__ == "__main__":
    server = NekoCodeMCPServer()
    asyncio.run(server.run())