//! Tests for the mock server module

use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

mod common;

// ============================================================================
// Route Pattern Tests
// ============================================================================

#[test]
fn test_route_exact_path() {
    use quicpulse::mock::routes::{RouteConfig, Route, HttpMethod, ResponseConfig};

    let config = RouteConfig {
        method: HttpMethod::Get,
        path: "/api/users".to_string(),
        response: ResponseConfig::text("users list"),
        priority: 0,
        enabled: true,
        name: None,
    };

    let route = Route::new(config).unwrap();

    // Should match exact path
    assert!(route.matches("GET", "/api/users").is_some());

    // Should not match different paths
    assert!(route.matches("GET", "/api/user").is_none());
    assert!(route.matches("GET", "/api/users/").is_none());
    assert!(route.matches("GET", "/api/users/1").is_none());

    // Should not match different methods
    assert!(route.matches("POST", "/api/users").is_none());
}

#[test]
fn test_route_with_parameter() {
    use quicpulse::mock::routes::{RouteConfig, Route, HttpMethod, ResponseConfig};

    let config = RouteConfig {
        method: HttpMethod::Get,
        path: "/api/users/:id".to_string(),
        response: ResponseConfig::text("user detail"),
        priority: 0,
        enabled: true,
        name: None,
    };

    let route = Route::new(config).unwrap();

    // Should match and extract parameter
    let params = route.matches("GET", "/api/users/123").unwrap();
    assert_eq!(params.get("id"), Some(&"123".to_string()));

    let params = route.matches("GET", "/api/users/abc").unwrap();
    assert_eq!(params.get("id"), Some(&"abc".to_string()));

    // Should not match without parameter
    assert!(route.matches("GET", "/api/users").is_none());
    assert!(route.matches("GET", "/api/users/").is_none());

    // Should not match nested paths
    assert!(route.matches("GET", "/api/users/123/posts").is_none());
}

#[test]
fn test_route_with_multiple_parameters() {
    use quicpulse::mock::routes::{RouteConfig, Route, HttpMethod, ResponseConfig};

    let config = RouteConfig {
        method: HttpMethod::Get,
        path: "/api/users/:userId/posts/:postId".to_string(),
        response: ResponseConfig::text("post detail"),
        priority: 0,
        enabled: true,
        name: None,
    };

    let route = Route::new(config).unwrap();

    let params = route.matches("GET", "/api/users/123/posts/456").unwrap();
    assert_eq!(params.get("userId"), Some(&"123".to_string()));
    assert_eq!(params.get("postId"), Some(&"456".to_string()));
}

#[test]
fn test_route_wildcard_single() {
    use quicpulse::mock::routes::{RouteConfig, Route, HttpMethod, ResponseConfig};

    let config = RouteConfig {
        method: HttpMethod::Get,
        path: "/api/*/items".to_string(),
        response: ResponseConfig::text("items"),
        priority: 0,
        enabled: true,
        name: None,
    };

    let route = Route::new(config).unwrap();

    // Should match single segment
    assert!(route.matches("GET", "/api/users/items").is_some());
    assert!(route.matches("GET", "/api/products/items").is_some());

    // Should not match multiple segments
    assert!(route.matches("GET", "/api/users/123/items").is_none());
}

#[test]
fn test_route_wildcard_double() {
    use quicpulse::mock::routes::{RouteConfig, Route, HttpMethod, ResponseConfig};

    let config = RouteConfig {
        method: HttpMethod::Get,
        path: "/api/**".to_string(),
        response: ResponseConfig::text("catch all"),
        priority: 0,
        enabled: true,
        name: None,
    };

    let route = Route::new(config).unwrap();

    // Should match any path under /api/
    assert!(route.matches("GET", "/api/users").is_some());
    assert!(route.matches("GET", "/api/users/123").is_some());
    assert!(route.matches("GET", "/api/users/123/posts/456").is_some());
}

