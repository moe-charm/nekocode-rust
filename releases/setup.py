#!/usr/bin/env python3
"""
NekoCode installer and wrappers (WSL-friendly)

Provides:
- --install: install PATH-safe wrappers to ~/.local/bin
- --install-docker: install Docker-backed wrappers to ~/.local/bin
- default: print MCP setup help (legacy behavior)
"""
import os
import sys
import stat
from textwrap import dedent
import shutil
import subprocess


def abs_paths():
    current_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(current_dir)
    possible_bins = [
        os.path.join(project_root, "releases", "nekocode"),
        os.path.join(project_root, "target", "release", "nekocode"),
    ]
    nekocode_bin = None
    for p in possible_bins:
        if os.path.exists(p):
            nekocode_bin = p
            break
    if nekocode_bin is None:
        nekocode_bin = possible_bins[0]

    mcp_server = os.path.join(project_root, "mcp-nekocode-server", "mcp_server_real.py")
    return os.path.abspath(nekocode_bin), os.path.abspath(mcp_server), project_root


def ensure_local_bin():
    user_base = subprocess.run([sys.executable, "-m", "site", "--user-base"],
                               capture_output=True, text=True).stdout.strip() or os.path.expanduser("~/.local")
    local_bin = os.path.join(user_base, "bin")
    os.makedirs(local_bin, exist_ok=True)
    return local_bin


def write_executable(path: str, content: str):
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)
    # chmod +x
    st = os.stat(path)
    os.chmod(path, st.st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def install_wrappers():
    nekocode_abs, mcp_server_abs, project_root = abs_paths()
    local_bin = ensure_local_bin()

    # PATH-safe wrapper for CLI
    cli_path = os.path.join(local_bin, "nekocode")
    cli_script = dedent(f"""
        #!/usr/bin/env bash
        set -euo pipefail
        # Prepend common user bins for external tools (cargo, vulture, etc.)
        USER_BIN="$(python3 -m site --user-base 2>/dev/null)/bin"
        if [[ -d "$USER_BIN" ]]; then export PATH="$USER_BIN:$PATH"; fi
        if [[ -d "$HOME/.cargo/bin" ]]; then export PATH="$HOME/.cargo/bin:$PATH"; fi
        NEKOCODE_BIN="{nekocode_abs}"
        exec "$NEKOCODE_BIN" "$@"
    """)
    write_executable(cli_path, cli_script)

    # PATH-safe wrapper for MCP (stdio by default)
    mcp_path = os.path.join(local_bin, "mcp-nekocode")
    mcp_script = dedent(f"""
        #!/usr/bin/env bash
        set -euo pipefail
        USER_BIN="$(python3 -m site --user-base 2>/dev/null)/bin"
        if [[ -d "$USER_BIN" ]]; then export PATH="$USER_BIN:$PATH"; fi
        if [[ -d "$HOME/.cargo/bin" ]]; then export PATH="$HOME/.cargo/bin:$PATH"; fi
        export NEKOCODE_BINARY_PATH="{nekocode_abs}"
        exec python3 "{mcp_server_abs}" "$@"
    """)
    write_executable(mcp_path, mcp_script)

    print("✅ Installed wrappers:")
    print(f"  - {cli_path}")
    print(f"  - {mcp_path}")
    print("\nNext steps:")
    print("  1) Ensure ~/.local/bin is on PATH (current shell):")
    print("     export PATH=\"$HOME/.local/bin:$PATH\"")
    print("  2) Persist PATH (bash):")
    print("     grep -qxF 'export PATH=\"$HOME/.local/bin:$PATH\"' ~/.bashrc || echo 'export PATH=\"$HOME/.local/bin:$PATH\"' >> ~/.bashrc")
    print("     source ~/.bashrc")
    print("  3) Test wrappers:")
    print("     nekocode --version || nekocode --help")
    print("     mcp-nekocode --stdio   # Ctrl+C to stop")
    print("\nClaude MCP: add server")
    print("  - WSL/Linux (recommended):")
    print("    claude mcp add nekocode -- mcp-nekocode --stdio")
    try:
        home = os.path.expanduser("~")
        abs_mcp = os.path.join(home, ".local", "bin", "mcp-nekocode")
        print("  - Windows client → WSL server:")
        print(f"    claude mcp add nekocode -- wsl.exe -e {abs_mcp} --stdio")
    except Exception:
        pass


def install_docker_wrappers(image: str = "ghcr.io/moe-charm/nekocode:latest"):
    local_bin = ensure_local_bin()

    docker_cli = os.path.join(local_bin, "nekocode")
    docker_cli_script = dedent(f"""
        #!/usr/bin/env bash
        set -euo pipefail
        # Pass through TTY if interactive
        DOCKER_TTY="-i"; if [ -t 1 ]; then DOCKER_TTY="-it"; fi
        exec docker run --rm $DOCKER_TTY \
          -v "$PWD":/work -w /work \
          {image} \
          nekocode "$@"
    """)
    write_executable(docker_cli, docker_cli_script)

    docker_mcp = os.path.join(local_bin, "mcp-nekocode")
    docker_mcp_script = dedent(f"""
        #!/usr/bin/env bash
        set -euo pipefail
        exec docker run --rm -i \
          -v "$PWD":/work -w /work \
          {image} \
          mcp-nekocode --stdio "$@"
    """)
    write_executable(docker_mcp, docker_mcp_script)

    print("✅ Installed Docker-backed wrappers (no local deps needed):")
    print(f"  - {docker_cli}")
    print(f"  - {docker_mcp}")
    print("\nUse:")
    print("  - nekocode session-create . --complete --external --format summary")
    print("  - mcp-nekocode --stdio (configure in your MCP-capable client)")


def print_mcp_help():
    nekocode_abs, mcp_server_abs, _ = abs_paths()
    print(dedent(f"""
    🚀 NekoCode Rust MCP セットアップ (ガイド)
    ========================================
    プロジェクトのルートで実行してください:

      claude mcp add nekocode \
        -e NEKOCODE_BINARY_PATH={nekocode_abs} \
        -- python3 {mcp_server_abs}

    もしくは setup.py --install で mcp-nekocode ラッパを導入後、
      mcp-nekocode --stdio
    をサーバコマンドに設定できます。
    """))


def main():
    args = sys.argv[1:]
    if not args:
        print_mcp_help()
        return

    if "--install" in args:
        install_wrappers()
        return

    if "--install-docker" in args:
        # Optional: custom image via --image=<name>
        image = "ghcr.io/moe-charm/nekocode:latest"
        for a in args:
            if a.startswith("--image="):
                image = a.split("=", 1)[1].strip()
        install_docker_wrappers(image)
        return

    # Fallback to help
    print_mcp_help()


if __name__ == "__main__":
    main()
