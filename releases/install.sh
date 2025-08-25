#!/usr/bin/env bash
set -euo pipefail

echo "[+] NekoCode external tools installer (user-level, no sudo)"

# Detect user paths
USER_BASE=$(python3 -m site --user-base 2>/dev/null || echo "$HOME/.local")
USER_BIN="$USER_BASE/bin"
CARGO_BIN="$HOME/.cargo/bin"

mkdir -p "$USER_BIN" "$CARGO_BIN"

echo "[+] Ensuring PATH entries in current shell"
export PATH="$USER_BIN:$CARGO_BIN:$PATH"

echo "[+] Appending PATH to ~/.bashrc if missing"
grep -q "$USER_BIN" ~/.bashrc 2>/dev/null || echo "export PATH=\"$USER_BIN:\$PATH\"" >> ~/.bashrc
grep -q "$CARGO_BIN" ~/.bashrc 2>/dev/null || echo "export PATH=\"$CARGO_BIN:\$PATH\"" >> ~/.bashrc

have() { command -v "$1" >/dev/null 2>&1; }

# Rustup + clippy
if ! have cargo; then
  echo "[+] Installing rustup (Rust toolchain)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

echo "[+] Adding component: clippy"
rustup component add clippy || true

# Python vulture
echo "[+] Installing Python vulture (dead code detector)"
python3 -m pip install --user -U vulture || true

# Optional: staticcheck (Go)
if have go; then
  echo "[+] Installing staticcheck (Go)"
  GO111MODULE=on GOPATH="$HOME/go" go install honnef.co/go/tools/cmd/staticcheck@latest || true
  ln -sf "$HOME/go/bin/staticcheck" "$CARGO_BIN/staticcheck" || true
else
  echo "[i] Go not found; skipping staticcheck"
fi

# Optional: eslint (Node)
if have npm; then
  echo "[+] Installing eslint (Node)"
  npm install -g eslint || true
  # Link into cargo bin for safer discovery
  if command -v eslint >/dev/null 2>&1; then
    ln -sf "$(command -v eslint)" "$CARGO_BIN/eslint" || true
  fi
else
  echo "[i] npm not found; skipping eslint"
fi

# Symlink vulture into ~/.cargo/bin so child PATH (which prepends ~/.cargo/bin) finds it reliably
if command -v vulture >/dev/null 2>&1; then
  echo "[+] Linking vulture into $CARGO_BIN"
  ln -sf "$(command -v vulture)" "$CARGO_BIN/vulture"
fi

echo "\n✅ Done. Summary:"
command -v cargo >/dev/null && echo "  - cargo: $(cargo --version | head -n1)" || echo "  - cargo: missing"
command -v clippy-driver >/dev/null && echo "  - clippy: installed" || echo "  - clippy: missing"
command -v vulture >/dev/null && echo "  - vulture: $(vulture --version 2>/dev/null || echo installed)" || echo "  - vulture: missing"
command -v staticcheck >/dev/null && echo "  - staticcheck: $(staticcheck -version 2>/dev/null || echo installed)" || echo "  - staticcheck: missing"
command -v eslint >/dev/null && echo "  - eslint: $(eslint -v 2>/dev/null || echo installed)" || echo "  - eslint: missing"

echo "\nNext:"
echo "  - Restart your shell or: source ~/.bashrc"
echo "  - Run: nekocode session-create . --complete --external --format summary"

