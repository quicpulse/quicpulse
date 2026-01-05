//! Bearer Token Authentication (RFC 6750)

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

use super::AuthError;

/// Bearer token authentication
#[derive(Debug, Clone)]
pub struct BearerAuth {
    token: String,
}

impl BearerAuth {
    /// Create new Bearer auth with token
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }

    /// Apply Bearer auth header to request
    pub fn apply(&self, headers: &mut HeaderMap) -> Result<(), AuthError> {
        let header_value = format!("Bearer {}", self.token);

        let value = HeaderValue::from_str(&header_value)
            .map_err(|e| AuthError::InvalidHeader(e.to_string()))?;

        headers.insert(AUTHORIZATION, value);
        Ok(())
    }
}
