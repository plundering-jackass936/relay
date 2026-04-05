#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BINARY="$SCRIPT_DIR/../core/target/release/relay"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.cargo/bin}"
[[ ! -f "$BINARY" ]] && "$SCRIPT_DIR/build.sh"
echo "=== Relay Install ==="
mkdir -p "$INSTALL_DIR" && cp "$BINARY" "$INSTALL_DIR/relay" && chmod +x "$INSTALL_DIR/relay"
mkdir -p "$HOME/.relay"
echo "✅  Installed to $INSTALL_DIR/relay"
echo "Run: relay init  — to create config"
echo "Run: relay agents — to check available agents"
