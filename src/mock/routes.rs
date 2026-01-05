//! Mock server route definitions and matching

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;

/// HTTP method for route matching
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    #[serde(rename = "*")]
    Any,
}

impl HttpMethod {
    pub fn matches(&self, method: &str) -> bool {
        match self {
            HttpMethod::Any => true,
            HttpMethod::Get => method.eq_ignore_ascii_case("GET"),
            HttpMethod::Post => method.eq_ignore_ascii_case("POST"),
            HttpMethod::Put => method.eq_ignore_ascii_case("PUT"),
            HttpMethod::Delete => method.eq_ignore_ascii_case("DELETE"),
            HttpMethod::Patch => method.eq_ignore_ascii_case("PATCH"),
            HttpMethod::Head => method.eq_ignore_ascii_case("HEAD"),
            HttpMethod::Options => method.eq_ignore_ascii_case("OPTIONS"),
        }
    }
}

impl Default for HttpMethod {
    fn default() -> Self {
        HttpMethod::Any
    }
}

/// Response configuration for a route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseConfig {
    /// HTTP status code
    #[serde(default = "default_status")]
    pub status: u16,

    /// Response headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Response body (string)
    #[serde(default)]
    pub body: Option<String>,

    /// Response body from file
    #[serde(default)]
    pub body_file: Option<String>,

    /// JSON body (will be serialized)
    #[serde(default)]
    pub json: Option<serde_json::Value>,

    /// Delay before responding (milliseconds)
    #[serde(default)]
    pub delay_ms: u64,

    /// Template variables to extract from request
    #[serde(default)]
    pub template: bool,
}

fn default_status() -> u16 {
    200
}

impl Default for ResponseConfig {
    fn default() -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: None,
            body_file: None,
            json: None,
            delay_ms: 0,
            template: false,
        }
    }
}

impl ResponseConfig {
    /// Create a simple text response
    pub fn text(body: &str) -> Self {
        Self {
            body: Some(body.to_string()),
            headers: [("Content-Type".to_string(), "text/plain".to_string())].into_iter().collect(),
            ..Default::default()
        }
    }

    /// Create a JSON response
    pub fn json_body(value: serde_json::Value) -> Self {
        Self {
            json: Some(value),
            headers: [("Content-Type".to_string(), "application/json".to_string())].into_iter().collect(),
            ..Default::default()
        }
    }

    /// Create an error response
    pub fn error(status: u16, message: &str) -> Self {
        Self {
            status,
            body: Some(message.to_string()),
            ..Default::default()
        }
    }

    /// Get the response body
    pub fn get_body(&self) -> Vec<u8> {
        if let Some(ref json) = self.json {
            serde_json::to_string_pretty(json)
                .unwrap_or_else(|_| "{}".to_string())
                .into_bytes()
        } else if let Some(ref body) = self.body {
            body.clone().into_bytes()
        } else if let Some(ref path) = self.body_file {
            std::fs::read(path).unwrap_or_default()
        } else {
            Vec::new()
        }
    }
}

/// Route configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    /// HTTP method to match
    #[serde(default)]
    pub method: HttpMethod,

    /// Path pattern (supports * wildcards and :param placeholders)
    pub path: String,

    /// Response configuration
    pub response: ResponseConfig,

    /// Route priority (higher = matched first)
    #[serde(default)]
    pub priority: i32,

    /// Whether this route is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Optional name for logging
    #[serde(default)]
    pub name: Option<String>,
}

fn default_true() -> bool {
    true
}

impl RouteConfig {
    /// Create a simple GET route
    pub fn get(path: &str, body: &str) -> Self {
        Self {
            method: HttpMethod::Get,
            path: path.to_string(),
            response: ResponseConfig::text(body),
            priority: 0,
            enabled: true,
            name: None,
        }
    }

    /// Create a POST route with JSON response
    pub fn post_json(path: &str, json: serde_json::Value) -> Self {
        Self {
            method: HttpMethod::Post,
            path: path.to_string(),
            response: ResponseConfig::json_body(json),
            priority: 0,
            enabled: true,
            name: None,
        }
    }
}

/// A compiled route for efficient matching
#[derive(Debug, Clone)]
pub struct Route {
    pub config: RouteConfig,
    path_regex: Regex,
    param_names: Vec<String>,
}

