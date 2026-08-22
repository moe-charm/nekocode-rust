#!/usr/bin/env bash
set -euo pipefail

# Build and stage only the Rust-first CLI.  Legacy five-binary artifacts are
# intentionally left untouched; commit or publish the staged file separately.
#
# Usage:
#   scripts/update_rust_first_release.sh [--clean] [--skip-build] [--output DIR]

ROOT_DIR=$(cd "$(dirname "$0")/.." && pwd)
MANIFEST="$ROOT_DIR/nekocode-workspace/Cargo.toml"
TARGET="$ROOT_DIR/nekocode-workspace/target/release/nekocode"
OUTPUT_DIR="$ROOT_DIR/releases"
CLEAN=0
SKIP_BUILD=0

while [ "$#" -gt 0 ]; do
  arg=$1
  case "$arg" in
    --clean) CLEAN=1 ;;
    --skip-build) SKIP_BUILD=1 ;;
    --output)
      shift
      if [ "$#" -eq 0 ] || [ -z "$1" ]; then
        echo "[!] --output requires a directory argument" >&2
        exit 2
      fi
      OUTPUT_DIR=$1
      ;;
    --output=*) OUTPUT_DIR=${arg#--output=} ;;
    *)
      echo "[!] Unknown arg: $arg" >&2
      exit 2
      ;;
  esac
  shift
done

if [ -z "$OUTPUT_DIR" ]; then
  echo "[!] Output directory must not be empty" >&2
  exit 2
fi

if [ "$SKIP_BUILD" -eq 0 ]; then
  command -v cargo >/dev/null 2>&1 || {
    echo "[!] cargo is required; use --skip-build with an existing release binary" >&2
    exit 1
  }
  if [ "$CLEAN" -eq 1 ]; then
    cargo clean --manifest-path "$MANIFEST"
  fi
  cargo build --manifest-path "$MANIFEST" --package nekocode --release
fi

if [ ! -x "$TARGET" ]; then
  echo "[!] canonical release binary not found: $TARGET" >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"
install -m 0755 "$TARGET" "$OUTPUT_DIR/nekocode"
"$OUTPUT_DIR/nekocode" --version
echo "[+] Rust-first CLI staged at $OUTPUT_DIR/nekocode"
