//! Unit tests for Kubernetes transparent port-forwarding URL parser (k8s://)

use quicpulse::k8s::{is_k8s_url, parse_k8s_url};

#[test]
fn test_is_k8s_url_detection() {
    assert!(is_k8s_url("k8s://api-service.default:8080/health"));
    assert!(is_k8s_url("k8s://db.prod/metrics"));
    assert!(!is_k8s_url("https://kubernetes.default.svc.cluster.local"));
    assert!(!is_k8s_url("http://localhost:8080"));
}

#[test]
fn test_parse_valid_k8s_urls() {
    let url1 = parse_k8s_url("k8s://api-service.default:8080/health?verbose=true").unwrap();
    assert_eq!(url1.service, "api-service");
    assert_eq!(url1.namespace, "default");
    assert_eq!(url1.port, 8080);
    assert_eq!(url1.path, "/health");
    assert_eq!(url1.query, Some("verbose=true".to_string()));

    let url2 = parse_k8s_url("k8s://redis-master.prod").unwrap();
    assert_eq!(url2.service, "redis-master");
    assert_eq!(url2.namespace, "prod");
    assert_eq!(url2.port, 80); // default port
    assert_eq!(url2.path, "/");
    assert_eq!(url2.query, None);
}

#[test]
fn test_parse_invalid_k8s_urls() {
    // Missing k8s:// prefix
    assert!(parse_k8s_url("http://service.default").is_err());

    // Missing namespace
    assert!(parse_k8s_url("k8s://service-only").is_err());

    // Invalid uppercase characters in service name
    assert!(parse_k8s_url("k8s://MyService.default").is_err());

    // Invalid port number
    assert!(parse_k8s_url("k8s://service.default:999999").is_err());
}
