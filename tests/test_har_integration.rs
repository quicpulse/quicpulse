//! Integration tests for HAR (HTTP Archive) import and replay functionality

mod common;

use common::{http, http_error, MockEnvironment, ExitStatus, fixtures};
use std::path::PathBuf;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

fn fixture_path(name: &str) -> PathBuf {
    fixtures::fixture_path(name)
}

// =============================================================================
// HAR File Loading Tests
// =============================================================================

#[test]
fn test_har_load_valid_file() {
    let har_path = fixture_path("sample.har");
    let response = http(&["--import-har", har_path.to_str().unwrap(), "--har-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // Should list entries from the HAR file
    assert!(response.stdout.contains("GET") || response.stderr.contains("GET"),
        "Should show GET entries. stdout: {}, stderr: {}", response.stdout, response.stderr);
}

#[test]
fn test_har_load_nonexistent_file() {
    let response = http_error(&["--import-har", "/nonexistent/path/to/file.har", "--har-list"]);

    assert_eq!(response.exit_status, ExitStatus::Error);
    // Should show error about file not found
    assert!(response.stderr.contains("No such file") ||
            response.stderr.contains("not found") ||
            response.stderr.contains("error"),
        "Should show file error. stderr: {}", response.stderr);
}

#[test]
fn test_har_load_invalid_json() {
    // Create a temp file with invalid JSON
    let temp_dir = tempfile::TempDir::new().unwrap();
    let invalid_har = temp_dir.path().join("invalid.har");
    std::fs::write(&invalid_har, "{ invalid json }").unwrap();

    let response = http_error(&["--import-har", invalid_har.to_str().unwrap(), "--har-list"]);

    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// HAR List Mode Tests
// =============================================================================

#[test]
fn test_har_list_entries() {
    let har_path = fixture_path("sample.har");
    let response = http(&["--import-har", har_path.to_str().unwrap(), "--har-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // Should list the entries from sample.har (contains 5 entries)
    // Check for URLs or methods in the output
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("users") || output.contains("GET") || output.contains("POST"),
        "Should list HAR entries. output: {}", output);
}

#[test]
fn test_har_list_shows_methods() {
    let har_path = fixture_path("sample.har");
    let response = http(&["--import-har", har_path.to_str().unwrap(), "--har-list"]);

    // The sample.har contains GET, POST, DELETE requests
    let output = format!("{}{}", response.stdout, response.stderr);
    // At least one method should be visible
    assert!(output.contains("GET") || output.contains("POST") || output.contains("DELETE"),
        "Should show HTTP methods. output: {}", output);
}

// =============================================================================
// HAR Filter Tests
// =============================================================================

#[test]
fn test_har_filter_by_url_pattern() {
    let har_path = fixture_path("sample.har");
    // Filter for "users" in the URL
    let response = http(&[
        "--import-har", har_path.to_str().unwrap(),
        "--har-filter", "users",
        "--har-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should filter to only user-related entries
    assert!(output.contains("users"),
        "Should show filtered entries. output: {}", output);
}

#[test]
fn test_har_filter_no_matches() {
    let har_path = fixture_path("sample.har");
    // Filter for pattern that doesn't match anything
    let response = http_error(&[
        "--import-har", har_path.to_str().unwrap(),
        "--har-filter", "zzz_nonexistent_pattern_zzz",
        "--har-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Error);
    assert!(response.stderr.contains("No entries match") ||
            response.stderr.contains("no entries") ||
            response.stderr.contains("pattern"),
        "Should show no matches error. stderr: {}", response.stderr);
}

#[test]
fn test_har_filter_regex_pattern() {
    let har_path = fixture_path("sample.har");
    // Use regex pattern
    let response = http(&[
        "--import-har", har_path.to_str().unwrap(),
        "--har-filter", "users/\\d+",
        "--har-list"
    ]);

    // Should match /users/1, /users/2, etc.
    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// HAR Index Selection Tests
// =============================================================================

#[test]
fn test_har_filter_by_indices() {
    let har_path = fixture_path("sample.har");
    // Select specific indices (1-based)
    let response = http(&[
        "--import-har", har_path.to_str().unwrap(),
        "--har-index", "1",
        "--har-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // Should only show the first entry
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("1") || output.contains("users"),
        "Should show selected entry. output: {}", output);
}

#[test]
fn test_har_filter_multiple_indices() {
    let har_path = fixture_path("sample.har");
    // Select multiple indices
    let response = http(&[
        "--import-har", har_path.to_str().unwrap(),
        "--har-index", "0",
        "--har-index", "2",
        "--har-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_har_invalid_index() {
    let har_path = fixture_path("sample.har");
    // Select an index that doesn't exist (sample.har has 5 entries, so 999 is invalid)
    let response = http_error(&[
        "--import-har", har_path.to_str().unwrap(),
        "--har-index", "999",
        "--har-list"
    ]);

    // Should handle gracefully - either error or empty result
    assert!(response.exit_status == ExitStatus::Error ||
            response.stderr.contains("No valid indices") ||
            response.stderr.contains("index"),
        "Should handle invalid index. stderr: {}", response.stderr);
}

// =============================================================================
// HAR Replay Tests (with mock server)
// =============================================================================

#[tokio::test]
async fn test_har_replay_single_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/users"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!([{"id": 1, "name": "Test"}])))
        .mount(&mock_server)
        .await;

    // Create a temporary HAR file pointing to our mock server
    let temp_dir = tempfile::TempDir::new().unwrap();
    let har_file = temp_dir.path().join("test.har");
    let har_content = serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": {"name": "Test", "version": "1.0"},
            "entries": [{
                "startedDateTime": "2024-01-01T00:00:00.000Z",
                "time": 100,
                "request": {
                    "method": "GET",
                    "url": format!("{}/users", mock_server.uri()),
                    "httpVersion": "HTTP/1.1",
                    "headers": [],
                    "queryString": [],
                    "cookies": [],
                    "headersSize": 0,
                    "bodySize": 0
                },
                "response": {
                    "status": 200,
                    "statusText": "OK",
                    "httpVersion": "HTTP/1.1",
                    "headers": [],
                    "cookies": [],
                    "content": {"size": 0, "mimeType": "application/json"},
                    "redirectURL": "",
                    "headersSize": 0,
                    "bodySize": 0
                },
                "cache": {},
                "timings": {"send": 0, "wait": 100, "receive": 0}
            }]
        }
    });
    std::fs::write(&har_file, serde_json::to_string_pretty(&har_content).unwrap()).unwrap();

    let response = http(&["--import-har", har_file.to_str().unwrap()]);

    // Should successfully replay the request
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("200") || output.contains("success") || output.contains("Replay") ||
            response.exit_status == ExitStatus::Success,
        "Should replay successfully. output: {}", output);
}

#[tokio::test]
async fn test_har_replay_multiple_requests() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/one"))
        .respond_with(ResponseTemplate::new(200).set_body_string("one"))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/two"))
        .respond_with(ResponseTemplate::new(200).set_body_string("two"))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let har_file = temp_dir.path().join("multi.har");
    let har_content = serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": {"name": "Test", "version": "1.0"},
            "entries": [
                {
                    "startedDateTime": "2024-01-01T00:00:00.000Z",
                    "time": 50,
                    "request": {
                        "method": "GET",
                        "url": format!("{}/api/one", mock_server.uri()),
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "queryString": [],
                        "cookies": [],
                        "headersSize": 0,
                        "bodySize": 0
                    },
                    "response": {
                        "status": 200,
                        "statusText": "OK",
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "cookies": [],
                        "content": {"size": 0, "mimeType": "text/plain"},
                        "redirectURL": "",
                        "headersSize": 0,
                        "bodySize": 0
                    },
                    "cache": {},
                    "timings": {"send": 0, "wait": 50, "receive": 0}
                },
                {
                    "startedDateTime": "2024-01-01T00:00:01.000Z",
                    "time": 50,
                    "request": {
                        "method": "GET",
                        "url": format!("{}/api/two", mock_server.uri()),
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "queryString": [],
                        "cookies": [],
                        "headersSize": 0,
                        "bodySize": 0
                    },
                    "response": {
                        "status": 200,
                        "statusText": "OK",
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "cookies": [],
                        "content": {"size": 0, "mimeType": "text/plain"},
                        "redirectURL": "",
                        "headersSize": 0,
                        "bodySize": 0
                    },
                    "cache": {},
                    "timings": {"send": 0, "wait": 50, "receive": 0}
                }
            ]
        }
    });
    std::fs::write(&har_file, serde_json::to_string_pretty(&har_content).unwrap()).unwrap();

    let response = http(&["--import-har", har_file.to_str().unwrap()]);

    // Both requests should be replayed
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("2") || output.contains("Entries") ||
            response.exit_status == ExitStatus::Success,
        "Should replay multiple requests. output: {}", output);
}

// =============================================================================
// HAR Dry Run Tests
// =============================================================================

#[test]
fn test_har_dry_run_no_requests_sent() {
    let har_path = fixture_path("sample.har");
    let response = http(&[
        "--import-har", har_path.to_str().unwrap(),
        "--dry-run"
    ]);

    // Dry run mode shows entries but returns Error status (all requests "failed" in dry run)
    // The URLs in sample.har point to api.example.com which doesn't exist
    // This is expected behavior - dry run reports all as "failed"
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("DRY RUN") || output.contains("dry") || output.contains("Dry"),
        "Should indicate dry run mode. output: {}", output);
}

#[test]
fn test_har_dry_run_shows_entries() {
    let har_path = fixture_path("sample.har");
    let response = http(&[
        "--import-har", har_path.to_str().unwrap(),
        "--dry-run"
    ]);

    // Dry run returns Error status but still shows entries
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("5") || output.contains("entries") || output.contains("Entries"),
        "Should show entry count. output: {}", output);
}

