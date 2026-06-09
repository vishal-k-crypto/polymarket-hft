#!/usr/bin/env bash
# Gracefully stop the Rust bot (SIGTERM → ordered shutdown)
set -euo pipefail
cd "$(dirname "$0")"

PID_FILE=.bot.pid

stop_by_pid() {
    local pid=$1
    echo "Stopping bot pid=$pid..."
    kill -TERM "$pid" 2>/dev/null || true
    for i in $(seq 1 50); do
        if ! kill -0 "$pid" 2>/dev/null; then
            echo "Bot stopped gracefully"
            rm -f "$PID_FILE"
            return 0
        fi
        sleep 0.1
    done
    echo "Bot did not stop, sending SIGKILL..."
    kill -KILL "$pid" 2>/dev/null || true
    rm -f "$PID_FILE"
    echo "Done"
}

if [[ -f "$PID_FILE" ]]; then
    PID=$(cat "$PID_FILE")
    if kill -0 "$PID" 2>/dev/null; then
        stop_by_pid "$PID"
        exit 0
    fi
fi

# No PID file or dead PID — kill by process name
if pgrep -f "polymarket-hft" > /dev/null 2>&1; then
    PID=$(pgrep -f "polymarket-hft" | head -1)
    stop_by_pid "$PID"
else
    echo "Bot not running"
fi