#[test]
fn test_route_method_any() {
    use quicpulse::mock::routes::{RouteConfig, Route, HttpMethod, ResponseConfig};

    let config = RouteConfig {
        method: HttpMethod::Any,
        path: "/api/endpoint".to_string(),
        response: ResponseConfig::text("any method"),
        priority: 0,
        enabled: true,
        name: None,
    };

    let route = Route::new(config).unwrap();

    // Should match any method
    assert!(route.matches("GET", "/api/endpoint").is_some());
    assert!(route.matches("POST", "/api/endpoint").is_some());
    assert!(route.matches("PUT", "/api/endpoint").is_some());
    assert!(route.matches("DELETE", "/api/endpoint").is_some());
    assert!(route.matches("PATCH", "/api/endpoint").is_some());
}

#[test]
fn test_route_disabled() {
    use quicpulse::mock::routes::{RouteConfig, Route, HttpMethod, ResponseConfig};

    let config = RouteConfig {
        method: HttpMethod::Get,
        path: "/api/disabled".to_string(),
        response: ResponseConfig::text("disabled"),
        priority: 0,
        enabled: false,
        name: None,
    };

    let route = Route::new(config).unwrap();

    // Should not match when disabled
    assert!(route.matches("GET", "/api/disabled").is_none());
}

// ============================================================================
// HTTP Method Tests
// ============================================================================

#[test]
fn test_http_method_matching() {
    use quicpulse::mock::routes::HttpMethod;

    assert!(HttpMethod::Get.matches("GET"));
    assert!(HttpMethod::Get.matches("get"));
    assert!(HttpMethod::Get.matches("Get"));
    assert!(!HttpMethod::Get.matches("POST"));

    assert!(HttpMethod::Post.matches("POST"));
    assert!(HttpMethod::Put.matches("PUT"));
    assert!(HttpMethod::Delete.matches("DELETE"));
    assert!(HttpMethod::Patch.matches("PATCH"));
    assert!(HttpMethod::Head.matches("HEAD"));
    assert!(HttpMethod::Options.matches("OPTIONS"));

    assert!(HttpMethod::Any.matches("GET"));
    assert!(HttpMethod::Any.matches("POST"));
    assert!(HttpMethod::Any.matches("CUSTOM"));
}

// ============================================================================
// Response Config Tests
// ============================================================================

#[test]
fn test_response_config_default() {
    use quicpulse::mock::routes::ResponseConfig;

    let config = ResponseConfig::default();
    assert_eq!(config.status, 200);
    assert!(config.headers.is_empty());
    assert!(config.body.is_none());
    assert!(config.json.is_none());
    assert_eq!(config.delay_ms, 0);
}

#[test]
fn test_response_config_text() {
    use quicpulse::mock::routes::ResponseConfig;

    let config = ResponseConfig::text("Hello, World!");
    assert_eq!(config.status, 200);
    assert_eq!(config.body, Some("Hello, World!".to_string()));
    assert_eq!(config.headers.get("Content-Type"), Some(&"text/plain".to_string()));
}

#[test]
fn test_response_config_json() {
    use quicpulse::mock::routes::ResponseConfig;
    use serde_json::json;

    let config = ResponseConfig::json_body(json!({"message": "Hello"}));
    assert_eq!(config.status, 200);
    assert_eq!(config.json, Some(json!({"message": "Hello"})));
    assert_eq!(config.headers.get("Content-Type"), Some(&"application/json".to_string()));
}

#[test]
fn test_response_config_error() {
    use quicpulse::mock::routes::ResponseConfig;

    let config = ResponseConfig::error(404, "Not Found");
    assert_eq!(config.status, 404);
    assert_eq!(config.body, Some("Not Found".to_string()));
}

#[test]
fn test_response_config_get_body() {
    use quicpulse::mock::routes::ResponseConfig;
    use serde_json::json;

    // Text body
    let config = ResponseConfig::text("Hello");
    assert_eq!(config.get_body(), b"Hello");

    // JSON body
    let config = ResponseConfig::json_body(json!({"key": "value"}));
    let body = String::from_utf8(config.get_body()).unwrap();
    assert!(body.contains("key"));
    assert!(body.contains("value"));

    // Empty body
    let config = ResponseConfig::default();
    assert!(config.get_body().is_empty());
}

