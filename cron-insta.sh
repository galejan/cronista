#!/usr/bin/env bash
# Cron-Insta v0.1.3 — Launch script for Arch Linux
# Place this script anywhere, or symlink to /usr/local/bin/cron-insta

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$SCRIPT_DIR/src-tauri/target/release/cron-insta"

if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found. Build it first: cd $SCRIPT_DIR && pnpm tauri build"
    exit 1
fi

exec "$BINARY" "$@"
