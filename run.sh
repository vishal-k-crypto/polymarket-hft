#!/usr/bin/env bash
# Build (release) and start the Rust bot.  PID goes to .bot.pid
set -euo pipefail
cd "$(dirname "$0")"

PID_FILE=.bot.pid
BIN=target/release/polymarket-hft

if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    echo "Bot already running (pid $(cat "$PID_FILE")). Use ./stop.sh first."
    exit 1
fi

echo "==> Building release binary (target-cpu=native)..."
RUSTFLAGS="-C target-cpu=native" cargo build --release

echo "==> Starting bot..."
nohup "$BIN" > logs/stdout.log 2>&1 &
PID=$!
echo "$PID" > "$PID_FILE"
disown

echo "Bot started — pid=$PID"
echo "  health : curl http://localhost:8080/health"
echo "  logs   : tail -f logs/stdout.log  or  ./logs.sh"
echo "  stop   : ./stop.sh"