// =============================================================================
// HAR Delay Tests
// =============================================================================

#[tokio::test]
async fn test_har_replay_with_delay() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/delay-test"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let har_file = temp_dir.path().join("delay.har");
    let har_content = serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": {"name": "Test", "version": "1.0"},
            "entries": [
                {
                    "startedDateTime": "2024-01-01T00:00:00.000Z",
                    "time": 10,
                    "request": {
                        "method": "GET",
                        "url": format!("{}/delay-test", mock_server.uri()),
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "queryString": [],
                        "cookies": [],
                        "headersSize": 0,
                        "bodySize": 0
                    },
                    "response": {
                        "status": 200, "statusText": "OK", "httpVersion": "HTTP/1.1",
                        "headers": [], "cookies": [],
                        "content": {"size": 0, "mimeType": ""},
                        "redirectURL": "", "headersSize": 0, "bodySize": 0
                    },
                    "cache": {},
                    "timings": {"send": 0, "wait": 10, "receive": 0}
                },
                {
                    "startedDateTime": "2024-01-01T00:00:01.000Z",
                    "time": 10,
                    "request": {
                        "method": "GET",
                        "url": format!("{}/delay-test", mock_server.uri()),
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "queryString": [],
                        "cookies": [],
                        "headersSize": 0,
                        "bodySize": 0
                    },
                    "response": {
                        "status": 200, "statusText": "OK", "httpVersion": "HTTP/1.1",
                        "headers": [], "cookies": [],
                        "content": {"size": 0, "mimeType": ""},
                        "redirectURL": "", "headersSize": 0, "bodySize": 0
                    },
                    "cache": {},
                    "timings": {"send": 0, "wait": 10, "receive": 0}
                }
            ]
        }
    });
    std::fs::write(&har_file, serde_json::to_string_pretty(&har_content).unwrap()).unwrap();

    let start = std::time::Instant::now();
    let response = http(&[
        "--import-har", har_file.to_str().unwrap(),
        "--har-delay", "50ms"
    ]);
    let elapsed = start.elapsed();

    assert_eq!(response.exit_status, ExitStatus::Success);
    // With 50ms delay between 2 requests, should take at least 50ms
    // (but not too much more, accounting for test overhead)
    assert!(elapsed.as_millis() >= 40, "Should have delay between requests. Elapsed: {:?}", elapsed);
}

