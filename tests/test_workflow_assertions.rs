//! Unit tests for Workflow and CLI Response Assertions (status, time, headers, body)

use quicpulse::cli::Args;
use quicpulse::pipeline::assertions::{build_assertions, check_assertions, Assertion};
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;

#[test]
fn test_build_assertions_from_args() {
    let args = Args {
        assert_status: Some("200".to_string()),
        assert_time: Some("<500ms".to_string()),
        assert_body: Some("ok".to_string()),
        assert_header: vec![
            "Content-Type:application/json".to_string(),
            "X-Custom".to_string(),
        ],
        ..Default::default()
    };

    let assertions = build_assertions(&args);
    assert_eq!(assertions.len(), 5);
}

#[test]
fn test_check_assertions_status_patterns() {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let exact = vec![Assertion::Status("200".to_string())];
    let res_pass = check_assertions(&exact, 200, Duration::from_millis(50), &headers, "{}");
    assert!(res_pass[0].passed);

    let res_fail = check_assertions(&exact, 404, Duration::from_millis(50), &headers, "{}");
    assert!(!res_fail[0].passed);

    let wildcard = vec![Assertion::Status("2xx".to_string())];
    let res_wildcard = check_assertions(&wildcard, 201, Duration::from_millis(50), &headers, "{}");
    assert!(res_wildcard[0].passed);

    let range = vec![Assertion::Status("200-299".to_string())];
    let res_range = check_assertions(&range, 204, Duration::from_millis(50), &headers, "{}");
    assert!(res_range[0].passed);
}

#[test]
fn test_check_assertions_time_and_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    headers.insert("X-Request-Id", HeaderValue::from_static("abc-123"));

    let time_assert = vec![Assertion::Time(Duration::from_millis(200))];
    let res_time_pass = check_assertions(
        &time_assert,
        200,
        Duration::from_millis(150),
        &headers,
        "{}",
    );
    assert!(res_time_pass[0].passed);

    let res_time_fail = check_assertions(
        &time_assert,
        200,
        Duration::from_millis(350),
        &headers,
        "{}",
    );
    assert!(!res_time_fail[0].passed);

    let header_assert = vec![
        Assertion::Header(
            "Content-Type".to_string(),
            Some("application/json".to_string()),
        ),
        Assertion::Header("X-Request-Id".to_string(), None),
        Assertion::Header("X-Missing".to_string(), None),
    ];
    let res_header = check_assertions(
        &header_assert,
        200,
        Duration::from_millis(50),
        &headers,
        "{}",
    );
    assert!(res_header[0].passed);
    assert!(res_header[1].passed);
    assert!(!res_header[2].passed);
}
