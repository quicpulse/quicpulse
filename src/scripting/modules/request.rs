//! Request module for Rune scripts
//!
//! Provides HTTP request capabilities from within scripts.
//! This enables intra-workflow requests and ad-hoc HTTP calls.
//!
//! # Example
//!
//! ```rune
//! let response = request::get("https://api.example.com/data");
//! let data = json::parse(response);
//! ```

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use std::time::Duration;

/// Create the request module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("request")?;

    // HTTP methods
    module.function("get", http_get).build()?;
    module.function("post", http_post).build()?;
    module.function("put", http_put).build()?;
    module.function("patch", http_patch).build()?;
    module.function("delete", http_delete).build()?;

    // Generic request
    module.function("send", http_send).build()?;

    Ok(module)
}

/// Perform a GET request and return JSON response string
fn http_get(url: &str) -> RuneString {
    http_request("GET", url, "")
}

/// Perform a POST request with body and return JSON response string
fn http_post(url: &str, body: &str) -> RuneString {
    http_request("POST", url, body)
}

/// Perform a PUT request with body and return JSON response string
fn http_put(url: &str, body: &str) -> RuneString {
    http_request("PUT", url, body)
}

/// Perform a PATCH request with body and return JSON response string
fn http_patch(url: &str, body: &str) -> RuneString {
    http_request("PATCH", url, body)
}

/// Perform a DELETE request and return JSON response string
fn http_delete(url: &str) -> RuneString {
    http_request("DELETE", url, "")
}

/// Send an HTTP request with method, url, and body
/// Returns JSON string with response data
fn http_send(method: &str, url: &str, body: &str) -> RuneString {
    http_request(method, url, body)
}

/// Internal function to make HTTP requests
/// Returns a JSON string with the response:
/// {
///   "status": 200,
///   "ok": true,
///   "body": "...",
///   "duration_ms": 123,
///   "error": null
/// }
fn http_request(method: &str, url: &str, body: &str) -> RuneString {
    use std::str::FromStr;

    let start = std::time::Instant::now();

    // Create a blocking client
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Failed to create client: {}", e)),
    };

    // Parse method
    let method = match reqwest::Method::from_str(&method.to_uppercase()) {
        Ok(m) => m,
        Err(_) => return error_response(&format!("Invalid method: {}", method)),
    };

    // Build request
    let mut request = client.request(method, url);

    // Add body if not empty
    if !body.is_empty() {
        request = request.body(body.to_string());
        // Auto-detect JSON
        if body.trim().starts_with('{') || body.trim().starts_with('[') {
            request = request.header("Content-Type", "application/json");
        }
    }

    // Execute request
    match request.send() {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status().as_u16() as i64;
            let ok = response.status().is_success();

            // Collect headers
            let headers: serde_json::Map<String, serde_json::Value> = response.headers()
                .iter()
                .map(|(k, v)| {
                    (k.to_string(), serde_json::Value::String(
                        v.to_str().unwrap_or("").to_string()
                    ))
                })
                .collect();

            // Get body
            let body = response.text().unwrap_or_default();

            // Try to parse body as JSON
            let json_body: serde_json::Value = serde_json::from_str(&body)
                .unwrap_or(serde_json::Value::String(body.clone()));

            let result = serde_json::json!({
                "status": status,
                "ok": ok,
                "body": body,
                "json": json_body,
                "headers": headers,
                "duration_ms": duration.as_millis() as i64,
                "error": null
            });

            RuneString::try_from(result.to_string()).unwrap_or_default()
        }
        Err(e) => {
            let duration = start.elapsed();
            let result = serde_json::json!({
                "status": 0,
                "ok": false,
                "body": "",
                "json": null,
                "headers": {},
                "duration_ms": duration.as_millis() as i64,
                "error": e.to_string()
            });

            RuneString::try_from(result.to_string()).unwrap_or_default()
        }
    }
}

/// Build an error response JSON string
fn error_response(message: &str) -> RuneString {
    let result = serde_json::json!({
        "status": 0,
        "ok": false,
        "body": "",
        "json": null,
        "headers": {},
        "duration_ms": 0,
        "error": message
    });

    RuneString::try_from(result.to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creation() {
        let module = module();
        assert!(module.is_ok());
    }

    #[test]
    fn test_error_response_format() {
        let response = error_response("test error");
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(json["status"], 0);
        assert_eq!(json["ok"], false);
        assert_eq!(json["body"], "");
        assert!(json["json"].is_null());
        assert_eq!(json["duration_ms"], 0);
        assert_eq!(json["error"], "test error");
    }

    #[test]
    fn test_invalid_method_returns_error() {
        let response = http_send("INVALID_METHOD", "http://localhost", "");
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(json["status"], 0);
        assert_eq!(json["ok"], false);
        // Error should be non-null (either "Invalid method" or reqwest's error)
        assert!(!json["error"].is_null());
        assert!(!json["error"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_http_methods_exist() {
        // These should not panic - just verify the functions exist
        // We can't actually make requests in unit tests
        let _ = http_get as fn(&str) -> RuneString;
        let _ = http_post as fn(&str, &str) -> RuneString;
        let _ = http_put as fn(&str, &str) -> RuneString;
        let _ = http_patch as fn(&str, &str) -> RuneString;
        let _ = http_delete as fn(&str) -> RuneString;
        let _ = http_send as fn(&str, &str, &str) -> RuneString;
    }
}