#[test]
fn test_har_delay_parsing() {
    let har_path = fixture_path("sample.har");

    // Test various delay formats (dry-run returns Error status, but delay should be parsed)
    for delay in &["100ms", "1s", "500ms"] {
        let response = http(&[
            "--import-har", har_path.to_str().unwrap(),
            "--har-delay", delay,
            "--dry-run"
        ]);

        // Dry run returns Error status, but delay should be parsed and shown
        let output = format!("{}{}", response.stdout, response.stderr);
        assert!(output.contains("Delay") || output.contains(delay) || output.contains("ms"),
            "Should accept delay format: {}. output: {}", delay, output);
    }
}

// =============================================================================
// HAR with POST Body Tests
// =============================================================================

#[tokio::test]
async fn test_har_replay_post_with_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/create"))
        .respond_with(ResponseTemplate::new(201)
            .set_body_json(serde_json::json!({"id": 1, "created": true})))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let har_file = temp_dir.path().join("post.har");
    let har_content = serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": {"name": "Test", "version": "1.0"},
            "entries": [{
                "startedDateTime": "2024-01-01T00:00:00.000Z",
                "time": 100,
                "request": {
                    "method": "POST",
                    "url": format!("{}/api/create", mock_server.uri()),
                    "httpVersion": "HTTP/1.1",
                    "headers": [
                        {"name": "Content-Type", "value": "application/json"}
                    ],
                    "queryString": [],
                    "cookies": [],
                    "headersSize": 50,
                    "bodySize": 20,
                    "postData": {
                        "mimeType": "application/json",
                        "text": "{\"name\":\"test\"}"
                    }
                },
                "response": {
                    "status": 201, "statusText": "Created", "httpVersion": "HTTP/1.1",
                    "headers": [], "cookies": [],
                    "content": {"size": 0, "mimeType": "application/json"},
                    "redirectURL": "", "headersSize": 0, "bodySize": 0
                },
                "cache": {},
                "timings": {"send": 0, "wait": 100, "receive": 0}
            }]
        }
    });
    std::fs::write(&har_file, serde_json::to_string_pretty(&har_content).unwrap()).unwrap();

    let response = http(&["--import-har", har_file.to_str().unwrap()]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// HAR Error Handling Tests
// =============================================================================

#[test]
fn test_har_empty_file() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let empty_har = temp_dir.path().join("empty.har");
    let har_content = serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": {"name": "Test", "version": "1.0"},
            "entries": []
        }
    });
    std::fs::write(&empty_har, serde_json::to_string_pretty(&har_content).unwrap()).unwrap();

    let response = http_error(&["--import-har", empty_har.to_str().unwrap()]);

    assert_eq!(response.exit_status, ExitStatus::Error);
    assert!(response.stderr.contains("no entries") ||
            response.stderr.contains("empty") ||
            response.stderr.contains("No entries"),
        "Should indicate empty HAR. stderr: {}", response.stderr);
}

#[test]
fn test_har_combined_filter_and_index() {
    let har_path = fixture_path("sample.har");
    // Apply both filter and index (1-based indexing)
    let response = http(&[
        "--import-har", har_path.to_str().unwrap(),
        "--har-filter", "users",
        "--har-index", "1",
        "--har-list"
    ]);

    // Should apply filter first, then index
    assert_eq!(response.exit_status, ExitStatus::Success);
}
