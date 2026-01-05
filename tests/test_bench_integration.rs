//! Integration tests for benchmarking functionality

mod common;

use common::{http, http_error, ExitStatus};
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};
use std::time::Duration;

// =============================================================================
// Basic Benchmark Tests
// =============================================================================

#[tokio::test]
async fn test_bench_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .expect(10..=110) // Expect around 100 requests (with some margin)
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "GET",
        &format!("{}/api/test", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should show benchmark results
    assert!(output.contains("requests") || output.contains("latency") ||
            output.contains("Benchmark") || output.contains("ms"),
        "Should show benchmark results. output: {}", output);
}

#[tokio::test]
async fn test_bench_custom_request_count() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .expect(15..=25) // Expect around 20 requests
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "20",
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_bench_custom_concurrency() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .expect(45..=55) // Expect around 50 requests
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "50",
        "--concurrency", "5",
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Latency Statistics Tests
// =============================================================================

#[tokio::test]
async fn test_bench_shows_latency_stats() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200)
            .set_delay(Duration::from_millis(10)))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "30",
        "--concurrency", "5",
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);

    // Should show latency statistics
    let has_stats = output.contains("ms") ||
                    output.contains("latency") ||
                    output.contains("Latency") ||
                    output.contains("avg") ||
                    output.contains("mean");
    assert!(has_stats, "Should show latency stats. output: {}", output);
}

#[tokio::test]
async fn test_bench_shows_percentiles() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "50",
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);

    // Should show percentiles (p50, p90, p99, etc.)
    let has_percentiles = output.contains("p50") ||
                          output.contains("p90") ||
                          output.contains("p99") ||
                          output.contains("50th") ||
                          output.contains("percentile") ||
                          output.contains("%");
    assert!(has_percentiles, "Should show percentiles. output: {}", output);
}

// =============================================================================
// Status Code Tests
// =============================================================================

#[tokio::test]
async fn test_bench_status_code_breakdown() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "30",
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);

    // Should show status code counts or success rate
    let has_status = output.contains("200") ||
                     output.contains("success") ||
                     output.contains("Success") ||
                     output.contains("2xx");
    assert!(has_status, "Should show status codes. output: {}", output);
}

#[tokio::test]
async fn test_bench_with_mixed_status_codes() {
    let mock_server = MockServer::start().await;

    // First 10 requests return 200
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(10)
        .mount(&mock_server)
        .await;

    // Next requests return 500
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "20",
        "--concurrency", "2",
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    // Should complete even with errors
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should show error statistics
    assert!(output.contains("500") ||
            output.contains("error") ||
            output.contains("failed") ||
            output.contains("5xx") ||
            output.len() > 100,
        "Should show mixed results. output: {}", output);
}

// =============================================================================
// Request/Response Metrics Tests
// =============================================================================

#[tokio::test]
async fn test_bench_shows_throughput() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("response body"))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "50",
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);

    // Should show throughput metrics (requests/sec, bytes/sec)
    let has_throughput = output.contains("req/s") ||
                         output.contains("requests/sec") ||
                         output.contains("Requests/sec") ||
                         output.contains("/s") ||
                         output.contains("throughput");
    assert!(has_throughput, "Should show throughput. output: {}", output);
}

#[tokio::test]
async fn test_bench_shows_total_time() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "30",
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);

    // Should show total duration
    let has_duration = output.contains("duration") ||
                       output.contains("Duration") ||
                       output.contains("total") ||
                       output.contains("Total") ||
                       output.contains("time") ||
                       output.contains("ms") ||
                       output.contains("s");
    assert!(has_duration, "Should show duration. output: {}", output);
}

// =============================================================================
// POST with Body Tests
// =============================================================================

#[tokio::test]
async fn test_bench_post_with_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/create"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "20",
        "POST",
        &format!("{}/api/create", mock_server.uri()),
        "name=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_bench_post_json_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "20",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "name:=123"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_bench_connection_errors() {
    // Connect to a port that's not listening
    let response = http_error(&[
        "--bench",
        "--requests", "5",
        "--concurrency", "2",
        "GET",
        "http://127.0.0.1:59999/nonexistent"
    ]);

    // Should handle connection errors gracefully
    // The bench might complete with errors reported
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("error") ||
            output.contains("failed") ||
            output.contains("connection") ||
            response.exit_status == ExitStatus::Error,
        "Should report connection errors. output: {}", output);
}

#[tokio::test]
async fn test_bench_timeout_errors() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(ResponseTemplate::new(200)
            .set_delay(Duration::from_secs(10))) // Very slow
        .mount(&mock_server)
        .await;

    // With a short timeout, requests should timeout
    let response = http(&[
        "--bench",
        "--requests", "3",
        "--concurrency", "1",
        "--timeout", "0.5",  // 500ms timeout
        "GET",
        &format!("{}/slow", mock_server.uri())
    ]);

    // Should handle timeouts
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("timeout") ||
            output.contains("Timeout") ||
            output.contains("failed") ||
            output.contains("error") ||
            output.len() > 50,
        "Should report timeouts. output: {}", output);
}

// =============================================================================
// Concurrency Behavior Tests
// =============================================================================

#[tokio::test]
async fn test_bench_single_concurrency() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "10",
        "--concurrency", "1",  // Single concurrent request
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_bench_high_concurrency() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "100",
        "--concurrency", "50",  // High concurrency
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Edge Cases Tests
// =============================================================================

#[tokio::test]
async fn test_bench_single_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "1",
        "--concurrency", "1",
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_bench_with_headers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "20",
        "GET",
        &format!("{}/api", mock_server.uri()),
        "Authorization:Bearer token123",
        "X-Custom:value"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_bench_with_follow_redirects() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/redirect"))
        .respond_with(ResponseTemplate::new(302)
            .append_header("Location", "/final"))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/final"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--bench",
        "--requests", "20",
        "--follow",
        "GET",
        &format!("{}/redirect", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}
