//! Utility functions
//!
//! This module re-exports utilities from specialized submodules for
//! backward compatibility. New code should import from the specific modules.

// Re-export from specialized modules for backward compatibility
pub use crate::signals::{was_interrupted, set_interrupted, reset_interrupted};
pub use crate::binary::{is_binary, format_bytes};
pub use crate::cookies::{split_cookies, parse_set_cookie_header, ExpiredCookie, is_cookie_expired, current_timestamp};
pub use crate::fs::{get_filename_from_content_disposition, sanitize_filename};
pub use crate::json::load_json_preserve_order;
pub use crate::mime::{parse_content_type_header, get_content_type};
pub use crate::strings::{truncate_str, is_version_greater};

use std::net::IpAddr;
use url::Url;

/// Extract host from URL, stripping user info
///
/// # Examples
/// ```
/// use quicpulse::utils::url_as_host;
/// assert_eq!(url_as_host("https://user:pass@example.com:8080/path"), "example.com:8080");
/// assert_eq!(url_as_host("http://example.com"), "example.com");
/// ```
pub fn url_as_host(url_str: &str) -> String {
    if let Ok(url) = Url::parse(url_str) {
        let host = url.host_str().unwrap_or("");
        match url.port() {
            Some(port) => format!("{}:{}", host, port),
            None => host.to_string(),
        }
    } else {
        url_str.to_string()
    }
}

/// Marker type to disable .netrc authentication lookup
#[derive(Debug, Clone, Default)]
pub struct DisableNetrcAuth;

/// Check if domain is localhost (Firefox-style secure context)
///
/// Uses `IpAddr::is_loopback()` for proper IP address detection,
/// and hostname pattern matching for localhost domains.
///
/// # Examples
/// ```
/// use quicpulse::utils::is_localhost;
/// assert!(is_localhost("localhost"));
/// assert!(is_localhost("app.localhost"));
/// assert!(is_localhost("127.0.0.1"));
/// assert!(is_localhost("::1"));
/// assert!(is_localhost("127.0.0.255"));  // Any 127.x.x.x is loopback
/// assert!(!is_localhost("example.com"));
/// ```
pub fn is_localhost(domain: &str) -> bool {
    // Try parsing as an IP address first
    if let Ok(ip) = domain.parse::<IpAddr>() {
        return ip.is_loopback();
    }

    // For hostnames, check the standard localhost convention
    domain == "localhost" || domain.ends_with(".localhost")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_as_host() {
        assert_eq!(url_as_host("https://example.com/path"), "example.com");
        assert_eq!(url_as_host("https://user:pass@example.com:8080/"), "example.com:8080");
    }

    #[test]
    fn test_is_localhost() {
        // Standard localhost
        assert!(is_localhost("localhost"));

        // Subdomains of localhost (per RFC 6761)
        assert!(is_localhost("app.localhost"));
        assert!(is_localhost("api.dev.localhost"));

        // IPv4 loopback - entire 127.0.0.0/8 block
        assert!(is_localhost("127.0.0.1"));
        assert!(is_localhost("127.0.0.255"));
        assert!(is_localhost("127.255.255.255"));

        // IPv6 loopback
        assert!(is_localhost("::1"));

        // Not localhost
        assert!(!is_localhost("example.com"));
        assert!(!is_localhost("mylocalhost.com"));
        assert!(!is_localhost("192.168.1.1"));
        assert!(!is_localhost("10.0.0.1"));
        assert!(!is_localhost("::2"));
    }
}
