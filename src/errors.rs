//! Error types for QuicPulse

use thiserror::Error;

/// Main error type for QuicPulse
#[derive(Error, Debug)]
pub enum QuicpulseError {
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Nested JSON syntax error at position {position}: {message}")]
    NestedJsonSyntax {
        position: usize,
        message: String,
    },

    #[error("Session error: {0}")]
    Session(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Timeout after {0:.1} seconds")]
    Timeout(f64),

    #[error("Too many redirects (max {0})")]
    TooManyRedirects(u32),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("SSL error: {0}")]
    Ssl(String),

    #[error("Invalid argument: {0}")]
    Argument(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Content-Range error: {0}")]
    ContentRange(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Script error: {0}")]
    Script(String),

    #[error("gRPC error: {0}")]
    Grpc(String),

    #[error("Pipeline error: {0}")]
    Pipeline(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),
}

// Implement From for rquickjs errors (only when javascript feature is enabled)
#[cfg(feature = "javascript")]
impl From<rquickjs::Error> for QuicpulseError {
    fn from(err: rquickjs::Error) -> Self {
        QuicpulseError::Script(format!("JavaScript error: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, QuicpulseError>;
