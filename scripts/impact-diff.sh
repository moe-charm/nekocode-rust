#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "$0")/.." && pwd)
BIN_DIR="$ROOT_DIR/target/release"
FALLBACK_DIR="$ROOT_DIR/releases"

NEKOCODE="$BIN_DIR/nekocode"
NEKOIMPACT="$BIN_DIR/nekoimpact"

COMPARE_REF="${1:-origin/main}"
OUT_FILE="${2:-}"
INCLUDE_WORKING="${INCLUDE_WORKING:-false}"

# Build if target binaries missing; otherwise fallback to releases
if [[ ! -x "$NEKOCODE" || ! -x "$NEKOIMPACT" ]]; then
  if [[ -x "$FALLBACK_DIR/nekocode" && -x "$FALLBACK_DIR/nekoimpact" ]]; then
    NEKOCODE="$FALLBACK_DIR/nekocode"
    NEKOIMPACT="$FALLBACK_DIR/nekoimpact"
  else
    echo "::group::Build binaries"
    cargo build --release
    echo "::endgroup::"
  fi
fi

chmod +x "$NEKOCODE" "$NEKOIMPACT" || true

echo "::group::Create session"
SESSION_OUTPUT=$("$NEKOCODE" session-create "$ROOT_DIR" -n ci_impact || true)
echo "$SESSION_OUTPUT"
SESSION_ID=$(echo "$SESSION_OUTPUT" | sed -n 's/.*Created session: \([A-Za-z0-9_-]*\).*/\1/p' | tail -n1)
if [[ -z "$SESSION_ID" ]]; then
  SESSION_ID=$("$NEKOCODE" session-list --detailed | sed -n 's/^🆔 \([A-Za-z0-9_-]*\).*/\1/p' | head -n1)
fi
echo "SESSION_ID=$SESSION_ID"
echo "::endgroup::"

if [[ -z "$SESSION_ID" ]]; then
  echo "Error: Failed to obtain session id" >&2
  exit 2
fi

INCLUDE_FLAG=()
if [[ "$INCLUDE_WORKING" == "true" ]]; then
  INCLUDE_FLAG+=("--include-working")
fi

echo "::group::Run impact diff ($COMPARE_REF)"
COMMENT=$("$NEKOIMPACT" diff "$SESSION_ID" --compare-ref "$COMPARE_REF" "${INCLUDE_FLAG[@]}" --format github-comment || true)
echo "$COMMENT"
echo "::endgroup::"

if [[ -n "$OUT_FILE" ]]; then
  printf "%s\n" "$COMMENT" >"$OUT_FILE"
fi

exit 0