// ============================================================================
// Mock Server Config Tests
// ============================================================================

#[test]
fn test_mock_server_config_default() {
    use quicpulse::mock::config::MockServerConfig;

    let config = MockServerConfig::default();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8080);
    assert!(config.log_requests);
    assert!(!config.cors);
    assert!(config.routes.is_empty());
}

#[test]
fn test_mock_server_config_address() {
    use quicpulse::mock::config::MockServerConfig;

    let config = MockServerConfig::default();
    assert_eq!(config.address(), "127.0.0.1:8080");

    let config = MockServerConfig {
        host: "0.0.0.0".to_string(),
        port: 9000,
        ..Default::default()
    };
    assert_eq!(config.address(), "0.0.0.0:9000");
}

#[test]
fn test_mock_server_config_with_port() {
    use quicpulse::mock::config::MockServerConfig;

    let config = MockServerConfig::new().with_port(3000);
    assert_eq!(config.port, 3000);
}

#[test]
fn test_mock_server_config_add_route() {
    use quicpulse::mock::config::MockServerConfig;
    use quicpulse::mock::routes::RouteConfig;

    let config = MockServerConfig::new()
        .add_route(RouteConfig::get("/health", "OK"));

    assert_eq!(config.routes.len(), 1);
    assert_eq!(config.routes[0].path, "/health");
}

#[test]
fn test_mock_server_config_yaml_parse() {
    use quicpulse::mock::config::MockServerConfig;

    let yaml = r#"
host: 0.0.0.0
port: 9000
cors: true
log_requests: false
routes:
  - method: GET
    path: /api/health
    response:
      status: 200
      body: "OK"
  - method: POST
    path: /api/echo
    response:
      status: 201
      template: true
"#;

    let config = MockServerConfig::from_yaml(yaml).unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 9000);
    assert!(config.cors);
    assert!(!config.log_requests);
    assert_eq!(config.routes.len(), 2);
}

#[test]
fn test_mock_server_config_validate_valid() {
    use quicpulse::mock::config::MockServerConfig;
    use quicpulse::mock::routes::RouteConfig;

    let config = MockServerConfig::new()
        .add_route(RouteConfig::get("/api/users/:id", "user"));

    assert!(config.validate().is_ok());
}

#[test]
fn test_mock_server_config_validate_invalid_route() {
    use quicpulse::mock::config::MockServerConfig;
    use quicpulse::mock::routes::{RouteConfig, HttpMethod, ResponseConfig};

    let config = MockServerConfig {
        routes: vec![RouteConfig {
            method: HttpMethod::Get,
            path: "/api/users/:".to_string(), // Invalid: empty parameter name
            response: ResponseConfig::default(),
            priority: 0,
            enabled: true,
            name: None,
        }],
        ..Default::default()
    };

    assert!(config.validate().is_err());
}

// ============================================================================
// Request Info Tests
// ============================================================================

#[test]
fn test_request_info() {
    use quicpulse::mock::routes::RequestInfo;

    let info = RequestInfo::new(
        "GET".to_string(),
        "/api/users".to_string(),
        HashMap::from([("page".to_string(), "1".to_string())]),
        HashMap::from([("content-type".to_string(), "application/json".to_string())]),
        "{}".to_string(),
        HashMap::from([("id".to_string(), "123".to_string())]),
        "127.0.0.1".to_string(),
    );

    assert_eq!(info.method, "GET");
    assert_eq!(info.path, "/api/users");
    assert_eq!(info.query.get("page"), Some(&"1".to_string()));
    assert_eq!(info.params.get("id"), Some(&"123".to_string()));
    assert_eq!(info.client_ip, "127.0.0.1");
    assert!(!info.timestamp.is_empty());
}

// ============================================================================
// Mock Server Tests
// ============================================================================

#[test]
fn test_mock_server_simple_config() {
    use quicpulse::mock::MockServer;

    let config = MockServer::simple_config(vec![
        ("GET", "/health", "OK"),
        ("POST", "/api/echo", "received"),
    ]);

    assert_eq!(config.routes.len(), 2);
}

