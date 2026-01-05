//! HTTP method constants and utilities

/// HTTP GET method
pub const GET: &str = "GET";

/// HTTP POST method
pub const POST: &str = "POST";

/// HTTP PUT method
pub const PUT: &str = "PUT";

/// HTTP PATCH method
pub const PATCH: &str = "PATCH";

/// HTTP DELETE method
pub const DELETE: &str = "DELETE";

/// HTTP HEAD method
pub const HEAD: &str = "HEAD";

/// HTTP OPTIONS method
pub const OPTIONS: &str = "OPTIONS";

/// HTTP TRACE method
pub const TRACE: &str = "TRACE";

/// HTTP CONNECT method
pub const CONNECT: &str = "CONNECT";

/// All standard HTTP methods
pub const STANDARD_METHODS: &[&str] = &[
    GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, TRACE, CONNECT,
];

/// Check if a string is a standard HTTP method
pub fn is_standard(method: &str) -> bool {
    STANDARD_METHODS.iter().any(|&m| m.eq_ignore_ascii_case(method))
}

/// Check if a string looks like an HTTP method (all uppercase, reasonable length)
pub fn looks_like_method(s: &str) -> bool {
    if s.is_empty() || s.len() > 10 {
        return false;
    }

    // Must be all ASCII uppercase letters
    if !s.chars().all(|c| c.is_ascii_uppercase()) {
        return false;
    }

    // Exclude common hostnames that might be uppercase
    let upper = s.to_uppercase();
    if upper == "LOCALHOST" || upper == "HOST" || upper == "SERVER" {
        return false;
    }

    true
}

/// Infer HTTP method based on whether the request has data
pub fn infer(has_data: bool) -> &'static str {
    if has_data {
        POST
    } else {
        GET
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_standard() {
        assert!(is_standard("GET"));
        assert!(is_standard("get"));
        assert!(is_standard("Post"));
        assert!(!is_standard("INVALID"));
        assert!(!is_standard("CUSTOM"));
    }

    #[test]
    fn test_looks_like_method() {
        assert!(looks_like_method("GET"));
        assert!(looks_like_method("POST"));
        assert!(looks_like_method("CUSTOM"));
        assert!(!looks_like_method("get")); // lowercase
        assert!(!looks_like_method("Get")); // mixed case
        assert!(!looks_like_method("LOCALHOST")); // common hostname
        assert!(!looks_like_method("")); // empty
        assert!(!looks_like_method("VERYLONGMETHODNAME")); // too long
    }

    #[test]
    fn test_infer() {
        assert_eq!(infer(false), GET);
        assert_eq!(infer(true), POST);
    }
}
