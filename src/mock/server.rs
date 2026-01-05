//! Mock HTTP server implementation

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::config::MockServerConfig;
use super::routes::{Route, ResponseConfig, RequestInfo};
use crate::errors::QuicpulseError;

/// Mock HTTP server
pub struct MockServer {
    config: MockServerConfig,
    routes: Vec<Route>,
    request_log: Arc<RwLock<Vec<RequestInfo>>>,
}

impl MockServer {
    /// Create a new mock server from config
    pub fn new(config: MockServerConfig) -> Result<Self, QuicpulseError> {
        config.validate()?;

        let mut routes: Vec<Route> = config.routes.iter()
            .map(|rc| Route::new(rc.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| QuicpulseError::Config(format!("Invalid route: {}", e)))?;

        // Sort by priority (higher first)
        routes.sort_by(|a, b| b.config.priority.cmp(&a.config.priority));

        Ok(Self {
            config,
            routes,
            request_log: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Start the server
    pub async fn run(&self) -> Result<(), QuicpulseError> {
        let addr: SocketAddr = self.config.address().parse()
            .map_err(|e| QuicpulseError::Config(format!("Invalid address: {}", e)))?;

        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| QuicpulseError::Io(e))?;

        eprintln!("Mock server listening on http://{}", addr);

        if self.routes.is_empty() {
            eprintln!("Warning: No routes configured, all requests will return 404");
        } else {
            eprintln!("Configured routes:");
            for route in &self.routes {
                let name = route.config.name.as_deref().unwrap_or("");
                eprintln!("  {:?} {} -> {} {}",
                    route.config.method,
                    route.config.path,
                    route.config.response.status,
                    name
                );
            }
        }

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let routes = self.routes.clone();
                    let config = self.config.clone();
                    let log = Arc::clone(&self.request_log);

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, peer_addr, &routes, &config, log).await {
                            eprintln!("Connection error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                }
            }
        }
    }

    /// Get the request log
    pub async fn get_requests(&self) -> Vec<RequestInfo> {
        self.request_log.read().await.clone()
    }

    /// Clear the request log
    pub async fn clear_requests(&self) {
        self.request_log.write().await.clear();
    }

    /// Generate a simple config for the given routes
    pub fn simple_config(routes: Vec<(&str, &str, &str)>) -> MockServerConfig {
        let mut config = MockServerConfig::default();
        for (method, path, body) in routes {
            config.routes.push(super::routes::RouteConfig {
                method: match method.to_uppercase().as_str() {
                    "GET" => super::routes::HttpMethod::Get,
                    "POST" => super::routes::HttpMethod::Post,
                    "PUT" => super::routes::HttpMethod::Put,
                    "DELETE" => super::routes::HttpMethod::Delete,
                    "PATCH" => super::routes::HttpMethod::Patch,
                    _ => super::routes::HttpMethod::Any,
                },
                path: path.to_string(),
                response: ResponseConfig::text(body),
                priority: 0,
                enabled: true,
                name: None,
            });
        }
        config
    }
}

/// Handle a single connection
async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    peer_addr: SocketAddr,
    routes: &[Route],
    config: &MockServerConfig,
    log: Arc<RwLock<Vec<RequestInfo>>>,
) -> Result<(), QuicpulseError> {
    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await.map_err(QuicpulseError::Io)?;

    if n == 0 {
        return Ok(());
    }

    let request_str = String::from_utf8_lossy(&buf[..n]);
    let request = parse_request(&request_str)?;

    // Log request if enabled
    if config.log_requests {
        let info = RequestInfo::new(
            request.method.clone(),
            request.path.clone(),
            request.query.clone(),
            request.headers.clone(),
            request.body.clone(),
            HashMap::new(),
            peer_addr.ip().to_string(),
        );

        eprintln!("[{}] {} {} {} from {}",
            info.timestamp,
            request.method,
            request.path,
            if request.query.is_empty() { String::new() } else { format!("?{}", query_string(&request.query)) },
            peer_addr
        );

        log.write().await.push(info);
    }

    // Find matching route
    let (response, params) = find_route(routes, &request.method, &request.path, config);

    // Apply latency if configured
    if let Some((min, max)) = config.latency {
        let delay = if min == max {
            min
        } else {
            min + rand::random::<u64>() % (max - min)
        };
        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
    }

    // Apply route-specific delay
    if response.delay_ms > 0 {
        tokio::time::sleep(tokio::time::Duration::from_millis(response.delay_ms)).await;
    }

    // Build response
    let mut response_headers = response.headers.clone();

    // Add CORS headers if enabled
    if config.cors {
        response_headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        response_headers.insert("Access-Control-Allow-Methods".to_string(), "GET, POST, PUT, DELETE, PATCH, OPTIONS".to_string());
        response_headers.insert("Access-Control-Allow-Headers".to_string(), "*".to_string());
    }

    // Get body, applying template if needed
    let body = if response.template {
        apply_template(&response.get_body(), &request, &params)
    } else {
        response.get_body()
    };

    // Set content-length
    response_headers.insert("Content-Length".to_string(), body.len().to_string());

    // Build HTTP response
    let status_text = http_status_text(response.status);
    let mut response_str = format!("HTTP/1.1 {} {}\r\n", response.status, status_text);

    for (name, value) in &response_headers {
        response_str.push_str(&format!("{}: {}\r\n", name, value));
    }
    response_str.push_str("\r\n");

    // Send response
    stream.write_all(response_str.as_bytes()).await.map_err(QuicpulseError::Io)?;
    stream.write_all(&body).await.map_err(QuicpulseError::Io)?;
    stream.flush().await.map_err(QuicpulseError::Io)?;

    Ok(())
}

/// Parsed HTTP request
struct ParsedRequest {
    method: String,
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: String,
}

/// Parse an HTTP request
fn parse_request(data: &str) -> Result<ParsedRequest, QuicpulseError> {
    let mut lines = data.lines();

    // Parse request line
    let request_line = lines.next()
        .ok_or_else(|| QuicpulseError::Argument("Empty request".to_string()))?;

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(QuicpulseError::Argument("Invalid request line".to_string()));
    }

