#!/usr/bin/env bash
# Stop and restart the Rust bot
set -euo pipefail
cd "$(dirname "$0")"
./stop.sh
sleep 1
./run.sh
