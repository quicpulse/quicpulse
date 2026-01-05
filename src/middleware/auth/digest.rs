//! HTTP Digest Authentication (RFC 7616)
//!
//! Implements the full Digest authentication challenge-response mechanism
//! supporting MD5, SHA-256, and SHA-512-256 algorithms.

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use std::sync::atomic::{AtomicU32, Ordering};

use super::AuthError;

/// Supported digest algorithms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DigestAlgorithm {
    MD5,
    SHA256,
    SHA512_256,
}

impl DigestAlgorithm {
    /// Parse algorithm from string (case-insensitive)
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "SHA-256" | "SHA256" => DigestAlgorithm::SHA256,
            "SHA-512-256" | "SHA512-256" => DigestAlgorithm::SHA512_256,
            _ => DigestAlgorithm::MD5, // Default to MD5 for compatibility
        }
    }

    /// Get algorithm name for Authorization header
    pub fn as_str(&self) -> &'static str {
        match self {
            DigestAlgorithm::MD5 => "MD5",
            DigestAlgorithm::SHA256 => "SHA-256",
            DigestAlgorithm::SHA512_256 => "SHA-512-256",
        }
    }
}

/// Parsed digest challenge from WWW-Authenticate header
#[derive(Debug, Clone)]
pub struct DigestChallenge {
    pub realm: String,
    pub nonce: String,
    pub algorithm: DigestAlgorithm,
    pub qop: Option<String>,
    pub opaque: Option<String>,
    pub stale: bool,
    pub domain: Option<String>,
}

impl DigestChallenge {
    /// Parse WWW-Authenticate: Digest header value
    pub fn parse(header: &str) -> Result<Self, AuthError> {
        // Strip "Digest " prefix if present
        let params_str = header
            .strip_prefix("Digest ")
            .or_else(|| header.strip_prefix("digest "))
            .unwrap_or(header);

        let mut realm = None;
        let mut nonce = None;
        let mut algorithm = DigestAlgorithm::MD5;
        let mut qop = None;
        let mut opaque = None;
        let mut stale = false;
        let mut domain = None;

        // Parse key=value pairs
        for part in split_digest_params(params_str) {
            let part = part.trim();
            if let Some((key, value)) = parse_param(part) {
                match key.to_lowercase().as_str() {
                    "realm" => realm = Some(value),
                    "nonce" => nonce = Some(value),
                    "algorithm" => algorithm = DigestAlgorithm::from_str(&value),
                    "qop" => qop = Some(value),
                    "opaque" => opaque = Some(value),
                    "stale" => stale = value.eq_ignore_ascii_case("true"),
                    "domain" => domain = Some(value),
                    _ => {} // Ignore unknown parameters
                }
            }
        }

        let realm = realm.ok_or_else(|| {
            AuthError::InvalidChallenge("Missing realm in Digest challenge".to_string())
        })?;
        let nonce = nonce.ok_or_else(|| {
            AuthError::InvalidChallenge("Missing nonce in Digest challenge".to_string())
        })?;

        Ok(DigestChallenge {
            realm,
            nonce,
            algorithm,
            qop,
            opaque,
            stale,
            domain,
        })
    }
}

/// HTTP Digest Authentication
#[derive(Debug)]
pub struct DigestAuth {
    username: String,
    password: String,
    nc: AtomicU32, // Nonce count for replay protection
}

impl Clone for DigestAuth {
    fn clone(&self) -> Self {
        Self {
            username: self.username.clone(),
            password: self.password.clone(),
            nc: AtomicU32::new(self.nc.load(Ordering::SeqCst)),
        }
    }
}

