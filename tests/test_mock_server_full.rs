//! Unit tests for MockServerConfig, Route patterns, and response matching

use quicpulse::mock::routes::{HttpMethod, ResponseConfig, Route, RouteConfig};
use quicpulse::mock::MockServerConfig;
use serde_json::json;

#[test]
fn test_http_method_matching() {
    assert!(HttpMethod::Get.matches("get"));
    assert!(HttpMethod::Get.matches("GET"));
    assert!(!HttpMethod::Get.matches("POST"));

    assert!(HttpMethod::Post.matches("post"));
    assert!(HttpMethod::Put.matches("put"));
    assert!(HttpMethod::Delete.matches("delete"));
    assert!(HttpMethod::Patch.matches("patch"));
    assert!(HttpMethod::Head.matches("head"));
    assert!(HttpMethod::Options.matches("options"));
    assert!(HttpMethod::Any.matches("ANY_CUSTOM_METHOD"));
}

#[test]
fn test_route_pattern_parameter_matching() {
    let route_cfg = RouteConfig {
        method: HttpMethod::Get,
        path: "/api/users/:user_id/posts/:post_id".to_string(),
        response: ResponseConfig::text("User Post Details"),
        priority: 10,
        enabled: true,
        name: Some("get_user_post".to_string()),
    };

    let route = Route::new(route_cfg).unwrap();
    let params_opt = route.matches("GET", "/api/users/42/posts/101");
    assert!(params_opt.is_some());
    let params = params_opt.unwrap();
    assert_eq!(params.get("user_id"), Some(&"42".to_string()));
    assert_eq!(params.get("post_id"), Some(&"101".to_string()));

    assert!(route.matches("POST", "/api/users/42/posts/101").is_none());
    assert!(route.matches("GET", "/api/users/42").is_none());
}

#[test]
fn test_route_wildcard_matching() {
    let route_cfg = RouteConfig {
        method: HttpMethod::Any,
        path: "/static/**".to_string(),
        response: ResponseConfig::text("static asset"),
        priority: 0,
        enabled: true,
        name: None,
    };

    let route = Route::new(route_cfg).unwrap();
    assert!(route.matches("GET", "/static/images/logo.png").is_some());
    assert!(route.matches("HEAD", "/static/css/style.css").is_some());
}

#[test]
fn test_mock_server_config_builder() {
    let config = MockServerConfig::new()
        .with_port(9595)
        .add_route(RouteConfig::get("/health", "ok"))
        .add_route(RouteConfig::post_json("/echo", json!({"received": true})));

    assert_eq!(config.port, 9595);
    assert_eq!(config.routes.len(), 2);
}
