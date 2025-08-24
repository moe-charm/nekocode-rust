#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "$0")/.." && pwd)
BIN_DIR="$ROOT_DIR/target/release"
FALLBACK_DIR="$ROOT_DIR/releases"

# Detect tool variants
NEKOCODE_FIVE="$BIN_DIR/nekocode"
NEKOIMPACT_FIVE="$BIN_DIR/nekoimpact"
NEKOCODE_RUST="$BIN_DIR/nekocode-rust"

COMPARE_REF="${1:-origin/main}"
OUT_FILE="${2:-}"
INCLUDE_WORKING="${INCLUDE_WORKING:-false}"

# Try to build if nothing is present
VARIANT=""
if [[ -x "$NEKOCODE_RUST" ]]; then
  VARIANT="rust-single"
elif [[ -x "$NEKOCODE_FIVE" && -x "$NEKOIMPACT_FIVE" ]]; then
  VARIANT="five-binary"
elif [[ -x "$FALLBACK_DIR/nekocode-rust" ]]; then
  NEKOCODE_RUST="$FALLBACK_DIR/nekocode-rust"; VARIANT="rust-single"
elif [[ -x "$FALLBACK_DIR/nekocode" && -x "$FALLBACK_DIR/nekoimpact" ]]; then
  NEKOCODE_FIVE="$FALLBACK_DIR/nekocode"; NEKOIMPACT_FIVE="$FALLBACK_DIR/nekoimpact"; VARIANT="five-binary"
else
  echo "::group::Build binaries"
  cargo build --release
  echo "::endgroup::"
  if [[ -x "$NEKOCODE_RUST" ]]; then VARIANT="rust-single"; fi
  if [[ -z "$VARIANT" && -x "$NEKOCODE_FIVE" && -x "$NEKOIMPACT_FIVE" ]]; then VARIANT="five-binary"; fi
fi

if [[ "$VARIANT" == "rust-single" ]]; then
  chmod +x "$NEKOCODE_RUST" || true
else
  chmod +x "$NEKOCODE_FIVE" "$NEKOIMPACT_FIVE" || true
fi

echo "::group::Create session"
if [[ "$VARIANT" == "rust-single" ]]; then
  SESSION_OUTPUT=$("$NEKOCODE_RUST" session-create "$ROOT_DIR" -n ci_impact || true)
else
  SESSION_OUTPUT=$("$NEKOCODE_FIVE" session-create "$ROOT_DIR" -n ci_impact || true)
fi
echo "$SESSION_OUTPUT"
SESSION_ID=$(echo "$SESSION_OUTPUT" | sed -n 's/.*Created session: \([A-Za-z0-9_-]*\).*/\1/p' | tail -n1)
if [[ -z "$SESSION_ID" ]]; then
  if [[ "$VARIANT" == "rust-single" ]]; then
    SESSION_ID=$("$NEKOCODE_RUST" session-list --detailed | sed -n 's/^🆔 \([A-Za-z0-9_-]*\).*/\1/p' | head -n1)
  else
    SESSION_ID=$("$NEKOCODE_FIVE" session-list --detailed | sed -n 's/^🆔 \([A-Za-z0-9_-]*\).*/\1/p' | head -n1)
  fi
fi
echo "SESSION_ID=$SESSION_ID"
echo "::endgroup::"

if [[ -z "$SESSION_ID" ]]; then
  echo "Error: Failed to obtain session id" >&2
  exit 2
fi

INCLUDE_FLAG=()
if [[ "$INCLUDE_WORKING" == "true" ]]; then INCLUDE_FLAG+=("--include-working"); fi

echo "::group::Run impact diff ($COMPARE_REF)"
COMMENT=""
if [[ "$VARIANT" == "five-binary" ]]; then
  COMMENT=$("$NEKOIMPACT_FIVE" diff "$SESSION_ID" --compare-ref "$COMPARE_REF" "${INCLUDE_FLAG[@]}" --format github-comment || true)
  # Fallback if five-binary diff is not available
  if [[ -z "$COMMENT" ]] || echo "$COMMENT" | grep -qi "not yet implemented"; then
    echo "⚠️ Five-binary diff not available. Generating simple impact summary..."
    CHANGED=$(git -C "$ROOT_DIR" diff --name-only "$COMPARE_REF"..HEAD || true)
    COUNT=$(printf "%s" "$CHANGED" | sed '/^$/d' | wc -l | tr -d ' ')
    COMMENT=$(
      {
        echo "## 🔍 Impact Diff (Simple Summary)"; echo;
        echo "- Base: \`$COMPARE_REF\`  ";
        echo "- Session: \`$SESSION_ID\`"; 
        echo "- Changed files: **$COUNT**"; echo;
        printf "%s\n" "$CHANGED" | sed 's/^/- `/' | sed 's/$/`/'
        echo; echo "---"; echo "*Simple summary generated due to missing native diff command*";
      }
    )
  fi
else
  if "$NEKOCODE_RUST" --help 2>/dev/null | grep -qi "diff"; then
    COMMENT=$("$NEKOCODE_RUST" diff --session-id "$SESSION_ID" --compare-ref "$COMPARE_REF" "${INCLUDE_FLAG[@]}" --format github-comment || true)
  fi
  if [[ -z "$COMMENT" ]] || echo "$COMMENT" | grep -qi "not yet implemented"; then
    echo "⚠️ Native diff not available. Generating simple impact summary..."
    CHANGED=$(git -C "$ROOT_DIR" diff --name-only "$COMPARE_REF"..HEAD || true)
    COUNT=$(printf "%s" "$CHANGED" | sed '/^$/d' | wc -l | tr -d ' ')
    COMMENT=$(
      {
        echo "## 🔍 Impact Diff (Simple Summary)"; echo;
        echo "- Base: \`$COMPARE_REF\`  ";
        echo "- Session: \`$SESSION_ID\`"; 
        echo "- Changed files: **$COUNT**"; echo;
        printf "%s\n" "$CHANGED" | sed 's/^/- `/' | sed 's/$/`/'
        echo; echo "---"; echo "*Simple summary generated due to missing native diff command*";
      }
    )
  fi
fi
echo "$COMMENT"
echo "::endgroup::"

if [[ -n "$OUT_FILE" ]]; then printf "%s\n" "$COMMENT" >"$OUT_FILE"; fi

exit 0