impl DigestAuth {
    /// Create new Digest auth credentials
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            nc: AtomicU32::new(0),
        }
    }

    /// Parse credentials from "username:password" format
    pub fn from_credentials(creds: &str) -> Result<Self, AuthError> {
        let parts: Vec<&str> = creds.splitn(2, ':').collect();
        let username = parts.first().ok_or(AuthError::MissingCredentials)?;
        let password = parts.get(1).unwrap_or(&"");
        Ok(Self::new(*username, *password))
    }

    /// This method is a placeholder - Digest auth requires challenge-response
    /// Use `respond_to_challenge` after receiving a 401 response
    pub fn apply(&self, _headers: &mut HeaderMap) -> Result<(), AuthError> {
        // Digest auth cannot be applied without a challenge
        // The first request should be sent without auth,
        // then respond_to_challenge should be called after 401
        Ok(())
    }

    /// Generate Authorization header from a digest challenge
    pub fn respond_to_challenge(
        &self,
        challenge: &DigestChallenge,
        method: &str,
        uri: &str,
    ) -> Result<String, AuthError> {
        // Increment nonce count
        let nc = self.nc.fetch_add(1, Ordering::SeqCst) + 1;
        let nc_str = format!("{:08x}", nc);

        // Generate client nonce
        let cnonce = generate_cnonce();

        // Compute response hash
        let response = compute_digest_response(
            challenge.algorithm,
            &self.username,
            &challenge.realm,
            &self.password,
            &challenge.nonce,
            nc,
            &cnonce,
            challenge.qop.as_deref(),
            method,
            uri,
        );

        // Build Authorization header value
        let mut auth_value = format!(
            "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", response=\"{}\"",
            self.username, challenge.realm, challenge.nonce, uri, response
        );

        // Add algorithm
        auth_value.push_str(&format!(", algorithm={}", challenge.algorithm.as_str()));

        // Add qop-specific parameters
        if let Some(ref qop) = challenge.qop {
            // Use first qop option (typically "auth")
            let qop_value = qop.split(',').next().unwrap_or("auth").trim();
            auth_value.push_str(&format!(", qop={}, nc={}, cnonce=\"{}\"", qop_value, nc_str, cnonce));
        }

        // Add opaque if present
        if let Some(ref opaque) = challenge.opaque {
            auth_value.push_str(&format!(", opaque=\"{}\"", opaque));
        }

        Ok(auth_value)
    }

    /// Apply digest response to headers
    pub fn apply_response(
        &self,
        headers: &mut HeaderMap,
        challenge: &DigestChallenge,
        method: &str,
        uri: &str,
    ) -> Result<(), AuthError> {
        let auth_value = self.respond_to_challenge(challenge, method, uri)?;
        let header_value = HeaderValue::try_from(auth_value)
            .map_err(|e| AuthError::InvalidHeader(e.to_string()))?;
        headers.insert(AUTHORIZATION, header_value);
        Ok(())
    }
}

/// Compute the digest response hash according to RFC 7616
fn compute_digest_response(
    algorithm: DigestAlgorithm,
    username: &str,
    realm: &str,
    password: &str,
    nonce: &str,
    nc: u32,
    cnonce: &str,
    qop: Option<&str>,
    method: &str,
    uri: &str,
) -> String {
    // A1 = username:realm:password
    let a1 = format!("{}:{}:{}", username, realm, password);
    let ha1 = hash(algorithm, &a1);

    // A2 = method:uri (for qop=auth or no qop)
    // A2 = method:uri:H(body) (for qop=auth-int, not implemented)
    let a2 = format!("{}:{}", method, uri);
    let ha2 = hash(algorithm, &a2);

    // Response calculation
    if let Some(qop_value) = qop {
        // qop present: response = H(H(A1):nonce:nc:cnonce:qop:H(A2))
        let qop_first = qop_value.split(',').next().unwrap_or("auth").trim();
        let response_data = format!(
            "{}:{}:{:08x}:{}:{}:{}",
            ha1, nonce, nc, cnonce, qop_first, ha2
        );
        hash(algorithm, &response_data)
    } else {
        // No qop: response = H(H(A1):nonce:H(A2))
        let response_data = format!("{}:{}:{}", ha1, nonce, ha2);
        hash(algorithm, &response_data)
    }
}

