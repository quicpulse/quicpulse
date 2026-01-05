//! WebSocket client implementation

use std::sync::Arc;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::{
        protocol::Message,
        client::IntoClientRequest,
        http::HeaderValue,
    },
    Connector,
    MaybeTlsStream,
    WebSocketStream,
};
use tokio::net::TcpStream;
use rustls::ClientConfig;

use crate::cli::Args;
use crate::errors::QuicpulseError;
use super::types::{WsEndpoint, WsMessage, WsOptions};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// WebSocket client for sending and receiving messages
pub struct WsClient {
    stream: WsStream,
    #[allow(dead_code)]
    endpoint: WsEndpoint,
}

impl WsClient {
    /// Connect to a WebSocket server
    pub async fn connect(
        endpoint: &WsEndpoint,
        options: &WsOptions,
        args: &Args,
    ) -> Result<Self, QuicpulseError> {
        let url = endpoint.url();

        // Build the request
        let mut request = url.into_client_request()
            .map_err(|e| QuicpulseError::WebSocket(format!("Invalid WebSocket URL: {}", e)))?;

        // Add custom headers
        let headers = request.headers_mut();
        for (key, value) in &options.headers {
            if let (Ok(name), Ok(val)) = (
                key.parse::<tokio_tungstenite::tungstenite::http::HeaderName>(),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        // Add subprotocol header if specified
        if let Some(ref proto) = endpoint.subprotocol {
            if let Ok(val) = HeaderValue::from_str(proto) {
                headers.insert("Sec-WebSocket-Protocol", val);
            }
        }

        // Configure TLS if needed
        let connector = if endpoint.use_tls {
            let verify = args.verify.to_lowercase();
            let skip_verify = verify == "no" || verify == "false" || verify == "0";

            let tls_config = if skip_verify {
                // Dangerous: skip certificate verification
                let config = ClientConfig::builder()
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(NoVerifier))
                    .with_no_client_auth();
                config
            } else {
                // Use system root certificates (matching HTTP client's rustls-native-certs behavior)
                // This ensures corporate proxies with custom CAs work for WebSocket too
                let mut root_store = rustls::RootCertStore::empty();
                let cert_result = rustls_native_certs::load_native_certs();
                
                // CertificateResult has certs and errors fields (rustls-native-certs 0.8 API)
                for cert in cert_result.certs {
                    root_store.add(cert).ok(); // Ignore errors for individual certs
                }
                
                // If no certs loaded, fall back to webpki-roots
                if root_store.is_empty() {
                    root_store = rustls::RootCertStore::from_iter(
                        webpki_roots::TLS_SERVER_ROOTS.iter().cloned()
                    );
                }
                
                ClientConfig::builder()
                    .with_root_certificates(root_store)
                    .with_no_client_auth()
            };

            Some(Connector::Rustls(Arc::new(tls_config)))
        } else {
            None
        };

        // Connect with optional timeout
        let connect_future = connect_async_tls_with_config(request, None, false, connector);

        let (stream, response) = if let Some(timeout) = options.timeout {
            tokio::time::timeout(timeout, connect_future)
                .await
                .map_err(|_| QuicpulseError::WebSocket("Connection timeout".to_string()))?
                .map_err(|e| QuicpulseError::WebSocket(format!("Connection failed: {}", e)))?
        } else {
            connect_future
                .await
                .map_err(|e| QuicpulseError::WebSocket(format!("Connection failed: {}", e)))?
        };

        // Check for negotiated subprotocol
        if let Some(proto) = response.headers().get("Sec-WebSocket-Protocol") {
            if args.verbose > 0 {
                eprintln!("  Negotiated subprotocol: {:?}", proto);
            }
        }

        Ok(Self {
            stream,
            endpoint: endpoint.clone(),
        })
    }

    /// Connect to a WebSocket server without requiring Args (for workflow pipelines)
    /// Uses standard TLS verification and the provided options.
    pub async fn connect_simple(
        endpoint: &WsEndpoint,
        options: &WsOptions,
        skip_tls_verify: bool,
    ) -> Result<Self, QuicpulseError> {
        let url = endpoint.url();

        // Build the request
        let mut request = url.into_client_request()
            .map_err(|e| QuicpulseError::WebSocket(format!("Invalid WebSocket URL: {}", e)))?;

        // Add custom headers
        let headers = request.headers_mut();
        for (key, value) in &options.headers {
            if let (Ok(name), Ok(val)) = (
                key.parse::<tokio_tungstenite::tungstenite::http::HeaderName>(),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        // Add subprotocol header if specified
        if let Some(ref proto) = endpoint.subprotocol {
            if let Ok(val) = HeaderValue::from_str(proto) {
                headers.insert("Sec-WebSocket-Protocol", val);
            }
        }

        // Configure TLS if needed
        let connector = if endpoint.use_tls {
            let tls_config = if skip_tls_verify {
                // Dangerous: skip certificate verification
                ClientConfig::builder()
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(NoVerifier))
                    .with_no_client_auth()
            } else {
                // Use system root certificates (matching HTTP client's rustls-native-certs behavior)
                // This ensures corporate proxies with custom CAs work for WebSocket too
                let mut root_store = rustls::RootCertStore::empty();
                let cert_result = rustls_native_certs::load_native_certs();
                
                // CertificateResult has certs and errors fields (rustls-native-certs 0.8 API)
                for cert in cert_result.certs {
                    root_store.add(cert).ok(); // Ignore errors for individual certs
                }
                
                // If no certs loaded, fall back to webpki-roots
                if root_store.is_empty() {
                    root_store = rustls::RootCertStore::from_iter(
                        webpki_roots::TLS_SERVER_ROOTS.iter().cloned()
                    );
                }
                
                ClientConfig::builder()
                    .with_root_certificates(root_store)
                    .with_no_client_auth()
            };

            Some(Connector::Rustls(Arc::new(tls_config)))
        } else {
            None
        };

        // Connect with optional timeout
        let connect_future = connect_async_tls_with_config(request, None, false, connector);

        let (stream, _response) = if let Some(timeout) = options.timeout {
            tokio::time::timeout(timeout, connect_future)
                .await
                .map_err(|_| QuicpulseError::WebSocket("Connection timeout".to_string()))?
                .map_err(|e| QuicpulseError::WebSocket(format!("Connection failed: {}", e)))?
        } else {
            connect_future
                .await
                .map_err(|e| QuicpulseError::WebSocket(format!("Connection failed: {}", e)))?
        };

        Ok(Self {
            stream,
            endpoint: endpoint.clone(),
        })
    }

    /// Send a text message
    pub async fn send_text(&mut self, text: &str) -> Result<(), QuicpulseError> {
        self.stream.send(Message::Text(text.to_string().into()))
            .await
            .map_err(|e| QuicpulseError::WebSocket(format!("Send failed: {}", e)))
    }

    /// Send a binary message
    pub async fn send_binary(&mut self, data: &[u8]) -> Result<(), QuicpulseError> {
        self.stream.send(Message::Binary(data.to_vec().into()))
            .await
            .map_err(|e| QuicpulseError::WebSocket(format!("Send failed: {}", e)))
    }

    /// Send a ping
    pub async fn send_ping(&mut self, data: &[u8]) -> Result<(), QuicpulseError> {
        self.stream.send(Message::Ping(data.to_vec().into()))
            .await
            .map_err(|e| QuicpulseError::WebSocket(format!("Ping failed: {}", e)))
    }

    /// Receive the next message
    pub async fn receive(&mut self) -> Result<Option<WsMessage>, QuicpulseError> {
        match self.stream.next().await {
            Some(Ok(msg)) => Ok(Some(Self::convert_message(msg))),
            Some(Err(e)) => Err(QuicpulseError::WebSocket(format!("Receive error: {}", e))),
            None => Ok(None),
        }
    }

    /// Receive with timeout
    pub async fn receive_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<Option<WsMessage>, QuicpulseError> {
        match tokio::time::timeout(timeout, self.stream.next()).await {
            Ok(Some(Ok(msg))) => Ok(Some(Self::convert_message(msg))),
            Ok(Some(Err(e))) => Err(QuicpulseError::WebSocket(format!("Receive error: {}", e))),
            Ok(None) => Ok(None),
            Err(_) => Ok(None), // Timeout - return None
        }
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<(), QuicpulseError> {
        self.stream.close(None)
            .await
            .map_err(|e| QuicpulseError::WebSocket(format!("Close failed: {}", e)))
    }

    /// Convert tungstenite message to our message type
    fn convert_message(msg: Message) -> WsMessage {
        match msg {
            Message::Text(s) => WsMessage::Text(s.to_string()),
            Message::Binary(b) => WsMessage::Binary(b.to_vec()),
            Message::Ping(b) => WsMessage::Ping(b.to_vec()),
            Message::Pong(b) => WsMessage::Pong(b.to_vec()),
            Message::Close(frame) => {
                let (code, reason) = frame
                    .map(|f| (Some(f.code.into()), f.reason.to_string()))
                    .unwrap_or((None, String::new()));
                WsMessage::Close(code, reason)
            }
            Message::Frame(_) => WsMessage::Binary(vec![]),
        }
    }

    /// Get mutable access to the underlying stream for select! loops
    pub fn stream_mut(&mut self) -> &mut WsStream {
        &mut self.stream
    }
}

/// Certificate verifier that accepts all certificates (insecure)
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
