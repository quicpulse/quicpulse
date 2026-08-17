use quicpulse::cli::Args;
use quicpulse::websocket::types::{BinaryMode, WsMessage};
use quicpulse::websocket::{is_ws_request, parse_ws_endpoint};

#[test]
fn test_is_ws_request_detection() {
    let args = Args::default();
    assert!(!is_ws_request(&args));

    let args_ws = Args {
        ws: true,
        ..Default::default()
    };
    assert!(is_ws_request(&args_ws));

    let args_url = Args {
        url: Some("ws://localhost:8080/chat".to_string()),
        ..Default::default()
    };
    assert!(is_ws_request(&args_url));

    let args_wss = Args {
        url: Some("wss://echo.websocket.events".to_string()),
        ..Default::default()
    };
    assert!(is_ws_request(&args_wss));

    let args_send = Args {
        ws_send: Some("ping".to_string()),
        ..Default::default()
    };
    assert!(is_ws_request(&args_send));
}

#[test]
fn test_parse_ws_endpoint_urls() {
    let args = Args::default();

    let ep = parse_ws_endpoint("ws://127.0.0.1:8080/ws/feed", &args).unwrap();
    assert_eq!(ep.host, "127.0.0.1");
    assert_eq!(ep.port, 8080);
    assert_eq!(ep.path, "/ws/feed");
    assert!(!ep.use_tls);
    assert_eq!(ep.url(), "ws://127.0.0.1:8080/ws/feed");
    assert_eq!(ep.http_url(), "http://127.0.0.1:8080/ws/feed");

    let ep_tls = parse_ws_endpoint("wss://secure.example.com/v1/stream", &args).unwrap();
    assert_eq!(ep_tls.host, "secure.example.com");
    assert_eq!(ep_tls.port, 443);
    assert!(ep_tls.use_tls);
    assert_eq!(ep_tls.url(), "wss://secure.example.com/v1/stream");
    assert_eq!(ep_tls.http_url(), "https://secure.example.com/v1/stream");
}

#[test]
fn test_ws_message_types_and_control_frames() {
    let text = WsMessage::Text("hello".to_string());
    assert!(!text.is_control());
    assert_eq!(text.type_name(), "text");

    let bin = WsMessage::Binary(vec![0x00, 0x01]);
    assert!(!bin.is_control());
    assert_eq!(bin.type_name(), "binary");

    let ping = WsMessage::Ping(vec![]);
    assert!(ping.is_control());
    assert_eq!(ping.type_name(), "ping");

    let pong = WsMessage::Pong(vec![]);
    assert!(pong.is_control());
    assert_eq!(pong.type_name(), "pong");

    let close = WsMessage::Close(Some(1000), "Normal Closure".to_string());
    assert!(close.is_control());
    assert_eq!(close.type_name(), "close");
}

#[test]
fn test_binary_mode_parsing() {
    use std::str::FromStr;

    assert_eq!(BinaryMode::from_str("hex").unwrap(), BinaryMode::Hex);
    assert_eq!(BinaryMode::from_str("base64").unwrap(), BinaryMode::Base64);
    assert_eq!(BinaryMode::from_str("b64").unwrap(), BinaryMode::Base64);
    assert!(BinaryMode::from_str("invalid").is_err());
}
