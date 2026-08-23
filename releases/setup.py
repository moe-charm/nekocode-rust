#!/usr/bin/env python3
"""
NekoCode installer and wrappers (WSL-friendly)

Provides:
- --install: install canonical Rust-first CLI/MCP wrappers to ~/.local/bin
- --install-legacy: install the historical five-binary wrappers
- --install-docker: install the historical all-in-one Docker wrappers
- default: print Rust-first MCP setup help
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
        os.path.join(project_root, "nekocode-workspace", "target", "release", "nekocode"),
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
        # Ensure unbuffered stdio for robust MCP handshakes
        export PYTHONUNBUFFERED=1
        exec python3 -u "{mcp_server_abs}" "$@"
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


def install_rust_first_wrapper():
    """Install canonical Rust-first CLI and MCP wrappers.

    This deliberately leaves the legacy five-binary wrapper files untouched. The binary is
    staged by ``scripts/update_rust_first_release.sh`` or supplied separately
    by a release package.
    """
    nekocode_abs, _, project_root = abs_paths()
    gateway_abs = os.path.join(project_root, "mcp-nekocode-server", "mcp_server_rust_first.py")
    local_bin = ensure_local_bin()
    cli_path = os.path.join(local_bin, "nekocode")
    cli_script = dedent(f"""
        #!/usr/bin/env bash
        set -euo pipefail
        if [[ ! -x "{nekocode_abs}" ]]; then
          echo "nekocode binary not found: {nekocode_abs}" >&2
          echo "Run make rust-first-release first, or install a Rust-first release package." >&2
          exit 1
        fi
        exec "{nekocode_abs}" "$@"
    """)
    write_executable(cli_path, cli_script)

    mcp_path = os.path.join(local_bin, "mcp-nekocode")
    mcp_script = dedent(f"""
        #!/usr/bin/env bash
        set -euo pipefail
        if [[ ! -x "{nekocode_abs}" ]]; then
          echo "nekocode binary not found: {nekocode_abs}" >&2
          echo "Run make rust-first-release first, or install a Rust-first release package." >&2
          exit 1
        fi
        export NEKOCODE_BINARY_PATH="{nekocode_abs}"
        export NEKOCODE_CLI_CWD="${{NEKOCODE_CLI_CWD:-$PWD}}"
        export PYTHONUNBUFFERED=1
        exec python3 -u "{gateway_abs}" "$@"
    """)
    write_executable(mcp_path, mcp_script)

    print("✅ Installed Rust-first CLI wrapper:")
    print(f"  - {cli_path}")
    print("✅ Installed Rust-first MCP wrapper:")
    print(f"  - {mcp_path}")
    print("  (legacy five-binary sources and artifacts were not changed)")


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
    nekocode_abs, _, project_root = abs_paths()
    gateway_abs = os.path.abspath(os.path.join(project_root, "mcp-nekocode-server", "mcp_server_rust_first.py"))
    print(dedent(f"""
    🚀 NekoCode Rust-first MCP セットアップ (ガイド)
    ========================================
    プロジェクトのルートで実行してください:

      claude mcp add nekocode -- python3 -u {gateway_abs}

    もしくは setup.py --install でRust-firstラッパを導入後、
      mcp-nekocode --stdio
    をサーバコマンドに設定できます。CLI binaryは次の候補から解決されます:
      {nekocode_abs}

    旧5-binary導線が必要な場合だけ setup.py --install-legacy を使います。
    """))


def build_rust_first_direct_json() -> str:
    """Build JSON registration for the canonical stdio gateway."""
    import json

    nekocode_abs, _, project_root = abs_paths()
    gateway_abs = os.path.abspath(os.path.join(project_root, "mcp-nekocode-server", "mcp_server_rust_first.py"))
    payload = {
        "command": "python3",
        "args": ["-u", gateway_abs],
        "env": {
            "PYTHONUNBUFFERED": "1",
            "NEKOCODE_BINARY_PATH": nekocode_abs,
        },
    }
    return json.dumps(payload)


def resolve_binaries(prefer: str | None = None):
    """Resolve binaries for direct python3 registration.

    prefer: 'bin' | 'releases' | None
    """
    current_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(current_dir)

    # Candidate orders
    nekocode_candidates_bin = [
        os.path.join(project_root, "bin", "nekocode"),
        os.path.join(project_root, "releases", "nekocode"),
        os.path.join(project_root, "nekocode-workspace", "target", "release", "nekocode"),
        os.path.join(project_root, "nekocode-workspace", "target", "debug", "nekocode"),
    ]
    nekocode_candidates_rel = [
        os.path.join(project_root, "releases", "nekocode"),
        os.path.join(project_root, "bin", "nekocode"),
        os.path.join(project_root, "nekocode-workspace", "target", "release", "nekocode"),
        os.path.join(project_root, "nekocode-workspace", "target", "debug", "nekocode"),
    ]
    nekorefactor_candidates_bin = [
        os.path.join(project_root, "bin", "nekorefactor"),
        os.path.join(project_root, "releases", "nekorefactor"),
        os.path.join(project_root, "nekocode-workspace", "target", "release", "nekorefactor"),
        os.path.join(project_root, "nekocode-workspace", "target", "debug", "nekorefactor"),
    ]
    nekorefactor_candidates_rel = [
        os.path.join(project_root, "releases", "nekorefactor"),
        os.path.join(project_root, "bin", "nekorefactor"),
        os.path.join(project_root, "nekocode-workspace", "target", "release", "nekorefactor"),
        os.path.join(project_root, "nekocode-workspace", "target", "debug", "nekorefactor"),
    ]

    def first_existing(paths):
        for p in paths:
            if os.path.exists(p):
                return os.path.abspath(p)
        # Fallback to first candidate
        return os.path.abspath(paths[0])

    if prefer == "releases":
        nekocode_abs = first_existing(nekocode_candidates_rel)
        nekorefactor_abs = first_existing(nekorefactor_candidates_rel)
    else:
        # default to bin preference
        nekocode_abs = first_existing(nekocode_candidates_bin)
        nekorefactor_abs = first_existing(nekorefactor_candidates_bin)

    mcp_server_abs = os.path.abspath(os.path.join(project_root, "mcp-nekocode-server", "mcp_server_real.py"))
    return nekocode_abs, nekorefactor_abs, mcp_server_abs, project_root


def build_direct_json(prefer: str | None = None) -> str:
    """Build JSON payload for `claude mcp add-json` (python3 direct)."""
    import json
    nekocode_abs, nekorefactor_abs, mcp_server_abs, _ = resolve_binaries(prefer)
    payload = {
        "command": "python3",
        "args": [mcp_server_abs],
        "env": {
            "PYTHONUNBUFFERED": "1",
            "NEKOCODE_BINARY_PATH": nekocode_abs,
            "NEKOREFACTOR_BINARY_PATH": nekorefactor_abs,
        },
    }
    return json.dumps(payload)


def main():
    args = sys.argv[1:]
    if not args:
        print_mcp_help()
        return

    if "--install-legacy" in args:
        install_wrappers()
        return

    if "--install" in args or "--install-rust-first" in args:
        install_rust_first_wrapper()
        return

    if "--install-docker" in args:
        # Optional: custom image via --image=<name>
        image = "ghcr.io/moe-charm/nekocode:latest"
        for a in args:
            if a.startswith("--image="):
                image = a.split("=", 1)[1].strip()
        install_docker_wrappers(image)
        return

    # Print direct registration JSON for Claude MCP (python3 direct).
    # Usage: --direct-json [--prefer=bin|releases]
    if "--direct-json-legacy" in args:
        prefer = None
        for a in args:
            if a.startswith("--prefer="):
                prefer = a.split("=", 1)[1].strip()
        js = build_direct_json(prefer)
        print("# Register legacy MCP server with python3 direct (copy/paste):")
        print("claude mcp add-json nekocode-legacy '" + js + "'")
        return

    if "--direct-json" in args:
        js = build_rust_first_direct_json()
        print("# Register Rust-first MCP server with python3 direct (copy/paste):")
        print("claude mcp add-json nekocode '" + js + "'")
        return

    # Install direct registration via claude CLI (if available)
    # Usage: --install-direct [--prefer=bin|releases]
    if "--install-direct-legacy" in args:
        prefer = None
        for a in args:
            if a.startswith("--prefer="):
                prefer = a.split("=", 1)[1].strip()
        js = build_direct_json(prefer)
        try:
            subprocess.run(["claude", "mcp", "add-json", "nekocode-legacy", js], check=True)
            print("✅ Registered legacy MCP server 'nekocode-legacy'")
        except Exception as e:
            print(f"⚠️ Failed to auto-register legacy MCP via claude CLI: {e}")
            print("You can register manually with:")
            print("  claude mcp add-json nekocode-legacy '" + js + "'")
        return

    if "--install-direct" in args:
        js = build_rust_first_direct_json()
        try:
            subprocess.run(["claude", "mcp", "add-json", "nekocode", js], check=True)
            print("✅ Registered Rust-first MCP server 'nekocode' (python3 direct)")
        except Exception as e:
            print(f"⚠️ Failed to auto-register via claude CLI: {e}")
            print("You can register manually with:")
            print("  claude mcp add-json nekocode '" + js + "'")
        return

    # Fallback to help
    print_mcp_help()


if __name__ == "__main__":
    main()