#[tokio::test]
async fn test_mock_server_creation() {
    use quicpulse::mock::{MockServer, MockServerConfig};
    use quicpulse::mock::routes::RouteConfig;

    let config = MockServerConfig::new()
        .with_port(0) // Use any available port
        .add_route(RouteConfig::get("/health", "OK"));

    let server = MockServer::new(config);
    assert!(server.is_ok());
}

#[tokio::test]
async fn test_mock_server_request_log() {
    use quicpulse::mock::{MockServer, MockServerConfig};

    let config = MockServerConfig::new();
    let server = MockServer::new(config).unwrap();

    // Initially empty
    let requests = server.get_requests().await;
    assert!(requests.is_empty());

    // Clear should work on empty log
    server.clear_requests().await;
    let requests = server.get_requests().await;
    assert!(requests.is_empty());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_mock_server_responds_to_requests() {
    use quicpulse::mock::{MockServer, MockServerConfig};
    use quicpulse::mock::routes::RouteConfig;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let config = MockServerConfig::new()
        .with_port(port)
        .add_route(RouteConfig::get("/health", "OK"));

    let server = MockServer::new(config).unwrap();

    // Start server in background
    let handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Make request
    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(2),
        client.get(format!("http://127.0.0.1:{}/health", port)).send()
    ).await;

    if let Ok(Ok(resp)) = response {
        assert_eq!(resp.status(), 200);
        let body = resp.text().await.unwrap();
        assert_eq!(body, "OK");
    }

    handle.abort();
}

#[tokio::test]
async fn test_mock_server_404_for_unknown_routes() {
    use quicpulse::mock::{MockServer, MockServerConfig};
    use quicpulse::mock::routes::RouteConfig;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let config = MockServerConfig::new()
        .with_port(port)
        .add_route(RouteConfig::get("/known", "found"));

    let server = MockServer::new(config).unwrap();

    let handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(2),
        client.get(format!("http://127.0.0.1:{}/unknown", port)).send()
    ).await;

    if let Ok(Ok(resp)) = response {
        assert_eq!(resp.status(), 404);
    }

    handle.abort();
}

#[tokio::test]
async fn test_mock_server_json_response() {
    use quicpulse::mock::{MockServer, MockServerConfig};
    use quicpulse::mock::routes::{RouteConfig, HttpMethod, ResponseConfig};
    use serde_json::json;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let config = MockServerConfig::new()
        .with_port(port)
        .add_route(RouteConfig {
            method: HttpMethod::Get,
            path: "/api/user".to_string(),
            response: ResponseConfig::json_body(json!({
                "id": 1,
                "name": "Test User"
            })),
            priority: 0,
            enabled: true,
            name: None,
        });

    let server = MockServer::new(config).unwrap();

    let handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(2),
        client.get(format!("http://127.0.0.1:{}/api/user", port)).send()
    ).await;

    if let Ok(Ok(resp)) = response {
        assert_eq!(resp.status(), 200);
        let json: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(json["id"], 1);
        assert_eq!(json["name"], "Test User");
    }

    handle.abort();
}

#[tokio::test]
async fn test_mock_server_cors_headers() {
    use quicpulse::mock::{MockServer, MockServerConfig};
    use quicpulse::mock::routes::RouteConfig;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let mut config = MockServerConfig::new()
        .with_port(port)
        .add_route(RouteConfig::get("/api/data", "data"));
    config.cors = true;

    let server = MockServer::new(config).unwrap();

    let handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(2),
        client.get(format!("http://127.0.0.1:{}/api/data", port)).send()
    ).await;

    if let Ok(Ok(resp)) = response {
        assert_eq!(resp.status(), 200);
        // Check CORS headers
        let cors_origin = resp.headers().get("access-control-allow-origin");
        assert!(cors_origin.is_some());
        assert_eq!(cors_origin.unwrap(), "*");
    }

    handle.abort();
}
