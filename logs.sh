#!/usr/bin/env bash
# Tail the bot's stdout + structured log
set -euo pipefail
cd "$(dirname "$0")"
echo "=== tail -f logs/stdout.log ==="
tail -f logs/stdout.log
