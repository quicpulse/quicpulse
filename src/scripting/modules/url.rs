//! URL module for Rune scripts
//!
//! Provides URL parsing, building, and manipulation utilities.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use url::Url;

/// Create the url module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("url")?;

    // Parsing
    module.function("parse", parse_url).build()?;
    module.function("is_valid", is_valid_url).build()?;

    // Component extraction
    module.function("scheme", get_scheme).build()?;
    module.function("host", get_host).build()?;
    module.function("port", get_port).build()?;
    module.function("path", get_path).build()?;
    module.function("query", get_query).build()?;
    module.function("fragment", get_fragment).build()?;
    module.function("username", get_username).build()?;
    module.function("password", get_password).build()?;

    // Query string operations
    module.function("query_param", get_query_param).build()?;
    module.function("query_params", get_all_query_params).build()?;
    module.function("set_query_param", set_query_param).build()?;
    module.function("remove_query_param", remove_query_param).build()?;

    // Building/modification
    module.function("join", join_url).build()?;
    module.function("set_path", set_path).build()?;
    module.function("set_query", set_query).build()?;
    module.function("set_fragment", set_fragment).build()?;

    // Encoding
    module.function("encode", url_encode).build()?;
    module.function("decode", url_decode).build()?;
    module.function("encode_component", encode_component).build()?;
    module.function("decode_component", decode_component).build()?;

    Ok(module)
}

/// Parse a URL and return as JSON with components
fn parse_url(url_str: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(url) => {
            let result = serde_json::json!({
                "scheme": url.scheme(),
                "host": url.host_str(),
                "port": url.port(),
                "path": url.path(),
                "query": url.query(),
                "fragment": url.fragment(),
                "username": if url.username().is_empty() { None } else { Some(url.username()) },
                "password": url.password(),
            });
            RuneString::try_from(result.to_string()).unwrap_or_default()
        }
        Err(_) => RuneString::try_from("null").unwrap_or_default(),
    }
}

/// Check if a URL is valid
fn is_valid_url(url_str: &str) -> bool {
    Url::parse(url_str).is_ok()
}

/// Get the scheme (protocol)
fn get_scheme(url_str: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(url) => RuneString::try_from(url.scheme().to_string()).unwrap_or_default(),
        Err(_) => RuneString::new(),
    }
}

/// Get the host
fn get_host(url_str: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(url) => {
            url.host_str()
                .map(|h| RuneString::try_from(h.to_string()).unwrap_or_default())
                .unwrap_or_default()
        }
        Err(_) => RuneString::new(),
    }
}

/// Get the port (returns -1 if not specified)
fn get_port(url_str: &str) -> i64 {
    match Url::parse(url_str) {
        Ok(url) => url.port().map(|p| p as i64).unwrap_or(-1),
        Err(_) => -1,
    }
}

/// Get the path
fn get_path(url_str: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(url) => RuneString::try_from(url.path().to_string()).unwrap_or_default(),
        Err(_) => RuneString::new(),
    }
}

/// Get the query string (without ?)
fn get_query(url_str: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(url) => {
            url.query()
                .map(|q| RuneString::try_from(q.to_string()).unwrap_or_default())
                .unwrap_or_default()
        }
        Err(_) => RuneString::new(),
    }
}

/// Get the fragment (without #)
fn get_fragment(url_str: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(url) => {
            url.fragment()
                .map(|f| RuneString::try_from(f.to_string()).unwrap_or_default())
                .unwrap_or_default()
        }
        Err(_) => RuneString::new(),
    }
}

/// Get the username
fn get_username(url_str: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(url) => {
            if url.username().is_empty() {
                RuneString::new()
            } else {
                RuneString::try_from(url.username().to_string()).unwrap_or_default()
            }
        }
        Err(_) => RuneString::new(),
    }
}

/// Get the password
fn get_password(url_str: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(url) => {
            url.password()
                .map(|p| RuneString::try_from(p.to_string()).unwrap_or_default())
                .unwrap_or_default()
        }
        Err(_) => RuneString::new(),
    }
}

/// Get a specific query parameter value
fn get_query_param(url_str: &str, key: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(url) => {
            for (k, v) in url.query_pairs() {
                if k == key {
                    return RuneString::try_from(v.to_string()).unwrap_or_default();
                }
            }
            RuneString::new()
        }
        Err(_) => RuneString::new(),
    }
}

/// Get all query parameters as JSON array of [key, value] pairs
/// Bug #6 fix: Returns array instead of map to preserve duplicate keys
/// Example: ?id=1&id=2 returns [["id", "1"], ["id", "2"]] instead of {"id": "2"}
fn get_all_query_params(url_str: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(url) => {
            // Bug #6 fix: Use array of pairs to preserve duplicate keys
            // HTTP allows duplicate query keys like ?id=1&id=2
            // A Map would overwrite, losing data
            let pairs: Vec<(String, String)> = url.query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            let json = serde_json::to_string(&pairs).unwrap_or("[]".to_string());
            RuneString::try_from(json).unwrap_or_default()
        }
        Err(_) => RuneString::try_from("[]").unwrap_or_default(),
    }
}

