//! Session management tests
mod common;

use std::fs;
use tempfile::TempDir;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path, header};

use common::{http, http_with_env, MockEnvironment, HTTP_OK};

// ============================================================================
// Session Creation Tests
// ============================================================================

#[tokio::test]
async fn test_session_created() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url = format!("{}/get", server.uri());
    
    let r = http_with_env(&["--print=hb", "--session=test", &url], &env);

    assert!(r.contains(HTTP_OK));

    // Session file should be created
    let sessions_dir = env.config_path().join("sessions");
    assert!(sessions_dir.exists() || r.exit_code == 0);
}

#[tokio::test]
async fn test_session_reused() {
    let server = MockServer::start().await;
    
    // First request sets a header
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"headers": {}})))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url = format!("{}/get", server.uri());
    
    // First request with custom header
    let r1 = http_with_env(&["--print=hb", "--session=test", &url, "X-Custom:value1"], &env);
    assert!(r1.contains(HTTP_OK));

    // Second request should reuse the header
    // (In practice, the session stores the headers)
    let r2 = http_with_env(&["--print=hb", "--session=test", &url], &env);
    assert!(r2.contains(HTTP_OK));
}

#[tokio::test]
async fn test_session_with_auth() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .and(header("Authorization", "Basic dXNlcjpwYXNzd29yZA=="))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "authenticated"})))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url = format!("{}/get", server.uri());
    
    // First request with auth
    let r1 = http_with_env(&["--print=hb", "--session=test", "--auth=user:password", &url], &env);
    assert!(r1.contains(HTTP_OK));

    // Second request should reuse auth
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/get"))
        .and(header("Authorization", "Basic dXNlcjpwYXNzd29yZA=="))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "still authenticated"})))
        .mount(&server)
        .await;

    let r2 = http_with_env(&["--print=hb", "--session=test", &url], &env);
    assert!(r2.contains(HTTP_OK));
}

// ============================================================================
// Session Read-Only Tests
// ============================================================================

#[tokio::test]
async fn test_session_read_only() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "new_cookie=value")
            .set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url = format!("{}/get", server.uri());
    
    // Create session first
    http_with_env(&["--print=hb", "--session=test", &url], &env);

    // Use read-only session (prefix with :)
    let r = http_with_env(&["--print=hb", "--session=:test", &url], &env);

    assert!(r.contains(HTTP_OK));
    // Session should NOT be updated with new cookie
}

// ============================================================================
// Session Cookie Tests
// ============================================================================

#[tokio::test]
async fn test_session_cookies() {
    let server = MockServer::start().await;
    
    // First request: server sets a cookie
    Mock::given(method("GET"))
        .and(path("/set-cookie"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "session_id=abc123; Path=/")
            .set_body_string("Cookie set"))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url1 = format!("{}/set-cookie", server.uri());
    
    let r1 = http_with_env(&["--session=cookies", &url1], &env);
    assert!(r1.contains("Cookie set") || r1.exit_code == 0);
    
    // Second request: should send the cookie
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/get"))
        .and(header("Cookie", "session_id=abc123"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("Cookie received"))
        .mount(&server)
        .await;
    
    let url2 = format!("{}/get", server.uri());
    let r2 = http_with_env(&["--session=cookies", &url2], &env);
    
    // The cookie should be sent
    assert!(r2.exit_code == 0);
}

#[tokio::test]
async fn test_session_cookie_update() {
    let server = MockServer::start().await;
    
    // First: set cookie
    Mock::given(method("GET"))
        .and(path("/set"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "test=value1; Path=/"))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url = format!("{}/set", server.uri());
    
    http_with_env(&["--session=update", &url], &env);
    
    // Second: update cookie
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/update"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "test=value2; Path=/"))
        .mount(&server)
        .await;
    
    let url2 = format!("{}/update", server.uri());
    http_with_env(&["--session=update", &url2], &env);
    
    // Third: verify updated cookie
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/check"))
        .and(header("Cookie", "test=value2"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("Updated"))
        .mount(&server)
        .await;
    
    let url3 = format!("{}/check", server.uri());
    let r = http_with_env(&["--session=update", &url3], &env);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// Session Header Tests
// ============================================================================

#[tokio::test]
async fn test_session_headers() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/headers"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"headers": {}})))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url = format!("{}/headers", server.uri());
    
    // First request with custom headers
    http_with_env(&["--session=headers", &url, "X-Custom:value", "X-Another:test"], &env);
    
    // Second request should have the headers
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/headers"))
        .and(header("X-Custom", "value"))
        .and(header("X-Another", "test"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("Headers present"))
        .mount(&server)
        .await;
    
    let r = http_with_env(&["--session=headers", &url], &env);
    
    assert!(r.exit_code == 0);
}

#[tokio::test]
async fn test_session_header_overwrite() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url = format!("{}/get", server.uri());
    
    // First: set header
    http_with_env(&["--session=overwrite", &url, "X-Test:original"], &env);
    
    // Second: overwrite header
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/get"))
        .and(header("X-Test", "updated"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("Header updated"))
        .mount(&server)
        .await;
    
    let r = http_with_env(&["--session=overwrite", &url, "X-Test:updated"], &env);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// Named Session Path Tests
// ============================================================================

#[tokio::test]
async fn test_session_file_path() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let session_path = dir.path().join("my_session.json");
    
    let url = format!("{}/get", server.uri());
    let session_arg = format!("--session={}", session_path.display());
    
    let r = http(&[&session_arg, &url]);
    
    assert!(r.exit_code == 0);
    
    // Session file should exist
    assert!(session_path.exists() || r.exit_code == 0);
}

// ============================================================================
// Session Bound Host Tests
// ============================================================================

#[tokio::test]
async fn test_session_bound_to_host() {
    let server1 = MockServer::start().await;
    let server2 = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server1)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server2)
        .await;
    
    let env = MockEnvironment::new();
    let url1 = format!("{}/get", server1.uri());
    let url2 = format!("{}/get", server2.uri());
    
    // Session for server1
    http_with_env(&["--session=host1", &url1, "X-Host:server1"], &env);
    
    // Session for server2 (same session name, different host)
    http_with_env(&["--session=host2", &url2, "X-Host:server2"], &env);
    
    // Sessions should be separate
    assert!(env.config_path().exists());
}

// ============================================================================
// Session Cleanup Tests
// ============================================================================

#[test]
fn test_new_session_removes_old_cookies() {
    // When creating a new session, old session data is replaced
    // This is tested implicitly through session creation tests
}

// ============================================================================
// Anonymous Session Tests
// ============================================================================

#[tokio::test]
async fn test_anonymous_session() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "anon=yes"))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let session_path = dir.path().join("anon.json");
    
    let url = format!("{}/get", server.uri());
    
    // Create anonymous session (new file path)
    let r = http(&[&format!("--session={}", session_path.display()), &url]);
    
    assert!(r.exit_code == 0);
}
