//! WebSocket support module

pub mod types;
pub mod codec;
pub mod client;
pub mod stream;
pub mod interactive;

pub use types::{WsEndpoint, WsMessage, WsOptions, WsMode, BinaryMode};

use crate::cli::Args;
use crate::cli::parser::ProcessedArgs;
use crate::context::Environment;
use crate::errors::QuicpulseError;
use crate::status::ExitStatus;

/// Check if request should be treated as WebSocket
pub fn is_ws_request(args: &Args) -> bool {
    // Check for explicit --ws flag
    if args.ws {
        return true;
    }

    // Check for any --ws-* flags
    if args.ws_send.is_some()
        || args.ws_interactive
        || args.ws_listen
        || args.ws_subprotocol.is_some()
        || args.ws_binary.is_some()
        || args.ws_compress
        || args.ws_ping_interval.is_some()
    {
        return true;
    }

    // Check for ws:// or wss:// URL scheme in the url field
    if let Some(ref url) = args.url {
        let url_lower = url.to_lowercase();
        if url_lower.starts_with("ws://") || url_lower.starts_with("wss://") {
            return true;
        }
    }

    // Also check if the "method" field contains a ws:// URL
    // (happens when URL is passed as first positional arg without explicit method)
    if let Some(ref method) = args.method {
        let method_lower = method.to_lowercase();
        if method_lower.starts_with("ws://") || method_lower.starts_with("wss://") {
            return true;
        }
    }

    false
}

/// Parse WebSocket endpoint from URL
pub fn parse_ws_endpoint(url: &str, args: &Args) -> Result<WsEndpoint, QuicpulseError> {
    let url = url.trim();

    // Determine TLS and strip scheme
    let (use_tls, url_without_scheme) = if url.to_lowercase().starts_with("wss://") {
        (true, &url[6..])
    } else if url.to_lowercase().starts_with("ws://") {
        (false, &url[5..])
    } else if url.to_lowercase().starts_with("https://") {
        (true, &url[8..])
    } else if url.to_lowercase().starts_with("http://") {
        (false, &url[7..])
    } else {
        // No scheme, default based on --default-scheme or assume ws://
        let use_tls = args.default_scheme == "https";
        (use_tls, url)
    };

    // Split host:port from path
    let (host_port, path) = if let Some(idx) = url_without_scheme.find('/') {
        (&url_without_scheme[..idx], &url_without_scheme[idx..])
    } else {
        (url_without_scheme, "/")
    };

    // Parse host and port
    let (host, port) = if let Some(idx) = host_port.rfind(':') {
        let port_str = &host_port[idx + 1..];
        if let Ok(port) = port_str.parse::<u16>() {
            (&host_port[..idx], port)
        } else {
            (host_port, if use_tls { 443 } else { 80 })
        }
    } else {
        (host_port, if use_tls { 443 } else { 80 })
    };

    if host.is_empty() {
        return Err(QuicpulseError::Argument("WebSocket URL must include a host".to_string()));
    }

    Ok(WsEndpoint {
        host: host.to_string(),
        port,
        path: path.to_string(),
        use_tls,
        subprotocol: args.ws_subprotocol.clone(),
    })
}

/// Determine the WebSocket operation mode
fn determine_mode(args: &Args, env: &Environment) -> WsMode {
    if args.ws_interactive {
        WsMode::Interactive
    } else if let Some(ref msg) = args.ws_send {
        WsMode::Send(msg.clone())
    } else if args.ws_listen {
        WsMode::Listen
    } else if !env.stdin_isatty && !args.ignore_stdin {
        WsMode::Stdin
    } else {
        // Default to interactive if TTY, otherwise listen
        if env.stdin_isatty && env.stdout_isatty {
            WsMode::Interactive
        } else {
            WsMode::Listen
        }
    }
}

