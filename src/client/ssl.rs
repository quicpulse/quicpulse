//! SSL/TLS configuration for HTTPS connections
//!
//! This module handles SSL version selection, certificate loading, and cipher configuration.
//!
//! # SSL Version Selection
//!
//! Use `--ssl <VERSION>` to specify the minimum TLS version:
//! - `tls1.2` - TLS 1.2 (recommended minimum)
//! - `tls1.3` - TLS 1.3 only
//!
//! Note: TLS versions below 1.2 are not supported by the underlying TLS library (rustls).
//!
//! # Certificate Verification
//!
//! Use `--verify <VALUE>` to control server certificate verification:
//! - `yes` (default) - Verify server certificates using system CA store
//! - `no` - Disable verification (insecure, for testing only)
//! - `/path/to/ca-bundle.pem` - Use a custom CA bundle file
//!
//! # Client Certificates
//!
//! Use `--cert`, `--cert-key`, and `--cert-key-pass` for client certificate authentication:
//! - `--cert /path/to/cert.pem` - Client certificate file
//! - `--cert-key /path/to/key.pem` - Private key file (if separate from cert)
//! - `--cert-key-pass <PASSWORD>` - Password for encrypted private key
//!
//! # Cipher Suites
//!
//! Note: Custom cipher suite selection (`--ciphers`) has limited support with rustls.
//! The default cipher suites provided by rustls are secure and recommended for most use cases.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::errors::QuicpulseError;

/// TLS protocol versions
///
/// Only TLS 1.2 and 1.3 are supported by the underlying TLS library (rustls).
/// Earlier versions (TLS 1.0, 1.1, SSL) are deprecated and not available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TlsVersion {
    /// Auto-negotiate (use system defaults)
    #[default]
    Auto,
    /// TLS 1.2 (minimum supported)
    Tls1_2,
    /// TLS 1.3 (latest)
    Tls1_3,
}

impl TlsVersion {
    /// Parse a TLS version string
    ///
    /// Accepts modern names and legacy aliases for compatibility.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            // Modern names
            "auto" => Some(TlsVersion::Auto),
            "tls1.2" | "tlsv1.2" => Some(TlsVersion::Tls1_2),
            "tls1.3" | "tlsv1.3" => Some(TlsVersion::Tls1_3),
            // Legacy aliases (mapped to Auto since rustls uses TLS 1.2+ anyway)
            "ssl2.3" | "ssl23" | "tls1" | "tls1.0" | "tlsv1" | "tls1.1" | "tlsv1.1" => {
                Some(TlsVersion::Auto)
            }
            _ => None,
        }
    }

    /// Get the minimum TLS version for reqwest
    pub fn min_tls_version(&self) -> Option<reqwest::tls::Version> {
        match self {
            TlsVersion::Auto => None, // Let reqwest use its defaults
            TlsVersion::Tls1_2 => Some(reqwest::tls::Version::TLS_1_2),
            TlsVersion::Tls1_3 => Some(reqwest::tls::Version::TLS_1_3),
        }
    }
}

/// Type alias for backward compatibility
#[deprecated(since = "0.1.0", note = "use TlsVersion instead")]
pub type SslVersion = TlsVersion;

/// Client certificate configuration
#[derive(Debug, Clone, Default)]
pub struct Certificate {
    /// Path to the certificate file (PEM or PKCS#12)
    pub cert_file: Option<PathBuf>,
    /// Path to the private key file (PEM)
    pub key_file: Option<PathBuf>,
    /// Password for encrypted key file
    pub key_password: Option<String>,
}

impl Certificate {
    /// Create a new empty certificate configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any certificate is configured
    pub fn is_configured(&self) -> bool {
        self.cert_file.is_some()
    }

