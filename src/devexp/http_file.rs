//! .http/.rest file parser and executor
//!
//! Parses .http and .rest files (REST Client format used by VS Code)
//! and executes the HTTP requests within.
//!
//! Format:
//! ```http
//! ### Request name (optional comment)
//! GET https://api.example.com/users
//! Authorization: Bearer token
//!
//! ### POST with body
//! POST https://api.example.com/users
//! Content-Type: application/json
//!
//! {"name": "John"}
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::cli::Args;
use crate::errors::QuicpulseError;

/// A parsed HTTP request from a .http file
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// Optional name/comment for the request
    pub name: Option<String>,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request URL (may contain variables like {{baseUrl}})
    pub url: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (if any)
    pub body: Option<String>,
    /// Line number in the file (for error reporting)
    pub line_number: usize,
}

/// Parse a .http/.rest file into a list of requests
pub fn parse_http_file(path: &Path) -> Result<Vec<HttpRequest>, QuicpulseError> {
    let content = fs::read_to_string(path)
        .map_err(|e| QuicpulseError::Io(e))?;

    parse_http_content(&content)
}

/// Parse .http content string into requests
pub fn parse_http_content(content: &str) -> Result<Vec<HttpRequest>, QuicpulseError> {
    let mut requests = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // Skip empty lines and variable definitions
        if lines[i].trim().is_empty() || lines[i].starts_with('@') {
            i += 1;
            continue;
        }

        // Check for request separator/name (###)
        let name = if lines[i].trim().starts_with("###") {
            let name_line = lines[i].trim().trim_start_matches('#').trim();
            i += 1;
            if name_line.is_empty() {
                None
            } else {
                Some(name_line.to_string())
            }
        } else if lines[i].trim().starts_with('#') {
            // Skip comment lines
            i += 1;
            continue;
        } else {
            None
        };

        // Skip empty lines after name
        while i < lines.len() && lines[i].trim().is_empty() {
            i += 1;
        }

        if i >= lines.len() {
            break;
        }

        // Parse request line (METHOD URL [HTTP/version])
        let request_line = lines[i].trim();
        if request_line.is_empty() || request_line.starts_with('#') {
            i += 1;
            continue;
        }

        let request_line_num = i + 1;
        let (method, url) = parse_request_line(request_line)
            .ok_or_else(|| QuicpulseError::Parse(format!(
                "Invalid request line at line {}: '{}'",
                request_line_num, request_line
            )))?;
        i += 1;

        // Parse headers until empty line or end of file
        let mut headers = HashMap::new();
        while i < lines.len() {
            let line = lines[i].trim();

            // Empty line marks end of headers
            if line.is_empty() {
                i += 1;
                break;
            }

            // Comment line
            if line.starts_with('#') {
                i += 1;
                continue;
            }

            // New request separator
            if line.starts_with("###") {
                break;
            }

            // Parse header
            if let Some((name, value)) = parse_header_line(line) {
                headers.insert(name, value);
            }
            i += 1;
        }

        // Parse body until next request or end of file
        let mut body_lines = Vec::new();
        while i < lines.len() {
            let line = lines[i];

            // New request separator marks end of body
            if line.trim().starts_with("###") {
                break;
            }

            // Check if this looks like a new request line (METHOD URL)
            if parse_request_line(line.trim()).is_some() && body_lines.is_empty() {
                break;
            }

            body_lines.push(line);
            i += 1;
        }

        // Trim empty lines from body
        while !body_lines.is_empty() && body_lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
            body_lines.pop();
        }
        while !body_lines.is_empty() && body_lines.first().map(|l| l.trim().is_empty()).unwrap_or(false) {
            body_lines.remove(0);
        }

        let body = if body_lines.is_empty() {
            None
        } else {
            Some(body_lines.join("\n"))
        };

        requests.push(HttpRequest {
            name,
            method,
            url,
            headers,
            body,
            line_number: request_line_num,
        });
    }

    Ok(requests)
}

/// Parse a request line like "GET https://example.com" or "POST /api HTTP/1.1"
fn parse_request_line(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let method = parts[0].to_uppercase();
    // Validate it's a known HTTP method
    let valid_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "TRACE", "CONNECT"];
    if !valid_methods.contains(&method.as_str()) {
        return None;
    }

    // URL is the second part (ignore optional HTTP version at end)
    let url = parts[1].to_string();

    Some((method, url))
}

/// Parse a header line like "Content-Type: application/json"
fn parse_header_line(line: &str) -> Option<(String, String)> {
    let colon_pos = line.find(':')?;
    let name = line[..colon_pos].trim().to_string();
    let value = line[colon_pos + 1..].trim().to_string();

    if name.is_empty() {
        return None;
    }

    Some((name, value))
}

