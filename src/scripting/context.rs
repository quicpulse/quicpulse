//! Script execution context
//!
//! Provides the context (request, response, variables) for script execution.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Request data available to scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestData {
    /// HTTP method
    pub method: String,
    /// Request URL
    pub url: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (as JSON if applicable)
    pub body: Option<JsonValue>,
    /// Query parameters
    pub query: HashMap<String, String>,
    /// Form data (for form submissions)
    pub form: HashMap<String, String>,
}

impl RequestData {
    /// Create a new empty request
    pub fn new(method: &str, url: &str) -> Self {
        Self {
            method: method.to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: None,
            query: HashMap::new(),
            form: HashMap::new(),
        }
    }

    /// Set a header
    pub fn set_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    /// Get a header
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    /// Remove a header
    pub fn remove_header(&mut self, key: &str) -> Option<String> {
        self.headers.remove(key)
    }

    /// Set the body
    pub fn set_body(&mut self, body: JsonValue) {
        self.body = Some(body);
    }

    /// Add a query parameter
    pub fn add_query(&mut self, key: &str, value: &str) {
        self.query.insert(key.to_string(), value.to_string());
    }

    /// Add a form field
    pub fn add_form(&mut self, key: &str, value: &str) {
        self.form.insert(key.to_string(), value.to_string());
    }

    /// Convert to JSON for script access
    pub fn to_json(&self) -> JsonValue {
        serde_json::to_value(self).unwrap_or(JsonValue::Null)
    }
}

impl Default for RequestData {
    fn default() -> Self {
        Self::new("GET", "")
    }
}

/// Response data available to scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseData {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body (parsed as JSON if possible)
    pub body: JsonValue,
    /// Response time in milliseconds
    pub elapsed_ms: u64,
}

impl ResponseData {
    /// Create a new response
    pub fn new(status: u16, body: JsonValue) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body,
            elapsed_ms: 0,
        }
    }

    /// Check if status is successful (2xx)
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Check if status is client error (4xx)
    pub fn is_client_error(&self) -> bool {
        self.status >= 400 && self.status < 500
    }

    /// Check if status is server error (5xx)
    pub fn is_server_error(&self) -> bool {
        self.status >= 500 && self.status < 600
    }

    /// Get a header value
    pub fn get_header(&self, key: &str) -> Option<&String> {
        // Case-insensitive header lookup
        let key_lower = key.to_lowercase();
        self.headers.iter()
            .find(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v)
    }

    /// Check if header exists
    pub fn has_header(&self, key: &str) -> bool {
        self.get_header(key).is_some()
    }

    /// Get body as string
    pub fn body_text(&self) -> String {
        match &self.body {
            JsonValue::String(s) => s.clone(),
            _ => serde_json::to_string(&self.body).unwrap_or_default(),
        }
    }

    /// Get a JSON path from body (simple implementation)
    pub fn json_path(&self, path: &str) -> Option<JsonValue> {
        let parts: Vec<&str> = path.trim_start_matches('.').split('.').collect();
        let mut current = &self.body;

        for part in parts {
            if part.is_empty() {
                continue;
            }

            // Check if it's an array index
            if let Some(idx) = part.strip_prefix('[').and_then(|p| p.strip_suffix(']')) {
                if let Ok(index) = idx.parse::<usize>() {
                    if let Some(arr) = current.as_array() {
                        current = arr.get(index)?;
                        continue;
                    }
                }
                return None;
            }

            // Object key access
            current = current.get(part)?;
        }

        Some(current.clone())
    }

    /// Convert to JSON for script access
    pub fn to_json(&self) -> JsonValue {
        serde_json::to_value(self).unwrap_or(JsonValue::Null)
    }
}

impl Default for ResponseData {
    fn default() -> Self {
        Self::new(0, JsonValue::Null)
    }
}

/// Script execution context containing all available data
#[derive(Debug, Clone, Default)]
pub struct ScriptContext {
    /// Current request (if available)
    request: Option<RequestData>,
    /// Current response (if available)
    response: Option<ResponseData>,
    /// Custom variables
    variables: HashMap<String, JsonValue>,
    /// Environment variables (read-only)
    env: HashMap<String, String>,
    /// Extracted values from previous steps (for workflows)
    extracted: HashMap<String, JsonValue>,
    /// Logs from script execution
    logs: Vec<String>,
}

impl ScriptContext {
    /// Create a new empty context
    pub fn new() -> Self {
        let mut env = HashMap::new();

        // Copy some safe environment variables
        for key in &["HOME", "USER", "PATH", "SHELL", "TERM", "LANG"] {
            if let Ok(value) = std::env::var(key) {
                env.insert(key.to_string(), value);
            }
        }

        Self {
            request: None,
            response: None,
            variables: HashMap::new(),
            env,
            extracted: HashMap::new(),
            logs: Vec::new(),
        }
    }

