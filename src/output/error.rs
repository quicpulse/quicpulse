//! Output stream error types
//!
//! Provides proper error handling for stream operations instead of
//! relying on Option or panics.

use thiserror::Error;

/// Errors that can occur during output stream operations
#[derive(Debug, Error)]
pub enum StreamError {
    /// Encoding/decoding error
    #[error("encoding error: {0}")]
    Encoding(String),

    /// Syntax highlighting error
    #[error("highlight error: {0}")]
    Highlight(String),

    /// I/O error during stream operations
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Data too large for processing
    #[error("data too large: {size} bytes exceeds limit of {limit} bytes")]
    TooLarge {
        size: usize,
        limit: usize,
    },

    /// Invalid data format
    #[error("invalid format: {0}")]
    InvalidFormat(String),

    /// Stream exhausted
    #[error("stream exhausted")]
    Exhausted,
}

impl StreamError {
    /// Create an encoding error
    pub fn encoding(msg: impl Into<String>) -> Self {
        StreamError::Encoding(msg.into())
    }

    /// Create a highlight error
    pub fn highlight(msg: impl Into<String>) -> Self {
        StreamError::Highlight(msg.into())
    }

    /// Create a data too large error
    pub fn too_large(size: usize, limit: usize) -> Self {
        StreamError::TooLarge { size, limit }
    }

    /// Create an invalid format error
    pub fn invalid_format(msg: impl Into<String>) -> Self {
        StreamError::InvalidFormat(msg.into())
    }
}

/// Result type for stream operations
pub type StreamResult<T> = Result<T, StreamError>;
