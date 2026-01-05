//! OAuth 2.0 Client Credentials Flow
//!
//! Provides support for obtaining access tokens using the OAuth 2.0
//! client credentials grant type.

use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use crate::errors::QuicpulseError;

/// OAuth 2.0 configuration
#[derive(Debug, Clone)]
pub struct OAuth2Config {
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Token endpoint URL
    pub token_url: String,
    /// Requested scopes
    pub scopes: Vec<String>,
}

impl OAuth2Config {
    /// Create from credentials string (client_id:client_secret)
    pub fn from_credentials(
        credentials: &str,
        token_url: String,
        scopes: Vec<String>,
    ) -> Result<Self, QuicpulseError> {
        let parts: Vec<&str> = credentials.splitn(2, ':').collect();

        if parts.len() != 2 {
            return Err(QuicpulseError::Argument(
                "OAuth2 credentials must be in format: CLIENT_ID:CLIENT_SECRET".to_string()
            ));
        }

        Ok(Self {
            client_id: parts[0].to_string(),
            client_secret: parts[1].to_string(),
            token_url,
            scopes,
        })
    }

    /// Try to load from environment variables
    pub fn from_env(token_url: String, scopes: Vec<String>) -> Result<Self, QuicpulseError> {
        let client_id = std::env::var("OAUTH_CLIENT_ID")
            .or_else(|_| std::env::var("CLIENT_ID"))
            .map_err(|_| QuicpulseError::Argument(
                "OAUTH_CLIENT_ID environment variable not set".to_string()
            ))?;

        let client_secret = std::env::var("OAUTH_CLIENT_SECRET")
            .or_else(|_| std::env::var("CLIENT_SECRET"))
            .map_err(|_| QuicpulseError::Argument(
                "OAUTH_CLIENT_SECRET environment variable not set".to_string()
            ))?;

        let token_url = if token_url.is_empty() {
            std::env::var("OAUTH_TOKEN_URL")
                .map_err(|_| QuicpulseError::Argument(
                    "OAuth token URL not provided and OAUTH_TOKEN_URL not set".to_string()
                ))?
        } else {
            token_url
        };

        Ok(Self {
            client_id,
            client_secret,
            token_url,
            scopes,
        })
    }
}

/// OAuth 2.0 token response
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    /// The access token
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Token expiration in seconds (optional)
    pub expires_in: Option<u64>,
    /// Refresh token (optional)
    pub refresh_token: Option<String>,
    /// Granted scope (optional)
    pub scope: Option<String>,
}

/// OAuth 2.0 error response
#[derive(Debug, Clone, Deserialize)]
pub struct TokenError {
    pub error: String,
    pub error_description: Option<String>,
    pub error_uri: Option<String>,
}

/// Token request body
#[derive(Debug, Serialize)]
struct TokenRequest {
    grant_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
}

/// Cached token with expiration tracking
#[derive(Debug, Clone)]
pub struct CachedToken {
    pub access_token: String,
    pub token_type: String,
    pub obtained_at: Instant,
    pub expires_in: Option<Duration>,
    /// Refresh token for automatic renewal
    pub refresh_token: Option<String>,
}

impl CachedToken {
    /// Check if the token is still valid (with 30 second buffer)
    pub fn is_valid(&self) -> bool {
        if let Some(expires_in) = self.expires_in {
            let elapsed = self.obtained_at.elapsed();
            // Consider expired 30 seconds early to avoid edge cases
            elapsed < expires_in.saturating_sub(Duration::from_secs(30))
        } else {
            // If no expiration, assume valid
            true
        }
    }

    /// Check if the token needs refresh (expires within 5 minutes)
    pub fn needs_refresh(&self) -> bool {
        if let Some(expires_in) = self.expires_in {
            let elapsed = self.obtained_at.elapsed();
            // Refresh if less than 5 minutes remaining
            elapsed > expires_in.saturating_sub(Duration::from_secs(300))
        } else {
            false
        }
    }

    /// Check if token can be refreshed
    pub fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
    }

    /// Get the Authorization header value
    pub fn authorization_header(&self) -> String {
        format!("{} {}", self.token_type, self.access_token)
    }
}

