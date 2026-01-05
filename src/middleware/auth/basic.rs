//! HTTP Basic Authentication (RFC 7617)

use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

use super::AuthError;

/// HTTP Basic Authentication credentials
#[derive(Debug, Clone)]
pub struct BasicAuth {
    username: String,
    password: String,
}

impl BasicAuth {
    /// Create new Basic auth with username and password
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Apply Basic auth header to request
    pub fn apply(&self, headers: &mut HeaderMap) -> Result<(), AuthError> {
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        let header_value = format!("Basic {}", encoded);

        let value = HeaderValue::from_str(&header_value)
            .map_err(|e| AuthError::InvalidHeader(e.to_string()))?;

        headers.insert(AUTHORIZATION, value);
        Ok(())
    }
}
