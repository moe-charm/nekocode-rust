#!/usr/bin/env bash
set -euo pipefail

# Build and stage the canonical CLI for a reproducible release artifact.
#
# Usage:
#   scripts/update_rust_first_release.sh [--clean] [--skip-build] [--tag TAG] [--output DIR]

ROOT_DIR=$(cd "$(dirname "$0")/.." && pwd)
MANIFEST="$ROOT_DIR/nekocode-workspace/Cargo.toml"
TARGET="$ROOT_DIR/nekocode-workspace/target/release/nekocode"
OUTPUT_DIR="$ROOT_DIR/dist"
CLEAN=0
SKIP_BUILD=0
TAG=""

while [ "$#" -gt 0 ]; do
  arg=$1
  case "$arg" in
    --clean) CLEAN=1 ;;
    --skip-build) SKIP_BUILD=1 ;;
    --tag)
      shift
      if [ "$#" -eq 0 ] || [ -z "$1" ]; then
        echo "[!] --tag requires a tag name" >&2
        exit 2
      fi
      TAG=$1
      ;;
    --tag=*) TAG=${arg#--tag=} ;;
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

case "$OUTPUT_DIR" in
  /*) ;;
  *) OUTPUT_DIR="$ROOT_DIR/$OUTPUT_DIR" ;;
esac

command -v git >/dev/null 2>&1 || {
  echo "[!] git is required for release provenance" >&2
  exit 1
}

is_tracked_file() {
  git -C "$ROOT_DIR" ls-files --error-unmatch -- "$1" >/dev/null 2>&1
}

reject_untracked_cargo_config() {
  local search_dir="$ROOT_DIR"
  while :; do
    for name in ".cargo/config.toml" ".cargo/config"; do
      local candidate="$search_dir/$name"
      if [ -f "$candidate" ] && ! is_tracked_file "${candidate#"$ROOT_DIR/"}"; then
        echo "[!] untracked or ignored Cargo configuration affects release input: $candidate" >&2
        echo "    track it deliberately or remove it from the release environment" >&2
        return 1
      fi
    done
    [ "$search_dir" = "/" ] && break
    search_dir=$(dirname "$search_dir")
  done

  for name in ".cargo/config.toml" ".cargo/config"; do
    local workspace_candidate="$ROOT_DIR/nekocode-workspace/$name"
    if [ -f "$workspace_candidate" ] && ! is_tracked_file "nekocode-workspace/$name"; then
      echo "[!] untracked or ignored Cargo configuration affects release input: $workspace_candidate" >&2
      echo "    track it deliberately or remove it from the release environment" >&2
      return 1
    fi
  done

  local cargo_home=${CARGO_HOME:-${HOME:-}/.cargo}
  for name in "config.toml" "config"; do
    local global_candidate="$cargo_home/$name"
    if [ -f "$global_candidate" ]; then
      echo "[!] global Cargo configuration is not an explicit release input: $global_candidate" >&2
      echo "    use a clean CARGO_HOME for release packaging" >&2
      return 1
    fi
  done
}

reject_untracked_cargo_config

if [ -n "$(git -C "$ROOT_DIR" status --porcelain --untracked-files=all)" ]; then
  echo "[!] release packaging requires a clean Git worktree" >&2
  exit 1
fi

COMMIT=$(git -C "$ROOT_DIR" rev-parse HEAD)
if [ -n "$TAG" ]; then
  TAG_COMMIT=$(git -C "$ROOT_DIR" rev-parse "${TAG}^{commit}" 2>/dev/null || true)
  if [ -z "$TAG_COMMIT" ] || [ "$TAG_COMMIT" != "$COMMIT" ]; then
    echo "[!] tag does not resolve to HEAD: $TAG" >&2
    exit 1
  fi
fi

if [ "$SKIP_BUILD" -eq 0 ]; then
  command -v cargo >/dev/null 2>&1 || {
    echo "[!] cargo is required; use --skip-build with an existing release binary" >&2
    exit 1
  }
  EXPECTED_RUSTC_VERSION=${NEKOCODE_RELEASE_RUSTC_VERSION:-1.85.0}
  ACTUAL_RUSTC_VERSION=$(rustc --version | awk '{print $2}')
  if [ "$ACTUAL_RUSTC_VERSION" != "$EXPECTED_RUSTC_VERSION" ]; then
    echo "[!] release requires rustc $EXPECTED_RUSTC_VERSION (found $ACTUAL_RUSTC_VERSION)" >&2
    echo "    set up the pinned toolchain or use the matching release container" >&2
    exit 1
  fi
  if [ "$CLEAN" -eq 1 ]; then
    cargo clean --manifest-path "$MANIFEST"
  fi
  cargo build --manifest-path "$MANIFEST" --locked --package nekocode --release
fi

if [ ! -x "$TARGET" ]; then
  echo "[!] canonical release binary not found: $TARGET" >&2
  exit 1
fi

if [ -e "$OUTPUT_DIR" ]; then
  if [ -n "$(find "$OUTPUT_DIR" -mindepth 1 -print -quit 2>/dev/null)" ]; then
    echo "[!] release staging directory must be empty: $OUTPUT_DIR" >&2
    echo "    choose a new versioned output directory" >&2
    exit 1
  fi
else
  mkdir -p "$OUTPUT_DIR"
fi
install -m 0755 "$TARGET" "$OUTPUT_DIR/nekocode"
VERSION=$("$OUTPUT_DIR/nekocode" --version | tr -d '\r\n')

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$OUTPUT_DIR" && sha256sum nekocode > nekocode.sha256)
elif command -v shasum >/dev/null 2>&1; then
  (cd "$OUTPUT_DIR" && shasum -a 256 nekocode > nekocode.sha256)
else
  echo "[!] sha256sum or shasum is required for release checksums" >&2
  exit 1
fi

SHA256=$(awk '{print $1}' "$OUTPUT_DIR/nekocode.sha256")
command -v python3 >/dev/null 2>&1 || {
  echo "[!] python3 is required for release provenance" >&2
  exit 1
}
CARGO_VERSION=$(cargo --version 2>/dev/null || true)
RUSTC_VERSION=$(rustc --version 2>/dev/null || true)
GENERATED_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)
python3 - "$OUTPUT_DIR/nekocode.provenance.json" "$VERSION" "$COMMIT" "$TAG" "$SHA256" "$CARGO_VERSION" "$RUSTC_VERSION" "$GENERATED_AT" <<'PY'
import json
import sys

output, version, commit, tag, sha256, cargo, rustc, generated_at = sys.argv[1:]
provenance = {
    "schema_version": 1,
    "artifact": "nekocode",
    "version": version,
    "git_commit": commit,
    "git_tag": tag or None,
    "sha256": sha256,
    "cargo": cargo or None,
    "rustc": rustc or None,
    "generated_at": generated_at,
}
with open(output, "w", encoding="utf-8") as handle:
    json.dump(provenance, handle, ensure_ascii=False, indent=2, sort_keys=True)
    handle.write("\n")
PY

echo "[+] CLI staged at $OUTPUT_DIR/nekocode"
echo "[+] SHA-256 written to $OUTPUT_DIR/nekocode.sha256"
echo "[+] Provenance written to $OUTPUT_DIR/nekocode.provenance.json"