    /// Create context with request
    pub fn with_request(request: RequestData) -> Self {
        let mut ctx = Self::new();
        ctx.request = Some(request);
        ctx
    }

    /// Create context with response
    pub fn with_response(response: ResponseData) -> Self {
        let mut ctx = Self::new();
        ctx.response = Some(response);
        ctx
    }

    /// Create context with both request and response
    pub fn with_request_response(request: RequestData, response: ResponseData) -> Self {
        let mut ctx = Self::new();
        ctx.request = Some(request);
        ctx.response = Some(response);
        ctx
    }

    /// Set the request
    pub fn set_request(&mut self, request: RequestData) {
        self.request = Some(request);
    }

    /// Get the request
    pub fn get_request(&self) -> Option<RequestData> {
        self.request.clone()
    }

    /// Get mutable request reference
    pub fn get_request_mut(&mut self) -> Option<&mut RequestData> {
        self.request.as_mut()
    }

    /// Set the response
    pub fn set_response(&mut self, response: ResponseData) {
        self.response = Some(response);
    }

    /// Get the response
    pub fn get_response(&self) -> Option<ResponseData> {
        self.response.clone()
    }

    /// Get mutable response reference
    pub fn get_response_mut(&mut self) -> Option<&mut ResponseData> {
        self.response.as_mut()
    }

    /// Set a variable
    pub fn set_variable(&mut self, name: &str, value: JsonValue) {
        self.variables.insert(name.to_string(), value);
    }

    /// Get a variable
    pub fn get_variable(&self, name: &str) -> Option<&JsonValue> {
        self.variables.get(name)
    }

    /// Remove a variable
    pub fn remove_variable(&mut self, name: &str) -> Option<JsonValue> {
        self.variables.remove(name)
    }

    /// Get all variables
    pub fn variables(&self) -> &HashMap<String, JsonValue> {
        &self.variables
    }

    /// Set extracted value
    pub fn set_extracted(&mut self, name: &str, value: JsonValue) {
        self.extracted.insert(name.to_string(), value);
    }

    /// Get extracted value
    pub fn get_extracted(&self, name: &str) -> Option<&JsonValue> {
        self.extracted.get(name)
    }

    /// Get all extracted values
    pub fn extracted(&self) -> &HashMap<String, JsonValue> {
        &self.extracted
    }

    /// Import extracted values from another context
    pub fn import_extracted(&mut self, other: &ScriptContext) {
        for (k, v) in &other.extracted {
            self.extracted.insert(k.clone(), v.clone());
        }
    }

    /// Get environment variable
    pub fn get_env(&self, name: &str) -> Option<&String> {
        self.env.get(name)
    }

    /// Get all environment variables
    pub fn env(&self) -> &HashMap<String, String> {
        &self.env
    }

    /// Add a log message
    pub fn log(&mut self, message: &str) {
        self.logs.push(message.to_string());
    }

    /// Get all log messages
    pub fn logs(&self) -> &[String] {
        &self.logs
    }

    /// Clear logs
    pub fn clear_logs(&mut self) {
        self.logs.clear();
    }

    /// Convert the entire context to JSON for debugging
    pub fn to_json(&self) -> JsonValue {
        serde_json::json!({
            "request": self.request.as_ref().map(|r| r.to_json()),
            "response": self.response.as_ref().map(|r| r.to_json()),
            "variables": self.variables,
            "extracted": self.extracted,
            "logs": self.logs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_data() {
        let mut req = RequestData::new("POST", "https://api.example.com/users");
        req.set_header("Content-Type", "application/json");
        req.set_body(serde_json::json!({"name": "test"}));

        assert_eq!(req.method, "POST");
        assert_eq!(req.get_header("Content-Type"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_response_data() {
        let mut resp = ResponseData::new(200, serde_json::json!({"id": 1, "name": "test"}));
        resp.headers.insert("content-type".to_string(), "application/json".to_string());

        assert!(resp.is_success());
        assert!(!resp.is_client_error());
        assert_eq!(resp.json_path(".id"), Some(serde_json::json!(1)));
        assert_eq!(resp.json_path(".name"), Some(serde_json::json!("test")));
    }

    #[test]
    fn test_script_context() {
        let mut ctx = ScriptContext::new();
        ctx.set_variable("user_id", serde_json::json!(123));

        assert_eq!(ctx.get_variable("user_id"), Some(&serde_json::json!(123)));

        ctx.set_extracted("token", serde_json::json!("abc123"));
        assert_eq!(ctx.get_extracted("token"), Some(&serde_json::json!("abc123")));
    }

    #[test]
    fn test_response_json_path() {
        let resp = ResponseData::new(200, serde_json::json!({
            "user": {
                "name": "John",
                "emails": ["john@example.com", "j@test.com"]
            }
        }));

        assert_eq!(resp.json_path(".user.name"), Some(serde_json::json!("John")));
        assert_eq!(resp.json_path("user.emails"), Some(serde_json::json!(["john@example.com", "j@test.com"])));
    }
}
