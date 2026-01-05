//! MIME type utilities
//!
//! Functions for parsing and handling MIME types and Content-Type headers.

use mime::Mime;
use std::collections::HashMap;

/// Parse Content-Type header into MIME type and parameters
///
/// # Examples
/// ```
/// use quicpulse::mime::parse_content_type_header;
/// let (mime, params) = parse_content_type_header("application/json; charset=utf-8");
/// assert_eq!(mime, "application/json");
/// assert_eq!(params.get("charset"), Some(&"utf-8".to_string()));
/// ```
pub fn parse_content_type_header(header: &str) -> (String, HashMap<String, String>) {
    match header.parse::<Mime>() {
        Ok(m) => {
            let mime_type = format!("{}/{}", m.type_(), m.subtype());
            let params: HashMap<_, _> = m.params()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            (mime_type, params)
        }
        Err(_) => (header.to_string(), HashMap::new()),
    }
}

/// Get content type for a filename using mime_guess
pub fn get_content_type(filename: &str) -> Option<String> {
    mime_guess::from_path(filename)
        .first()
        .map(|m| m.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_type() {
        let (mime, params) = parse_content_type_header("application/json; charset=utf-8");
        assert_eq!(mime, "application/json");
        assert_eq!(params.get("charset"), Some(&"utf-8".to_string()));
    }

    #[test]
    fn test_parse_content_type_simple() {
        let (mime, params) = parse_content_type_header("text/html");
        assert_eq!(mime, "text/html");
        assert!(params.is_empty());
    }

    #[test]
    fn test_get_content_type() {
        assert_eq!(get_content_type("file.json"), Some("application/json".to_string()));
        assert_eq!(get_content_type("file.txt"), Some("text/plain".to_string()));
    }
}