/// Set or add a query parameter
fn set_query_param(url_str: &str, key: &str, value: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(mut url) => {
            // Collect existing params, replace if key exists
            let mut params: Vec<(String, String)> = url
                .query_pairs()
                .filter(|(k, _)| k != key)
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            params.push((key.to_string(), value.to_string()));

            // Clear and rebuild query
            url.set_query(None);
            {
                let mut query_pairs = url.query_pairs_mut();
                for (k, v) in params {
                    query_pairs.append_pair(&k, &v);
                }
            }

            RuneString::try_from(url.to_string()).unwrap_or_default()
        }
        Err(_) => RuneString::try_from(url_str.to_string()).unwrap_or_default(),
    }
}

/// Remove a query parameter
fn remove_query_param(url_str: &str, key: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(mut url) => {
            let params: Vec<(String, String)> = url
                .query_pairs()
                .filter(|(k, _)| k != key)
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            url.set_query(None);
            if !params.is_empty() {
                let mut query_pairs = url.query_pairs_mut();
                for (k, v) in params {
                    query_pairs.append_pair(&k, &v);
                }
            }

            RuneString::try_from(url.to_string()).unwrap_or_default()
        }
        Err(_) => RuneString::try_from(url_str.to_string()).unwrap_or_default(),
    }
}

/// Join a base URL with a relative path
fn join_url(base: &str, relative: &str) -> RuneString {
    match Url::parse(base) {
        Ok(base_url) => {
            match base_url.join(relative) {
                Ok(joined) => RuneString::try_from(joined.to_string()).unwrap_or_default(),
                Err(_) => RuneString::try_from(base.to_string()).unwrap_or_default(),
            }
        }
        Err(_) => RuneString::try_from(base.to_string()).unwrap_or_default(),
    }
}

/// Set the path component
fn set_path(url_str: &str, new_path: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(mut url) => {
            url.set_path(new_path);
            RuneString::try_from(url.to_string()).unwrap_or_default()
        }
        Err(_) => RuneString::try_from(url_str.to_string()).unwrap_or_default(),
    }
}

/// Set the query string
fn set_query(url_str: &str, query: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(mut url) => {
            url.set_query(Some(query));
            RuneString::try_from(url.to_string()).unwrap_or_default()
        }
        Err(_) => RuneString::try_from(url_str.to_string()).unwrap_or_default(),
    }
}

/// Set the fragment
fn set_fragment(url_str: &str, fragment: &str) -> RuneString {
    match Url::parse(url_str) {
        Ok(mut url) => {
            url.set_fragment(Some(fragment));
            RuneString::try_from(url.to_string()).unwrap_or_default()
        }
        Err(_) => RuneString::try_from(url_str.to_string()).unwrap_or_default(),
    }
}

/// URL encode a string
fn url_encode(input: &str) -> RuneString {
    let encoded = urlencoding::encode(input);
    RuneString::try_from(encoded.to_string()).unwrap_or_default()
}

/// URL decode a string
fn url_decode(input: &str) -> RuneString {
    match urlencoding::decode(input) {
        Ok(decoded) => RuneString::try_from(decoded.to_string()).unwrap_or_default(),
        Err(_) => RuneString::try_from(input.to_string()).unwrap_or_default(),
    }
}

/// Encode a URL component (for use in paths or query values)
fn encode_component(input: &str) -> RuneString {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    let encoded = utf8_percent_encode(input, NON_ALPHANUMERIC).to_string();
    RuneString::try_from(encoded).unwrap_or_default()
}

/// Decode a URL component
fn decode_component(input: &str) -> RuneString {
    use percent_encoding::percent_decode_str;
    match percent_decode_str(input).decode_utf8() {
        Ok(decoded) => RuneString::try_from(decoded.to_string()).unwrap_or_default(),
        Err(_) => RuneString::try_from(input.to_string()).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url() {
        let result = parse_url("https://example.com:8080/path?query=value#fragment");
        assert!(result.contains("example.com"));
        assert!(result.contains("8080"));
    }

    #[test]
    fn test_get_components() {
        let url = "https://user:pass@example.com:8080/path?key=value#section";
        assert_eq!(get_scheme(url).as_str(), "https");
        assert_eq!(get_host(url).as_str(), "example.com");
        assert_eq!(get_port(url), 8080);
        assert_eq!(get_path(url).as_str(), "/path");
        assert_eq!(get_query(url).as_str(), "key=value");
        assert_eq!(get_fragment(url).as_str(), "section");
        assert_eq!(get_username(url).as_str(), "user");
        assert_eq!(get_password(url).as_str(), "pass");
    }

    #[test]
    fn test_query_params() {
        let url = "https://example.com?a=1&b=2";
        assert_eq!(get_query_param(url, "a").as_str(), "1");
        assert_eq!(get_query_param(url, "b").as_str(), "2");
    }

    #[test]
    fn test_set_query_param() {
        let url = "https://example.com?a=1";
        let result = set_query_param(url, "b", "2");
        assert!(result.contains("a=1"));
        assert!(result.contains("b=2"));
    }

    #[test]
    fn test_join_url() {
        let result = join_url("https://example.com/path/", "../other");
        assert!(result.contains("/other"));
    }

    #[test]
    fn test_encode_decode() {
        let input = "hello world";
        let encoded = url_encode(input);
        assert!(encoded.contains("%20") || encoded.contains("+"));
        let decoded = url_decode(&encoded);
        assert_eq!(decoded.as_str(), input);
    }
}