    /// Load the certificate for reqwest
    ///
    /// Returns the certificate identity if configured.
    /// If the key is encrypted and no password is provided, prompts the user interactively.
    pub fn load_identity(&self) -> Result<Option<reqwest::Identity>, QuicpulseError> {
        let cert_path = match &self.cert_file {
            Some(path) => path,
            None => return Ok(None),
        };

        // Read certificate file
        let cert_data = fs::read(cert_path)
            .map_err(|e| QuicpulseError::Ssl(format!("Failed to read certificate '{}': {}", cert_path.display(), e)))?;

        // Check if it's a PKCS#12 file
        if is_pkcs12(&cert_data) || cert_path.extension().map(|e| e == "p12" || e == "pfx").unwrap_or(false) {
            // PKCS#12 format - not supported without native-tls
            return Err(QuicpulseError::Ssl(
                "PKCS#12 certificates are not supported. Use PEM format instead.".to_string()
            ));
        }

        // PEM format - need to combine cert and key
        if let Some(key_path) = &self.key_file {
            let key_data = fs::read(key_path)
                .map_err(|e| QuicpulseError::Ssl(format!("Failed to read key file '{}': {}", key_path.display(), e)))?;

            // Check if key is encrypted
            if is_key_encrypted(&key_data) {
                let password = match &self.key_password {
                    Some(p) => p.clone(),
                    None => prompt_password(&format!("Enter passphrase for '{}': ", key_path.display()))?,
                };

                // Decrypt the key using rustls-pemfile
                let decrypted_key = decrypt_pem_key(&key_data, &password)?;

                // Combine PEM cert and decrypted key
                let mut combined = cert_data.clone();
                combined.extend_from_slice(b"\n");
                combined.extend_from_slice(&decrypted_key);

                let identity = reqwest::Identity::from_pem(&combined)
                    .map_err(|e| QuicpulseError::Ssl(format!("Failed to load PEM identity: {}", e)))?;
                return Ok(Some(identity));
            }

            // Combine PEM cert and key
            let mut combined = cert_data.clone();
            combined.extend_from_slice(b"\n");
            combined.extend_from_slice(&key_data);

            let identity = reqwest::Identity::from_pem(&combined)
                .map_err(|e| QuicpulseError::Ssl(format!("Failed to load PEM identity: {}", e)))?;
            return Ok(Some(identity));
        }

        // Try loading cert alone (might include key)
        let identity = reqwest::Identity::from_pem(&cert_data)
            .map_err(|e| QuicpulseError::Ssl(format!("Failed to load PEM certificate: {}", e)))?;
        Ok(Some(identity))
    }
}

/// Check if data appears to be PKCS#12 format
fn is_pkcs12(data: &[u8]) -> bool {
    // PKCS#12 files start with specific ASN.1 sequence
    data.len() > 2 && data[0] == 0x30
}

/// Check if a PEM key file is encrypted
fn is_key_encrypted(data: &[u8]) -> bool {
    // Look for "Proc-Type: 4,ENCRYPTED" or "ENCRYPTED" in the PEM data
    if let Ok(text) = std::str::from_utf8(data) {
        text.contains("ENCRYPTED")
    } else {
        false
    }
}

/// Prompt for a password interactively
fn prompt_password(prompt: &str) -> Result<String, QuicpulseError> {
    // Print prompt to stderr (so it doesn't interfere with stdout output)
    eprint!("{}", prompt);
    io::stderr().flush().map_err(|e| QuicpulseError::Io(e))?;

    // Read password without echoing
    rpassword::read_password()
        .map_err(|e| QuicpulseError::Ssl(format!("Failed to read password: {}", e)))
}

/// Decrypt an encrypted PEM private key
///
/// Note: This is a simplified implementation. For full support of encrypted keys,
/// consider using the `openssl` crate or similar.
fn decrypt_pem_key(_encrypted_data: &[u8], _password: &str) -> Result<Vec<u8>, QuicpulseError> {
    // rustls-pemfile doesn't support encrypted keys directly
    // For now, we provide a helpful error message
    //
    // Future enhancement: Use the `openssl` crate for decryption:
    // ```
    // use openssl::pkcs8::EncryptedPrivateKeyInfo;
    // let decrypted = EncryptedPrivateKeyInfo::from_pem(encrypted_data)?
    //     .decrypt(password)?;
    // ```

    Err(QuicpulseError::Ssl(
        "Encrypted PEM private keys are not fully supported yet.\n\
         Workarounds:\n\
         1. Convert to unencrypted PEM: openssl rsa -in encrypted.key -out decrypted.key\n\
         2. Use PKCS#12 format: openssl pkcs12 -export -in cert.pem -inkey key.pem -out cert.p12\n\
         3. Provide an unencrypted key file".to_string()
    ))
}