    let method = parts[0].to_string();
    let full_path = parts[1];

    // Parse path and query
    let (path, query_str) = full_path.split_once('?').unwrap_or((full_path, ""));
    let query = parse_query_string(query_str);

    // Parse headers
    let mut headers = HashMap::new();
    let mut body_start = false;
    let mut body_lines = Vec::new();

    for line in lines {
        if body_start {
            body_lines.push(line);
        } else if line.is_empty() {
            body_start = true;
        } else if let Some((name, value)) = line.split_once(':') {
            headers.insert(
                name.trim().to_lowercase(),
                value.trim().to_string()
            );
        }
    }

    Ok(ParsedRequest {
        method,
        path: path.to_string(),
        query,
        headers,
        body: body_lines.join("\n"),
    })
}

/// Parse query string into map
fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if query.is_empty() {
        return map;
    }

    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            map.insert(
                urlencoding::decode(key).unwrap_or_else(|_| key.into()).to_string(),
                urlencoding::decode(value).unwrap_or_else(|_| value.into()).to_string()
            );
        }
    }
    map
}

/// Convert query map back to string
fn query_string(query: &HashMap<String, String>) -> String {
    query.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&")
}

/// Find matching route or return default response
fn find_route(
    routes: &[Route],
    method: &str,
    path: &str,
    config: &MockServerConfig,
) -> (ResponseConfig, HashMap<String, String>) {
    for route in routes {
        if let Some(params) = route.matches(method, path) {
            return (route.config.response.clone(), params);
        }
    }

    // Return default or 404
    let default_response = config.default_response.clone().unwrap_or_else(|| {
        ResponseConfig::error(404, "Not Found")
    });

    (default_response, HashMap::new())
}

