//! Unit tests for HAR archive parsing, entry filtering, and replay options

use quicpulse::har::{filter_by_indices, filter_entries, format_har_list, load_har, parse_delay};
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_parse_delay_strings() {
    assert_eq!(parse_delay("100ms").unwrap(), Duration::from_millis(100));
    assert_eq!(parse_delay("2s").unwrap(), Duration::from_secs(2));
    assert_eq!(parse_delay("500").unwrap(), Duration::from_millis(500));
    assert!(parse_delay("invalid_delay").is_err());
}

#[test]
fn test_load_har_and_filter_entries() {
    let dir = TempDir::new().unwrap();
    let har_file = dir.path().join("sample.har");
    let har_json = r#"{
      "log": {
        "version": "1.2",
        "creator": { "name": "WebInspector", "version": "537.36" },
        "entries": [
          {
            "startedDateTime": "2026-08-17T12:00:00.000Z",
            "time": 50,
            "request": {
              "method": "GET",
              "url": "https://api.example.com/v1/users",
              "httpVersion": "HTTP/1.1",
              "cookies": [],
              "headers": [],
              "queryString": [],
              "headersSize": 100,
              "bodySize": 0
            },
            "response": {
              "status": 200,
              "statusText": "OK",
              "httpVersion": "HTTP/1.1",
              "cookies": [],
              "headers": [],
              "content": { "size": 2, "mimeType": "application/json", "text": "[]" },
              "redirectURL": "",
              "headersSize": 100,
              "bodySize": 2
            },
            "cache": {},
            "timings": { "send": 0, "wait": 50, "receive": 0 }
          },
          {
            "startedDateTime": "2026-08-17T12:00:01.000Z",
            "time": 40,
            "request": {
              "method": "POST",
              "url": "https://api.example.com/v1/auth/login",
              "httpVersion": "HTTP/1.1",
              "cookies": [],
              "headers": [],
              "queryString": [],
              "headersSize": 100,
              "bodySize": 20
            },
            "response": {
              "status": 200,
              "statusText": "OK",
              "httpVersion": "HTTP/1.1",
              "cookies": [],
              "headers": [],
              "content": { "size": 15, "mimeType": "application/json", "text": "{\"token\":\"xyz\"}" },
              "redirectURL": "",
              "headersSize": 100,
              "bodySize": 15
            },
            "cache": {},
            "timings": { "send": 0, "wait": 40, "receive": 0 }
          }
        ]
      }
    }"#;
    fs::write(&har_file, har_json).unwrap();

    let mut har = load_har(&har_file).unwrap();
    assert_eq!(har.log.entries.len(), 2);

    let list_str = format_har_list(&har);
    assert!(list_str.contains("/v1/users") && list_str.contains("/v1/auth/login"));

    filter_entries(&mut har, "auth").unwrap();
    assert_eq!(har.log.entries.len(), 1);
    assert_eq!(har.log.entries[0].request.method, "POST");

    let mut har2 = load_har(&har_file).unwrap();
    filter_by_indices(&mut har2, &[1]);
    assert_eq!(har2.log.entries.len(), 1);
    assert_eq!(
        har2.log.entries[0].request.url,
        "https://api.example.com/v1/users"
    );
}