/// Load multiple CA certificates from a PEM bundle file
fn load_ca_bundle(path: &Path) -> Result<Vec<reqwest::Certificate>, QuicpulseError> {
    let ca_data = fs::read(path)
        .map_err(|e| QuicpulseError::Ssl(format!("Failed to read CA bundle '{}': {}", path.display(), e)))?;

    let mut certs = Vec::new();

    // Use rustls-pemfile to parse multiple certificates
    let mut reader = std::io::BufReader::new(ca_data.as_slice());

    for cert_result in rustls_pemfile::certs(&mut reader) {
        match cert_result {
            Ok(cert) => {
                let reqwest_cert = reqwest::Certificate::from_der(&cert)
                    .map_err(|e| QuicpulseError::Ssl(format!("Failed to parse CA certificate: {}", e)))?;
                certs.push(reqwest_cert);
            }
            Err(e) => {
                return Err(QuicpulseError::Ssl(format!("Failed to parse CA bundle: {}", e)));
            }
        }
    }

    if certs.is_empty() {
        // Try loading as a single PEM certificate
        let cert = reqwest::Certificate::from_pem(&ca_data)
            .map_err(|e| QuicpulseError::Ssl(format!("Failed to parse CA bundle as PEM: {}", e)))?;
        certs.push(cert);
    }

    Ok(certs)
}

/// SSL/TLS configuration options
#[derive(Debug, Clone, Default)]
pub struct SslConfig {
    /// Minimum TLS version
    pub version: Option<TlsVersion>,
    /// Custom ciphers string
    pub ciphers: Option<String>,
    /// Whether to verify server certificates
    pub verify: bool,
    /// Custom CA bundle path
    pub ca_bundle: Option<PathBuf>,
    /// Client certificate
    pub client_cert: Certificate,
}

impl SslConfig {
    /// Create a new default SSL config (verify enabled)
    pub fn new() -> Self {
        Self {
            verify: true,
            ..Default::default()
        }
    }

    /// Create SSL config from CLI arguments
    pub fn from_args(
        verify: &str,
        ssl_version: Option<&str>,
        ciphers: Option<&str>,
        cert: Option<&str>,
        cert_key: Option<&str>,
        cert_key_pass: Option<&str>,
    ) -> Self {
        let mut config = Self::new();

        // Parse verify option
        config.verify = match verify.to_lowercase().as_str() {
            "no" | "false" | "0" => false,
            _ if Path::new(verify).exists() => {
                config.ca_bundle = Some(PathBuf::from(verify));
                true
            }
            _ => true,
        };

        // Parse TLS version
        if let Some(ver) = ssl_version {
            config.version = TlsVersion::parse(ver);
        }

        // Set ciphers
        config.ciphers = ciphers.map(|s| s.to_string());

        // Set client certificate
        if let Some(cert_path) = cert {
            config.client_cert.cert_file = Some(PathBuf::from(cert_path));
        }
        if let Some(key_path) = cert_key {
            config.client_cert.key_file = Some(PathBuf::from(key_path));
        }
        if let Some(pass) = cert_key_pass {
            config.client_cert.key_password = Some(pass.to_string());
        }

        config
    }

