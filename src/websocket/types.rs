//! WebSocket types and data structures

use std::time::Duration;

/// Parsed WebSocket endpoint
#[derive(Debug, Clone)]
pub struct WsEndpoint {
    pub host: String,
    pub port: u16,
    pub path: String,
    pub use_tls: bool,
    pub subprotocol: Option<String>,
}

impl WsEndpoint {
    /// Get the full WebSocket URL
    pub fn url(&self) -> String {
        let scheme = if self.use_tls { "wss" } else { "ws" };
        if self.port == 80 && !self.use_tls || self.port == 443 && self.use_tls {
            format!("{}://{}{}", scheme, self.host, self.path)
        } else {
            format!("{}://{}:{}{}", scheme, self.host, self.port, self.path)
        }
    }

    /// Get the HTTP URL for the initial connection
    pub fn http_url(&self) -> String {
        let scheme = if self.use_tls { "https" } else { "http" };
        if self.port == 80 && !self.use_tls || self.port == 443 && self.use_tls {
            format!("{}://{}{}", scheme, self.host, self.path)
        } else {
            format!("{}://{}:{}{}", scheme, self.host, self.port, self.path)
        }
    }
}

/// WebSocket message types
#[derive(Debug, Clone)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close(Option<u16>, String),
}

impl WsMessage {
    /// Check if message is a control frame
    pub fn is_control(&self) -> bool {
        matches!(self, WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Close(_, _))
    }

    /// Get message type name
    pub fn type_name(&self) -> &'static str {
        match self {
            WsMessage::Text(_) => "text",
            WsMessage::Binary(_) => "binary",
            WsMessage::Ping(_) => "ping",
            WsMessage::Pong(_) => "pong",
            WsMessage::Close(_, _) => "close",
        }
    }
}

/// WebSocket connection options
#[derive(Debug, Clone)]
pub struct WsOptions {
    pub timeout: Option<Duration>,
    pub compress: bool,
    pub binary_mode: Option<BinaryMode>,
    pub ping_interval: Option<Duration>,
    pub max_messages: usize,
    pub headers: Vec<(String, String)>,
}

impl Default for WsOptions {
    fn default() -> Self {
        Self {
            timeout: None,
            compress: false,
            binary_mode: None,
            ping_interval: None,
            max_messages: 0,
            headers: Vec::new(),
        }
    }
}

/// WebSocket operation mode
#[derive(Debug, Clone, PartialEq)]
pub enum WsMode {
    /// Interactive REPL mode
    Interactive,
    /// Send a single message and disconnect
    Send(String),
    /// Listen mode - receive messages only
    Listen,
    /// Read messages from stdin (NDJSON)
    Stdin,
}

/// Binary encoding mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryMode {
    Hex,
    Base64,
}

impl std::str::FromStr for BinaryMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hex" => Ok(BinaryMode::Hex),
            "base64" | "b64" => Ok(BinaryMode::Base64),
            _ => Err(format!("Unknown binary mode: '{}'. Use 'hex' or 'base64'", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_url() {
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
    fn test_endpoint_url_with_port() {
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
    fn test_binary_mode_parse() {
        assert_eq!("hex".parse::<BinaryMode>().unwrap(), BinaryMode::Hex);
        assert_eq!("base64".parse::<BinaryMode>().unwrap(), BinaryMode::Base64);
        assert_eq!("b64".parse::<BinaryMode>().unwrap(), BinaryMode::Base64);
        assert!("invalid".parse::<BinaryMode>().is_err());
    }

    #[test]
    fn test_message_type_name() {
        assert_eq!(WsMessage::Text("hello".to_string()).type_name(), "text");
        assert_eq!(WsMessage::Binary(vec![1, 2, 3]).type_name(), "binary");
        assert_eq!(WsMessage::Ping(vec![]).type_name(), "ping");
    }
}
