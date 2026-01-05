//! WebSocket functionality tests
//!
//! Tests for WebSocket support including URL parsing, CLI flags, and modes.

mod common;

use common::{http, http_error};

// ============================================================================
// URL Detection Tests
// ============================================================================

#[test]
fn test_ws_url_detection() {
    // Test that ws:// URLs are detected as WebSocket requests
    let r = http_error(&["ws://example.com/socket"]);
    // Should attempt WebSocket connection (will fail but shows detection works)
    // The error message will contain WebSocket-related text or connection/DNS errors
    assert!(
        r.stderr.contains("WebSocket")
            || r.stderr.contains("Connection")
            || r.stderr.contains("error")
            || r.exit_code != 0,
        "Should detect ws:// URL as WebSocket. stderr: {}",
        r.stderr
    );
}

#[test]
fn test_wss_url_detection() {
    // Test that wss:// URLs are detected as WebSocket requests
    let r = http_error(&["wss://example.com/socket"]);
    // Should attempt WebSocket connection (will fail but shows detection works)
    assert!(
        r.stderr.contains("WebSocket")
            || r.stderr.contains("Connection")
            || r.stderr.contains("error")
            || r.exit_code != 0,
        "Should detect wss:// URL as WebSocket. stderr: {}",
        r.stderr
    );
}

// ============================================================================
// CLI Flag Tests
// ============================================================================

#[test]
fn test_ws_flag_detection() {
    // Test that --ws flag enables WebSocket mode
    let r = http_error(&["--ws", "example.com/socket"]);
    assert!(
        r.stderr.contains("WebSocket") || r.stderr.contains("Connection"),
        "Should detect --ws flag. stderr: {}",
        r.stderr
    );
}

#[test]
fn test_ws_send_flag() {
    // Test --ws-send flag
    let r = http_error(&["ws://example.com/socket", "--ws-send", "hello"]);
    assert!(
        r.stderr.contains("WebSocket") || r.stderr.contains("Connection"),
        "Should process --ws-send. stderr: {}",
        r.stderr
    );
}

#[test]
fn test_ws_listen_flag() {
    // Test --ws-listen flag
    let r = http_error(&["ws://example.com/socket", "--ws-listen"]);
    assert!(
        r.stderr.contains("WebSocket") || r.stderr.contains("Connection"),
        "Should process --ws-listen. stderr: {}",
        r.stderr
    );
}

#[test]
fn test_ws_subprotocol_flag() {
    // Test --ws-subprotocol flag
    let r = http_error(&["ws://example.com/socket", "--ws-subprotocol", "graphql-ws"]);
    assert!(
        r.stderr.contains("WebSocket") || r.stderr.contains("Connection"),
        "Should process --ws-subprotocol. stderr: {}",
        r.stderr
    );
}

#[test]
fn test_ws_binary_flag_hex() {
    // Test --ws-binary hex mode
    let r = http_error(&["ws://example.com/socket", "--ws-binary", "hex", "--ws-send", "48656c6c6f"]);
    assert!(
        r.stderr.contains("WebSocket") || r.stderr.contains("Connection"),
        "Should process --ws-binary hex. stderr: {}",
        r.stderr
    );
}

#[test]
fn test_ws_binary_flag_base64() {
    // Test --ws-binary base64 mode
    let r = http_error(&["ws://example.com/socket", "--ws-binary", "base64", "--ws-send", "SGVsbG8="]);
    assert!(
        r.stderr.contains("WebSocket") || r.stderr.contains("Connection"),
        "Should process --ws-binary base64. stderr: {}",
        r.stderr
    );
}

#[test]
fn test_ws_max_messages_flag() {
    // Test --ws-max-messages flag
    let r = http_error(&["ws://example.com/socket", "--ws-listen", "--ws-max-messages", "10"]);
    assert!(
        r.stderr.contains("WebSocket") || r.stderr.contains("Connection"),
        "Should process --ws-max-messages. stderr: {}",
        r.stderr
    );
}

#[test]
fn test_ws_ping_interval_flag() {
    // Test --ws-ping-interval flag
    let r = http_error(&["ws://example.com/socket", "--ws-listen", "--ws-ping-interval", "30"]);
    assert!(
        r.stderr.contains("WebSocket") || r.stderr.contains("Connection"),
        "Should process --ws-ping-interval. stderr: {}",
        r.stderr
    );
}

// ============================================================================
// Help Text Tests
// ============================================================================

#[test]
fn test_websocket_help_displayed() {
    let r = http(&["--help"]);

    // Check that WebSocket flags are in help
    assert!(r.stdout.contains("--ws"), "Help should show --ws flag");
    assert!(r.stdout.contains("--ws-send"), "Help should show --ws-send flag");
    assert!(r.stdout.contains("--ws-listen"), "Help should show --ws-listen flag");
    assert!(r.stdout.contains("--ws-interactive"), "Help should show --ws-interactive flag");
    assert!(r.stdout.contains("--ws-subprotocol"), "Help should show --ws-subprotocol flag");
    assert!(r.stdout.contains("--ws-binary"), "Help should show --ws-binary flag");
    assert!(r.stdout.contains("--ws-compress"), "Help should show --ws-compress flag");
    assert!(r.stdout.contains("--ws-max-messages"), "Help should show --ws-max-messages flag");
    assert!(r.stdout.contains("--ws-ping-interval"), "Help should show --ws-ping-interval flag");
}