/// Main entry point for WebSocket requests
pub async fn run_websocket(
    args: &Args,
    processed: &ProcessedArgs,
    env: &Environment,
) -> Result<ExitStatus, QuicpulseError> {
    use crate::input::InputItem;

    let endpoint = parse_ws_endpoint(&processed.url, args)?;
    let mode = determine_mode(args, env);

    // Collect headers from request items
    let headers: Vec<(String, String)> = processed.items.iter()
        .filter_map(|item| {
            match item {
                InputItem::Header { name, value } => Some((name.clone(), value.clone())),
                InputItem::EmptyHeader { name } => Some((name.clone(), String::new())),
                InputItem::HeaderFile { name, path } => {
                    std::fs::read_to_string(path).ok().map(|v| (name.clone(), v.trim().to_string()))
                }
                _ => None,
            }
        })
        .collect();

    // Build JSON message from request items if present
    let json_body = if processed.items.iter().any(|item| item.is_data()) {
        let mut obj = serde_json::Map::new();
        for item in &processed.items {
            let (key, value) = match item {
                InputItem::DataField { key, value } => {
                    (key.clone(), serde_json::json!(value))
                }
                InputItem::DataFieldFile { key, path } => {
                    let content = std::fs::read_to_string(path).unwrap_or_default();
                    (key.clone(), serde_json::json!(content.trim()))
                }
                InputItem::JsonField { key, value } => {
                    (key.clone(), value.clone())
                }
                InputItem::JsonFieldFile { key, path } => {
                    let content = std::fs::read_to_string(path).unwrap_or_default();
                    let json_val = serde_json::from_str(&content).unwrap_or(serde_json::json!(content));
                    (key.clone(), json_val)
                }
                _ => continue,
            };
            obj.insert(key, value);
        }
        Some(serde_json::Value::Object(obj))
    } else {
        None
    };

    // Parse binary mode
    let binary_mode = if let Some(ref mode_str) = args.ws_binary {
        Some(mode_str.parse::<BinaryMode>()
            .map_err(|e| QuicpulseError::Argument(e))?)
    } else {
        None
    };

    let options = WsOptions {
        timeout: args.timeout.map(|t| std::time::Duration::from_secs_f64(t)),
        compress: args.ws_compress,
        binary_mode,
        ping_interval: args.ws_ping_interval.map(std::time::Duration::from_secs),
        max_messages: args.ws_max_messages,
        headers,
    };

    if args.verbose > 0 {
        eprintln!("WebSocket: {}", endpoint.url());
        eprintln!("  Mode: {:?}", mode);
        if let Some(ref proto) = endpoint.subprotocol {
            eprintln!("  Subprotocol: {}", proto);
        }
        if options.compress {
            eprintln!("  Compression: enabled");
        }
    }

    // Connect to WebSocket server
    let mut ws_client = client::WsClient::connect(&endpoint, &options, args).await?;

    // Send JSON body if present and not in listen mode
    if let Some(ref json) = json_body {
        if mode != WsMode::Listen {
            let msg = serde_json::to_string(json)
                .map_err(|e| QuicpulseError::WebSocket(format!("Failed to serialize JSON: {}", e)))?;
            ws_client.send_text(&msg).await?;
            if args.verbose > 0 {
                eprintln!("Sent: {}", msg);
            }
        }
    }

    match mode {
        WsMode::Send(ref msg) => {
            // Send the message if we haven't already sent JSON body
            if json_body.is_none() {
                ws_client.send_text(msg).await?;
                if args.verbose > 0 {
                    eprintln!("Sent: {}", msg);
                }
            }

            // Wait for one response
            if let Some(response) = ws_client.receive().await? {
                stream::print_message(&response, &options);
            }

            ws_client.close().await?;
            Ok(ExitStatus::Success)
        }

        WsMode::Listen => {
            stream::run_listen_mode(&mut ws_client, &options).await
        }

        WsMode::Interactive => {
            interactive::run_interactive_mode(&mut ws_client, &options, env).await
        }

        WsMode::Stdin => {
            stream::run_stdin_mode(&mut ws_client, &options).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_args() -> Args {
        Args::default()
    }

    #[test]
    fn test_is_ws_request_by_url() {
        let mut args = default_args();
        args.url = Some("ws://example.com/socket".to_string());
        assert!(is_ws_request(&args));

        args.url = Some("wss://example.com/socket".to_string());
        assert!(is_ws_request(&args));

        args.url = Some("http://example.com".to_string());
        assert!(!is_ws_request(&args));
    }

    #[test]
    fn test_is_ws_request_by_flag() {
        let mut args = default_args();
        args.ws = true;
        assert!(is_ws_request(&args));

        let mut args = default_args();
        args.ws_interactive = true;
        assert!(is_ws_request(&args));

        let mut args = default_args();
        args.ws_send = Some("hello".to_string());
        assert!(is_ws_request(&args));
    }

    #[test]
    fn test_parse_ws_endpoint() {
        let args = default_args();

        let endpoint = parse_ws_endpoint("ws://localhost:8080/ws", &args).unwrap();
        assert_eq!(endpoint.host, "localhost");
        assert_eq!(endpoint.port, 8080);
        assert_eq!(endpoint.path, "/ws");
        assert!(!endpoint.use_tls);

        let endpoint = parse_ws_endpoint("wss://example.com/socket", &args).unwrap();
        assert_eq!(endpoint.host, "example.com");
        assert_eq!(endpoint.port, 443);
        assert_eq!(endpoint.path, "/socket");
        assert!(endpoint.use_tls);
    }

    #[test]
    fn test_parse_ws_endpoint_no_path() {
        let args = default_args();
        let endpoint = parse_ws_endpoint("ws://localhost:3000", &args).unwrap();
        assert_eq!(endpoint.path, "/");
    }
}