    /// Apply SSL config to a reqwest ClientBuilder
    pub fn apply_to_builder(
        &self,
        mut builder: reqwest::ClientBuilder,
    ) -> Result<reqwest::ClientBuilder, QuicpulseError> {
        // Set minimum TLS version
        if let Some(version) = &self.version {
            if let Some(min_version) = version.min_tls_version() {
                builder = builder.min_tls_version(min_version);
            }
        }

        // Set certificate verification
        if !self.verify {
            builder = builder.danger_accept_invalid_certs(true);
        }

        // Add custom CA bundle (supports multiple certificates)
        if let Some(ca_path) = &self.ca_bundle {
            let certs = load_ca_bundle(ca_path)?;
            for cert in certs {
                builder = builder.add_root_certificate(cert);
            }
        }

        // Add client certificate
        if let Some(identity) = self.client_cert.load_identity()? {
            builder = builder.identity(identity);
        }

        // Note: Custom ciphers are not directly supported by reqwest with rustls
        // If ciphers are specified, log a warning
        if let Some(ciphers) = &self.ciphers {
            eprintln!(
                "Warning: Custom cipher suite '{}' specified, but rustls uses its own \
                 secure defaults. Custom ciphers are ignored.",
                ciphers
            );
        }

        Ok(builder)
    }
}

