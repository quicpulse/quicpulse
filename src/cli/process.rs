//! Post-processing argument logic
//!
//! This module handles the post-parsing processing of CLI arguments,
//! including URL normalization, method inference, and request item parsing.

use crate::cli::args::Args;
use crate::errors::QuicpulseError;
use crate::http;
use crate::input::InputItem;
use crate::magic::expand_magic_values;
use crate::models::types::RequestType;
use url::Url;

/// Check if a string has a valid URL scheme (e.g., "http://", "https://")
/// Per RFC 3986: scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
fn has_url_scheme(s: &str) -> bool {
    if let Some(pos) = s.find("://") {
        let scheme = &s[..pos];
        !scheme.is_empty()
            && scheme.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
            && scheme.chars().skip(1).all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    } else {
        false
    }
}

/// Check if a string ends with a port number (e.g., ":8080")
fn ends_with_port(s: &str) -> bool {
    if let Some(colon_pos) = s.rfind(':') {
        let port_part = &s[colon_pos + 1..];
        !port_part.is_empty() && port_part.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

/// Parse localhost shorthand (:PORT/path or :/path)
/// Returns (port, rest) if it matches the pattern
fn parse_localhost_shorthand(s: &str) -> Option<(&str, &str)> {
    // Must start with : but not :: (IPv6)
    if !s.starts_with(':') || s.starts_with("::") {
        return None;
    }

    let after_colon = &s[1..];

    // Find where the port ends (at / or end of string)
    let (port, rest) = if let Some(slash_pos) = after_colon.find('/') {
        (&after_colon[..slash_pos], &after_colon[slash_pos..])
    } else {
        (after_colon, "")
    };

    // Port must be all digits (can be empty for just ":/path")
    if port.chars().all(|c| c.is_ascii_digit()) {
        Some((port, rest))
    } else {
        None
    }
}

/// Separator patterns for identifying request items
const ITEM_SEPARATORS: &[&str] = &["==@", ":=@", ":@", "=@", "==", ":=", "@", "=", ":", ";"];

/// Processed arguments ready for request building
#[derive(Debug, Clone)]
pub struct ProcessedArgs {
    /// HTTP method
    pub method: String,
    /// Fully qualified URL
    pub url: String,
    /// Parsed request items
    pub items: Vec<InputItem>,
    /// Request body type
    pub request_type: RequestType,
    /// Whether the request has data
    pub has_data: bool,
}

/// Process raw CLI arguments into a usable form
pub fn process_args(args: &Args) -> Result<ProcessedArgs, QuicpulseError> {
    // 1. Determine request type
    let request_type = determine_request_type(args);

    // Check if something looks like a URL (has scheme or looks like hostname)
    let looks_like_url = |s: &str| -> bool {
        // Has a scheme (http://, https://, etc.)
        if has_url_scheme(s) {
            return true;
        }
        // Localhost shorthand (:3000, :8080/path)
        if parse_localhost_shorthand(s).is_some() {
            return true;
        }
        // Has domain-like pattern before any separator
        if let Some(first_sep_pos) = ITEM_SEPARATORS.iter().filter_map(|sep| s.find(sep)).min() {
            let before_sep = &s[..first_sep_pos];
            if before_sep.contains('.') || ends_with_port(before_sep) {
                return true;
            }
        } else if s.contains('.') || s.starts_with("localhost") {
            return true;
        }
        false
    };

    // Check if a string looks like a request item
    let looks_like_item = |s: &str| -> bool {
        if looks_like_url(s) {
            return false;
        }
        ITEM_SEPARATORS.iter().any(|sep| s.contains(sep))
    };

    // Collect all request items
    let mut all_items: Vec<String> = Vec::new();

    let (method_or_url, actual_url) = match (&args.method, &args.url) {
        (Some(m), Some(u)) => {
            let m_upper = m.to_uppercase();
            if http::is_standard(&m_upper) || http::looks_like_method(&m_upper) {
                (Some(m.clone()), u.clone())
            } else if looks_like_item(u) {
                all_items.push(u.clone());
                (None, m.clone())
            } else {
                all_items.push(u.clone());
                (None, m.clone())
            }
        }
        (Some(u), None) => {
            (None, u.clone())
        }
        (None, _) => {
            return Err(QuicpulseError::Argument("URL is required".to_string()));
        }
    };

    // Add the explicit request items
    all_items.extend(args.request_items.iter().cloned());

    // 2. Parse request items (with magic value expansion)
    let items = parse_request_items(&all_items)?;

    // 3. Check if we have data
    let has_data = items.iter().any(|i| i.is_data()) || args.raw.is_some();

    // 4. Determine method
    let method = if let Some(m) = method_or_url {
        m.to_uppercase()
    } else {
        http::infer(has_data).to_string()
    };

    // 5. Process the URL
    let url = process_url(&actual_url, &args.default_scheme)?;

    // 6. Expand magic values in URL
    let url = expand_magic_values(&url).value;

    Ok(ProcessedArgs {
        method,
        url,
        items,
        request_type,
        has_data,
    })
}

/// Determine the request type from flags
fn determine_request_type(args: &Args) -> RequestType {
    if args.multipart {
        RequestType::Multipart
    } else if args.form {
        RequestType::Form
    } else {
        RequestType::Json
    }
}

/// Parse all request items and expand magic values
fn parse_request_items(items: &[String]) -> Result<Vec<InputItem>, QuicpulseError> {
    items.iter()
        .map(|s| {
            let expanded = expand_magic_values(s);
            InputItem::parse(&expanded.value)
        })
        .collect()
}

/// Process URL: add scheme, handle localhost shorthand
fn process_url(raw_url: &str, default_scheme: &str) -> Result<String, QuicpulseError> {
    let mut url = raw_url.to_string();

    // Handle :// paste shortcut
    if url.starts_with("://") {
        url = url[3..].to_string();
    }

    if !has_url_scheme(&url) {
        // Handle localhost shorthand (:3000/path, :/path)
        if let Some((port, rest)) = parse_localhost_shorthand(&url) {
            url = if port.is_empty() {
                format!("localhost{}", rest)
            } else {
                format!("localhost:{}{}", port, rest)
            };
        }

        url = format!("{}://{}", default_scheme, url);
    }

    Url::parse(&url).map_err(|e| QuicpulseError::Parse(format!("Invalid URL '{}': {}", url, e)))?;

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_url_with_scheme() {
        let url = process_url("https://example.com", "http").unwrap();
        assert_eq!(url, "https://example.com");
    }

    #[test]
    fn test_process_url_without_scheme() {
        let url = process_url("example.com", "http").unwrap();
        assert_eq!(url, "http://example.com");
    }

    #[test]
    fn test_process_url_localhost_shorthand() {
        let url = process_url(":3000/api", "http").unwrap();
        assert_eq!(url, "http://localhost:3000/api");
    }

    #[test]
    fn test_process_url_paste_shortcut() {
        let url = process_url("://example.com/path", "http").unwrap();
        assert_eq!(url, "http://example.com/path");
    }

    #[test]
    fn test_process_url_ipv6_not_localhost() {
        let url = process_url("[::1]", "http").unwrap();
        assert_eq!(url, "http://[::1]");

        let url = process_url("[::1]:8080", "http").unwrap();
        assert_eq!(url, "http://[::1]:8080");
    }
}
