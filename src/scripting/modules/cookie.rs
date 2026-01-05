//! Cookie module for Rune scripts
//!
//! Provides cookie parsing and manipulation utilities.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use serde_json::{json, Value as JsonValue};

/// Create the cookie module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("cookie")?;

    // Parsing
    module.function("parse", parse_cookie_header).build()?;
    module.function("parse_set_cookie", parse_set_cookie).build()?;
    module.function("get", get_cookie_value).build()?;

    // Building
    module.function("build", build_cookie).build()?;
    module.function("build_set_cookie", build_set_cookie).build()?;

    // Manipulation
    module.function("merge", merge_cookies).build()?;
    module.function("remove", remove_cookie).build()?;
    module.function("to_header", cookies_to_header).build()?;

    // Validation
    module.function("is_expired", is_cookie_expired).build()?;
    module.function("is_secure", is_secure_cookie).build()?;
    module.function("is_http_only", is_http_only).build()?;

    Ok(module)
}

/// Parse a Cookie header string into JSON object
fn parse_cookie_header(cookie_str: &str) -> RuneString {
    let mut cookies = serde_json::Map::new();

    for part in cookie_str.split(';') {
        let part = part.trim();
        if let Some(pos) = part.find('=') {
            let name = part[..pos].trim();
            let value = part[pos + 1..].trim().trim_matches('"');
            cookies.insert(name.to_string(), JsonValue::String(value.to_string()));
        }
    }

    RuneString::try_from(serde_json::to_string(&cookies).unwrap_or("{}".to_string())).unwrap_or_default()
}

/// Parse a Set-Cookie header into detailed JSON
fn parse_set_cookie(set_cookie_str: &str) -> RuneString {
    let parts: Vec<&str> = set_cookie_str.split(';').collect();

    if parts.is_empty() {
        return RuneString::try_from("{}").unwrap_or_default();
    }

    // First part is name=value
    let (name, value) = if let Some(pos) = parts[0].find('=') {
        (
            parts[0][..pos].trim().to_string(),
            parts[0][pos + 1..].trim().trim_matches('"').to_string(),
        )
    } else {
        return RuneString::try_from("{}").unwrap_or_default();
    };

    let mut cookie = json!({
        "name": name,
        "value": value,
    });

    // Parse attributes
    for part in parts.iter().skip(1) {
        let part = part.trim();
        if let Some(pos) = part.find('=') {
            let attr_name = part[..pos].trim().to_lowercase();
            let attr_value = part[pos + 1..].trim();

            match attr_name.as_str() {
                "expires" => {
                    cookie["expires"] = JsonValue::String(attr_value.to_string());
                }
                "max-age" => {
                    if let Ok(max_age) = attr_value.parse::<i64>() {
                        cookie["maxAge"] = JsonValue::Number(max_age.into());
                    }
                }
                "domain" => {
                    cookie["domain"] = JsonValue::String(attr_value.to_string());
                }
                "path" => {
                    cookie["path"] = JsonValue::String(attr_value.to_string());
                }
                "samesite" => {
                    cookie["sameSite"] = JsonValue::String(attr_value.to_string());
                }
                _ => {}
            }
        } else {
            // Boolean attributes
            match part.to_lowercase().as_str() {
                "secure" => {
                    cookie["secure"] = JsonValue::Bool(true);
                }
                "httponly" => {
                    cookie["httpOnly"] = JsonValue::Bool(true);
                }
                _ => {}
            }
        }
    }

    RuneString::try_from(cookie.to_string()).unwrap_or_default()
}

/// Get a specific cookie value from Cookie header
fn get_cookie_value(cookie_str: &str, name: &str) -> RuneString {
    for part in cookie_str.split(';') {
        let part = part.trim();
        if let Some(pos) = part.find('=') {
            let cookie_name = part[..pos].trim();
            if cookie_name == name {
                let value = part[pos + 1..].trim().trim_matches('"');
                return RuneString::try_from(value.to_string()).unwrap_or_default();
            }
        }
    }
    RuneString::new()
}

/// Build a simple cookie string (name=value)
fn build_cookie(name: &str, value: &str) -> RuneString {
    RuneString::try_from(format!("{}={}", name, value)).unwrap_or_default()
}

/// Build a Set-Cookie header with options (JSON input)
fn build_set_cookie(cookie_json: &str) -> RuneString {
    let cookie: JsonValue = match serde_json::from_str(cookie_json) {
        Ok(v) => v,
        Err(_) => return RuneString::new(),
    };

    let name = cookie.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let value = cookie.get("value").and_then(|v| v.as_str()).unwrap_or("");

    if name.is_empty() {
        return RuneString::new();
    }

    let mut parts = vec![format!("{}={}", name, value)];

    if let Some(expires) = cookie.get("expires").and_then(|v| v.as_str()) {
        parts.push(format!("Expires={}", expires));
    }

    if let Some(max_age) = cookie.get("maxAge").and_then(|v| v.as_i64()) {
        parts.push(format!("Max-Age={}", max_age));
    }

    if let Some(domain) = cookie.get("domain").and_then(|v| v.as_str()) {
        parts.push(format!("Domain={}", domain));
    }

    if let Some(path) = cookie.get("path").and_then(|v| v.as_str()) {
        parts.push(format!("Path={}", path));
    }

    if let Some(same_site) = cookie.get("sameSite").and_then(|v| v.as_str()) {
        parts.push(format!("SameSite={}", same_site));
    }

    if cookie.get("secure").and_then(|v| v.as_bool()).unwrap_or(false) {
        parts.push("Secure".to_string());
    }

    if cookie.get("httpOnly").and_then(|v| v.as_bool()).unwrap_or(false) {
        parts.push("HttpOnly".to_string());
    }

    RuneString::try_from(parts.join("; ")).unwrap_or_default()
}

