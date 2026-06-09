#![allow(dead_code)]

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("missing required env var: {0}")]
    MissingEnvVar(String),

    #[error("invalid env value for {name}: {value}")]
    InvalidEnvValue { name: String, value: String },

    #[error("config error: {0}")]
    Config(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("channel send error")]
    ChannelSend,

    #[error("channel receive error")]
    ChannelRecv,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("order rejected: {0}")]
    OrderRejected(String),

    #[error("market not found: {0}")]
    MarketNotFound(String),

    #[error("internal: {0}")]
    Internal(String),
}

pub type BotResult<T> = Result<T, BotError>;
