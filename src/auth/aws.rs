//! AWS Signature Version 4 authentication
//!
//! Provides support for signing requests with AWS SigV4 for services like
//! API Gateway, S3, Lambda, and other AWS APIs.

use std::time::SystemTime;
use aws_sigv4::http_request::{sign, SignableBody, SignableRequest, SigningSettings};
use aws_sigv4::sign::v4;
use aws_credential_types::Credentials;
use crate::errors::QuicpulseError;

/// AWS SigV4 configuration
#[derive(Debug, Clone)]
pub struct AwsSigV4Config {
    /// AWS access key ID
    pub access_key_id: String,
    /// AWS secret access key
    pub secret_access_key: String,
    /// AWS session token (optional, for temporary credentials)
    pub session_token: Option<String>,
    /// AWS region (e.g., "us-east-1")
    pub region: String,
    /// AWS service name (e.g., "execute-api", "s3")
    pub service: String,
}

impl AwsSigV4Config {
    /// Create from credentials string (access_key:secret_key or access_key:secret_key:session_token)
    pub fn from_credentials(
        credentials: &str,
        region: String,
        service: String,
    ) -> Result<Self, QuicpulseError> {
        let parts: Vec<&str> = credentials.splitn(3, ':').collect();

        match parts.len() {
            2 => Ok(Self {
                access_key_id: parts[0].to_string(),
                secret_access_key: parts[1].to_string(),
                session_token: None,
                region,
                service,
            }),
            3 => Ok(Self {
                access_key_id: parts[0].to_string(),
                secret_access_key: parts[1].to_string(),
                session_token: Some(parts[2].to_string()),
                region,
                service,
            }),
            _ => Err(QuicpulseError::Argument(
                "AWS credentials must be in format: ACCESS_KEY:SECRET_KEY or ACCESS_KEY:SECRET_KEY:SESSION_TOKEN".to_string()
            )),
        }
    }

    /// Try to load from environment variables
    pub fn from_env(region: String, service: String) -> Result<Self, QuicpulseError> {
        let access_key = std::env::var("AWS_ACCESS_KEY_ID")
            .or_else(|_| std::env::var("AWS_ACCESS_KEY"))
            .map_err(|_| QuicpulseError::Argument(
                "AWS_ACCESS_KEY_ID environment variable not set".to_string()
            ))?;

        let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
            .or_else(|_| std::env::var("AWS_SECRET_KEY"))
            .map_err(|_| QuicpulseError::Argument(
                "AWS_SECRET_ACCESS_KEY environment variable not set".to_string()
            ))?;

        let session_token = std::env::var("AWS_SESSION_TOKEN").ok();

        // Region can come from arg or env
        let region = if region.is_empty() {
            std::env::var("AWS_REGION")
                .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
                .unwrap_or_else(|_| "us-east-1".to_string())
        } else {
            region
        };

        Ok(Self {
            access_key_id: access_key,
            secret_access_key: secret_key,
            session_token,
            region,
            service,
        })
    }

    /// Load from AWS profile (from ~/.aws/credentials and ~/.aws/config)
    ///
    /// Supports:
    /// - Static credentials profiles
    /// - SSO profiles (requires prior `aws sso login`)
    /// - AssumeRole profiles (automatically calls STS)
    pub async fn from_profile(
        profile_name: &str,
        region_override: Option<String>,
        service: String,
    ) -> Result<Self, QuicpulseError> {
        use super::aws_config;

        let profile = aws_config::load_profile(profile_name)?;

        // Determine region: override > profile > env > default
        let region = region_override
            .or(profile.region.clone())
            .or_else(|| std::env::var("AWS_REGION").ok())
            .or_else(|| std::env::var("AWS_DEFAULT_REGION").ok())
            .unwrap_or_else(|| "us-east-1".to_string());

        // Handle different profile types
        if profile.is_assume_role_profile() {
            // AssumeRole profile - need to get credentials from source profile first
            super::aws_sts::resolve_assume_role_profile(&profile, region, service).await
        } else if profile.is_sso_profile() {
            // SSO profile - load from SSO token cache
            super::aws_sso::resolve_sso_profile(&profile, region, service).await
        } else if profile.has_static_credentials() {
            // Static credentials
            Ok(Self {
                access_key_id: profile.access_key_id.unwrap(),
                secret_access_key: profile.secret_access_key.unwrap(),
                session_token: profile.session_token,
                region,
                service,
            })
        } else if profile.has_credential_process() {
            // Credential process - run external command
            Self::from_credential_process(profile.credential_process.as_ref().unwrap(), region, service)
        } else {
            Err(QuicpulseError::Config(format!(
                "AWS profile '{}' has no valid credentials source",
                profile_name
            )))
        }
    }

