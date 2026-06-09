#!/usr/bin/env bash
# Show bot health status + key PNL / latency metrics
set -euo pipefail
cd "$(dirname "$0")"

PID_FILE=.bot.pid
RUNNING=false

if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    RUNNING=true
fi

echo "=== PROCESS ==="
if $RUNNING; then
    echo "Status : RUNNING (pid $(cat "$PID_FILE"))"
else
    echo "Status : STOPPED"
fi

echo ""
echo "=== HEALTH (curl localhost:8080/health) ==="
if ! HEALTH=$(curl -s --max-time 3 http://localhost:8080/health 2>/dev/null); then
    echo "Health endpoint not reachable"
    exit 1
fi

echo "$HEALTH" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f'  Status        : {d[\"status\"]}')
print(f'  Uptime        : {d[\"uptime_secs\"]}s')
print(f'  Paper mode    : {d[\"paper_mode\"]}')
print(f'  Kill switch   : {d[\"kill_switch\"]}')
print(f'  PNL           : \${d[\"pnl\"]:.4f}')
print(f'  Orders        : {d[\"orders_submitted\"]} submitted / {d[\"orders_cancelled\"]} cancelled / {d[\"orders_failed\"]} failed')
print(f'  Latency (max) : {d[\"latency_us\"]} µs')
for m in d.get('markets', []):
    src = m.get('oracle_source', '?')
    mid = m.get('binance_mid', 0)
    bid = m.get('poly_up_bid', 0)
    ask = m.get('poly_up_ask', 0)
    vol = m.get('binance_volatility', 0)
    print(f'  {m[\"asset\"]:>5s}  mid={mid:>10.2f}  vol={vol:.8f}  up_bid={bid:.4f}  up_ask={ask:.4f}  binance={m[\"binance_connected\"]}  epoch={m[\"epoch_id\"]}')
" 2>/dev/null || echo "  (could not parse health JSON)"
