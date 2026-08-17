//! Unit tests for OAuth 2.0 Client Credentials, PKCE challenge generation, and flow configs

use quicpulse::auth::oauth2::OAuth2Config;
use quicpulse::auth::oauth2_flows::{AuthCodeConfig, DeviceFlowConfig, PkceChallenge};

#[test]
fn test_oauth2_config_from_credentials() {
    let creds = "my_client_id:my_client_secret_xyz";
    let config = OAuth2Config::from_credentials(
        creds,
        "https://auth.example.com/oauth/token".to_string(),
        vec!["read".to_string(), "write".to_string()],
    )
    .unwrap();

    assert_eq!(config.client_id, "my_client_id");
    assert_eq!(config.client_secret, "my_client_secret_xyz");
    assert_eq!(config.token_url, "https://auth.example.com/oauth/token");
    assert_eq!(config.scopes, vec!["read", "write"]);
}

#[test]
fn test_oauth2_config_invalid_credentials() {
    let invalid = "only_client_id_no_secret";
    assert!(OAuth2Config::from_credentials(
        invalid,
        "https://auth.example.com/token".to_string(),
        vec![]
    )
    .is_err());
}

#[test]
fn test_pkce_challenge_generation() {
    let pkce = PkceChallenge::generate();
    assert_eq!(pkce.method, "S256");
    assert!(!pkce.verifier.is_empty());
    assert!(!pkce.challenge.is_empty());
    // Verifier should be url-safe base64
    assert!(pkce.verifier.len() >= 43);
    assert!(pkce.challenge.len() >= 43);

    // Two consecutive PKCE challenges should have distinct random verifiers
    let pkce2 = PkceChallenge::generate();
    assert_ne!(pkce.verifier, pkce2.verifier);
    assert_ne!(pkce.challenge, pkce2.challenge);
}

#[test]
fn test_auth_code_and_device_flow_config_structs() {
    let auth_config = AuthCodeConfig {
        client_id: "client123".to_string(),
        client_secret: Some("secret456".to_string()),
        auth_url: "https://auth.com/authorize".to_string(),
        token_url: "https://auth.com/token".to_string(),
        redirect_uri: "http://localhost:8080".to_string(),
        scopes: vec!["profile".to_string()],
        use_pkce: true,
    };
    assert_eq!(auth_config.client_id, "client123");
    assert!(auth_config.use_pkce);

    let device_config = DeviceFlowConfig {
        client_id: "device_client".to_string(),
        device_auth_url: "https://auth.com/device".to_string(),
        token_url: "https://auth.com/token".to_string(),
        scopes: vec!["offline_access".to_string()],
    };
    assert_eq!(device_config.client_id, "device_client");
}
