use quicpulse::cli::args::{SecretString, SensitiveUrl};
use std::str::FromStr;

#[test]
fn test_secret_string_empty() {
    let empty = SecretString::default();
    assert_eq!(empty.as_str(), "");
    assert_eq!(format!("{}", empty), "");
    assert_eq!(format!("{:?}", empty), "SecretString(\"\")");
    assert_eq!(empty.into_inner(), "");
}

#[test]
fn test_secret_string_redaction() {
    let secret = SecretString::from("super_secret_password_123".to_string());
    assert_eq!(secret.as_str(), "super_secret_password_123");
    assert_eq!(&*secret, "super_secret_password_123");
    assert_eq!(format!("{}", secret), "[REDACTED]");
    assert_eq!(format!("{:?}", secret), "SecretString(\"[REDACTED]\")");
    assert_eq!(secret.into_inner(), "super_secret_password_123");
}

#[test]
fn test_secret_string_traits() {
    let secret = SecretString::from_str("my_token").unwrap();
    assert_eq!(secret.as_ref(), "my_token");
    let cloned = secret.clone();
    assert_eq!(cloned.as_str(), "my_token");
}

#[test]
fn test_sensitive_url_empty() {
    let empty = SensitiveUrl::default();
    assert_eq!(empty.as_str(), "");
    assert_eq!(format!("{}", empty), "");
    assert_eq!(format!("{:?}", empty), "SensitiveUrl(\"\")");
    assert_eq!(empty.into_inner(), "");
}

#[test]
fn test_sensitive_url_with_credentials() {
    let url_with_auth =
        SensitiveUrl::from("http://admin:secret123@example.com:8080/path?foo=bar".to_string());
    assert_eq!(
        url_with_auth.as_str(),
        "http://admin:secret123@example.com:8080/path?foo=bar"
    );
    assert_eq!(
        &*url_with_auth,
        "http://admin:secret123@example.com:8080/path?foo=bar"
    );

    // Display and Debug should redact username/password
    let display_str = format!("{}", url_with_auth);
    assert!(!display_str.contains("secret123"));
    assert!(display_str.contains("REDACTED"));
    assert!(display_str.contains("example.com:8080/path"));

    let debug_str = format!("{:?}", url_with_auth);
    assert!(!debug_str.contains("secret123"));
    assert!(debug_str.contains("SensitiveUrl("));
    assert!(debug_str.contains("REDACTED"));
}

#[test]
fn test_sensitive_url_without_credentials() {
    let plain_url = SensitiveUrl::from_str("https://api.github.com/repos").unwrap();
    assert_eq!(plain_url.as_str(), "https://api.github.com/repos");
    assert_eq!(format!("{}", plain_url), "https://api.github.com/repos");
    assert_eq!(
        format!("{:?}", plain_url),
        "SensitiveUrl(\"https://api.github.com/repos\")"
    );
}

#[test]
fn test_sensitive_url_invalid_url_fallback() {
    let invalid_url = SensitiveUrl::from("not a valid url".to_string());
    assert_eq!(invalid_url.as_ref(), "not a valid url");
    assert_eq!(format!("{}", invalid_url), "not a valid url");
    assert_eq!(
        format!("{:?}", invalid_url),
        "SensitiveUrl(\"not a valid url\")"
    );
}
