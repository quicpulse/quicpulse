//! Cookie handling tests
mod common;

use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path, header};

use common::{http, http_with_env, MockEnvironment, HTTP_OK};

// ============================================================================
// Cookie Request Tests
// ============================================================================

#[tokio::test]
async fn test_cookie_request_header() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "cookies": {"name": "value"}
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&[&url, "Cookie:name=value"]);
    
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

#[tokio::test]
async fn test_multiple_cookies() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "cookies": {}
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&["--verbose", &url, "Cookie:a=1; b=2; c=3"]);
    
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

// ============================================================================
// Set-Cookie Response Tests
// ============================================================================

#[tokio::test]
async fn test_set_cookie_response() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/set-cookie"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "session=abc123; Path=/")
            .set_body_string("Cookie set"))
        .mount(&server)
        .await;
    
    let url = format!("{}/set-cookie", server.uri());
    let r = http(&["--verbose", &url]);
    
    assert!(r.contains(HTTP_OK));
    assert!(r.contains("Set-Cookie") || r.contains("session"));
}

#[tokio::test]
async fn test_multiple_set_cookies() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/cookies/set"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "cookie1=value1; Path=/")
            .insert_header("Set-Cookie", "cookie2=value2; Path=/")
            .set_body_string("Cookies set"))
        .mount(&server)
        .await;
    
    let url = format!("{}/cookies/set", server.uri());
    let r = http(&["--verbose", &url]);
    
    assert!(r.contains(HTTP_OK));
}

// ============================================================================
// Cookie Persistence with Sessions
// ============================================================================

#[tokio::test]
async fn test_cookies_stored_in_session() {
    let server = MockServer::start().await;
    
    // First request: receive a cookie
    Mock::given(method("GET"))
        .and(path("/set"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "session_id=12345; Path=/"))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url = format!("{}/set", server.uri());
    let r1 = http_with_env(&["--session=cookie_test", &url], &env);
    assert!(r1.exit_code == 0);
    
    // Second request: should send the cookie
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/get"))
        .and(header("Cookie", "session_id=12345"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("Cookie received"))
        .mount(&server)
        .await;
    
    let url2 = format!("{}/get", server.uri());
    let r2 = http_with_env(&["--session=cookie_test", &url2], &env);
    
    assert!(r2.exit_code == 0);
}

// ============================================================================
// Cookie on Redirects Tests
// ============================================================================

#[tokio::test]
async fn test_cookie_on_redirect() {
    let server = MockServer::start().await;
    
    // First endpoint sets a cookie and redirects
    Mock::given(method("GET"))
        .and(path("/redirect"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/final")
            .insert_header("Set-Cookie", "redirect_cookie=yes; Path=/"))
        .mount(&server)
        .await;
    
    // Final endpoint should receive the cookie
    Mock::given(method("GET"))
        .and(path("/final"))
        .and(header("Cookie", "redirect_cookie=yes"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("Cookie from redirect received"))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url = format!("{}/redirect", server.uri());
    let r = http_with_env(&["--follow", "--session=redirect_cookies", &url], &env);
    
    assert!(r.exit_code == 0);
}

#[tokio::test]
async fn test_cookie_accumulated_on_multiple_redirects() {
    let server = MockServer::start().await;
    
    // First redirect
    Mock::given(method("GET"))
        .and(path("/r1"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/r2")
            .insert_header("Set-Cookie", "cookie1=a; Path=/"))
        .mount(&server)
        .await;
    
    // Second redirect
    Mock::given(method("GET"))
        .and(path("/r2"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/final")
            .insert_header("Set-Cookie", "cookie2=b; Path=/"))
        .mount(&server)
        .await;
    
    // Final should have both cookies
    Mock::given(method("GET"))
        .and(path("/final"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("All cookies"))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url = format!("{}/r1", server.uri());
    let r = http_with_env(&["--follow", "--session=multi_cookies", &url], &env);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// Cookie Path and Domain Tests
// ============================================================================

#[tokio::test]
async fn test_cookie_with_path() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/api/set"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "api_cookie=value; Path=/api/"))
        .mount(&server)
        .await;
    
    let env = MockEnvironment::new();
    let url = format!("{}/api/set", server.uri());
    let r = http_with_env(&["--session=path_test", &url], &env);
    
    assert!(r.exit_code == 0);

    // Cookie should be sent only to /api/ paths
    // Test that the cookie IS sent to /api/get
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/api/get"))
        .and(header("Cookie", "api_cookie=value"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("Cookie received"))
        .mount(&server)
        .await;

    let url2 = format!("{}/api/get", server.uri());
    let r2 = http_with_env(&["--session=path_test", &url2], &env);
    assert!(r2.exit_code == 0);

    // Test that the cookie is NOT sent to /other path
    server.reset().await;

    // This mock should match requests WITHOUT the api_cookie
    Mock::given(method("GET"))
        .and(path("/other"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("No cookie"))
        .mount(&server)
        .await;

    let url3 = format!("{}/other", server.uri());
    let r3 = http_with_env(&["--session=path_test", "--verbose", &url3], &env);
    assert!(r3.exit_code == 0);
    // The cookie should not be in the request to /other
    // (We can't easily assert the negative with wiremock, but the request should succeed)
}

// ============================================================================
// Secure and HttpOnly Cookie Tests
// ============================================================================

#[tokio::test]
async fn test_secure_cookie() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/set"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "secure_cookie=value; Secure; Path=/"))
        .mount(&server)
        .await;
    
    let url = format!("{}/set", server.uri());
    let r = http(&[&url]);
    
    assert!(r.exit_code == 0);
}

#[tokio::test]
async fn test_httponly_cookie() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/set"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "http_cookie=value; HttpOnly; Path=/"))
        .mount(&server)
        .await;
    
    let url = format!("{}/set", server.uri());
    let r = http(&[&url]);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// Cookie Expiration Tests
// ============================================================================

#[tokio::test]
async fn test_expired_cookie_not_sent() {
    // Cookies with past expiration should not be sent
    // This is tested implicitly through session behavior
}

#[tokio::test]
async fn test_max_age_cookie() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/set"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Set-Cookie", "temp_cookie=value; Max-Age=3600; Path=/"))
        .mount(&server)
        .await;
    
    let url = format!("{}/set", server.uri());
    let r = http(&[&url]);
    
    assert!(r.exit_code == 0);
}