    /// Load credentials from an external credential process
    fn from_credential_process(
        command: &str,
        region: String,
        service: String,
    ) -> Result<Self, QuicpulseError> {
        use std::process::Command;

        // Run the credential process
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .output()
        } else {
            Command::new("sh")
                .args(["-c", command])
                .output()
        };

        let output = output.map_err(|e| {
            QuicpulseError::Config(format!("Failed to run credential_process: {}", e))
        })?;

        if !output.status.success() {
            return Err(QuicpulseError::Config(format!(
                "credential_process failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Parse JSON output
        let json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| QuicpulseError::Config(format!(
                "Failed to parse credential_process output: {}", e
            )))?;

        let access_key = json["AccessKeyId"]
            .as_str()
            .ok_or_else(|| QuicpulseError::Config(
                "credential_process output missing AccessKeyId".to_string()
            ))?
            .to_string();

        let secret_key = json["SecretAccessKey"]
            .as_str()
            .ok_or_else(|| QuicpulseError::Config(
                "credential_process output missing SecretAccessKey".to_string()
            ))?
            .to_string();

        let session_token = json["SessionToken"].as_str().map(|s| s.to_string());

        Ok(Self {
            access_key_id: access_key,
            secret_access_key: secret_key,
            session_token,
            region,
            service,
        })
    }
}

/// Sign an HTTP request with AWS SigV4
///
/// For multipart uploads, set `unsigned_payload` to true to use UNSIGNED-PAYLOAD
/// as the content hash. This is required because multipart form bodies cannot be
/// known ahead of time (reqwest generates the boundary and body lazily).
pub fn sign_request(
    config: &AwsSigV4Config,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
    unsigned_payload: bool,
) -> Result<Vec<(String, String)>, QuicpulseError> {
    // Parse URL
    let parsed_url = url::Url::parse(url)
        .map_err(|e| QuicpulseError::Parse(format!("Invalid URL: {}", e)))?;

    // Build the request for signing
    let uri = format!(
        "{}{}",
        parsed_url.path(),
        parsed_url.query().map(|q| format!("?{}", q)).unwrap_or_default()
    );

    // Create credentials and convert to Identity
    let credentials = Credentials::new(
        &config.access_key_id,
        &config.secret_access_key,
        config.session_token.clone(),
        None, // expiry
        "quicpulse",
    );
    let identity = credentials.into();

    // Signing settings
    let settings = SigningSettings::default();

    // Build signing params using v4
    let signing_params = v4::SigningParams::builder()
        .identity(&identity)
        .region(&config.region)
        .name(&config.service)
        .time(SystemTime::now())
        .settings(settings)
        .build()
        .map_err(|e| QuicpulseError::Auth(format!("Failed to build signing params: {}", e)))?;

    // Build the signable request
    // For multipart/streaming uploads, use UNSIGNED-PAYLOAD to indicate
    // the body hash should not be included in the signature
    let signable_body = if unsigned_payload {
        SignableBody::UnsignedPayload
    } else {
        let body_bytes = body.unwrap_or(&[]);
        if body_bytes.is_empty() {
            SignableBody::empty()
        } else {
            SignableBody::Bytes(body_bytes)
        }
    };

    // Convert headers to http::HeaderMap format
    let mut header_map = http::HeaderMap::new();
    for (name, value) in headers {
        if let Ok(header_name) = http::header::HeaderName::try_from(name.as_str()) {
            if let Ok(header_value) = http::header::HeaderValue::from_str(value) {
                header_map.insert(header_name, header_value);
            }
        }
    }

    // Compute the correct Host header value for signing
    // Must match what will actually be sent in the request
    let computed_host = if let Some(host) = parsed_url.host_str() {
        if let Some(port) = parsed_url.port() {
            // Include port only if it's non-standard for the scheme
            let is_standard_port = match parsed_url.scheme() {
                "https" => port == 443,
                "http" => port == 80,
                _ => false,
            };
            if is_standard_port {
                host.to_string()
            } else {
                format!("{}:{}", host, port)
            }
        } else {
            host.to_string()
        }
    } else {
        String::new()
    };

    // Add host header if not present
    if !header_map.contains_key(http::header::HOST) && !computed_host.is_empty() {
        if let Ok(value) = http::header::HeaderValue::from_str(&computed_host) {
            header_map.insert(http::header::HOST, value);
        }
    }

    // Verify existing Host header matches URL to prevent signature mismatches
    if let Some(host_header) = header_map.get(http::header::HOST) {
        if let Ok(header_host) = host_header.to_str() {
            if !computed_host.is_empty() && header_host != computed_host {
                return Err(QuicpulseError::Auth(format!(
                    "Host header '{}' does not match URL host '{}'. AWS SigV4 requires matching hosts.",
                    header_host, computed_host
                )));
            }
        }
    }

    // Create signable request
    let signable_request = SignableRequest::new(
        method,
        &uri,
        header_map.iter().map(|(k, v)| (k.as_str(), v.to_str().unwrap_or(""))),
        signable_body,
    ).map_err(|e| QuicpulseError::Auth(format!("Failed to create signable request: {}", e)))?;

    // Sign the request - convert to SigningParams enum
    let signing_output = sign(signable_request, &signing_params.into())
        .map_err(|e| QuicpulseError::Auth(format!("Failed to sign request: {}", e)))?;

    // Extract signature headers using the headers() method
    let mut auth_headers = Vec::new();
    let (signing_instructions, _signature) = signing_output.into_parts();

    for (name, value) in signing_instructions.headers() {
        auth_headers.push((name.to_string(), value.to_string()));
    }

    // Bug #9 fix: Include the computed Host header in the returned headers
    // This ensures reqwest uses the same Host header that was used for signing.
    // If they differ (e.g., due to different port handling), the signature will fail.
    if !computed_host.is_empty() {
        // Only add if not already present from signing_instructions
        if !auth_headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
            auth_headers.push(("host".to_string(), computed_host));
        }
    }

    Ok(auth_headers)
}

/// Compute SHA256 hash of data (for x-amz-content-sha256 header)
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_credentials() {
        let config = AwsSigV4Config::from_credentials(
            "AKID:SECRET",
            "us-east-1".to_string(),
            "execute-api".to_string(),
        ).unwrap();

        assert_eq!(config.access_key_id, "AKID");
        assert_eq!(config.secret_access_key, "SECRET");
        assert!(config.session_token.is_none());
    }

    #[test]
    fn test_parse_credentials_with_token() {
        let config = AwsSigV4Config::from_credentials(
            "AKID:SECRET:TOKEN",
            "us-west-2".to_string(),
            "s3".to_string(),
        ).unwrap();

        assert_eq!(config.access_key_id, "AKID");
        assert_eq!(config.secret_access_key, "SECRET");
        assert_eq!(config.session_token, Some("TOKEN".to_string()));
    }

    #[test]
    fn test_sha256_hex() {
        let hash = sha256_hex(b"hello");
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }
}
