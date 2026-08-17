//! Unit tests for system utilities: cookies, filesystem, strings, mime, and signals

use quicpulse::cookies::{current_timestamp, is_cookie_expired, split_cookies};
use quicpulse::fs::{get_filename_from_content_disposition, sanitize_filename};
use quicpulse::mime::{get_content_type, parse_content_type_header};
use quicpulse::signals::{reset_interrupted, set_interrupted, was_interrupted};
use quicpulse::strings::{is_version_greater, truncate_str};

#[test]
fn test_cookie_utilities() {
    let now = current_timestamp();
    assert!(now > 0);

    assert!(is_cookie_expired(Some(now - 100)));
    assert!(!is_cookie_expired(Some(now + 1000)));
    assert!(!is_cookie_expired(None)); // Session cookie

    let cookie_header =
        "session=xyz123; Path=/, token=abc456; Expires=Wed, 21 Oct 2026 07:28:00 GMT; Secure";
    let cookies = split_cookies(cookie_header);
    assert_eq!(cookies.len(), 2);
    assert!(cookies[0].starts_with("session=xyz123"));
    assert!(cookies[1].starts_with("token=abc456"));
}

#[test]
fn test_fs_utilities() {
    let cd_header = "attachment; filename=\"invoice_2026.pdf\"";
    assert_eq!(
        get_filename_from_content_disposition(cd_header),
        Some("invoice_2026.pdf".to_string())
    );

    let unsafe_name = "reports/../../etc/passwd:*?.txt";
    let sanitized = sanitize_filename(unsafe_name);
    assert!(!sanitized.contains('/'));
    assert!(!sanitized.contains(':'));
    assert!(!sanitized.contains('*'));
}

#[test]
fn test_strings_utilities() {
    assert_eq!(truncate_str("hello world", 8), "hello...");
    assert_eq!(truncate_str("short", 10), "short");
    assert_eq!(truncate_str("hello", 2), "...");

    assert!(is_version_greater("1.2.0", "1.1.9"));
    assert!(is_version_greater("2.0.0", "1.9.9"));
    assert!(!is_version_greater("1.0.0", "1.0.0"));
    assert!(!is_version_greater("0.9.0", "1.0.0"));
}

#[test]
fn test_mime_utilities() {
    let (mime, params) =
        parse_content_type_header("application/json; charset=UTF-8; boundary=something");
    assert_eq!(mime, "application/json");
    assert_eq!(params.get("charset"), Some(&"utf-8".to_string()));

    assert_eq!(
        get_content_type("data.json"),
        Some("application/json".to_string())
    );
    assert_eq!(get_content_type("image.png"), Some("image/png".to_string()));
    assert_eq!(get_content_type("page.html"), Some("text/html".to_string()));
}

#[test]
fn test_signals_lifecycle() {
    reset_interrupted();
    assert!(!was_interrupted());

    set_interrupted();
    assert!(was_interrupted());

    reset_interrupted();
    assert!(!was_interrupted());
}