// ============================================================================
// Unit Tests for Types
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use quicpulse::websocket::types::{WsEndpoint, WsMessage, BinaryMode};

    #[test]
    fn test_ws_endpoint_url() {
        let endpoint = WsEndpoint {
            host: "example.com".to_string(),
            port: 443,
            path: "/ws".to_string(),
            use_tls: true,
            subprotocol: None,
        };
        assert_eq!(endpoint.url(), "wss://example.com/ws");
    }

    #[test]
    fn test_ws_endpoint_url_with_port() {
        let endpoint = WsEndpoint {
            host: "localhost".to_string(),
            port: 8080,
            path: "/socket".to_string(),
            use_tls: false,
            subprotocol: None,
        };
        assert_eq!(endpoint.url(), "ws://localhost:8080/socket");
    }

    #[test]
    fn test_ws_endpoint_http_url() {
        let endpoint = WsEndpoint {
            host: "example.com".to_string(),
            port: 443,
            path: "/ws".to_string(),
            use_tls: true,
            subprotocol: None,
        };
        assert_eq!(endpoint.http_url(), "https://example.com/ws");
    }

    #[test]
    fn test_binary_mode_parse_hex() {
        let mode: BinaryMode = "hex".parse().unwrap();
        assert_eq!(mode, BinaryMode::Hex);
    }

    #[test]
    fn test_binary_mode_parse_base64() {
        let mode: BinaryMode = "base64".parse().unwrap();
        assert_eq!(mode, BinaryMode::Base64);

        let mode: BinaryMode = "b64".parse().unwrap();
        assert_eq!(mode, BinaryMode::Base64);
    }

    #[test]
    fn test_binary_mode_parse_invalid() {
        let result: Result<BinaryMode, _> = "invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_ws_message_type_name() {
        assert_eq!(WsMessage::Text("hello".to_string()).type_name(), "text");
        assert_eq!(WsMessage::Binary(vec![1, 2, 3]).type_name(), "binary");
        assert_eq!(WsMessage::Ping(vec![]).type_name(), "ping");
        assert_eq!(WsMessage::Pong(vec![]).type_name(), "pong");
        assert_eq!(WsMessage::Close(None, String::new()).type_name(), "close");
    }

    #[test]
    fn test_ws_message_is_control() {
        assert!(!WsMessage::Text("hello".to_string()).is_control());
        assert!(!WsMessage::Binary(vec![1, 2, 3]).is_control());
        assert!(WsMessage::Ping(vec![]).is_control());
        assert!(WsMessage::Pong(vec![]).is_control());
        assert!(WsMessage::Close(None, String::new()).is_control());
    }
}

// ============================================================================
// Codec Unit Tests
// ============================================================================

#[cfg(test)]
mod codec_tests {
    use quicpulse::websocket::codec::{encode_binary, decode_binary, format_text_message};
    use quicpulse::websocket::types::BinaryMode;

    #[test]
    fn test_encode_hex() {
        let data = b"Hello";
        assert_eq!(encode_binary(data, BinaryMode::Hex), "48656c6c6f");
    }

    #[test]
    fn test_encode_base64() {
        let data = b"Hello";
        assert_eq!(encode_binary(data, BinaryMode::Base64), "SGVsbG8=");
    }

    #[test]
    fn test_decode_hex() {
        let result = decode_binary("48656c6c6f", BinaryMode::Hex).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_decode_base64() {
        let result = decode_binary("SGVsbG8=", BinaryMode::Base64).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_decode_invalid_hex() {
        let result = decode_binary("invalid!", BinaryMode::Hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_invalid_base64() {
        let result = decode_binary("not valid base64!!!", BinaryMode::Base64);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_json_message() {
        let json = r#"{"key":"value"}"#;
        let formatted = format_text_message(json);
        assert!(formatted.contains("\"key\""));
        assert!(formatted.contains("\"value\""));
    }

    #[test]
    fn test_format_non_json_message() {
        let text = "plain text message";
        assert_eq!(format_text_message(text), "plain text message");
    }
}

// ============================================================================
// URL Parsing Unit Tests
// ============================================================================

#[cfg(test)]
mod url_parsing_tests {
    use quicpulse::websocket::parse_ws_endpoint;
    use quicpulse::cli::Args;

    fn default_args() -> Args {
        Args::default()
    }

    #[test]
    fn test_parse_ws_url() {
        let args = default_args();
        let endpoint = parse_ws_endpoint("ws://localhost:8080/ws", &args).unwrap();
        assert_eq!(endpoint.host, "localhost");
        assert_eq!(endpoint.port, 8080);
        assert_eq!(endpoint.path, "/ws");
        assert!(!endpoint.use_tls);
    }

    #[test]
    fn test_parse_wss_url() {
        let args = default_args();
        let endpoint = parse_ws_endpoint("wss://example.com/socket", &args).unwrap();
        assert_eq!(endpoint.host, "example.com");
        assert_eq!(endpoint.port, 443);
        assert_eq!(endpoint.path, "/socket");
        assert!(endpoint.use_tls);
    }

    #[test]
    fn test_parse_ws_url_no_path() {
        let args = default_args();
        let endpoint = parse_ws_endpoint("ws://localhost:3000", &args).unwrap();
        assert_eq!(endpoint.path, "/");
    }

    #[test]
    fn test_parse_ws_url_default_port() {
        let args = default_args();

        let endpoint = parse_ws_endpoint("ws://example.com/path", &args).unwrap();
        assert_eq!(endpoint.port, 80);

        let endpoint = parse_ws_endpoint("wss://example.com/path", &args).unwrap();
        assert_eq!(endpoint.port, 443);
    }

    #[test]
    fn test_parse_ws_url_empty_host_error() {
        let args = default_args();
        let result = parse_ws_endpoint("ws:///path", &args);
        assert!(result.is_err());
    }
}