impl Route {
    /// Create a new route from config
    pub fn new(config: RouteConfig) -> Result<Self, String> {
        let (regex, params) = compile_path_pattern(&config.path)?;
        Ok(Self {
            config,
            path_regex: regex,
            param_names: params,
        })
    }

    /// Check if this route matches the request
    pub fn matches(&self, method: &str, path: &str) -> Option<HashMap<String, String>> {
        if !self.config.enabled {
            return None;
        }

        if !self.config.method.matches(method) {
            return None;
        }

        self.path_regex.captures(path).map(|caps| {
            let mut params = HashMap::new();
            for (i, name) in self.param_names.iter().enumerate() {
                if let Some(m) = caps.get(i + 1) {
                    params.insert(name.clone(), m.as_str().to_string());
                }
            }
            params
        })
    }
}

/// Compile a path pattern into a regex
fn compile_path_pattern(pattern: &str) -> Result<(Regex, Vec<String>), String> {
    let mut regex_str = String::from("^");
    let mut param_names = Vec::new();

    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ':' => {
                // Parameter placeholder :name
                let mut param_name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        param_name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if param_name.is_empty() {
                    return Err("Empty parameter name in path pattern".to_string());
                }
                param_names.push(param_name);
                regex_str.push_str("([^/]+)");
            }
            '*' => {
                // Wildcard
                if chars.peek() == Some(&'*') {
                    chars.next();
                    // ** matches anything including /
                    regex_str.push_str("(.*)");
                    param_names.push("**".to_string());
                } else {
                    // * matches anything except /
                    regex_str.push_str("([^/]*)");
                    param_names.push("*".to_string());
                }
            }
            '.' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '\\' | '^' | '$' | '|' => {
                // Escape regex special characters
                regex_str.push('\\');
                regex_str.push(c);
            }
            _ => regex_str.push(c),
        }
    }

    regex_str.push('$');

    Regex::new(&regex_str)
        .map(|r| (r, param_names))
        .map_err(|e| format!("Invalid path pattern: {}", e))
}

/// Request information for template rendering and logging
#[derive(Debug, Clone, Serialize)]
pub struct RequestInfo {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub params: HashMap<String, String>,
    pub timestamp: String,
    pub client_ip: String,
}

impl RequestInfo {
    pub fn new(
        method: String,
        path: String,
        query: HashMap<String, String>,
        headers: HashMap<String, String>,
        body: String,
        params: HashMap<String, String>,
        client_ip: String,
    ) -> Self {
        Self {
            method,
            path,
            query,
            headers,
            body,
            params,
            timestamp: chrono::Utc::now().to_rfc3339(),
            client_ip,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_pattern_exact() {
        let (regex, params) = compile_path_pattern("/api/users").unwrap();
        assert!(params.is_empty());
        assert!(regex.is_match("/api/users"));
        assert!(!regex.is_match("/api/users/"));
        assert!(!regex.is_match("/api/user"));
    }

    #[test]
    fn test_path_pattern_params() {
        let (regex, params) = compile_path_pattern("/api/users/:id").unwrap();
        assert_eq!(params, vec!["id"]);

        let caps = regex.captures("/api/users/123").unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "123");
    }

    #[test]
    fn test_path_pattern_wildcard() {
        let (regex, _) = compile_path_pattern("/api/*").unwrap();
        assert!(regex.is_match("/api/anything"));
        assert!(!regex.is_match("/api/foo/bar"));

        let (regex2, _) = compile_path_pattern("/api/**").unwrap();
        assert!(regex2.is_match("/api/foo/bar/baz"));
    }

    #[test]
    fn test_route_matching() {
        let route = Route::new(RouteConfig::get("/users/:id", "User")).unwrap();

        let params = route.matches("GET", "/users/42").unwrap();
        assert_eq!(params.get("id"), Some(&"42".to_string()));

        assert!(route.matches("POST", "/users/42").is_none());
        assert!(route.matches("GET", "/users/").is_none());
    }

    #[test]
    fn test_http_method_matching() {
        assert!(HttpMethod::Get.matches("GET"));
        assert!(HttpMethod::Get.matches("get"));
        assert!(!HttpMethod::Get.matches("POST"));
        assert!(HttpMethod::Any.matches("DELETE"));
    }
}
