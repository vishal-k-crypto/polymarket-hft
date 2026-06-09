#![allow(dead_code)]

use alloy_primitives::{Address, U256};
use alloy_sol_types::sol;

sol! {
    /// Polymarket CLOB V2 signed order struct (11-field EIP-712).
    /// metadata and builder are included in the signature hash.
    #[derive(Debug, Default)]
    struct Order {
        uint256 salt;
        address maker;
        address signer;
        uint256 tokenId;
        uint256 makerAmount;
        uint256 takerAmount;
        uint8 side;             // 0 = BUY, 1 = SELL
        uint8 signatureType;     // 0 = EOA, 3 = POLY_1271
        uint256 timestamp;       // ms since epoch
        bytes32 metadata;        // included in V2 EIP-712 signed struct
        bytes32 builder;         // included in V2 EIP-712 signed struct
    }
}

/// BUY = 0, SELL = 1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OrderSide {
    Buy = 0,
    Sell = 1,
}

impl OrderSide {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_str(self) -> &'static str {
        match self {
            OrderSide::Buy => "BUY",
            OrderSide::Sell => "SELL",
        }
    }
}

/// UP or DOWN token slot on a market.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TokenSlot {
    #[default]
    Up = 0,
    Down = 1,
}

impl TokenSlot {
    pub fn flip(self) -> Self {
        match self {
            TokenSlot::Up => TokenSlot::Down,
            TokenSlot::Down => TokenSlot::Up,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TokenSlot::Up => "UP",
            TokenSlot::Down => "DOWN",
        }
    }
}

/// Simplified representation of Polymarket-specific order parameters.
/// This is what the strategy produces and what the executor consumes.
#[derive(Debug, Clone, Copy)]
pub struct OrderParams {
    pub token_id: U256,
    pub side: OrderSide,
    pub size_usd: f64,
    pub price: f64,
    pub worst_price: f64,
    pub maker: Address,
    pub signer: Address,
    pub signature_type: u8,
    pub fee_rate_bps: u64,
}
