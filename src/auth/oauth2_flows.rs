//! OAuth 2.0 Authorization Flows
//!
//! Provides support for:
//! - Authorization Code Flow (web-based with user interaction)
//! - PKCE Extension (Proof Key for Code Exchange)
//! - Device Authorization Flow (for headless/CLI devices)

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use crate::errors::QuicpulseError;
use super::oauth2::{CachedToken, TokenResponse, TokenError};

/// OAuth 2.0 Authorization Code Flow Configuration
#[derive(Debug, Clone)]
pub struct AuthCodeConfig {
    /// Client ID
    pub client_id: String,
    /// Client secret (optional for public clients using PKCE)
    pub client_secret: Option<String>,
    /// Authorization endpoint URL
    pub auth_url: String,
    /// Token endpoint URL
    pub token_url: String,
    /// Redirect URI (usually http://localhost:PORT)
    pub redirect_uri: String,
    /// Requested scopes
    pub scopes: Vec<String>,
    /// Use PKCE (recommended for public clients)
    pub use_pkce: bool,
}

/// Device Authorization Flow Configuration
#[derive(Debug, Clone)]
pub struct DeviceFlowConfig {
    /// Client ID
    pub client_id: String,
    /// Device authorization endpoint URL
    pub device_auth_url: String,
    /// Token endpoint URL
    pub token_url: String,
    /// Requested scopes
    pub scopes: Vec<String>,
}

/// Device Authorization Response
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceAuthResponse {
    /// Device code for polling
    pub device_code: String,
    /// User code to display
    pub user_code: String,
    /// Verification URI
    pub verification_uri: String,
    /// Optional verification URI with user code embedded
    pub verification_uri_complete: Option<String>,
    /// Expiration time in seconds
    pub expires_in: u64,
    /// Polling interval in seconds
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

/// PKCE Code Verifier and Challenge
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    /// The code verifier (random string, 43-128 chars)
    pub verifier: String,
    /// The code challenge (SHA256 hash of verifier, base64url encoded)
    pub challenge: String,
    /// Challenge method (always S256)
    pub method: String,
}

impl PkceChallenge {
    /// Generate a new PKCE challenge
    pub fn generate() -> Self {
        // Generate random 32 bytes (will be 43 chars when base64url encoded)
        let random_bytes: [u8; 32] = rand::rng().random();
        let verifier = URL_SAFE_NO_PAD.encode(random_bytes);

        // Create challenge: base64url(SHA256(verifier))
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

        Self {
            verifier,
            challenge,
            method: "S256".to_string(),
        }
    }
}

/// Start Authorization Code Flow
///
/// 1. Opens browser to authorization URL
/// 2. Starts local HTTP server to receive callback
/// 3. Exchanges authorization code for tokens
pub async fn authorization_code_flow(
    config: &AuthCodeConfig,
) -> Result<CachedToken, QuicpulseError> {
    // Generate state for CSRF protection
    let state: String = {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::rng();
        (0..32)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    };

    // Generate PKCE challenge if enabled
    let pkce = if config.use_pkce {
        Some(PkceChallenge::generate())
    } else {
        None
    };

    // Build authorization URL
    let mut auth_url = url::Url::parse(&config.auth_url)
        .map_err(|e| QuicpulseError::Argument(format!("Invalid auth URL: {}", e)))?;

    {
        let mut params = auth_url.query_pairs_mut();
        params.append_pair("response_type", "code");
        params.append_pair("client_id", &config.client_id);
        params.append_pair("redirect_uri", &config.redirect_uri);
        params.append_pair("state", &state);

        if !config.scopes.is_empty() {
            params.append_pair("scope", &config.scopes.join(" "));
        }

        if let Some(ref pkce) = pkce {
            params.append_pair("code_challenge", &pkce.challenge);
            params.append_pair("code_challenge_method", &pkce.method);
        }
    }

    // Extract port from redirect URI
    let redirect_url = url::Url::parse(&config.redirect_uri)
        .map_err(|e| QuicpulseError::Argument(format!("Invalid redirect URI: {}", e)))?;

    let port = redirect_url.port().unwrap_or(8080);

    // Start local server to receive callback
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .map_err(|e| QuicpulseError::Io(e))?;

    // Set a timeout for the listener
    listener.set_nonblocking(false)
        .map_err(|e| QuicpulseError::Io(e))?;

    // Print instructions
    eprintln!("\n🔐 OAuth 2.0 Authorization Code Flow");
    eprintln!("Opening browser to authenticate...\n");
    eprintln!("Authorization URL:");
    eprintln!("  {}\n", auth_url);

    // Try to open browser
    if webbrowser::open(auth_url.as_str()).is_err() {
        eprintln!("⚠️  Could not open browser automatically.");
        eprintln!("Please open the URL above in your browser.\n");
    }

    eprintln!("Waiting for callback on {}...", config.redirect_uri);

    // Wait for callback (with 5 minute timeout)
    let (mut stream, _) = listener.accept()
        .map_err(|e| QuicpulseError::Io(e))?;

    // Read the HTTP request
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)
        .map_err(|e| QuicpulseError::Io(e))?;

    // Parse the request to get the code and state
    let (code, received_state) = parse_callback_request(&request_line)?;

    // Verify state
    if received_state != state {
        // Send error response
        send_html_response(&mut stream, 400, "State mismatch - possible CSRF attack")?;
        return Err(QuicpulseError::Auth("State mismatch in OAuth callback".to_string()));
    }

    // Send success response to browser
    send_html_response(&mut stream, 200,
        "<h1>✅ Authorization successful!</h1><p>You can close this window.</p>")?;

    // Exchange code for tokens
    exchange_code_for_token(config, &code, pkce.as_ref()).await
}

