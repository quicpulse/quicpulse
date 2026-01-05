//! Cookie handling for sessions

use crate::sessions::session::SessionCookie;
use crate::utils::is_localhost;

/// Cookie policy for session management
#[derive(Debug, Clone, Default)]
pub struct CookiePolicy {
    /// Whether to accept cookies
    pub accept_cookies: bool,
    /// Whether to send cookies
    pub send_cookies: bool,
}

impl CookiePolicy {
    /// Default policy: accept and send cookies
    pub fn default_policy() -> Self {
        Self {
            accept_cookies: true,
            send_cookies: true,
        }
    }

    /// Ignore all cookies
    pub fn ignore() -> Self {
        Self {
            accept_cookies: false,
            send_cookies: false,
        }
    }
}

/// Check if a cookie should be sent for a given request
pub fn should_send_cookie(
    cookie: &SessionCookie,
    domain: &str,
    path: &str,
    is_secure: bool,
) -> bool {
    // Check domain
    let domain_match = cookie.domain.as_ref()
        .map(|d| domain_matches(domain, d))
        .unwrap_or(true);

    // Check path
    let path_match = cookie.path.as_ref()
        .map(|p| path.starts_with(p))
        .unwrap_or(true);

    // Check secure - localhost is treated as secure context
    let secure_match = !cookie.secure || is_secure || is_localhost(domain);

    domain_match && path_match && secure_match
}

/// Check if a request domain matches a cookie domain
///
/// Handles the leading dot in cookie domains per RFC 6265.
/// Example: "api.example.com" matches ".example.com"
pub fn domain_matches(request_domain: &str, cookie_domain: &str) -> bool {
    let cookie_domain = cookie_domain.trim_start_matches('.');

    request_domain == cookie_domain
        || request_domain.ends_with(&format!(".{}", cookie_domain))
}