/// Hash a string using the specified algorithm
fn hash(algorithm: DigestAlgorithm, data: &str) -> String {
    use sha2::Digest;

    match algorithm {
        DigestAlgorithm::MD5 => {
            // Use md5_digest (md-5 crate) which implements the Digest trait
            let mut hasher = md5_digest::Md5::new();
            hasher.update(data.as_bytes());
            hex::encode(hasher.finalize())
        }
        DigestAlgorithm::SHA256 => {
            let mut hasher = sha2::Sha256::new();
            hasher.update(data.as_bytes());
            hex::encode(hasher.finalize())
        }
        DigestAlgorithm::SHA512_256 => {
            let mut hasher = sha2::Sha512_256::new();
            hasher.update(data.as_bytes());
            hex::encode(hasher.finalize())
        }
    }
}

/// Generate a random client nonce
fn generate_cnonce() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let bytes: [u8; 16] = rng.random();
    hex::encode(bytes)
}

/// Split digest parameters, handling quoted values with commas
fn split_digest_params(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in s.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            ',' if !in_quotes => {
                if !current.trim().is_empty() {
                    parts.push(current.trim().to_string());
                }
                current = String::new();
            }
            _ => current.push(c),
        }
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}

/// Parse a single key=value or key="value" parameter
fn parse_param(s: &str) -> Option<(String, String)> {
    let mut parts = s.splitn(2, '=');
    let key = parts.next()?.trim().to_string();
    let value = parts.next()?.trim();

    // Remove surrounding quotes if present
    let value = if value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    };

    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_digest_challenge() {
        let header = r#"Digest realm="test@example.com", nonce="abc123", qop="auth", algorithm=MD5"#;
        let challenge = DigestChallenge::parse(header).unwrap();

        assert_eq!(challenge.realm, "test@example.com");
        assert_eq!(challenge.nonce, "abc123");
        assert_eq!(challenge.algorithm, DigestAlgorithm::MD5);
        assert_eq!(challenge.qop, Some("auth".to_string()));
    }

    #[test]
    fn test_parse_sha256_challenge() {
        let header = r#"Digest realm="api", nonce="xyz789", algorithm=SHA-256, qop="auth""#;
        let challenge = DigestChallenge::parse(header).unwrap();

        assert_eq!(challenge.algorithm, DigestAlgorithm::SHA256);
    }

    #[test]
    fn test_digest_response_generation() {
        let auth = DigestAuth::new("user", "password");
        let challenge = DigestChallenge {
            realm: "testrealm@host.com".to_string(),
            nonce: "dcd98b7102dd2f0e8b11d0f600bfb0c093".to_string(),
            algorithm: DigestAlgorithm::MD5,
            qop: Some("auth".to_string()),
            opaque: Some("5ccc069c403ebaf9f0171e9517f40e41".to_string()),
            stale: false,
            domain: None,
        };

        let response = auth.respond_to_challenge(&challenge, "GET", "/dir/index.html");
        assert!(response.is_ok());

        let auth_header = response.unwrap();
        assert!(auth_header.contains("Digest username=\"user\""));
        assert!(auth_header.contains("realm=\"testrealm@host.com\""));
        assert!(auth_header.contains("response="));
        assert!(auth_header.contains("qop=auth"));
        assert!(auth_header.contains("nc=00000001"));
        assert!(auth_header.contains("cnonce="));
    }

    #[test]
    fn test_md5_hash() {
        // Test vector from RFC 7616
        let result = hash(DigestAlgorithm::MD5, "test");
        assert_eq!(result, "098f6bcd4621d373cade4e832627b4f6");
    }

    #[test]
    fn test_sha256_hash() {
        let result = hash(DigestAlgorithm::SHA256, "test");
        assert_eq!(result, "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08");
    }

    #[test]
    fn test_nonce_count_increment() {
        let auth = DigestAuth::new("user", "pass");
        let challenge = DigestChallenge {
            realm: "test".to_string(),
            nonce: "abc".to_string(),
            algorithm: DigestAlgorithm::MD5,
            qop: Some("auth".to_string()),
            opaque: None,
            stale: false,
            domain: None,
        };

        let resp1 = auth.respond_to_challenge(&challenge, "GET", "/").unwrap();
        assert!(resp1.contains("nc=00000001"));

        let resp2 = auth.respond_to_challenge(&challenge, "GET", "/").unwrap();
        assert!(resp2.contains("nc=00000002"));
    }
}