pub async fn obtain_token(config: &OAuth2Config) -> Result<CachedToken, QuicpulseError> {
    let client = &*OAUTH_CLIENT;

    // Build scope string
    let scope = if config.scopes.is_empty() {
        None
    } else {
        Some(config.scopes.join(" "))
    };

    // Build form data
    let mut form_params = vec![
        ("grant_type", "client_credentials".to_string()),
    ];
    if let Some(ref scope) = scope {
        form_params.push(("scope", scope.clone()));
    }

    let obtained_at = Instant::now();

    // Send token request with Basic auth
    let response = client
        .post(&config.token_url)
        .basic_auth(&config.client_id, Some(&config.client_secret))
        .form(&form_params)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| QuicpulseError::Request(e))?;

    let status = response.status();
    let body = response.text().await
        .map_err(|e| QuicpulseError::Request(e))?;

    if !status.is_success() {
        // Try to parse error response
        if let Ok(error) = serde_json::from_str::<TokenError>(&body) {
            let msg = match error.error_description {
                Some(desc) => format!("{}: {}", error.error, desc),
                None => error.error,
            };
            return Err(QuicpulseError::Auth(format!("OAuth2 token request failed: {}", msg)));
        }
        return Err(QuicpulseError::Auth(format!(
            "OAuth2 token request failed with status {}: {}",
            status, body
        )));
    }

    // Parse success response
    let token: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| QuicpulseError::Auth(format!("Failed to parse token response: {}", e)))?;

    Ok(CachedToken {
        access_token: token.access_token,
        token_type: if token.token_type.is_empty() { "Bearer".to_string() } else { token.token_type },
        obtained_at,
        expires_in: token.expires_in.map(Duration::from_secs),
        refresh_token: token.refresh_token,
    })
}

/// Refresh an access token using a refresh token
pub async fn refresh_token(
    token_url: &str,
    client_id: &str,
    client_secret: Option<&str>,
    refresh_token: &str,
) -> Result<CachedToken, QuicpulseError> {
    let client = &*OAUTH_CLIENT;

    let mut form_params = vec![
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh_token.to_string()),
        ("client_id", client_id.to_string()),
    ];

    if let Some(secret) = client_secret {
        form_params.push(("client_secret", secret.to_string()));
    }

    let obtained_at = Instant::now();

    let response = client
        .post(token_url)
        .form(&form_params)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| QuicpulseError::Request(e))?;

    let status = response.status();
    let body = response.text().await
        .map_err(|e| QuicpulseError::Request(e))?;

    if !status.is_success() {
        if let Ok(error) = serde_json::from_str::<TokenError>(&body) {
            let msg = match error.error_description {
                Some(desc) => format!("{}: {}", error.error, desc),
                None => error.error,
            };
            return Err(QuicpulseError::Auth(format!("Token refresh failed: {}", msg)));
        }
        return Err(QuicpulseError::Auth(format!(
            "Token refresh failed with status {}: {}",
            status, body
        )));
    }

    let token: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| QuicpulseError::Auth(format!("Failed to parse refresh token response: {}", e)))?;

    Ok(CachedToken {
        access_token: token.access_token,
        token_type: if token.token_type.is_empty() { "Bearer".to_string() } else { token.token_type },
        obtained_at,
        expires_in: token.expires_in.map(Duration::from_secs),
        // Use new refresh token if provided, otherwise keep the old one
        refresh_token: token.refresh_token.or(Some(refresh_token.to_string())),
    })
}

use dashmap::{DashMap, DashSet};
use once_cell::sync::Lazy;
use std::sync::Arc;

static OAUTH_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create OAuth HTTP client")
});

/// Token cache: stores cached tokens (DashMap for non-blocking concurrent access)
static TOKEN_CACHE: Lazy<DashMap<String, CachedToken>> = Lazy::new(DashMap::new);

/// In-flight requests: prevents thundering herd by tracking which keys are being fetched
/// Uses DashSet for lock-free concurrent access
static IN_FLIGHT: Lazy<DashSet<String>> = Lazy::new(DashSet::new);