/// Expand variables in a string
/// Supports {{variable}} syntax and environment variables
pub fn expand_variables(s: &str, variables: &HashMap<String, String>) -> String {
    let mut result = s.to_string();

    // Expand {{variable}} syntax
    for (key, value) in variables {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }

    // Expand remaining {{$env.VAR}} or {{VAR}} from environment
    let var_pattern = regex::Regex::new(r"\{\{(\$env\.)?(\w+)\}\}").unwrap();
    let result = var_pattern.replace_all(&result, |caps: &regex::Captures| {
        let var_name = caps.get(2).unwrap().as_str();
        std::env::var(var_name).unwrap_or_else(|_| caps.get(0).unwrap().as_str().to_string())
    });

    result.to_string()
}

/// Convert an HttpRequest to Args for execution
pub fn request_to_args(request: &HttpRequest, variables: &HashMap<String, String>) -> Args {
    let mut args = Args::default();

    // Expand variables in URL
    let url = expand_variables(&request.url, variables);
    args.url = Some(url.clone());
    args.method = Some(request.method.clone());

    // Add headers as request items
    for (name, value) in &request.headers {
        let expanded_value = expand_variables(value, variables);
        args.request_items.push(format!("{}:{}", name, expanded_value));
    }

    // Set body if present
    if let Some(ref body) = request.body {
        let expanded_body = expand_variables(body, variables);

        // Check if it looks like JSON before moving
        if expanded_body.trim().starts_with('{') || expanded_body.trim().starts_with('[') {
            args.json = true;
        }

        args.raw = Some(expanded_body);
    }

    args
}

/// Parse variable definitions from .http file content
/// Variables are defined as @name = value
pub fn parse_variables(content: &str) -> HashMap<String, String> {
    let mut variables = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('@') && !line.starts_with("@@") {
            if let Some(eq_pos) = line.find('=') {
                let name = line[1..eq_pos].trim().to_string();
                let value = line[eq_pos + 1..].trim().to_string();
                variables.insert(name, value);
            }
        }
    }

    variables
}

/// List requests in a .http file
pub fn list_requests(path: &Path) -> Result<Vec<(usize, String, String, String)>, QuicpulseError> {
    let requests = parse_http_file(path)?;

    Ok(requests.iter().enumerate().map(|(i, req)| {
        let name = req.name.clone().unwrap_or_else(|| format!("Request {}", i + 1));
        (i + 1, name, req.method.clone(), req.url.clone())
    }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_request() {
        let content = r#"
GET https://api.example.com/users
"#;
        let requests = parse_http_content(content).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].url, "https://api.example.com/users");
    }

    #[test]
    fn test_parse_request_with_headers() {
        let content = r#"
GET https://api.example.com/users
Authorization: Bearer token123
Content-Type: application/json
"#;
        let requests = parse_http_content(content).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].headers.get("Authorization"), Some(&"Bearer token123".to_string()));
        assert_eq!(requests[0].headers.get("Content-Type"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_parse_request_with_body() {
        let content = r#"
POST https://api.example.com/users
Content-Type: application/json

{"name": "John", "email": "john@example.com"}
"#;
        let requests = parse_http_content(content).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "POST");
        assert!(requests[0].body.is_some());
        assert!(requests[0].body.as_ref().unwrap().contains("John"));
    }

    #[test]
    fn test_parse_multiple_requests() {
        let content = r#"
### List users
GET https://api.example.com/users

### Create user
POST https://api.example.com/users
Content-Type: application/json

{"name": "John"}

### Delete user
DELETE https://api.example.com/users/1
"#;
        let requests = parse_http_content(content).unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].name, Some("List users".to_string()));
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[1].name, Some("Create user".to_string()));
        assert_eq!(requests[1].method, "POST");
        assert_eq!(requests[2].name, Some("Delete user".to_string()));
        assert_eq!(requests[2].method, "DELETE");
    }

    #[test]
    fn test_parse_variables() {
        let content = r#"
@baseUrl = https://api.example.com
@token = secret123

GET {{baseUrl}}/users
Authorization: Bearer {{token}}
"#;
        let variables = parse_variables(content);
        assert_eq!(variables.get("baseUrl"), Some(&"https://api.example.com".to_string()));
        assert_eq!(variables.get("token"), Some(&"secret123".to_string()));
    }

    #[test]
    fn test_expand_variables() {
        let mut variables = HashMap::new();
        variables.insert("baseUrl".to_string(), "https://api.example.com".to_string());
        variables.insert("token".to_string(), "secret123".to_string());

        let expanded = expand_variables("{{baseUrl}}/users", &variables);
        assert_eq!(expanded, "https://api.example.com/users");

        let expanded_header = expand_variables("Bearer {{token}}", &variables);
        assert_eq!(expanded_header, "Bearer secret123");
    }
}
