//! Unit tests for gRPC request detection, URL parsing, and endpoint helpers

use quicpulse::cli::Args;
use quicpulse::grpc::{is_grpc_request, parse_grpc_endpoint};

#[test]
fn test_is_grpc_request_flags() {
    let args = Args::default();
    assert!(!is_grpc_request(&args));

    let args_grpc = Args {
        grpc: true,
        ..Default::default()
    };
    assert!(is_grpc_request(&args_grpc));

    let args_list = Args {
        grpc_list: true,
        ..Default::default()
    };
    assert!(is_grpc_request(&args_list));

    let args_desc = Args {
        grpc_describe: Some("UserService".to_string()),
        ..Default::default()
    };
    assert!(is_grpc_request(&args_desc));

    let args_interactive = Args {
        grpc_interactive: true,
        ..Default::default()
    };
    assert!(is_grpc_request(&args_interactive));
}

#[test]
fn test_parse_grpc_endpoint_urls() {
    let ep = parse_grpc_endpoint("grpc://127.0.0.1:50051/user.UserService/GetUser").unwrap();
    assert_eq!(ep.host, "127.0.0.1");
    assert_eq!(ep.port, 50051);
    assert_eq!(ep.service, Some("user.UserService".to_string()));
    assert_eq!(ep.method, Some("GetUser".to_string()));
    assert_eq!(ep.address(), "127.0.0.1:50051");
    assert_eq!(ep.uri(), "http://127.0.0.1:50051");
    assert_eq!(
        ep.service_path(),
        Some("/user.UserService/GetUser".to_string())
    );

    let ep_tls = parse_grpc_endpoint("grpcs://api.example.com:443/Greeter/SayHello").unwrap();
    assert_eq!(ep_tls.host, "api.example.com");
    assert_eq!(ep_tls.port, 443);
    assert!(ep_tls.use_tls);
    assert_eq!(ep_tls.uri(), "https://api.example.com:443");
}