fn generate_cache_key(config: &OAuth2Config) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(config.client_secret.as_bytes());
    let secret_hash = hex::encode(hasher.finalize());

    format!(
        "{}:{}:{}:{}",
        config.token_url,
        config.client_id,
        secret_hash,
        config.scopes.join(",")
    )
}

/// RAII guard to ensure IN_FLIGHT cleanup on drop (handles cancellation/panic)
struct InFlightGuard {
    key: String,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        IN_FLIGHT.remove(&self.key);
    }
}

/// Get a cached token or obtain a new one
/// Uses DashMap/DashSet for lock-free thundering herd prevention
pub async fn get_token(config: &OAuth2Config) -> Result<CachedToken, QuicpulseError> {
    let cache_key = generate_cache_key(config);

    // Check cache first (non-blocking with DashMap)
    if let Some(cached) = TOKEN_CACHE.get(&cache_key) {
        if cached.is_valid() {
            return Ok(cached.clone());
        }
    }

    // Check if another task is already fetching this token
    // If so, wait and retry from cache
    loop {
        // Try to mark as in-flight (DashSet insert returns false if already present)
        if IN_FLIGHT.insert(cache_key.clone()) {
            // We successfully marked it - we're responsible for fetching
            break;
        }

        // Another task is fetching - wait a bit and check cache again
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Check if token is now available
        if let Some(cached) = TOKEN_CACHE.get(&cache_key) {
            if cached.is_valid() {
                return Ok(cached.clone());
            }
        }
    }

    // RAII guard ensures cleanup even if future is cancelled or panics
    let _guard = InFlightGuard { key: cache_key.clone() };

    // We're responsible for fetching the token
    let result = obtain_token(config).await;

    // Cache on success (guard will remove from IN_FLIGHT on drop)
    match result {
        Ok(token) => {
            TOKEN_CACHE.insert(cache_key, token.clone());
            Ok(token)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_credentials() {
        let config = OAuth2Config::from_credentials(
            "client_id:client_secret",
            "https://auth.example.com/token".to_string(),
            vec!["read".to_string(), "write".to_string()],
        ).unwrap();

        assert_eq!(config.client_id, "client_id");
        assert_eq!(config.client_secret, "client_secret");
    }

    #[test]
    fn test_cached_token_validity() {
        let token = CachedToken {
            access_token: "test".to_string(),
            token_type: "Bearer".to_string(),
            obtained_at: Instant::now(),
            expires_in: Some(Duration::from_secs(3600)),
            refresh_token: None,
        };

        assert!(token.is_valid());
    }

    #[test]
    fn test_cached_token_expired() {
        let token = CachedToken {
            access_token: "test".to_string(),
            token_type: "Bearer".to_string(),
            obtained_at: Instant::now() - Duration::from_secs(3700),
            expires_in: Some(Duration::from_secs(3600)),
            refresh_token: None,
        };

        assert!(!token.is_valid());
    }

    #[test]
    fn test_authorization_header() {
        let token = CachedToken {
            access_token: "mytoken".to_string(),
            token_type: "Bearer".to_string(),
            obtained_at: Instant::now(),
            expires_in: None,
            refresh_token: None,
        };

        assert_eq!(token.authorization_header(), "Bearer mytoken");
    }

    #[test]
    fn test_token_needs_refresh() {
        // Token that expires in 10 minutes - doesn't need refresh yet
        let token = CachedToken {
            access_token: "test".to_string(),
            token_type: "Bearer".to_string(),
            obtained_at: Instant::now(),
            expires_in: Some(Duration::from_secs(600)),
            refresh_token: Some("refresh123".to_string()),
        };
        assert!(!token.needs_refresh());
        assert!(token.can_refresh());

        // Token that expires in 2 minutes - needs refresh
        let token2 = CachedToken {
            access_token: "test".to_string(),
            token_type: "Bearer".to_string(),
            obtained_at: Instant::now() - Duration::from_secs(480),
            expires_in: Some(Duration::from_secs(600)),
            refresh_token: Some("refresh123".to_string()),
        };
        assert!(token2.needs_refresh());
    }
}
