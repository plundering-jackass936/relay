#!/usr/bin/env bash
# Relay PostToolUse hook — detects rate limits and auto-hands off to fallback agent.
set -euo pipefail

RELAY="${RELAY_BIN:-relay}"
if ! command -v "$RELAY" &>/dev/null; then
  for candidate in "$HOME/.cargo/bin/relay" "/usr/local/bin/relay" \
    "$(dirname "$0")/../core/target/release/relay"; do
    [[ -x "$candidate" ]] && { RELAY="$candidate"; break; }
  done
fi

INPUT=$(cat)

if command -v "$RELAY" &>/dev/null || [[ -x "$RELAY" ]]; then
  echo "$INPUT" | "$RELAY" hook --session "${CLAUDE_SESSION_ID:-default}" 2>/dev/null || echo "$INPUT"
else
  echo "$INPUT"
fi