/// Get the default cipher suites used by rustls
///
/// Note: Unlike OpenSSL, rustls has a fixed set of secure cipher suites
/// that cannot be customized at runtime.
pub fn get_default_ciphers() -> Vec<&'static str> {
    vec![
        // TLS 1.3 cipher suites
        "TLS_AES_256_GCM_SHA384",
        "TLS_AES_128_GCM_SHA256",
        "TLS_CHACHA20_POLY1305_SHA256",
        // TLS 1.2 cipher suites
        "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384",
        "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384",
        "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
        "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256",
        "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256",
        "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_version_parse() {
        // Modern formats
        assert_eq!(TlsVersion::parse("tls1.2"), Some(TlsVersion::Tls1_2));
        assert_eq!(TlsVersion::parse("TLS1.3"), Some(TlsVersion::Tls1_3));
        assert_eq!(TlsVersion::parse("auto"), Some(TlsVersion::Auto));

        // Alternative formats
        assert_eq!(TlsVersion::parse("tlsv1.2"), Some(TlsVersion::Tls1_2));
        assert_eq!(TlsVersion::parse("tlsv1.3"), Some(TlsVersion::Tls1_3));

        // Legacy aliases map to Auto
        assert_eq!(TlsVersion::parse("ssl2.3"), Some(TlsVersion::Auto));
        assert_eq!(TlsVersion::parse("ssl23"), Some(TlsVersion::Auto));
        assert_eq!(TlsVersion::parse("tls1"), Some(TlsVersion::Auto));
        assert_eq!(TlsVersion::parse("tls1.0"), Some(TlsVersion::Auto));
        assert_eq!(TlsVersion::parse("tls1.1"), Some(TlsVersion::Auto));

        // Invalid
        assert_eq!(TlsVersion::parse("invalid"), None);
        assert_eq!(TlsVersion::parse(""), None);
        assert_eq!(TlsVersion::parse("ssl3"), None);
    }

    #[test]
    fn test_tls_version_min_tls() {
        // TLS 1.2 and 1.3 return specific versions
        assert_eq!(TlsVersion::Tls1_2.min_tls_version(), Some(reqwest::tls::Version::TLS_1_2));
        assert_eq!(TlsVersion::Tls1_3.min_tls_version(), Some(reqwest::tls::Version::TLS_1_3));

        // Auto returns None (let reqwest decide)
        assert_eq!(TlsVersion::Auto.min_tls_version(), None);
    }

    #[test]
    fn test_is_key_encrypted() {
        // Encrypted key markers
        let encrypted1 = b"-----BEGIN RSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\n";
        let encrypted2 = b"-----BEGIN ENCRYPTED PRIVATE KEY-----\n";

        assert!(is_key_encrypted(encrypted1));
        assert!(is_key_encrypted(encrypted2));

        // Unencrypted keys
        let unencrypted = b"-----BEGIN RSA PRIVATE KEY-----\nMIIE...";
        let pkcs8 = b"-----BEGIN PRIVATE KEY-----\nMIIE...";

        assert!(!is_key_encrypted(unencrypted));
        assert!(!is_key_encrypted(pkcs8));

        // Binary data (not encrypted text)
        let binary = &[0x30, 0x82, 0x01, 0x00];
        assert!(!is_key_encrypted(binary));
    }

    #[test]
    fn test_ssl_config_verify() {
        // Enable verification
        let config = SslConfig::from_args("yes", None, None, None, None, None);
        assert!(config.verify);
        assert!(config.ca_bundle.is_none());

        let config = SslConfig::from_args("true", None, None, None, None, None);
        assert!(config.verify);

        let config = SslConfig::from_args("1", None, None, None, None, None);
        assert!(config.verify);

        // Disable verification
        let config = SslConfig::from_args("no", None, None, None, None, None);
        assert!(!config.verify);

        let config = SslConfig::from_args("false", None, None, None, None, None);
        assert!(!config.verify);

        let config = SslConfig::from_args("0", None, None, None, None, None);
        assert!(!config.verify);
    }

    #[test]
    fn test_ssl_config_with_tls_version() {
        let config = SslConfig::from_args("yes", Some("tls1.2"), None, None, None, None);
        assert_eq!(config.version, Some(TlsVersion::Tls1_2));

        let config = SslConfig::from_args("yes", Some("tls1.3"), None, None, None, None);
        assert_eq!(config.version, Some(TlsVersion::Tls1_3));

        // Legacy versions map to Auto
        let config = SslConfig::from_args("yes", Some("tls1.1"), None, None, None, None);
        assert_eq!(config.version, Some(TlsVersion::Auto));

        // Invalid version should result in None
        let config = SslConfig::from_args("yes", Some("invalid"), None, None, None, None);
        assert_eq!(config.version, None);
    }

    #[test]
    fn test_ssl_config_with_ciphers() {
        let config = SslConfig::from_args("yes", None, Some("AES256-SHA"), None, None, None);
        assert_eq!(config.ciphers, Some("AES256-SHA".to_string()));
    }

    #[test]
    fn test_ssl_config_with_client_cert() {
        let config = SslConfig::from_args(
            "yes",
            None,
            None,
            Some("/path/to/cert.pem"),
            Some("/path/to/key.pem"),
            Some("password123"),
        );

        assert!(config.client_cert.is_configured());
        assert_eq!(
            config.client_cert.cert_file,
            Some(PathBuf::from("/path/to/cert.pem"))
        );
        assert_eq!(
            config.client_cert.key_file,
            Some(PathBuf::from("/path/to/key.pem"))
        );
        assert_eq!(
            config.client_cert.key_password,
            Some("password123".to_string())
        );
    }

    #[test]
    fn test_certificate_not_configured() {
        let cert = Certificate::new();
        assert!(!cert.is_configured());

        // Load identity should return None when not configured
        let result = cert.load_identity();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_is_pkcs12() {
        // PKCS#12 starts with ASN.1 SEQUENCE tag
        let pkcs12_header = &[0x30, 0x82, 0x0A, 0x00];
        assert!(is_pkcs12(pkcs12_header));

        // PEM file starts with text
        let pem_header = b"-----BEGIN";
        assert!(!is_pkcs12(pem_header));

        // Empty or too short
        assert!(!is_pkcs12(&[]));
        assert!(!is_pkcs12(&[0x30]));
    }

    #[test]
    fn test_get_default_ciphers() {
        let ciphers = get_default_ciphers();

        // Should have both TLS 1.3 and TLS 1.2 ciphers
        assert!(ciphers.iter().any(|c| c.contains("TLS_AES_256_GCM")));
        assert!(ciphers.iter().any(|c| c.contains("ECDHE")));
        assert!(ciphers.iter().any(|c| c.contains("CHACHA20")));

        // Should not be empty
        assert!(!ciphers.is_empty());
    }

    #[test]
    fn test_ssl_config_default() {
        let config = SslConfig::new();
        assert!(config.verify);
        assert!(config.ca_bundle.is_none());
        assert!(config.version.is_none());
        assert!(config.ciphers.is_none());
        assert!(!config.client_cert.is_configured());
    }
}
