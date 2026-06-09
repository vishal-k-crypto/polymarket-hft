<div align="center">

<img src="https://capsule-render.vercel.app/api?type=waving&color=0:0d1117,50:161b22,100:0d1117&height=200&section=header&text=Polymarket%20HFT&fontSize=60&fontColor=58a6ff&animation=fadeIn&fontAlignY=35&desc=Sub-millisecond%20CLOB%20v2%20Execution%20Engine%20in%20Rust&descAlignY=55&descSize=18"/>

[![Rust](https://img.shields.io/badge/Rust-1.78+-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Tokio-Async%20Runtime-000000?style=for-the-badge&logo=tokio)](https://tokio.rs/)
[![License](https://img.shields.io/badge/License-MIT%20NC-2ea44f?style=for-the-badge)](LICENSE)

[📊 Architecture](#-architecture) · [⚡ Performance](#-performance) · [🚀 Quick Start](#-quick-start) · [📈 Benchmarks](#-benchmarks)

</div>

---

## 🎯 What is This?

**Polymarket HFT** is a high-frequency trading execution engine for Polymarket's CLOB v2 (Central Limit Order Book), written in Rust. It executes algorithmic strategies on binary option markets with **sub-millisecond latency** through aggressive compiler optimizations, lock-free state management, and zero-allocation hot paths.

> ⚠️ **This is a sanitized public showcase.** Strategy implementations, API credentials, and signing logic have been removed to protect intellectual property. The architecture, infrastructure, and performance characteristics are fully documented.

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         MARKET DATA FEEDS                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐  │
│  │ Binance WS   │  │ Polymarket   │  │ Chainlink Oracle             │  │
│  │ depth+trade  │  │ CLOB v2 WS   │  │ Polygon PoS eth_call         │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────────┬───────────────┘  │
└─────────┼─────────────────┼─────────────────────────┼──────────────────┘
          │                 │                         │
          ▼                 ▼                         ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      LOCK-FREE STATE LAYER (ArcSwap)                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐ │
│  │ BinanceSnap  │  │ L2Snapshot   │  │ OracleSnap   │  │ Execution   │ │
│  │ (pre-warmed) │  │ (pre-warmed) │  │ (pre-warmed) │  │ State       │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └─────────────┘ │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
          ┌──────────────────────────┼──────────────────────────┐
          │                          │                          │
          ▼                          ▼                          ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Strategy F    │    │   Strategy G    │    │   Strategy H    │
│  Fair Value     │    │  Flip Recovery  │    │  Hold Maturity  │
│  (Full exits)   │    │  (Flip on stop) │    │  (No interim)   │
└────────┬────────┘    └────────┬────────┘    └────────┬────────┘
         │                      │                      │
         └──────────────────────┼──────────────────────┘
                                │ SignalCmd (flume bounded)
                                ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    EXECUTOR (Dedicated Native OS Thread)                 │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  Zero-alloc JSON serialization · Buffer pool recycling           │   │
│  │  Persistent async dispatch (no per-order spawn)                  │   │
│  │  EIP-712 signing · FOK/FAK order types · Retry with backoff     │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Dedicated OS thread for executor** | Eliminates tokio scheduler jitter; `while let Ok(cmd) = rx.recv()` blocking loop |
| **ArcSwap lock-free state** | Zero Mutex/HashMap lookups on hot path; atomic pointer swaps |
| **Pre-warmed component pointers** | Strategy stores `Arc<ArcSwap<T>>` references at init; never looks up again |
| **Buffer pool recycling** | 64 pre-allocated `Vec<u8>` (2048B) for JSON serialization; zero global allocator calls |
| **Persistent async dispatch** | Single tokio task handles all HTTP; no `spawn()` per order |
| **4-thread tokio runtime** | Work-stealing multiplexes 8+ async tasks onto 4 OS threads |

---

## ⚡ Performance

### Compiler Optimizations

```toml
[profile.release]
opt-level = 3       # Max LLVM optimization + auto-vectorization
lto = true          # Fat LTO across crate boundaries
codegen-units = 1   # Single codegen block for maximum inlining
panic = "abort"     # No unwinding landing pads
strip = true        # Minimize icache footprint
```

### Benchmarks

```bash
$ cargo bench

hot_path/params_build       time:   [1.05 ns 1.07 ns 1.11 ns]    thrpt:  [903 Melem/s]
hot_path/stack_buffer_write time:   [306 ps 307 ps 309 ps]       thrpt:  [3.24 Gelem/s]
hot_path/price_math         time:   [256 ps 257 ps 259 ps]       thrpt:  [3.87 Gelem/s]
```

**Full-system targets (measured on production AWS c6i.xlarge):**

| Metric | Target | Achieved |
|--------|--------|----------|
| Order construction | < 1 µs | ✅ ~300 ns |
| EIP-712 hash | < 5 µs | ✅ ~1.2 µs |
| ECDSA sign | < 50 µs | ✅ ~15 µs |
| End-to-end hot path | < 100 µs | ✅ ~25 µs |
| Tick-to-signal latency | < 10 ms | ✅ ~3 ms |

### Why Rust vs Python

| Aspect | Python (asyncio) | This Rust Engine |
|--------|-----------------|------------------|
| GIL contention | ❌ Single-threaded CPU | ✅ True parallelism |
| Memory allocation | ❌ GC pauses | ✅ Zero-allocation hot path |
| Type safety | ❌ Runtime errors | ✅ Compile-time guarantees |
| Lock overhead | ❌ asyncio.Lock | ✅ ArcSwap atomic swaps |
| Binary size | ❌ Interpreter + deps | ✅ 12MB stripped binary |
| Startup time | ❌ ~2s | ✅ ~200ms |
| Throughput | ~1K orders/sec | ~50K orders/sec |

---

## 🚀 Quick Start

### Prerequisites

- Rust 1.78+ (`rustup update stable`)
- AWS account (for deployment)
- Polymarket API credentials

### Build

```bash
git clone https://github.com/vishal-k-crypto/polymarket-hft.git
cd polymarket-hft
cargo build --release
```

### Run Tests

```bash
cargo test
```

### Run Benchmarks

```bash
cargo bench
```

### Configuration

Create `.env` (see `.env.example` for required variables):

```bash
cp .env.example .env
# Edit with your credentials
```

**Required environment variables:**
- `POLY_ADDRESS` — your wallet address
- `POLY_PRIVATE_KEY` — your private key (with 0x prefix)
- `POLY_RPC_URL` — Polygon RPC endpoint

### Deploy to AWS

```bash
# Build for target CPU
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Deploy binary and config
scp target/release/polymarket-hft ubuntu@your-aws-ip:~/
scp .env.5m .env.15m ubuntu@your-aws-ip:~/

# Start on server
ssh ubuntu@your-aws-ip
./run.sh
```

---

## 📁 Project Structure

```
polymarket-hft/
├── src/
│   ├── main.rs              # Entry point, runtime setup, executor thread spawn
│   ├── lib.rs               # Module exports
│   ├── config.rs            # Environment configuration (.env parsing)
│   ├── state.rs             # Lock-free state management (ArcSwap stores)
│   ├── executor.rs          # Order execution engine (dedicated OS thread)
│   ├── logic_engine.rs      # Strategy orchestration, CSV logging workers
│   ├── signals.rs           # Inter-thread command types (SignalCmd)
│   ├── types.rs             # Shared data structures
│   ├── error.rs             # Error types
│   ├── logger.rs            # Structured logging (tracing)
│   ├── analytics.rs         # JSONL tick-level analytics pipeline
│   ├── event_buffer.rs      # Event buffering for replay
│   ├── oracle.rs            # Chainlink price feed polling
│   ├── crypto.rs            # EIP-712 signing, order hashing
│   ├── clob_client.rs       # Polymarket CLOB v2 API client
│   ├── market_discovery.rs  # Gamma API token ID discovery
│   ├── health.rs            # Axum HTTP dashboard (:8080/:8888)
│   ├── feeds/
│   │   ├── binance.rs       # Binance WebSocket feed
│   │   ├── polymarket.rs    # Polymarket CLOB WS feed
│   │   └── mod.rs           # Feed module exports
│   └── strategy/
│       ├── mod.rs           # Strategy trait
│       ├── fair_value.rs    # Strategy F — full exit stack
│       ├── flip_recovery.rs # Strategy G — flip on FAIR_STOP
│       └── hold_maturity.rs # Strategy H — hold to epoch end
│
├── benches/
│   └── hot_path.rs          # Criterion benchmarks
│
├── Cargo.toml               # Dependencies + release profile
├── rust-toolchain.toml      # Rust version pinning
└── scripts/
    ├── run.sh               # Start bot
    ├── stop.sh              # Stop bot
    ├── status.sh            # Check status
    ├── logs.sh              # Tail logs
    ├── restart.sh           # Restart bot
    └── bench.sh             # Run benchmarks
```

---

## 📊 Dashboard

The bot exposes an HTTP dashboard on `:8080` (5-min instance) or `:8888` (15-min instance):

- `GET /health` — liveness probe
- `GET /api/strategy/summary/{F,G,H}` — per-strategy PnL, positions, metrics
- WebSocket feed at `/ws` for real-time updates

---

## 🛡️ Risk Management

| Feature | Description |
|---------|-------------|
| **Circuit Breaker** | Kill switch on cumulative PnL < -$1.00 |
| **Daily Loss Limit** | Per-asset cooldown on loss > $1.00 |
| **Paper Trade Mode** | Full simulation without real orders |
| **Position Limits** | Max 1 position per asset per epoch |
| **Entry Cooldown** | 5s cooldown prevents retry storms |
| **Dust Filtering** | Skip positions below minimum size |

---

## 📈 Backtesting

The engine includes a Python backtester for strategy validation:

```bash
cd backtest_data
python backtester_g.py --start 2024-01-01 --end 2024-06-01 --strategy G
```

Backtest results include:
- PnL curve with drawdown analysis
- Win rate by market regime (trending, ranging, volatile)
- Exit reason breakdown (TAKE_PROFIT, FAIR_STOP, TIME_STOP, SETTLE)
- Parameter sensitivity analysis
- Monte Carlo projections (5th-95th percentile)

---

## 🤝 Contributing

This is a personal trading infrastructure project. The public showcase is provided for educational purposes. Strategy implementations are proprietary and not included.

---

## 📜 License

**MIT Non-Commercial License** — see [LICENSE](LICENSE) for full terms.

> ⚠️ **Commercial use is strictly prohibited** without explicit written permission. This software is provided for personal, educational, and non-commercial purposes only.

---

<div align="center">

**Built with** 🦀 **Rust** · ⚡ **Tokio** · 🔒 **EIP-712**

</div>
