//! AWS SSO / Identity Center support
//!
//! Provides credential retrieval from AWS SSO token cache for profiles
//! configured with `sso_start_url`, `sso_region`, `sso_account_id`, and `sso_role_name`.
//!
//! Prerequisites:
//! - User must have run `aws sso login --profile <profile_name>` to obtain SSO token
//! - Token must not be expired

use std::path::PathBuf;
use crate::errors::QuicpulseError;
use super::aws::AwsSigV4Config;
use super::aws_config::AwsProfile;

/// SSO token from cache
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SsoToken {
    access_token: String,
    expires_at: String,
    #[serde(default)]
    region: Option<String>,
    #[serde(default)]
    start_url: Option<String>,
}

/// SSO role credentials response
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SsoRoleCredentials {
    access_key_id: String,
    secret_access_key: String,
    session_token: String,
    expiration: i64,
}

/// Wrapper for the SSO API response
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SsoGetRoleCredentialsResponse {
    role_credentials: SsoRoleCredentials,
}

/// Resolve an SSO profile to actual credentials
pub async fn resolve_sso_profile(
    profile: &AwsProfile,
    region: String,
    service: String,
) -> Result<AwsSigV4Config, QuicpulseError> {
    // Get SSO configuration from profile
    let sso_start_url = profile.sso_start_url.as_ref()
        .ok_or_else(|| QuicpulseError::Config("SSO profile missing sso_start_url".to_string()))?;

    let sso_region = profile.sso_region.as_ref()
        .ok_or_else(|| QuicpulseError::Config("SSO profile missing sso_region".to_string()))?;

    let sso_account_id = profile.sso_account_id.as_ref()
        .ok_or_else(|| QuicpulseError::Config("SSO profile missing sso_account_id".to_string()))?;

    let sso_role_name = profile.sso_role_name.as_ref()
        .ok_or_else(|| QuicpulseError::Config("SSO profile missing sso_role_name".to_string()))?;

    // Load SSO token from cache
    let token = load_sso_token(sso_start_url)?;

    // Check token expiration
    if is_token_expired(&token.expires_at) {
        return Err(QuicpulseError::Auth(format!(
            "SSO token has expired. Run 'aws sso login --profile {}' to refresh.",
            profile.name
        )));
    }

    // Get role credentials from SSO
    let creds = get_role_credentials(
        &token.access_token,
        sso_account_id,
        sso_role_name,
        sso_region,
    ).await?;

    Ok(AwsSigV4Config {
        access_key_id: creds.access_key_id,
        secret_access_key: creds.secret_access_key,
        session_token: Some(creds.session_token),
        region,
        service,
    })
}

/// Load SSO token from cache directory
fn load_sso_token(start_url: &str) -> Result<SsoToken, QuicpulseError> {
    let cache_dir = get_sso_cache_dir()?;

    // SSO cache files are named by SHA1 hash of the start URL
    let cache_key = sha1_hex(start_url);
    let cache_file = cache_dir.join(format!("{}.json", cache_key));

    if !cache_file.exists() {
        // Try legacy cache format (without .json extension or different naming)
        // Also check for session-based tokens
        return Err(QuicpulseError::Auth(format!(
            "SSO token not found. Run 'aws sso login' to authenticate.\nExpected cache file: {:?}",
            cache_file
        )));
    }

    let content = std::fs::read_to_string(&cache_file)
        .map_err(|e| QuicpulseError::Config(format!("Failed to read SSO token cache: {}", e)))?;

    serde_json::from_str(&content)
        .map_err(|e| QuicpulseError::Config(format!("Failed to parse SSO token cache: {}", e)))
}

/// Get the SSO cache directory
fn get_sso_cache_dir() -> Result<PathBuf, QuicpulseError> {
    dirs::home_dir()
        .map(|h| h.join(".aws").join("sso").join("cache"))
        .ok_or_else(|| QuicpulseError::Config("Could not determine home directory".to_string()))
}

/// Compute SHA1 hash of a string (for SSO cache file naming)
fn sha1_hex(input: &str) -> String {
    use sha1::{Sha1, Digest};
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Check if an ISO 8601 timestamp is in the past
fn is_token_expired(expires_at: &str) -> bool {
    use chrono::{DateTime, Utc};

    // Parse the expiration time
    if let Ok(expiry) = DateTime::parse_from_rfc3339(expires_at) {
        return expiry < Utc::now();
    }

    // If we can't parse, assume expired for safety
    true
}

/// Call AWS SSO GetRoleCredentials API
async fn get_role_credentials(
    access_token: &str,
    account_id: &str,
    role_name: &str,
    sso_region: &str,
) -> Result<SsoRoleCredentials, QuicpulseError> {
    let client: reqwest::Client = reqwest::Client::new();

    // Build the SSO portal endpoint
    let endpoint = format!(
        "https://portal.sso.{}.amazonaws.com/federation/credentials?account_id={}&role_name={}",
        sso_region,
        urlencoding::encode(account_id),
        urlencoding::encode(role_name)
    );

    // Make the request
    let response: reqwest::Response = client
        .get(&endpoint)
        .header("x-amz-sso_bearer_token", access_token)
        .send()
        .await
        .map_err(|e| QuicpulseError::Config(format!("SSO GetRoleCredentials request failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body: String = response.text().await.unwrap_or_default();

        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(QuicpulseError::Auth(
                "SSO token is invalid or expired. Run 'aws sso login' to refresh.".to_string()
            ));
        }

        return Err(QuicpulseError::Auth(format!(
            "SSO GetRoleCredentials failed ({}): {}",
            status, body
        )));
    }

    // Parse response
    let response_body: SsoGetRoleCredentialsResponse = response.json().await
        .map_err(|e| QuicpulseError::Auth(format!("Failed to parse SSO response: {}", e)))?;

    Ok(response_body.role_credentials)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha1_hex() {
        // Known SHA1 hash
        let result = sha1_hex("https://my-sso-portal.awsapps.com/start");
        assert_eq!(result.len(), 40); // SHA1 produces 40 hex chars
    }

    #[test]
    fn test_is_token_expired() {
        // Past date
        assert!(is_token_expired("2020-01-01T00:00:00Z"));

        // Future date (assuming test runs before 2099)
        assert!(!is_token_expired("2099-01-01T00:00:00Z"));
    }

    #[test]
    fn test_parse_sso_token() {
        let json = r#"{
            "accessToken": "test-token",
            "expiresAt": "2099-01-01T12:00:00Z",
            "region": "us-east-1",
            "startUrl": "https://example.awsapps.com/start"
        }"#;

        let token: SsoToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.access_token, "test-token");
        assert_eq!(token.region, Some("us-east-1".to_string()));
    }
}
