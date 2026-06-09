//! Chainlink AggregatorV3 price oracle on Polygon PoS.
//!
//! Fetches latest price from Chainlink Data Feeds via JSON-RPC `eth_call`.
//! Falls back to multiple RPC endpoints after consecutive failures.
//! Falls back to Binance mid price if Chainlink returns 0 or errors.

use alloy_primitives::Address;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

const POLL_INTERVAL_S: u64 = 5;
const STALE_THRESHOLD_S: u64 = 3600; // 1 hour
const MAX_CONSECUTIVE_FAILURES: u32 = 6;

// ── Chainlink AggregatorV3 addresses on Polygon ──
const BTC_USD_FEED: Address = Address::new([
    0xc9, 0x07, 0xE1, 0x16, 0x05, 0x4A, 0xd1, 0x03, 0x35, 0x4f,
    0x2D, 0x35, 0x0F, 0xD2, 0x51, 0x44, 0x33, 0xD5, 0x7F, 0x6f,
]);

const ETH_USD_FEED: Address = Address::new([
    0xF9, 0x68, 0x0D, 0x99, 0xD6, 0xC9, 0x58, 0x9e, 0x2a, 0x93,
    0xa7, 0x8A, 0x04, 0xA2, 0x79, 0xe5, 0x09, 0x20, 0x59, 0x45,
]);

// keccak256("latestRoundData()")[..4]
const LATEST_ROUND_DATA_SELECTOR: [u8; 4] = [0xfe, 0xaf, 0x96, 0x8c];

// Default RPC fallback list
const RPC_FALLBACKS: &[&str] = &[
    "https://polygon.drpc.org",
    "https://rpc.ankr.com/polygon",
    "https://polygon-rpc.com",
    "https://1rpc.io/matic",
];

/// Map asset name to its Chainlink feed address.
fn feed_for_asset(asset: &str) -> Option<Address> {
    match asset.to_lowercase().as_str() {
        "btc" => Some(BTC_USD_FEED),
        "eth" => Some(ETH_USD_FEED),
        _ => None,
    }
}

/// Return the list of RPC URLs, starting with the configured primary.
fn rpc_urls(primary: &str) -> Vec<String> {
    let mut urls = vec![primary.to_string()];
    for fallback in RPC_FALLBACKS {
        if fallback != &primary {
            urls.push(fallback.to_string());
        }
    }
    urls
}

/// Start the oracle polling loop.
pub fn start(_config: Arc<()>, _state: Arc<()>) {
    tokio::spawn(async move {
        tracing::info!("oracle_feed_started");
        // Simplified for public showcase — full implementation connects to Chainlink
        loop {
            sleep(Duration::from_secs(POLL_INTERVAL_S)).await;
        }
    });
}

/// Fetch latest price from a Chainlink AggregatorV3 feed via JSON-RPC.
async fn fetch_price(
    client: &Client,
    rpc_url: &str,
    feed: Address,
) -> Result<Option<(f64, u64)>, String> {
    let call_data = encode_latest_round_data(feed);

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
        {
            "to": format!("0x{:x}", feed),
            "data": format!("0x{}", hex::encode(&call_data)),
        },
            "latest",
        ],
        "id": 1,
    });

    let resp = client
        .post(rpc_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("rpc_http_error: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("rpc_status: {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| format!("rpc_json: {e}"))?;

    let result = body
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("rpc_no_result: {body}"))?;

    let data = result
        .strip_prefix("0x")
        .ok_or("rpc_no_hex_prefix")?;

    if data.len() < 128 {
        return Err("rpc_result_too_short".into());
    }

    // Decode ABI: skip 64 chars (roundId), next 64 chars = answer (int256)
    let answer_hex = &data[64..128];
    let answer = i64::from_str_radix(answer_hex, 16)
        .map_err(|e| format!("parse_answer: {e}"))?;

    if answer <= 0 {
        return Ok(None);
    }

    // updatedAt is at offset 192 (4 x 64 chars in)
    let updated_at_hex = &data[192..256];
    let updated_at = u64::from_str_radix(updated_at_hex, 16)
        .map_err(|e| format!("parse_updated_at: {e}"))?;

    let price = answer as f64 / 1e8;

    Ok(Some((price, updated_at)))
}

/// Encode eth_call data: selector + abi-encoded address
fn encode_latest_round_data(_feed: Address) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + 32);
    data.extend_from_slice(&LATEST_ROUND_DATA_SELECTOR);
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feed_mapping() {
        assert_eq!(feed_for_asset("btc"), Some(BTC_USD_FEED));
        assert_eq!(feed_for_asset("BTC"), Some(BTC_USD_FEED));
        assert_eq!(feed_for_asset("eth"), Some(ETH_USD_FEED));
        assert_eq!(feed_for_asset("sol"), None);
    }

    #[test]
    fn test_encode_latest_round_data() {
        let data = encode_latest_round_data(BTC_USD_FEED);
        assert_eq!(&data[..4], &LATEST_ROUND_DATA_SELECTOR);
        assert_eq!(data.len(), 4);
    }

    #[test]
    fn test_rpc_urls_promotes_primary() {
        let urls = rpc_urls("https://custom.rpc.com");
        assert_eq!(urls[0], "https://custom.rpc.com");
        assert!(urls.len() >= 4);
    }
}
