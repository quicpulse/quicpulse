//! API Key Authentication via custom header

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use super::AuthError;

/// API Key authentication using a custom header
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    header_name: String,
    key: String,
}

impl ApiKeyAuth {
    /// Create new API Key auth with header name and key value
    pub fn new(header_name: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            header_name: header_name.into(),
            key: key.into(),
        }
    }

    /// Apply API Key header to request
    pub fn apply(&self, headers: &mut HeaderMap) -> Result<(), AuthError> {
        let name = HeaderName::try_from(&self.header_name)
            .map_err(|e| AuthError::InvalidHeader(format!("invalid header name: {}", e)))?;

        let value = HeaderValue::from_str(&self.key)
            .map_err(|e| AuthError::InvalidHeader(e.to_string()))?;

        headers.insert(name, value);
        Ok(())
    }
}