/// Parse the callback request to extract code and state
fn parse_callback_request(request: &str) -> Result<(String, String), QuicpulseError> {
    // Request looks like: GET /?code=xxx&state=yyy HTTP/1.1
    let parts: Vec<&str> = request.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(QuicpulseError::Auth("Invalid callback request".to_string()));
    }

    let path = parts[1];
    let url = url::Url::parse(&format!("http://localhost{}", path))
        .map_err(|_| QuicpulseError::Auth("Invalid callback URL".to_string()))?;

    let params: HashMap<_, _> = url.query_pairs().collect();

    // Check for error response
    if let Some(error) = params.get("error") {
        let desc = params.get("error_description")
            .map(|s| s.to_string())
            .unwrap_or_default();
        return Err(QuicpulseError::Auth(format!("OAuth error: {} - {}", error, desc)));
    }

    let code = params.get("code")
        .ok_or_else(|| QuicpulseError::Auth("No code in callback".to_string()))?
        .to_string();

    let state = params.get("state")
        .ok_or_else(|| QuicpulseError::Auth("No state in callback".to_string()))?
        .to_string();

    Ok((code, state))
}

/// Send an HTML response to the browser
fn send_html_response(stream: &mut std::net::TcpStream, status: u16, body: &str) -> Result<(), QuicpulseError> {
    let status_text = if status == 200 { "OK" } else { "Bad Request" };
    let response = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: text/html\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         <!DOCTYPE html><html><body>{}</body></html>",
        status, status_text, body.len() + 47, body
    );
    stream.write_all(response.as_bytes())
        .map_err(|e| QuicpulseError::Io(e))?;
    stream.flush()
        .map_err(|e| QuicpulseError::Io(e))
}

/// Exchange authorization code for tokens
async fn exchange_code_for_token(
    config: &AuthCodeConfig,
    code: &str,
    pkce: Option<&PkceChallenge>,
) -> Result<CachedToken, QuicpulseError> {
    let client = reqwest::Client::new();

    let mut params = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("redirect_uri", config.redirect_uri.clone()),
        ("client_id", config.client_id.clone()),
    ];

    if let Some(ref secret) = config.client_secret {
        params.push(("client_secret", secret.clone()));
    }

    if let Some(pkce) = pkce {
        params.push(("code_verifier", pkce.verifier.clone()));
    }

    let obtained_at = Instant::now();

    let response = client
        .post(&config.token_url)
        .form(&params)
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
            return Err(QuicpulseError::Auth(format!("Token exchange failed: {}", msg)));
        }
        return Err(QuicpulseError::Auth(format!(
            "Token exchange failed with status {}: {}", status, body
        )));
    }

    let token: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| QuicpulseError::Auth(format!("Failed to parse token response: {}", e)))?;

    eprintln!("\n✅ Authentication successful!");

    Ok(CachedToken {
        access_token: token.access_token,
        token_type: if token.token_type.is_empty() { "Bearer".to_string() } else { token.token_type },
        obtained_at,
        expires_in: token.expires_in.map(Duration::from_secs),
        refresh_token: token.refresh_token,
    })
}

/// Device Authorization Flow
///
/// 1. Requests device and user codes
/// 2. Displays verification URI and code to user
/// 3. Polls token endpoint until user authorizes
pub async fn device_flow(config: &DeviceFlowConfig) -> Result<CachedToken, QuicpulseError> {
    let client = reqwest::Client::new();

    // Step 1: Request device authorization
    let mut params = vec![
        ("client_id", config.client_id.clone()),
    ];

    if !config.scopes.is_empty() {
        params.push(("scope", config.scopes.join(" ")));
    }

    let response = client
        .post(&config.device_auth_url)
        .form(&params)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| QuicpulseError::Request(e))?;

    let status = response.status();
    let body = response.text().await
        .map_err(|e| QuicpulseError::Request(e))?;

    if !status.is_success() {
        return Err(QuicpulseError::Auth(format!(
            "Device authorization failed: {}", body
        )));
    }

    let device_auth: DeviceAuthResponse = serde_json::from_str(&body)
        .map_err(|e| QuicpulseError::Auth(format!("Failed to parse device auth response: {}", e)))?;

    // Step 2: Display instructions to user
    eprintln!("\n🔐 OAuth 2.0 Device Authorization Flow\n");
    eprintln!("To authorize this device, visit:");
    eprintln!("  {}\n", device_auth.verification_uri);
    eprintln!("And enter this code:");
    eprintln!("  ┌─────────────────┐");
    eprintln!("  │  {}  │", device_auth.user_code);
    eprintln!("  └─────────────────┘\n");

    if let Some(ref complete_uri) = device_auth.verification_uri_complete {
        eprintln!("Or open this URL directly:");
        eprintln!("  {}\n", complete_uri);
        // Try to open browser
        if webbrowser::open(complete_uri).is_ok() {
            eprintln!("✓ Browser opened automatically\n");
        }
    }

    eprintln!("Waiting for authorization (expires in {} seconds)...\n", device_auth.expires_in);

    // Step 3: Poll for token
    poll_for_token(config, &device_auth).await
}

