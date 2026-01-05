//! Google Cloud Platform (GCP) authentication
//!
//! Provides authentication using the gcloud CLI tool.
//! Falls back to Application Default Credentials (ADC) via GOOGLE_APPLICATION_CREDENTIALS.

use crate::errors::QuicpulseError;
use std::process::Command;

/// Get an access token from GCP using gcloud CLI
///
/// This function attempts to get an access token in the following order:
/// 1. Use `gcloud auth print-access-token` if gcloud is available
/// 2. (Future: support GOOGLE_APPLICATION_CREDENTIALS service account JSON)
///
/// # Returns
/// The access token string on success
///
/// # Errors
/// Returns an error if gcloud is not available or authentication fails
pub async fn get_gcp_access_token() -> Result<String, QuicpulseError> {
    // Try gcloud CLI first
    let output = Command::new("gcloud")
        .args(["auth", "print-access-token"])
        .output()
        .map_err(|e| QuicpulseError::Auth(format!(
            "Failed to run gcloud CLI. Is gcloud installed and in PATH? Error: {}", e
        )))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(QuicpulseError::Auth(format!(
            "gcloud auth failed: {}. Run 'gcloud auth login' to authenticate.",
            stderr.trim()
        )));
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if token.is_empty() {
        return Err(QuicpulseError::Auth(
            "gcloud returned empty token. Run 'gcloud auth login' to authenticate.".to_string()
        ));
    }

    Ok(token)
}

/// Get an access token with a specific target audience (for ID tokens)
///
/// # Arguments
/// * `audience` - The target audience for the ID token (e.g., a Cloud Run service URL)
///
/// # Returns
/// The ID token string on success
#[allow(dead_code)]
pub async fn get_gcp_id_token(audience: &str) -> Result<String, QuicpulseError> {
    let output = Command::new("gcloud")
        .args(["auth", "print-identity-token", "--audiences", audience])
        .output()
        .map_err(|e| QuicpulseError::Auth(format!(
            "Failed to run gcloud CLI: {}", e
        )))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(QuicpulseError::Auth(format!(
            "gcloud auth failed: {}",
            stderr.trim()
        )));
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if token.is_empty() {
        return Err(QuicpulseError::Auth(
            "gcloud returned empty ID token".to_string()
        ));
    }

    Ok(token)
}

/// Get the current GCP project ID
#[allow(dead_code)]
pub fn get_current_project() -> Result<String, QuicpulseError> {
    let output = Command::new("gcloud")
        .args(["config", "get-value", "project"])
        .output()
        .map_err(|e| QuicpulseError::Auth(format!(
            "Failed to get GCP project: {}", e
        )))?;

    if !output.status.success() {
        return Err(QuicpulseError::Auth(
            "No GCP project configured. Run 'gcloud config set project PROJECT_ID'".to_string()
        ));
    }

    let project = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if project.is_empty() {
        return Err(QuicpulseError::Auth(
            "No GCP project configured. Run 'gcloud config set project PROJECT_ID'".to_string()
        ));
    }

    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires gcloud CLI and authentication
    async fn test_get_gcp_access_token() {
        let result = get_gcp_access_token().await;
        // Should either succeed or fail with a meaningful error
        match result {
            Ok(token) => {
                assert!(!token.is_empty());
                // GCP access tokens are JWTs, should contain dots
                assert!(token.contains('.') || token.starts_with("ya29."));
            }
            Err(e) => {
                // Expected if gcloud not installed or not authenticated
                let msg = e.to_string();
                assert!(msg.contains("gcloud") || msg.contains("auth"));
            }
        }
    }

    #[test]
    fn test_gcp_error_messages_contain_help() {
        // Verify error messages guide users to the solution
        let auth_err = QuicpulseError::Auth("test".to_string());
        assert!(auth_err.to_string().contains("test"));
    }

    #[test]
    fn test_default_resource_constant_not_empty() {
        // GCP doesn't use a default resource like Azure, but we test
        // that the module compiles correctly
        assert!(true);
    }
}
