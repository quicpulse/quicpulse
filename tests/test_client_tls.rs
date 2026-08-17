//! Unit and integration tests for Client TLS, SSL versioning, and Certificate types

use quicpulse::client::ssl::{Certificate, TlsVersion};
use std::path::PathBuf;

#[test]
fn test_tls_version_parsing() {
    assert_eq!(TlsVersion::parse("auto"), Some(TlsVersion::Auto));
    assert_eq!(TlsVersion::parse("tls1.2"), Some(TlsVersion::Tls1_2));
    assert_eq!(TlsVersion::parse("tlsv1.2"), Some(TlsVersion::Tls1_2));
    assert_eq!(TlsVersion::parse("TLS1.2"), Some(TlsVersion::Tls1_2));
    assert_eq!(TlsVersion::parse("tls1.3"), Some(TlsVersion::Tls1_3));
    assert_eq!(TlsVersion::parse("tlsv1.3"), Some(TlsVersion::Tls1_3));

    // Legacy aliases
    assert_eq!(TlsVersion::parse("ssl2.3"), Some(TlsVersion::Auto));
    assert_eq!(TlsVersion::parse("tls1.0"), Some(TlsVersion::Auto));
    assert_eq!(TlsVersion::parse("tls1.1"), Some(TlsVersion::Auto));

    // Invalid strings
    assert_eq!(TlsVersion::parse("invalid"), None);
    assert_eq!(TlsVersion::parse("tls2.0"), None);
}

#[test]
fn test_min_tls_version_conversion() {
    assert_eq!(TlsVersion::Auto.min_tls_version(), None);
    assert_eq!(
        TlsVersion::Tls1_2.min_tls_version(),
        Some(reqwest::tls::Version::TLS_1_2)
    );
    assert_eq!(
        TlsVersion::Tls1_3.min_tls_version(),
        Some(reqwest::tls::Version::TLS_1_3)
    );
}

#[test]
fn test_certificate_configuration() {
    let mut cert = Certificate::new();
    assert!(cert.cert_file.is_none());
    assert!(cert.key_file.is_none());
    assert!(cert.key_password.is_none());

    cert.cert_file = Some(PathBuf::from("/etc/ssl/cert.pem"));
    cert.key_file = Some(PathBuf::from("/etc/ssl/key.pem"));
    cert.key_password = Some("secret".to_string());

    let cloned = cert.clone();
    assert_eq!(cloned.cert_file, Some(PathBuf::from("/etc/ssl/cert.pem")));
    assert_eq!(cloned.key_file, Some(PathBuf::from("/etc/ssl/key.pem")));
    assert_eq!(cloned.key_password, Some("secret".to_string()));
}