/// Apply template substitution to response body
fn apply_template(body: &[u8], request: &ParsedRequest, params: &HashMap<String, String>) -> Vec<u8> {
    let body_str = String::from_utf8_lossy(body);
    let mut result = body_str.to_string();

    // Replace {{param}} placeholders
    for (key, value) in params {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }

    // Replace {{query.key}} placeholders
    for (key, value) in &request.query {
        result = result.replace(&format!("{{{{query.{}}}}}", key), value);
    }

    // Replace {{header.key}} placeholders
    for (key, value) in &request.headers {
        result = result.replace(&format!("{{{{header.{}}}}}", key), value);
    }

    // Replace {{method}}, {{path}}, {{body}}
    result = result.replace("{{method}}", &request.method);
    result = result.replace("{{path}}", &request.path);
    result = result.replace("{{body}}", &request.body);

    result.into_bytes()
}

/// Get HTTP status text
fn http_status_text(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        410 => "Gone",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown",
    }
}

/// Run the mock server from CLI arguments
pub async fn run_mock_server(
    config_path: Option<&std::path::Path>,
    port: Option<u16>,
    routes_args: &[String],
) -> Result<(), QuicpulseError> {
    let mut config = if let Some(path) = config_path {
        MockServerConfig::load(path)?
    } else {
        MockServerConfig::default()
    };

    // Override port if specified
    if let Some(p) = port {
        config.port = p;
    }

    // Parse route arguments: "METHOD:PATH:BODY" or "METHOD:PATH:@FILE"
    for route_arg in routes_args {
        let parts: Vec<&str> = route_arg.splitn(3, ':').collect();
        if parts.len() < 2 {
            return Err(QuicpulseError::Argument(
                format!("Invalid route format '{}'. Expected METHOD:PATH or METHOD:PATH:BODY", route_arg)
            ));
        }

        let method = match parts[0].to_uppercase().as_str() {
            "GET" => super::routes::HttpMethod::Get,
            "POST" => super::routes::HttpMethod::Post,
            "PUT" => super::routes::HttpMethod::Put,
            "DELETE" => super::routes::HttpMethod::Delete,
            "PATCH" => super::routes::HttpMethod::Patch,
            "*" => super::routes::HttpMethod::Any,
            _ => return Err(QuicpulseError::Argument(format!("Unknown HTTP method: {}", parts[0]))),
        };

        let path = parts[1].to_string();

        let response = if parts.len() > 2 {
            let body = parts[2];
            if body.starts_with('@') {
                // Load from file
                let file_path = &body[1..];
                ResponseConfig {
                    body_file: Some(file_path.to_string()),
                    ..Default::default()
                }
            } else if body.starts_with('{') || body.starts_with('[') {
                // JSON body
                let json: serde_json::Value = serde_json::from_str(body)
                    .map_err(|e| QuicpulseError::Argument(format!("Invalid JSON: {}", e)))?;
                ResponseConfig::json_body(json)
            } else {
                ResponseConfig::text(body)
            }
        } else {
            ResponseConfig::text("OK")
        };

        config.routes.push(super::routes::RouteConfig {
            method,
            path,
            response,
            priority: 0,
            enabled: true,
            name: None,
        });
    }

    let server = MockServer::new(config)?;
    server.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let request = "GET /api/users?page=1 HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\r\n{\"test\": true}";
        let parsed = parse_request(request).unwrap();

        assert_eq!(parsed.method, "GET");
        assert_eq!(parsed.path, "/api/users");
        assert_eq!(parsed.query.get("page"), Some(&"1".to_string()));
        assert_eq!(parsed.headers.get("host"), Some(&"localhost".to_string()));
    }

    #[test]
    fn test_template_substitution() {
        let body = b"Hello {{name}}, you requested {{path}}";
        let request = ParsedRequest {
            method: "GET".to_string(),
            path: "/test".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
        };
        let params: HashMap<String, String> = [("name".to_string(), "World".to_string())].into_iter().collect();

        let result = apply_template(body, &request, &params);
        assert_eq!(String::from_utf8_lossy(&result), "Hello World, you requested /test");
    }
}