/// Poll the token endpoint until authorization is complete
async fn poll_for_token(
    config: &DeviceFlowConfig,
    device_auth: &DeviceAuthResponse,
) -> Result<CachedToken, QuicpulseError> {
    let client = reqwest::Client::new();
    let start = Instant::now();
    let timeout = Duration::from_secs(device_auth.expires_in);
    let interval = Duration::from_secs(device_auth.interval);

    loop {
        // Check timeout
        if start.elapsed() > timeout {
            return Err(QuicpulseError::Auth("Device authorization expired".to_string()));
        }

        // Wait for interval
        tokio::time::sleep(interval).await;

        // Poll token endpoint
        let params = vec![
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", &device_auth.device_code),
            ("client_id", &config.client_id),
        ];

        let obtained_at = Instant::now();

        let response = client
            .post(&config.token_url)
            .form(&params)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| QuicpulseError::Request(e))?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| QuicpulseError::Request(e))?;

        if status.is_success() {
            // Got the token!
            let token: TokenResponse = serde_json::from_str(&body)
                .map_err(|e| QuicpulseError::Auth(format!("Failed to parse token: {}", e)))?;

            eprintln!("✅ Authorization successful!\n");

            return Ok(CachedToken {
                access_token: token.access_token,
                token_type: if token.token_type.is_empty() { "Bearer".to_string() } else { token.token_type },
                obtained_at,
                expires_in: token.expires_in.map(Duration::from_secs),
                refresh_token: token.refresh_token,
            });
        }

        // Check error type
        if let Ok(error) = serde_json::from_str::<TokenError>(&body) {
            match error.error.as_str() {
                "authorization_pending" => {
                    // User hasn't authorized yet, continue polling
                    eprint!(".");
                    std::io::stderr().flush().ok();
                    continue;
                }
                "slow_down" => {
                    // Server wants us to slow down
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                "expired_token" => {
                    return Err(QuicpulseError::Auth("Device code expired".to_string()));
                }
                "access_denied" => {
                    return Err(QuicpulseError::Auth("User denied authorization".to_string()));
                }
                _ => {
                    let msg = error.error_description.unwrap_or(error.error);
                    return Err(QuicpulseError::Auth(format!("Device flow error: {}", msg)));
                }
            }
        }
    }
}

/// OAuth 2.0 Flow Type
#[derive(Debug, Clone, PartialEq)]
pub enum OAuth2FlowType {
    /// Client Credentials (existing implementation)
    ClientCredentials,
    /// Authorization Code (with optional PKCE)
    AuthorizationCode { use_pkce: bool },
    /// Device Authorization
    DeviceFlow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_challenge() {
        let pkce = PkceChallenge::generate();

        // Verifier should be 43 characters (32 bytes base64url encoded)
        assert_eq!(pkce.verifier.len(), 43);

        // Challenge should be 43 characters (32 bytes SHA256 base64url encoded)
        assert_eq!(pkce.challenge.len(), 43);

        // Method should be S256
        assert_eq!(pkce.method, "S256");

        // Verify the challenge is correct
        let mut hasher = Sha256::new();
        hasher.update(pkce.verifier.as_bytes());
        let expected_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
        assert_eq!(pkce.challenge, expected_challenge);
    }

    #[test]
    fn test_parse_callback_success() {
        let request = "GET /?code=abc123&state=xyz789 HTTP/1.1";
        let (code, state) = parse_callback_request(request).unwrap();
        assert_eq!(code, "abc123");
        assert_eq!(state, "xyz789");
    }

    #[test]
    fn test_parse_callback_error() {
        let request = "GET /?error=access_denied&error_description=User%20denied HTTP/1.1";
        let result = parse_callback_request(request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("access_denied"));
    }

    #[test]
    fn test_device_auth_response_deserialize() {
        let json = r#"{
            "device_code": "dev123",
            "user_code": "ABC-DEF",
            "verification_uri": "https://example.com/device",
            "expires_in": 1800,
            "interval": 5
        }"#;

        let response: DeviceAuthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.device_code, "dev123");
        assert_eq!(response.user_code, "ABC-DEF");
        assert_eq!(response.expires_in, 1800);
        assert_eq!(response.interval, 5);
    }

    #[test]
    fn test_device_auth_response_default_interval() {
        let json = r#"{
            "device_code": "dev123",
            "user_code": "ABC-DEF",
            "verification_uri": "https://example.com/device",
            "expires_in": 1800
        }"#;

        let response: DeviceAuthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.interval, 5); // default value
    }
}
