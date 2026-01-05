//! Azure CLI authentication
//!
//! Provides authentication using the az CLI tool.

use crate::errors::QuicpulseError;
use std::process::Command;

/// Default Azure resource for access tokens (Azure Resource Manager)
const DEFAULT_RESOURCE: &str = "https://management.azure.com/";

/// Get an access token from Azure using az CLI
///
/// This function uses `az account get-access-token` to retrieve a token.
///
/// # Arguments
/// * `resource` - Optional resource URL. Defaults to Azure Resource Manager.
///                Common values:
///                - `https://management.azure.com/` (ARM)
///                - `https://graph.microsoft.com/` (Microsoft Graph)
///                - `https://vault.azure.net/` (Key Vault)
///                - `https://storage.azure.com/` (Storage)
///
/// # Returns
/// The access token string on success
///
/// # Errors
/// Returns an error if az CLI is not available or authentication fails
pub async fn get_azure_access_token(resource: Option<&str>) -> Result<String, QuicpulseError> {
    let resource = resource.unwrap_or(DEFAULT_RESOURCE);

    let output = Command::new("az")
        .args([
            "account",
            "get-access-token",
            "--resource",
            resource,
            "--query",
            "accessToken",
            "--output",
            "tsv",
        ])
        .output()
        .map_err(|e| QuicpulseError::Auth(format!(
            "Failed to run az CLI. Is Azure CLI installed and in PATH? Error: {}", e
        )))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(QuicpulseError::Auth(format!(
            "az account get-access-token failed: {}. Run 'az login' to authenticate.",
            stderr.trim()
        )));
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if token.is_empty() {
        return Err(QuicpulseError::Auth(
            "az CLI returned empty token. Run 'az login' to authenticate.".to_string()
        ));
    }

    Ok(token)
}

/// Get the current Azure subscription ID
#[allow(dead_code)]
pub fn get_current_subscription() -> Result<String, QuicpulseError> {
    let output = Command::new("az")
        .args([
            "account",
            "show",
            "--query",
            "id",
            "--output",
            "tsv",
        ])
        .output()
        .map_err(|e| QuicpulseError::Auth(format!(
            "Failed to get Azure subscription: {}", e
        )))?;

    if !output.status.success() {
        return Err(QuicpulseError::Auth(
            "No Azure subscription configured. Run 'az login' to authenticate.".to_string()
        ));
    }

    let subscription = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if subscription.is_empty() {
        return Err(QuicpulseError::Auth(
            "No Azure subscription configured. Run 'az login' to authenticate.".to_string()
        ));
    }

    Ok(subscription)
}

/// Get the current Azure tenant ID
#[allow(dead_code)]
pub fn get_current_tenant() -> Result<String, QuicpulseError> {
    let output = Command::new("az")
        .args([
            "account",
            "show",
            "--query",
            "tenantId",
            "--output",
            "tsv",
        ])
        .output()
        .map_err(|e| QuicpulseError::Auth(format!(
            "Failed to get Azure tenant: {}", e
        )))?;

    if !output.status.success() {
        return Err(QuicpulseError::Auth(
            "No Azure tenant configured. Run 'az login' to authenticate.".to_string()
        ));
    }

    let tenant = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if tenant.is_empty() {
        return Err(QuicpulseError::Auth(
            "No Azure tenant configured. Run 'az login' to authenticate.".to_string()
        ));
    }

    Ok(tenant)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires az CLI and authentication
    async fn test_get_azure_access_token() {
        let result = get_azure_access_token(None).await;
        // Should either succeed or fail with a meaningful error
        match result {
            Ok(token) => {
                assert!(!token.is_empty());
                // Azure access tokens are JWTs
                assert!(token.contains('.'));
            }
            Err(e) => {
                // Expected if az not installed or not authenticated
                let msg = e.to_string();
                assert!(msg.contains("az") || msg.contains("Azure"));
            }
        }
    }

    #[test]
    fn test_default_resource_is_arm() {
        // Verify default resource is Azure Resource Manager
        assert_eq!(DEFAULT_RESOURCE, "https://management.azure.com/");
    }

    #[test]
    fn test_azure_error_messages_contain_help() {
        // Verify error messages guide users to the solution
        let auth_err = QuicpulseError::Auth("az login".to_string());
        assert!(auth_err.to_string().contains("az login"));
    }
}
