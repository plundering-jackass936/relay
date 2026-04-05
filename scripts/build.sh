#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../core"
echo "=== Relay Build ===" && cargo build --release
echo "✅  Built: target/release/relay ($(du -sh target/release/relay | cut -f1))"
