//! Cookie utilities
//!
//! Functions for cookie parsing and expiration handling.

use cookie::Cookie;
use std::time::{SystemTime, UNIX_EPOCH};

/// Expired cookie info
#[derive(Debug, Clone)]
pub struct ExpiredCookie {
    pub name: String,
    pub domain: Option<String>,
    pub path: Option<String>,
}

/// Get current Unix timestamp
pub fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Check if a cookie is expired based on expires timestamp
pub fn is_cookie_expired(expires: Option<i64>) -> bool {
    match expires {
        Some(exp) => exp < current_timestamp(),
        None => false, // Session cookie, not expired
    }
}

/// Split Set-Cookie header value into individual cookies
///
/// Handles the tricky case where cookie values may contain commas
/// (e.g., in Expires date), but cookies are separated by ", name=".
pub fn split_cookies(cookies: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = cookies.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Check for ", " followed by a cookie name pattern (word=)
        if i + 2 < len && chars[i] == ',' && chars[i + 1] == ' ' {
            // Look ahead to see if this looks like a new cookie (token=)
            let rest = &cookies[i + 2..];
            if looks_like_cookie_start(rest) {
                result.push(current.trim().to_string());
                current.clear();
                i += 2; // Skip ", "
                continue;
            }
        }
        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        result.push(current.trim().to_string());
    }

    result
}

/// Parse a Set-Cookie header into typed Cookie structs
///
/// Uses split_cookies() to handle multi-cookie headers, then parses each
/// cookie using the cookie crate for RFC-compliant parsing.
pub fn parse_set_cookie_header(header: &str) -> Vec<Cookie<'static>> {
    split_cookies(header)
        .into_iter()
        .filter_map(|s| Cookie::parse(s).ok())
        .map(|c| c.into_owned())
        .collect()
}

/// Check if string starts with a cookie name pattern (token=)
fn looks_like_cookie_start(s: &str) -> bool {
    let mut chars = s.chars().peekable();

    // Must start with a valid token character
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '-' => {}
        _ => return false,
    }

    // Continue until we find = or something invalid
    for c in chars {
        if c == '=' {
            return true;
        }
        if c == ' ' || c == ';' || c == ',' {
            return false;
        }
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
            return false;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_cookies() {
        let cookies = "session=abc123; Path=/; Secure, tracking=xyz; Path=/";
        let result = split_cookies(cookies);
        assert_eq!(result.len(), 2);
        assert!(result[0].starts_with("session="));
        assert!(result[1].starts_with("tracking="));
    }

    #[test]
    fn test_split_cookies_with_date() {
        // Expires contains a comma in the date
        let cookies = "session=abc; Expires=Mon, 01 Jan 2024 00:00:00 GMT, other=xyz";
        let result = split_cookies(cookies);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_is_cookie_expired() {
        assert!(!is_cookie_expired(None)); // Session cookie
        assert!(is_cookie_expired(Some(0))); // Epoch = expired
        assert!(!is_cookie_expired(Some(i64::MAX))); // Far future = not expired
    }

    #[test]
    fn test_parse_set_cookie_header() {
        let cookies = "session=abc123; Path=/; Secure, tracking=xyz; Path=/";
        let parsed = parse_set_cookie_header(cookies);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name(), "session");
        assert_eq!(parsed[0].value(), "abc123");
        assert_eq!(parsed[1].name(), "tracking");
        assert_eq!(parsed[1].value(), "xyz");
    }
}
