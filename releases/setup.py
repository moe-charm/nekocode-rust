#!/usr/bin/env python3
"""
🐱 NekoCode Rust MCP セットアップ - 超シンプル版
"""
import os

# 現在のディレクトリ（bin/）を取得
current_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.dirname(current_dir)

# NekoCodeバイナリパスを決定（優先順位順）
possible_paths = [
    os.path.join(current_dir, "nekocode_ai"),  # bin/nekocode_ai (既存)
    os.path.join(project_root, "releases", "nekocode-rust"),  # releases/nekocode-rust (新)
    os.path.join(project_root, "target", "release", "nekocode-rust"),  # target/release/nekocode-rust
]

nekocode_path = None
for path in possible_paths:
    if os.path.exists(path):
        nekocode_path = path
        break

if not nekocode_path:
    nekocode_path = possible_paths[0]  # デフォルトで bin/nekocode_ai

mcp_server_path = os.path.join(project_root, "mcp-nekocode-server", "mcp_server_real.py")

# 絶対パスに変換
nekocode_abs = os.path.abspath(nekocode_path)
mcp_server_abs = os.path.abspath(mcp_server_path)

print(f"""
🚀 NekoCode Rust MCP セットアップ (16倍高速版！)
================================================

⚠️ 重要: 以下のコマンドは【あなたが解析したいプロジェクトのルート】で実行してください！
        （Claude CodeがそのプロジェクトをNekoCodeで解析するため）

1. あなたのプロジェクトに移動:
   cd ~/your-awesome-project   # ← あなたが開発中のプロジェクト
   
   例: cd ~/my-react-app
       cd ~/rust-project  
       cd ~/python-app
   
   ※NekoCode Rustのディレクトリではありません！

2. そのプロジェクトのルートで以下を実行:

claude mcp add nekocode \\
  -e NEKOCODE_BINARY_PATH={nekocode_abs} \\
  -- python3 {mcp_server_abs}

または、手動で設定ファイルに追加：
~/.config/claude-desktop/config.json (Linux)
~/Library/Application Support/Claude/claude_desktop_config.json (Mac)

{{
  "mcpServers": {{
    "nekocode": {{
      "command": "python3",
      "args": ["{mcp_server_abs}"],
      "env": {{
        "NEKOCODE_BINARY_PATH": "{nekocode_abs}"
      }}
    }}
  }}
}}

設定後、Claude Codeを再起動してください！

========================================
📝 まとめ:
1. NekoCode Rustをインストール済み ✓
2. あなたのプロジェクトフォルダに移動
3. そこでclaude mcp addを実行
4. Claude Code再起動
5. そのプロジェクトでNekoCodeが使える！

🎯 特徴:
- 16倍高速 (C++版と比較)
- 96%軽量 (9MB vs 235MB)
- ビルド不要 (既にコンパイル済み)
========================================
""")