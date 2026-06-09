//! Zero-allocation analytics event pipeline.
//!
//! Produces JSONL (one JSON object per line) capturing full market context at
//! every strategy tick evaluation — both trades taken and trades not taken.
//! Offloaded to a dedicated native OS thread via `flume::Sender<String>`.

use std::fmt::Write;

/// JSONL line for a strategy tick evaluation. Called from the strategy tick
/// loop; output is sent via `try_send` on the analytics channel — never blocks.
#[allow(clippy::too_many_arguments)]
pub fn analytics_tick(
    asset: &str,
    strategy: &str,
    epoch_ms: u64,
    seconds_left: f64,
    epoch_id: i64,
    has_position: bool,
    action: &str,
    decision_side: &str,
    size_usd: f64,
    confidence_tier: &str,
    reason: &str,
) -> String {
    let now_ms = now_ms();
    let ts_utc = utc_timestamp_from_ms(now_ms);
    let hour_utc = ((now_ms / 3_600_000) % 24) as u8;
    let day_of_week = ((now_ms / 86_400_000 + 4) % 7) as u8;
    let epoch_secs_elapsed = epoch_ms.saturating_sub((seconds_left * 1000.0) as u64) as f64 / 1000.0;
    let epoch_close_ms = ((now_ms / epoch_ms) + 1) * epoch_ms;
    let epoch_progress_pct = if epoch_ms > 0 {
        (epoch_secs_elapsed / (epoch_ms as f64 / 1000.0)) * 100.0
    } else { 0.0 };

    let mut s = String::with_capacity(2800);

    write!(s, r#"{{"ts":"{}","ts_ms":{},"hour_utc":{},"dow":{},"asset":"{}","epoch_id":{},"epoch_close_ms":{},"epoch_elapsed_s":{:.2},"epoch_left_s":{:.2},"epoch_pct":{:.1},"strategy":"{}""#,
        ts_utc, now_ms, hour_utc, day_of_week, asset, epoch_id, epoch_close_ms,
        epoch_secs_elapsed, seconds_left, epoch_progress_pct, strategy,
    ).unwrap();

    write!(s, r#","pos":{},"action":"{}","side":"{}","size_usd":{:.4},"tier":"{}","reason":"{}"}}"#,
        has_position as u8, action, decision_side, size_usd, confidence_tier, reason,
    ).unwrap();

    s
}

/// Post-trade outcome event emitted when a position is closed.
pub fn analytics_close(
    asset: &str,
    strategy: &str,
    epoch_id: i64,
    side: &str,
    trade_pnl: f64,
    entry_price: f64,
    exit_price: f64,
    shares: f64,
    exit_reason: &str,
    cum_pnl: f64,
) -> String {
    let now_ms = now_ms();
    let ts_utc = utc_timestamp_from_ms(now_ms);
    let mut s = String::with_capacity(400);
    write!(s,
        r#"{{"ts":"{}","ts_ms":{},"asset":"{}","epoch_id":{},"strategy":"{}","action":"CLOSE","side":"{}","ep":{:.4},"xp":{:.4},"shares":{:.4},"pnl":{:.4},"cpnl":{:.4},"reason":"{}"}}"#,
        ts_utc, now_ms, asset, epoch_id, strategy, side,
        entry_price, exit_price, shares, trade_pnl, cum_pnl, exit_reason,
    ).unwrap();
    s
}

// ── helpers ──

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn utc_timestamp_from_ms(ts_ms: u64) -> String {
    let t = ts_ms / 1000;
    let secs = t % 86400;
    let z = t / 86400 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let yr = if m <= 2 { y + 1 } else { y };
    let hh = secs / 3600;
    let mm = (secs % 3600) / 60;
    let ss = secs % 60;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", yr, m, d, hh, mm, ss)
}

// ── gate bit positions ──
pub mod gate {
    pub const PRICE_MIN: u16       = 1 << 0;
    pub const PRICE_MAX: u16       = 1 << 1;
    pub const SECONDS_LEFT: u16    = 1 << 2;
    pub const CONVICTION: u16      = 1 << 3;
    pub const OFI_OPPOSE: u16      = 1 << 4;
    pub const GROSS_EXPOSURE: u16  = 1 << 5;
    pub const EPOCH_LOSS_LIMIT: u16 = 1 << 6;
    pub const HAS_TRADED: u16      = 1 << 7;
    pub const CIRCUIT_BREAKER: u16 = 1 << 8;
    pub const GTC_PENDING: u16     = 1 << 9;
    pub const ENTRY_COOLDOWN: u16  = 1 << 10;
}