/// Merge two cookie header strings
fn merge_cookies(cookies1: &str, cookies2: &str) -> RuneString {
    let mut all_cookies: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // Parse first cookie string
    for part in cookies1.split(';') {
        let part = part.trim();
        if let Some(pos) = part.find('=') {
            let name = part[..pos].trim().to_string();
            let value = part[pos + 1..].trim().to_string();
            all_cookies.insert(name, value);
        }
    }

    // Parse second cookie string (overwrites)
    for part in cookies2.split(';') {
        let part = part.trim();
        if let Some(pos) = part.find('=') {
            let name = part[..pos].trim().to_string();
            let value = part[pos + 1..].trim().to_string();
            all_cookies.insert(name, value);
        }
    }

    let result: Vec<String> = all_cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    RuneString::try_from(result.join("; ")).unwrap_or_default()
}

/// Remove a cookie from a cookie header string
fn remove_cookie(cookie_str: &str, name: &str) -> RuneString {
    let filtered: Vec<String> = cookie_str
        .split(';')
        .filter_map(|part| {
            let part = part.trim();
            if let Some(pos) = part.find('=') {
                let cookie_name = part[..pos].trim();
                if cookie_name != name {
                    return Some(part.to_string());
                }
            }
            None
        })
        .collect();

    RuneString::try_from(filtered.join("; ")).unwrap_or_default()
}

/// Convert JSON cookie object to Cookie header string
fn cookies_to_header(cookies_json: &str) -> RuneString {
    let cookies: JsonValue = match serde_json::from_str(cookies_json) {
        Ok(v) => v,
        Err(_) => return RuneString::new(),
    };

    if let Some(obj) = cookies.as_object() {
        let parts: Vec<String> = obj
            .iter()
            .map(|(k, v)| {
                let value = match v {
                    JsonValue::String(s) => s.clone(),
                    other => other.to_string(),
                };
                format!("{}={}", k, value)
            })
            .collect();
        return RuneString::try_from(parts.join("; ")).unwrap_or_default();
    }

    RuneString::new()
}

/// Check if a Set-Cookie has expired (from parsed JSON)
fn is_cookie_expired(cookie_json: &str) -> bool {
    let cookie: JsonValue = match serde_json::from_str(cookie_json) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Check Max-Age
    if let Some(max_age) = cookie.get("maxAge").and_then(|v| v.as_i64()) {
        if max_age <= 0 {
            return true;
        }
    }

    // Check Expires (simplified - just check if it parses and is in past)
    if let Some(expires) = cookie.get("expires").and_then(|v| v.as_str()) {
        use chrono::DateTime;
        if let Ok(exp_date) = DateTime::parse_from_rfc2822(expires) {
            if exp_date < chrono::Utc::now() {
                return true;
            }
        }
    }

    false
}

/// Check if cookie has Secure flag
fn is_secure_cookie(cookie_json: &str) -> bool {
    let cookie: JsonValue = match serde_json::from_str(cookie_json) {
        Ok(v) => v,
        Err(_) => return false,
    };

    cookie.get("secure").and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Check if cookie has HttpOnly flag
fn is_http_only(cookie_json: &str) -> bool {
    let cookie: JsonValue = match serde_json::from_str(cookie_json) {
        Ok(v) => v,
        Err(_) => return false,
    };

    cookie.get("httpOnly").and_then(|v| v.as_bool()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cookie_header() {
        let result = parse_cookie_header("session=abc123; user=john");
        assert!(result.contains("session"));
        assert!(result.contains("abc123"));
        assert!(result.contains("user"));
        assert!(result.contains("john"));
    }

    #[test]
    fn test_parse_set_cookie() {
        let result = parse_set_cookie("session=abc123; Path=/; HttpOnly; Secure");
        assert!(result.contains("session"));
        assert!(result.contains("abc123"));
        assert!(result.contains("httpOnly"));
        assert!(result.contains("secure"));
    }

    #[test]
    fn test_get_cookie_value() {
        let cookies = "session=abc123; user=john";
        assert_eq!(get_cookie_value(cookies, "session").as_str(), "abc123");
        assert_eq!(get_cookie_value(cookies, "user").as_str(), "john");
    }

    #[test]
    fn test_build_set_cookie() {
        let cookie_json = r#"{"name": "session", "value": "abc", "secure": true, "httpOnly": true, "path": "/"}"#;
        let result = build_set_cookie(cookie_json);
        assert!(result.contains("session=abc"));
        assert!(result.contains("Secure"));
        assert!(result.contains("HttpOnly"));
        assert!(result.contains("Path=/"));
    }

    #[test]
    fn test_merge_cookies() {
        let result = merge_cookies("a=1; b=2", "b=3; c=4");
        assert!(result.contains("a=1"));
        assert!(result.contains("b=3")); // b should be overwritten
        assert!(result.contains("c=4"));
    }
}
