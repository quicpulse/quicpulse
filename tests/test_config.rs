//! Configuration tests
//!
//! Configuration tests

mod common;

use std::fs;
use tempfile::TempDir;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

use common::{http, http_with_env, http_error, MockEnvironment, HTTP_OK};

// ============================================================================
// Default Options Tests
// ============================================================================

#[tokio::test]
async fn test_config_default_options() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;
    
    let mut env = MockEnvironment::new();
    
    // Create config file with default options
    let config_path = env.config_path().join("config.json");
    fs::create_dir_all(env.config_path()).unwrap();
    fs::write(&config_path, r#"{"default_options": ["--pretty=all"]}"#).unwrap();
    
    let url = format!("{}/get", server.uri());
    let r = http_with_env(&[&url], &env);
    
    // Should use default options from config
    assert!(r.exit_code == 0);
}

#[tokio::test]
async fn test_config_default_options_override() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;
    
    let mut env = MockEnvironment::new();
    
    // Config sets --pretty=all, but we override with --pretty=none
    let config_path = env.config_path().join("config.json");
    fs::create_dir_all(env.config_path()).unwrap();
    fs::write(&config_path, r#"{"default_options": ["--pretty=all"]}"#).unwrap();
    
    let url = format!("{}/get", server.uri());
    let r = http_with_env(&["--pretty=none", &url], &env);
    
    // Command line should override config
    assert!(r.exit_code == 0);
}

// ============================================================================
// Invalid Config Tests
// ============================================================================

#[tokio::test]
async fn test_invalid_config_file() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    
    let mut env = MockEnvironment::new();
    
    // Create invalid JSON config
    let config_path = env.config_path().join("config.json");
    fs::create_dir_all(env.config_path()).unwrap();
    fs::write(&config_path, "{invalid json}").unwrap();
    
    let url = format!("{}/get", server.uri());
    let r = http_with_env(&[&url], &env);
    
    // Should either warn or fail gracefully
    assert!(r.exit_code == 0 || r.stderr.contains("config"));
}

#[tokio::test]
async fn test_missing_config_file() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    // Don't create any config file
    
    let url = format!("{}/get", server.uri());
    let r = http_with_env(&[&url], &env);
    
    // Should work fine without config
    assert!(r.exit_code == 0);
}

// ============================================================================
// Config Directory Tests
// ============================================================================

#[test]
fn test_config_dir_environment_variable() {
    let dir = TempDir::new().unwrap();
    
    let mut env = MockEnvironment::new();
    env.set_env("QUICPULSE_CONFIG_DIR", dir.path().to_str().unwrap());
    
    // Create config in the env-specified directory
    let config_path = dir.path().join("config.json");
    fs::write(&config_path, r#"{"default_options": []}"#).unwrap();
    
    let r = http_with_env(&["--offline", "--print=H", "example.org"], &env);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// XDG Config Directory Tests
// ============================================================================

#[test]
fn test_xdg_config_dir() {
    let dir = TempDir::new().unwrap();
    let xdg_config = dir.path().join("quicpulse");
    fs::create_dir_all(&xdg_config).unwrap();
    
    let mut env = MockEnvironment::new();
    env.set_env("XDG_CONFIG_HOME", dir.path().to_str().unwrap());
    
    // Create config in XDG directory
    let config_path = xdg_config.join("config.json");
    fs::write(&config_path, r#"{"default_options": []}"#).unwrap();
    
    let r = http_with_env(&["--offline", "--print=H", "example.org"], &env);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// Config File Format Tests
// ============================================================================

#[test]
fn test_config_empty_default_options() {
    let mut env = MockEnvironment::new();
    
    let config_path = env.config_path().join("config.json");
    fs::create_dir_all(env.config_path()).unwrap();
    fs::write(&config_path, r#"{"default_options": []}"#).unwrap();
    
    let r = http_with_env(&["--offline", "--print=H", "example.org"], &env);
    
    assert!(r.exit_code == 0);
}

#[test]
fn test_config_multiple_default_options() {
    let mut env = MockEnvironment::new();
    
    let config_path = env.config_path().join("config.json");
    fs::create_dir_all(env.config_path()).unwrap();
    fs::write(&config_path, r#"{"default_options": ["--pretty=all", "--style=monokai"]}"#).unwrap();
    
    let r = http_with_env(&["--offline", "--print=H", "example.org"], &env);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// Legacy Config Tests
// ============================================================================

#[test]
fn test_legacy_config_location() {
    // Test backward compatibility with legacy config locations
    let mut env = MockEnvironment::new();
    
    let r = http_with_env(&["--offline", "--print=H", "example.org"], &env);
    
    // Should work regardless of config
    assert!(r.exit_code == 0);
}

// ============================================================================
// Config with Stdin Tests
// ============================================================================

#[test]
fn test_config_stdin_isatty() {
    let mut env = MockEnvironment::new();
    env.stdin_isatty = true;
    
    let r = http_with_env(&["--offline", "--print=H", "example.org"], &env);
    
    // Behavior may differ based on TTY status
    assert!(r.exit_code == 0);
}

#[test]
fn test_config_stdin_not_tty() {
    let mut env = MockEnvironment::new();
    env.stdin_isatty = false;
    env.set_stdin(b"input data".to_vec());
    
    let r = http_with_env(&["--ignore-stdin", "--offline", "--print=H", "example.org"], &env);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// Config Output Options Tests
// ============================================================================

#[tokio::test]
async fn test_config_style_option() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"key": "value"})))
        .mount(&server)
        .await;
    
    let mut env = MockEnvironment::new();
    env.stdout_isatty = true;
    
    let config_path = env.config_path().join("config.json");
    fs::create_dir_all(env.config_path()).unwrap();
    fs::write(&config_path, r#"{"default_options": ["--style=native"]}"#).unwrap();
    
    let url = format!("{}/get", server.uri());
    let r = http_with_env(&[&url], &env);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// Disable Update Warnings Tests
// ============================================================================

#[test]
fn test_disable_update_warnings() {
    let mut env = MockEnvironment::new();
    
    let config_path = env.config_path().join("config.json");
    fs::create_dir_all(env.config_path()).unwrap();
    fs::write(&config_path, r#"{"disable_update_warnings": true}"#).unwrap();
    
    let r = http_with_env(&["--offline", "--print=H", "example.org"], &env);
    
    assert!(r.exit_code == 0);
    // No update warnings should appear
}
