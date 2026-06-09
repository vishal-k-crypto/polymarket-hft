//! Signal commands and bounded lock-free channels.
//!
//! Strategies produce `SignalCmd` messages and push them into a bounded
//! `flume` channel. The executor consumes them on the dedicated OS thread.

use flume::{bounded, Receiver, Sender};
use std::fmt;

use crate::types::{OrderSide, TokenSlot};

// ═══════════════════════════════════════════════════════════════════════════
// Signal command
// ═══════════════════════════════════════════════════════════════════════════

/// A command sent from a strategy to the executor.
///
/// All fields are stack-allocated. No String, no Vec, no Box.
#[derive(Debug, Clone, Copy)]
pub struct SignalCmd {
    /// BUY or SELL or CANCEL_ALL
    pub side: SignalSide,
    /// Target limit price (from strategy fair value calculation)
    pub price: f64,
    /// Size in USD to trade
    pub size_usd: f64,
    /// Epoch close timestamp (ms)
    pub epoch_close_ms: u64,
    /// Asset identifier (first 4 bytes, e.g. `"btc\0"`)
    pub asset: [u8; 4],
    /// Which token slot: UP or DOWN
    pub target_token: TokenSlot,
    /// Millisecond timestamp when this signal was generated
    pub generated_at_ms: u64,
}

impl SignalCmd {
    pub fn new(
        side: SignalSide,
        price: f64,
        size_usd: f64,
        asset: &str,
        target_token: TokenSlot,
    ) -> Self {
        let mut asset_bytes = [0u8; 4];
        let asset_lower = asset.to_lowercase();
        let bytes = asset_lower.as_bytes();
        let len = bytes.len().min(4);
        asset_bytes[..len].copy_from_slice(&bytes[..len]);

        Self {
            side,
            price,
            size_usd,
            epoch_close_ms: 0,
            asset: asset_bytes,
            target_token,
            generated_at_ms: 0,
        }
    }

    pub fn asset_str(&self) -> &str {
        let end = self.asset.iter().position(|&b| b == 0).unwrap_or(4);
        std::str::from_utf8(&self.asset[..end]).unwrap_or("")
    }

    pub fn is_cancel_all(&self) -> bool {
        matches!(self.side, SignalSide::CancelAll)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalSide {
    Buy,
    Sell,
    CancelAll,
}

impl SignalSide {
    pub fn as_order_side(self) -> Option<OrderSide> {
        match self {
            SignalSide::Buy => Some(OrderSide::Buy),
            SignalSide::Sell => Some(OrderSide::Sell),
            SignalSide::CancelAll => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SignalSide::Buy => "BUY",
            SignalSide::Sell => "SELL",
            SignalSide::CancelAll => "CANCEL_ALL",
        }
    }
}

impl fmt::Display for SignalSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Log event (sent to async logger)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct LogEvent {
    pub level: LogLevel,
    pub module: &'static str,
    pub message: String,
    pub fields: Vec<(String, String)>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

// ═══════════════════════════════════════════════════════════════════════════
// Bounded channel constructors
// ═══════════════════════════════════════════════════════════════════════════

/// Signal channel: strategies → executor.
/// Bounded at 256 to prevent memory growth during market spikes.
const SIGNAL_CHANNEL_CAP: usize = 256;

/// Log channel: hot path → async logger.
/// Bounded at 1024.
const LOG_CHANNEL_CAP: usize = 1024;

pub fn signal_channel() -> (Sender<SignalCmd>, Receiver<SignalCmd>) {
    bounded(SIGNAL_CHANNEL_CAP)
}

pub fn log_channel() -> (Sender<LogEvent>, Receiver<LogEvent>) {
    bounded(LOG_CHANNEL_CAP)
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_cmd_asset_str() {
        let cmd = SignalCmd::new(SignalSide::Buy, 0.55, 5.0, "btc", TokenSlot::Up);
        assert_eq!(cmd.asset_str(), "btc");
        assert!(!cmd.is_cancel_all());
    }

    #[test]
    fn test_signal_cmd_cancel_all() {
        let cmd = SignalCmd::new(SignalSide::CancelAll, 0.0, 0.0, "eth", TokenSlot::Up);
        assert!(cmd.is_cancel_all());
        assert!(cmd.side.as_order_side().is_none());
    }

    #[test]
    fn test_signal_channel_send_recv() {
        let (tx, rx) = signal_channel();
        tx.send(SignalCmd::new(SignalSide::Buy, 0.55, 5.0, "btc", TokenSlot::Up))
            .unwrap();
        let cmd = rx.recv().unwrap();
        assert_eq!(cmd.asset_str(), "btc");
        assert_eq!(cmd.price, 0.55);
    }

    #[test]
    fn test_log_channel_send_recv() {
        let (tx, rx) = log_channel();
        tx.send(LogEvent {
            level: LogLevel::Info,
            module: "executor",
            message: "test".into(),
            fields: vec![],
            timestamp_ms: 0,
        })
        .unwrap();
        let event = rx.recv().unwrap();
        assert_eq!(event.module, "executor");
    }

    #[test]
    fn test_signal_side_as_order_side() {
        assert_eq!(SignalSide::Buy.as_order_side(), Some(OrderSide::Buy));
        assert_eq!(SignalSide::Sell.as_order_side(), Some(OrderSide::Sell));
        assert_eq!(SignalSide::CancelAll.as_order_side(), None);
    }
}
