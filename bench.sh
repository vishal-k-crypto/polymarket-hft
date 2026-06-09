#!/usr/bin/env bash
# Run hot-path crypto benchmarks
set -euo pipefail
cd "$(dirname "$0")"
echo "==> Running benchmarks (target-cpu=native)..."
RUSTFLAGS="-C target-cpu=native" cargo bench --bench hot_path
echo "==> Open target/criterion/report/index.html for full report"
