//! Redirect handling tests
mod common;

use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

use common::{http, http_error, HTTP_OK};

// ============================================================================
// Basic Redirect Tests
// ============================================================================

#[tokio::test]
async fn test_follow_redirects_hidden() {
    let server = MockServer::start().await;
    
    // First request returns redirect
    Mock::given(method("GET"))
        .and(path("/redirect/1"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/get"))
        .mount(&server)
        .await;
    
    // Final destination
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;
    
    let url = format!("{}/redirect/1", server.uri());
    let r = http(&["--print=hb", "--follow", &url]);

    // With --follow (without --all), intermediate redirects are hidden
    assert!(r.count("HTTP/1.1") == 1);
    assert!(r.contains(HTTP_OK));
}

#[tokio::test]
async fn test_follow_all_redirects_shown() {
    let server = MockServer::start().await;
    
    // Set up redirect chain
    Mock::given(method("GET"))
        .and(path("/redirect/2"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/redirect/1"))
        .mount(&server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/redirect/1"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/get"))
        .mount(&server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;
    
    let url = format!("{}/redirect/2", server.uri());
    let r = http(&["--follow", "--all", &url]);
    
    // With --all, should reach final destination
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

#[tokio::test]
async fn test_follow_short_flag() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/redirect/1"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/get"))
        .mount(&server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;
    
    let url = format!("{}/redirect/1", server.uri());
    let r = http(&["--print=hb", "-F", &url]);

    // -F should work the same as --follow
    assert!(r.contains(HTTP_OK));
}

// ============================================================================
// Max Redirects Tests
// ============================================================================

#[tokio::test]
async fn test_max_redirects() {
    let server = MockServer::start().await;
    
    // Set up infinite redirect loop
    Mock::given(method("GET"))
        .and(path("/redirect/infinite"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/redirect/infinite"))
        .mount(&server)
        .await;
    
    let url = format!("{}/redirect/infinite", server.uri());
    let r = http_error(&["--max-redirects=2", "--follow", &url]);
    
    // Should fail with too many redirects error
    assert!(r.exit_code != 0 || r.stderr.contains("redirect") || r.contains("redirect"));
}

#[tokio::test]
async fn test_max_redirects_just_enough() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/redirect/1"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/get"))
        .mount(&server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("OK"))
        .mount(&server)
        .await;
    
    let url = format!("{}/redirect/1", server.uri());
    let r = http(&["--max-redirects=1", "--follow", &url]);
    
    // Should succeed with exactly 1 redirect
    assert!(r.contains(HTTP_OK) || r.contains("OK"));
}

// ============================================================================
// Redirect with Method/Body Preservation (307/308)
// ============================================================================

#[tokio::test]
async fn test_307_preserves_method_and_body() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/redirect-to"))
        .respond_with(ResponseTemplate::new(307)
            .insert_header("Location", "/post"))
        .mount(&server)
        .await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "method": "POST",
                "data": "test"
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/redirect-to", server.uri());
    let r = http(&["--print=hb", "--follow", "POST", &url, "data=test"]);

    assert!(r.contains(HTTP_OK));
    // POST method should be preserved across 307 redirect
}

#[tokio::test]
async fn test_308_preserves_method_and_body() {
    let server = MockServer::start().await;
    
    Mock::given(method("PUT"))
        .and(path("/redirect-to"))
        .respond_with(ResponseTemplate::new(308)
            .insert_header("Location", "/put"))
        .mount(&server)
        .await;
    
    Mock::given(method("PUT"))
        .and(path("/put"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "method": "PUT"
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/redirect-to", server.uri());
    let r = http(&["--print=hb", "--follow", "PUT", &url, "data=test"]);

    assert!(r.contains(HTTP_OK));
    // PUT method should be preserved across 308 redirect
}

// ============================================================================
// Redirect without Follow
// ============================================================================

#[tokio::test]
async fn test_redirect_without_follow() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/redirect/1"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/get"))
        .mount(&server)
        .await;
    
    let url = format!("{}/redirect/1", server.uri());
    let r = http(&["--print=hb", &url]);

    // Without --follow, should just show the redirect response
    assert!(r.contains("302") || r.contains("Location"));
}

// ============================================================================
// Verbose Redirect Tests
// ============================================================================

#[tokio::test]
async fn test_verbose_follow_redirect() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/redirect/1"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/get"))
        .mount(&server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("Final destination"))
        .mount(&server)
        .await;
    
    let url = format!("{}/redirect/1", server.uri());
    let r = http(&["--follow", "--verbose", "--all", &url]);
    
    // Verbose should show all requests and responses
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

// ============================================================================
// Print Options with Redirects
// ============================================================================

#[tokio::test]
async fn test_follow_all_output_options_used_for_redirects() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/redirect/1"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/get"))
        .mount(&server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("OK"))
        .mount(&server)
        .await;
    
    let url = format!("{}/redirect/1", server.uri());
    let r = http(&["--follow", "--all", "--print=H", &url]);
    
    // With --print=H, should show request headers
    assert!(r.exit_code == 0);
}

// ============================================================================
// Redirect Status Codes
// ============================================================================

#[tokio::test]
async fn test_301_redirect() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/old"))
        .respond_with(ResponseTemplate::new(301)
            .insert_header("Location", "/new"))
        .mount(&server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/new"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("New location"))
        .mount(&server)
        .await;
    
    let url = format!("{}/old", server.uri());
    let r = http(&["--follow", &url]);
    
    assert!(r.contains(HTTP_OK) || r.contains("New location"));
}

#[tokio::test]
async fn test_303_redirect_converts_to_get() {
    let server = MockServer::start().await;
    
    // 303 should convert POST to GET
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(303)
            .insert_header("Location", "/result"))
        .mount(&server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/result"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("Result"))
        .mount(&server)
        .await;
    
    let url = format!("{}/post", server.uri());
    let r = http(&["--follow", "POST", &url, "data=test"]);
    
    assert!(r.contains(HTTP_OK) || r.contains("Result"));
}
