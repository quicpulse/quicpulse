//! Authentication middleware
//!
//! Provides authentication via enum variants rather than trait objects.
//! This is more idiomatic Rust - using sum types for a finite set of
//! authentication methods instead of runtime polymorphism.

mod basic;
mod bearer;
mod digest;
mod apikey;
mod ntlm;

pub use basic::BasicAuth;
pub use bearer::BearerAuth;
pub use digest::{DigestAuth, DigestChallenge, DigestAlgorithm};
pub use apikey::ApiKeyAuth;
pub use ntlm::{NtlmAuth, NegotiateAuth, KerberosAuth, Type2Message, parse_type2_message, extract_type2_from_header};

use reqwest::header::HeaderMap;
use thiserror::Error;

/// Authentication error types
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("digest authentication is not implemented: HTTP Digest Auth (RFC 7616) requires challenge-response")]
    DigestNotImplemented,

    #[error("invalid header value: {0}")]
    InvalidHeader(String),

    #[error("missing credentials")]
    MissingCredentials,

    #[error("Kerberos authentication requires system configuration: run 'kinit' to obtain a ticket or configure keytab")]
    KerberosNotConfigured,

    #[error("invalid authentication challenge: {0}")]
    InvalidChallenge(String),

    #[error("invalid credentials: {0}")]
    InvalidCredentials(String),
}

/// Authentication method enum - replaces trait objects with sum type
#[derive(Debug, Clone)]
pub enum Auth {
    /// HTTP Basic Authentication (RFC 7617)
    Basic(BasicAuth),
    /// Bearer token authentication (RFC 6750)
    Bearer(BearerAuth),
    /// HTTP Digest Authentication (RFC 7616)
    Digest(DigestAuth),
    /// API Key authentication via custom header
    ApiKey(ApiKeyAuth),
    /// NTLM authentication (Windows Integrated Auth)
    Ntlm(NtlmAuth),
    /// Negotiate (SPNEGO) - auto-selects Kerberos or NTLM
    Negotiate(NegotiateAuth),
    /// Kerberos authentication
    Kerberos(KerberosAuth),
}

impl Auth {
    /// Create Basic authentication
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Auth::Basic(BasicAuth::new(username, password))
    }

    /// Create Bearer token authentication
    pub fn bearer(token: impl Into<String>) -> Self {
        Auth::Bearer(BearerAuth::new(token))
    }

    /// Create Digest authentication (will error on apply)
    pub fn digest(username: impl Into<String>, password: impl Into<String>) -> Self {
        Auth::Digest(DigestAuth::new(username, password))
    }

    /// Create API Key authentication
    pub fn api_key(header_name: impl Into<String>, key: impl Into<String>) -> Self {
        Auth::ApiKey(ApiKeyAuth::new(header_name, key))
    }

    /// Create NTLM authentication
    pub fn ntlm(username: impl Into<String>, password: impl Into<String>) -> Self {
        Auth::Ntlm(NtlmAuth::new(username, password))
    }

    /// Create Negotiate (SPNEGO) authentication
    pub fn negotiate(username: impl Into<String>, password: impl Into<String>) -> Self {
        Auth::Negotiate(NegotiateAuth::new(username, password))
    }

    /// Create Kerberos authentication
    pub fn kerberos(principal: impl Into<String>, password: impl Into<String>) -> Self {
        Auth::Kerberos(KerberosAuth::new(principal, password))
    }

    /// Apply authentication to request headers
    pub fn apply(&self, headers: &mut HeaderMap) -> Result<(), AuthError> {
        match self {
            Auth::Basic(auth) => auth.apply(headers),
            Auth::Bearer(auth) => auth.apply(headers),
            Auth::Digest(auth) => auth.apply(headers),
            Auth::ApiKey(auth) => auth.apply(headers),
            Auth::Ntlm(auth) => auth.apply(headers),
            Auth::Negotiate(auth) => auth.apply(headers),
            Auth::Kerberos(auth) => auth.apply(headers),
        }
    }

    /// Whether this auth method parses credentials as "user:password"
    pub fn parses_credentials(&self) -> bool {
        match self {
            Auth::Basic(_) | Auth::Digest(_) | Auth::Ntlm(_) | Auth::Negotiate(_) | Auth::Kerberos(_) => true,
            Auth::Bearer(_) | Auth::ApiKey(_) => false,
        }
    }

    /// Whether to prompt for password if not provided
    pub fn prompts_password(&self) -> bool {
        match self {
            Auth::Basic(_) | Auth::Digest(_) | Auth::Ntlm(_) | Auth::Negotiate(_) | Auth::Kerberos(_) => true,
            Auth::Bearer(_) | Auth::ApiKey(_) => false,
        }
    }

    /// Authentication type name for display/debugging
    pub fn type_name(&self) -> &'static str {
        match self {
            Auth::Basic(_) => "basic",
            Auth::Bearer(_) => "bearer",
            Auth::Digest(_) => "digest",
            Auth::ApiKey(_) => "api-key",
            Auth::Ntlm(_) => "ntlm",
            Auth::Negotiate(_) => "negotiate",
            Auth::Kerberos(_) => "kerberos",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::AUTHORIZATION;

    #[test]
    fn test_basic_auth() {
        let auth = Auth::basic("user", "pass");
        let mut headers = HeaderMap::new();
        auth.apply(&mut headers).unwrap();

        let value = headers.get(AUTHORIZATION).unwrap().to_str().unwrap();
        assert!(value.starts_with("Basic "));

        // Verify base64 encoding
        let encoded = &value[6..];
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            encoded
        ).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "user:pass");
    }

    #[test]
    fn test_bearer_auth() {
        let auth = Auth::bearer("my-token");
        let mut headers = HeaderMap::new();
        auth.apply(&mut headers).unwrap();

        let value = headers.get(AUTHORIZATION).unwrap().to_str().unwrap();
        assert_eq!(value, "Bearer my-token");
    }

    #[test]
    fn test_digest_apply_succeeds() {
        // Digest auth apply() now succeeds - it's a no-op on first request
        // The actual auth happens via respond_to_challenge after 401
        let auth = Auth::digest("user", "pass");
        let mut headers = HeaderMap::new();
        let result = auth.apply(&mut headers);

        assert!(result.is_ok());
        // Headers should be empty (no Authorization header yet)
        assert!(!headers.contains_key(reqwest::header::AUTHORIZATION));
    }

    #[test]
    fn test_api_key_auth() {
        let auth = Auth::api_key("X-API-Key", "secret123");
        let mut headers = HeaderMap::new();
        auth.apply(&mut headers).unwrap();

        let value = headers.get("X-API-Key").unwrap().to_str().unwrap();
        assert_eq!(value, "secret123");
    }
}
