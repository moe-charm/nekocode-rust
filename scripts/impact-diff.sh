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
    MAX_FILES=${MAX_FILES:-50}
    SHOW_NUMSTAT=${SHOW_NUMSTAT:-true}
    SHOW_LANG_BREAKDOWN=${SHOW_LANG_BREAKDOWN:-true}
    SHOW_BREAKING_HINTS=${SHOW_BREAKING_HINTS:-true}
    SHOW_DEPS=${SHOW_DEPS:-true}

    # Shortstat
    NUMSTAT=""
    if [[ "$SHOW_NUMSTAT" == "true" ]]; then
      NUMSTAT=$(git -C "$ROOT_DIR" diff --shortstat "$COMPARE_REF"..HEAD || true)
    fi

    # Language breakdown by extension
    LANG_TABLE=""
    if [[ "$SHOW_LANG_BREAKDOWN" == "true" ]]; then
      LANG_TABLE=$(printf "%s\n" "$CHANGED" | sed '/^$/d' | awk -F. '{print $NF}' | tr '[:upper:]' '[:lower:]' | sort | uniq -c | sort -nr | awk '{printf "- %s: %s\n", $2, $1}')
    fi

    # Breaking change hints (Rust pub removals)
    BREAKING_COUNT="0"
    if [[ "$SHOW_BREAKING_HINTS" == "true" ]]; then
      BREAKING_COUNT=$(git -C "$ROOT_DIR" diff -U0 "$COMPARE_REF"..HEAD -- '**/*.rs' | grep -E '^-\s*pub\s+(fn|struct|enum|trait)\b' | wc -l | tr -d ' ' || echo 0)
      BREAKING_COUNT=$(printf "%s" "$BREAKING_COUNT" | tr -d '\n' )
    fi

    # Dependency change hints
    DEPS_HINT=""
    if [[ "$SHOW_DEPS" == "true" ]]; then
      if printf "%s\n" "$CHANGED" | grep -q '^Cargo\.toml$'; then
        TOML_CHANGES=$(git -C "$ROOT_DIR" diff "$COMPARE_REF"..HEAD -- Cargo.toml | grep -E '^[+-][^+]' | wc -l | tr -d ' ' || echo 0)
        DEPS_HINT="Cargo.toml changes: ${TOML_CHANGES} lines"
      fi
      if printf "%s\n" "$CHANGED" | grep -q '^Cargo\.lock$'; then
        LOCK_CHANGES=$(git -C "$ROOT_DIR" diff --numstat "$COMPARE_REF"..HEAD -- Cargo.lock | awk '{add+=$1; del+=$2} END{printf "+%s −%s", add+0, del+0}')
        if [[ -n "$LOCK_CHANGES" ]]; then
          DEPS_HINT=$(printf "%s; Cargo.lock: %s" "$DEPS_HINT" "$LOCK_CHANGES")
        fi
      fi
    fi

    # Trim file list
    FILE_LIST=$(printf "%s\n" "$CHANGED" | sed '/^$/d' | head -n "$MAX_FILES")
    REMAIN=$(( COUNT > MAX_FILES ? COUNT - MAX_FILES : 0 ))

    COMMENT=$(
      {
        echo "## 🔍 Impact Diff (Simple Summary)"; echo;
        echo "- Base: \`$COMPARE_REF\`  ";
        echo "- Session: \`$SESSION_ID\`"; 
        echo "- Changed files: **$COUNT**"; 
        if [[ -n "$NUMSTAT" ]]; then echo "- Changes: $NUMSTAT"; fi
        if [[ -n "$LANG_TABLE" ]]; then echo; echo "### 📚 Language Breakdown"; printf "%s\n" "$LANG_TABLE"; fi
        if [[ "$SHOW_BREAKING_HINTS" == "true" ]]; then echo; echo "### ⚠️ Potential Breaking Removals"; echo "- Rust public items removed: **$BREAKING_COUNT**"; fi
        if [[ -n "$DEPS_HINT" ]]; then echo; echo "### 📦 Dependency Changes"; echo "- $DEPS_HINT"; fi
        echo; echo "<details><summary>Changed files (first $MAX_FILES)</summary>"; echo; 
        printf "%s\n" "$FILE_LIST" | sed 's/^/- `/' | sed 's/$/`/'
        if [[ "$REMAIN" -gt 0 ]]; then echo; echo "…and ${REMAIN} more"; fi
        echo; echo "</details>"; echo; 
        echo "---"; echo "*Simple summary generated due to missing native diff command*";
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
    MAX_FILES=${MAX_FILES:-50}
    SHOW_NUMSTAT=${SHOW_NUMSTAT:-true}
    SHOW_LANG_BREAKDOWN=${SHOW_LANG_BREAKDOWN:-true}
    SHOW_BREAKING_HINTS=${SHOW_BREAKING_HINTS:-true}
    SHOW_DEPS=${SHOW_DEPS:-true}

    # Shortstat
    NUMSTAT=""
    if [[ "$SHOW_NUMSTAT" == "true" ]]; then
      NUMSTAT=$(git -C "$ROOT_DIR" diff --shortstat "$COMPARE_REF"..HEAD || true)
    fi

    # Language breakdown by extension
    LANG_TABLE=""
    if [[ "$SHOW_LANG_BREAKDOWN" == "true" ]]; then
      LANG_TABLE=$(printf "%s\n" "$CHANGED" | sed '/^$/d' | awk -F. '{print $NF}' | tr '[:upper:]' '[:lower:]' | sort | uniq -c | sort -nr | awk '{printf "- %s: %s\n", $2, $1}')
    fi

    # Breaking change hints (Rust pub removals)
    BREAKING_COUNT="0"
    if [[ "$SHOW_BREAKING_HINTS" == "true" ]]; then
      BREAKING_COUNT=$(git -C "$ROOT_DIR" diff -U0 "$COMPARE_REF"..HEAD -- '**/*.rs' | grep -E '^-\s*pub\s+(fn|struct|enum|trait)\b' | wc -l | tr -d ' ' || echo 0)
      BREAKING_COUNT=$(printf "%s" "$BREAKING_COUNT" | tr -d '\n' )
    fi

    # Dependency change hints
    DEPS_HINT=""
    if [[ "$SHOW_DEPS" == "true" ]]; then
      if printf "%s\n" "$CHANGED" | grep -q '^Cargo\.toml$'; then
        TOML_CHANGES=$(git -C "$ROOT_DIR" diff "$COMPARE_REF"..HEAD -- Cargo.toml | grep -E '^[+-][^+]' | wc -l | tr -d ' ' || echo 0)
        DEPS_HINT="Cargo.toml changes: ${TOML_CHANGES} lines"
      fi
      if printf "%s\n" "$CHANGED" | grep -q '^Cargo\.lock$'; then
        LOCK_CHANGES=$(git -C "$ROOT_DIR" diff --numstat "$COMPARE_REF"..HEAD -- Cargo.lock | awk '{add+=$1; del+=$2} END{printf "+%s −%s", add+0, del+0}')
        if [[ -n "$LOCK_CHANGES" ]]; then
          DEPS_HINT=$(printf "%s; Cargo.lock: %s" "$DEPS_HINT" "$LOCK_CHANGES")
        fi
      fi
    fi

    # Trim file list
    FILE_LIST=$(printf "%s\n" "$CHANGED" | sed '/^$/d' | head -n "$MAX_FILES")
    REMAIN=$(( COUNT > MAX_FILES ? COUNT - MAX_FILES : 0 ))

    COMMENT=$( 
      {
        echo "## 🔍 Impact Diff (Simple Summary)"; echo;
        echo "- Base: \`$COMPARE_REF\`  ";
        echo "- Session: \`$SESSION_ID\`"; 
        echo "- Changed files: **$COUNT**"; 
        if [[ -n "$NUMSTAT" ]]; then echo "- Changes: $NUMSTAT"; fi
        if [[ -n "$LANG_TABLE" ]]; then echo; echo "### 📚 Language Breakdown"; printf "%s\n" "$LANG_TABLE"; fi
        if [[ "$SHOW_BREAKING_HINTS" == "true" ]]; then echo; echo "### ⚠️ Potential Breaking Removals"; echo "- Rust public items removed: **$BREAKING_COUNT**"; fi
        if [[ -n "$DEPS_HINT" ]]; then echo; echo "### 📦 Dependency Changes"; echo "- $DEPS_HINT"; fi
        echo; echo "<details><summary>Changed files (first $MAX_FILES)</summary>"; echo; 
        printf "%s\n" "$FILE_LIST" | sed 's/^/- `/' | sed 's/$/`/'
        if [[ "$REMAIN" -gt 0 ]]; then echo; echo "…and ${REMAIN} more"; fi
        echo; echo "</details>"; echo; 
        echo "---"; echo "*Simple summary generated due to missing native diff command*";
      }
    )
  fi
fi
echo "$COMMENT"
echo "::endgroup::"

if [[ -n "$OUT_FILE" ]]; then printf "%s\n" "$COMMENT" >"$OUT_FILE"; fi

exit 0
