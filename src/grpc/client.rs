//! gRPC client implementation
//!
//! This module provides a gRPC client that can make dynamic calls
//! using JSON payloads, leveraging prost-reflect for proper protobuf
//! encoding/decoding when a proto schema is available.
//!
//! Supports all gRPC call types:
//! - Unary: single request, single response
//! - Server streaming: single request, stream of responses
//! - Client streaming: stream of requests, single response
//! - Bidirectional streaming: stream of requests, stream of responses

use std::time::Duration;
use std::path::Path;
use std::pin::Pin;
use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use tonic::metadata::{MetadataMap, MetadataKey, AsciiMetadataValue, Ascii};
use tonic::Status;
use serde_json::Value as JsonValue;
use crate::client::ssl::SslConfig;
use crate::errors::QuicpulseError;
use super::GrpcEndpoint;
use super::dynamic::{GrpcSchema, MethodInfo, RawMessage, RawCodec, decode_to_json_schemaless};
use super::proto_parser::ProtoSchema;

/// gRPC client for making dynamic calls
pub struct GrpcClient {
    channel: Channel,
    endpoint: GrpcEndpoint,
    metadata: MetadataMap,
    timeout: Option<Duration>,
    schema: Option<ProtoSchema>,
}

impl GrpcClient {
    /// Create a new gRPC client
    pub async fn connect(endpoint: GrpcEndpoint) -> Result<Self, QuicpulseError> {
        Self::connect_with_options(endpoint, None, None, None).await
    }

    /// Create a new gRPC client with options
    pub async fn connect_with_options(
        endpoint: GrpcEndpoint,
        timeout: Option<Duration>,
        headers: Option<Vec<(String, String)>>,
        ssl_config: Option<&SslConfig>,
    ) -> Result<Self, QuicpulseError> {
        Self::connect_with_proxy(endpoint, timeout, headers, ssl_config, None).await
    }

    /// Create a new gRPC client with options including proxy support
    pub async fn connect_with_proxy(
        endpoint: GrpcEndpoint,
        timeout: Option<Duration>,
        headers: Option<Vec<(String, String)>>,
        ssl_config: Option<&SslConfig>,
        proxy: Option<&str>,
    ) -> Result<Self, QuicpulseError> {
        if let Some(proxy_url) = proxy {
            eprintln!("Warning: gRPC proxy support is limited. Proxy '{}' may not be used for this connection.", proxy_url);
            eprintln!("For HTTP requests, proxy works normally. For gRPC, consider using a transparent proxy.");
        }

        let uri = endpoint.uri();

        let mut ep = Endpoint::from_shared(uri.clone())
            .map_err(|e| QuicpulseError::Connection(format!("Invalid endpoint: {}", e)))?;

        // Bug #2 fix: Configure TLS using the provided SslConfig if available
        if endpoint.use_tls {
            let tls_config = build_grpc_tls_config(ssl_config)?;
            ep = ep.tls_config(tls_config)
                .map_err(|e| QuicpulseError::Connection(format!("TLS configuration error: {}", e)))?;
        }

        // NOTE: Don't set endpoint-level timeout here - it applies to streaming too
        // and would kill long-lived streams. Instead, we apply timeout at the
        // request level for unary calls only.
        // For streaming, configure TCP keep-alive to detect dead connections:
        ep = ep.tcp_keepalive(Some(Duration::from_secs(60)));

        // Connect
        let channel = ep.connect()
            .await
            .map_err(|e| QuicpulseError::Connection(format!("Failed to connect to {}: {}", uri, e)))?;

        // Build metadata from headers
        let mut metadata = MetadataMap::new();
        if let Some(hdrs) = headers {
            for (key, value) in hdrs {
                if let Ok(key) = key.parse::<MetadataKey<Ascii>>() {
                    if let Ok(val) = value.parse::<AsciiMetadataValue>() {
                        metadata.insert(key, val);
                    }
                }
            }
        }

        Ok(Self {
            channel,
            endpoint,
            metadata,
            timeout,
            schema: None,
        })
    }

    /// Load a proto schema from a file
    pub fn load_proto(&mut self, path: &Path) -> Result<(), QuicpulseError> {
        let schema = ProtoSchema::from_file(path)?;
        self.schema = Some(schema);
        Ok(())
    }

    /// Load a proto schema from content string
    pub fn load_proto_content(&mut self, content: &str) -> Result<(), QuicpulseError> {
        let schema = ProtoSchema::parse(content)?;
        self.schema = Some(schema);
        Ok(())
    }

    /// Set the proto schema directly
    pub fn set_schema(&mut self, schema: ProtoSchema) {
        self.schema = Some(schema);
    }

    /// Get the loaded schema
    pub fn schema(&self) -> Option<&ProtoSchema> {
        self.schema.as_ref()
    }

    /// Get the channel for raw access
    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }

    /// Add metadata header
    pub fn add_metadata(&mut self, key: &str, value: &str) -> Result<(), QuicpulseError> {
        let key: MetadataKey<Ascii> = key.parse()
            .map_err(|_| QuicpulseError::Argument(format!("Invalid header key: {}", key)))?;
        let val = value.parse::<AsciiMetadataValue>()
            .map_err(|_| QuicpulseError::Argument(format!("Invalid header value: {}", value)))?;
        self.metadata.insert(key, val);
        Ok(())
    }

    /// Make a unary call with JSON request/response
    /// If a proto schema is loaded, uses prost-reflect for proper encoding/decoding.
    /// Bug #6 fix: Falls back to server reflection to fetch schema if not loaded.
    pub async fn call_unary(
        &mut self,
        service: &str,
        method: &str,
        request_json: &JsonValue,
    ) -> Result<GrpcResponse, QuicpulseError> {
        // Bug #6 fix: Try to load schema via reflection if not available
        if self.schema.is_none() || self.schema.as_ref().and_then(|s| s.grpc_schema()).is_none() {
            // Try to fetch schema from server reflection
            match super::reflection::fetch_schema_for_service(self.channel.clone(), service).await {
                Ok(grpc_schema) => {
                    // Create a ProtoSchema wrapper with the GrpcSchema
                    let mut proto_schema = ProtoSchema::default();
                    proto_schema.set_grpc_schema(grpc_schema);
                    self.schema = Some(proto_schema);
                }
                Err(_) => {
                    // Reflection failed - will error below if no schema
                }
            }
        }

        // Try to encode using GrpcSchema if available
        let request_bytes = if let Some(ref schema) = self.schema {
            if let Some(grpc_schema) = schema.grpc_schema() {
                // Use prost-reflect for proper encoding
                grpc_schema.encode_request(service, method, request_json)?
            } else {
                // Schema parsed but not compiled - this shouldn't happen with protox
                return Err(QuicpulseError::Argument(
                    "Proto schema loaded but not properly compiled. Please check your .proto file.".to_string()
                ));
            }
        } else {
            // No schema - can't encode properly
            return Err(QuicpulseError::Argument(
                "No proto schema loaded and server reflection unavailable. Use --proto to specify a .proto file for gRPC calls.".to_string()
            ));
        };

        // Build the gRPC path
        let path = format!("/{}/{}", service, method);

        // Create a gRPC client from the channel
        let mut client = tonic::client::Grpc::new(self.channel.clone());

        // Build the request with metadata
        let mut request = tonic::Request::new(RawMessage(request_bytes));

        // Apply timeout at request level (only for unary calls, not streaming)
        if let Some(t) = self.timeout {
            request.set_timeout(t);
        }

        // Add metadata headers
        for key_value in self.metadata.iter() {
            match key_value {
                tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
                    if let Ok(k) = key.as_str().parse::<tonic::metadata::MetadataKey<Ascii>>() {
                        request.metadata_mut().insert(k, value.clone());
                    }
                }
                tonic::metadata::KeyAndValueRef::Binary(key, value) => {
                    if let Ok(k) = key.as_str().parse::<tonic::metadata::MetadataKey<tonic::metadata::Binary>>() {
                        request.metadata_mut().insert_bin(k, value.clone());
                    }
                }
            }
        }

        // Make the unary call
        let path_uri: http::uri::PathAndQuery = path.parse()
            .map_err(|e| QuicpulseError::Argument(format!("Invalid gRPC path: {}", e)))?;

        let response = client.unary(request, path_uri, RawCodec).await;

        match response {
            Ok(resp) => {
                let (response_metadata, body, _extensions) = resp.into_parts();
                let response_bytes = body.0;

                // Decode the response using schema if available
                let response_json = if response_bytes.is_empty() {
                    JsonValue::Object(serde_json::Map::new())
                } else if let Some(ref schema) = self.schema {
                    if let Some(grpc_schema) = schema.grpc_schema() {
                        // Use prost-reflect for proper decoding
                        grpc_schema.decode_response(service, method, &response_bytes)?
                    } else {
                        // Fall back to schemaless decoding
                        decode_to_json_schemaless(&response_bytes)?
                    }
                } else {
                    // No schema - use schemaless decoder
                    decode_to_json_schemaless(&response_bytes)?
                };

                Ok(GrpcResponse {
                    status: Status::ok(""),
                    body: serde_json::to_vec(&response_json).unwrap_or_default(),
                    metadata: response_metadata,
                    trailing_metadata: MetadataMap::new(),
                })
            }
            Err(status) => {
                Ok(GrpcResponse {
                    status,
                    body: Vec::new(),
                    metadata: MetadataMap::new(),
                    trailing_metadata: MetadataMap::new(),
                })
            }
        }
    }

    /// Get the GrpcSchema if available
    pub fn grpc_schema(&self) -> Option<&GrpcSchema> {
        self.schema.as_ref().and_then(|s| s.grpc_schema())
    }

    /// Get method info (streaming type)
    pub fn get_method_info(&self, service: &str, method: &str) -> Option<MethodInfo> {
        self.grpc_schema().and_then(|s| s.get_method_info(service, method))
    }

    /// Make a server streaming call (single request, stream of responses)
    pub async fn call_server_streaming(
        &self,
        service: &str,
        method: &str,
        request_json: &JsonValue,
    ) -> Result<GrpcStreamingResponse, QuicpulseError> {
        // Encode the request
        let request_bytes = self.encode_request(service, method, request_json)?;

        // Build the gRPC path
        let path = format!("/{}/{}", service, method);
        let path_uri: http::uri::PathAndQuery = path.parse()
            .map_err(|e| QuicpulseError::Argument(format!("Invalid gRPC path: {}", e)))?;

        // Create a gRPC client
        let mut client = tonic::client::Grpc::new(self.channel.clone());

        // Build request with metadata
        let mut request = tonic::Request::new(RawMessage(request_bytes));
        self.apply_metadata(&mut request);

        // Make the server streaming call
        let response = client.server_streaming(request, path_uri, RawCodec).await;

        match response {
            Ok(resp) => {
                let (metadata, body_stream, _extensions) = resp.into_parts();

                // Clone schema for async closure
                let schema = self.schema.clone();
                let service_name = service.to_string();
                let method_name = method.to_string();

                Ok(GrpcStreamingResponse {
                    status: Status::ok(""),
                    metadata,
                    stream: Box::pin(body_stream.map(move |result: Result<RawMessage, Status>| {
                        match result {
                            Ok(raw_msg) => {
                                let bytes = raw_msg.0;
                                if bytes.is_empty() {
                                    Ok(JsonValue::Object(serde_json::Map::new()))
                                } else if let Some(ref schema) = schema {
                                    if let Some(grpc_schema) = schema.grpc_schema() {
                                        grpc_schema.decode_response(&service_name, &method_name, &bytes)
                                    } else {
                                        decode_to_json_schemaless(&bytes)
                                    }
                                } else {
                                    decode_to_json_schemaless(&bytes)
                                }
                            }
                            Err(status) => Err(QuicpulseError::Connection(format!(
                                "Stream error: {:?} - {}", status.code(), status.message()
                            ))),
                        }
                    })),
                })
            }
            Err(status) => {
                // Return error as a stream that yields one error
                Ok(GrpcStreamingResponse {
                    status,
                    metadata: MetadataMap::new(),
                    stream: Box::pin(futures::stream::empty()),
                })
            }
        }
    }

    /// Make a client streaming call (stream of requests, single response)
    pub async fn call_client_streaming<S>(
        &self,
        service: &str,
        method: &str,
        request_stream: S,
    ) -> Result<GrpcResponse, QuicpulseError>
    where
        S: Stream<Item = JsonValue> + Send + 'static,
    {
        // Clone schema for the stream transformation
        let schema = self.schema.clone();
        let service_name = service.to_string();
        let method_name = method.to_string();

        // Transform JSON stream to RawMessage stream
        let raw_stream = request_stream.filter_map(move |json| {
            let schema = schema.clone();
            let svc = service_name.clone();
            let mth = method_name.clone();
            async move {
                if let Some(ref schema) = schema {
                    if let Some(grpc_schema) = schema.grpc_schema() {
                        match grpc_schema.encode_request(&svc, &mth, &json) {
                            Ok(bytes) => Some(RawMessage(bytes)),
                            Err(_) => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        });

        // Build the gRPC path
        let path = format!("/{}/{}", service, method);
        let path_uri: http::uri::PathAndQuery = path.parse()
            .map_err(|e| QuicpulseError::Argument(format!("Invalid gRPC path: {}", e)))?;

        // Create a gRPC client
        let mut client = tonic::client::Grpc::new(self.channel.clone());

        // Build request with metadata
        let mut request = tonic::Request::new(raw_stream);
        self.apply_metadata(&mut request);

        // Make the client streaming call
        let response = client.client_streaming(request, path_uri, RawCodec).await;

        match response {
            Ok(resp) => {
                let (response_metadata, body, _extensions) = resp.into_parts();
                let response_bytes = body.0;

                // Decode the response
                let response_json = if response_bytes.is_empty() {
                    JsonValue::Object(serde_json::Map::new())
                } else if let Some(ref schema) = self.schema {
                    if let Some(grpc_schema) = schema.grpc_schema() {
                        grpc_schema.decode_response(service, method, &response_bytes)?
                    } else {
                        decode_to_json_schemaless(&response_bytes)?
                    }
                } else {
                    decode_to_json_schemaless(&response_bytes)?
                };

                Ok(GrpcResponse {
                    status: Status::ok(""),
                    body: serde_json::to_vec(&response_json).unwrap_or_default(),
                    metadata: response_metadata,
                    trailing_metadata: MetadataMap::new(),
                })
            }
            Err(status) => {
                Ok(GrpcResponse {
                    status,
                    body: Vec::new(),
                    metadata: MetadataMap::new(),
                    trailing_metadata: MetadataMap::new(),
                })
            }
        }
    }

    /// Make a bidirectional streaming call (stream of requests, stream of responses)
    pub async fn call_bidi_streaming<S>(
        &self,
        service: &str,
        method: &str,
        request_stream: S,
    ) -> Result<GrpcStreamingResponse, QuicpulseError>
    where
        S: Stream<Item = JsonValue> + Send + 'static,
    {
        // Clone schema for the stream transformation
        let schema = self.schema.clone();
        let service_name = service.to_string();
        let method_name = method.to_string();

        // Transform JSON stream to RawMessage stream
        let raw_stream = request_stream.filter_map(move |json| {
            let schema = schema.clone();
            let svc = service_name.clone();
            let mth = method_name.clone();
            async move {
                if let Some(ref schema) = schema {
                    if let Some(grpc_schema) = schema.grpc_schema() {
                        match grpc_schema.encode_request(&svc, &mth, &json) {
                            Ok(bytes) => Some(RawMessage(bytes)),
                            Err(_) => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        });

        // Build the gRPC path
        let path = format!("/{}/{}", service, method);
        let path_uri: http::uri::PathAndQuery = path.parse()
            .map_err(|e| QuicpulseError::Argument(format!("Invalid gRPC path: {}", e)))?;

        // Create a gRPC client
        let mut client = tonic::client::Grpc::new(self.channel.clone());

        // Build request with metadata
        let mut request = tonic::Request::new(raw_stream);
        self.apply_metadata(&mut request);

        // Make the bidirectional streaming call
        // Clone schema for async closure
        let schema = self.schema.clone();
        let svc = service.to_string();
        let mth = method.to_string();

        let response = client.streaming(request, path_uri, RawCodec).await;

        match response {
            Ok(resp) => {
                let (metadata, body_stream, _extensions) = resp.into_parts();

                Ok(GrpcStreamingResponse {
                    status: Status::ok(""),
                    metadata,
                    stream: Box::pin(body_stream.map(move |result: Result<RawMessage, Status>| {
                        match result {
                            Ok(raw_msg) => {
                                let bytes = raw_msg.0;
                                if bytes.is_empty() {
                                    Ok(JsonValue::Object(serde_json::Map::new()))
                                } else if let Some(ref schema) = schema {
                                    if let Some(grpc_schema) = schema.grpc_schema() {
                                        grpc_schema.decode_response(&svc, &mth, &bytes)
                                    } else {
                                        decode_to_json_schemaless(&bytes)
                                    }
                                } else {
                                    decode_to_json_schemaless(&bytes)
                                }
                            }
                            Err(status) => Err(QuicpulseError::Connection(format!(
                                "Stream error: {:?} - {}", status.code(), status.message()
                            ))),
                        }
                    })),
                })
            }
            Err(status) => {
                Ok(GrpcStreamingResponse {
                    status,
                    metadata: MetadataMap::new(),
                    stream: Box::pin(futures::stream::empty()),
                })
            }
        }
    }

    /// Helper to encode a request using the schema
    fn encode_request(&self, service: &str, method: &str, json: &JsonValue) -> Result<Bytes, QuicpulseError> {
        if let Some(ref schema) = self.schema {
            if let Some(grpc_schema) = schema.grpc_schema() {
                grpc_schema.encode_request(service, method, json)
            } else {
                Err(QuicpulseError::Argument(
                    "Proto schema loaded but not properly compiled.".to_string()
                ))
            }
        } else {
            Err(QuicpulseError::Argument(
                "No proto schema loaded. Use --proto to specify a .proto file.".to_string()
            ))
        }
    }

    /// Helper to apply metadata to a request
    fn apply_metadata<T>(&self, request: &mut tonic::Request<T>) {
        for key_value in self.metadata.iter() {
            match key_value {
                tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
                    if let Ok(k) = key.as_str().parse::<tonic::metadata::MetadataKey<Ascii>>() {
                        request.metadata_mut().insert(k, value.clone());
                    }
                }
                tonic::metadata::KeyAndValueRef::Binary(key, value) => {
                    if let Ok(k) = key.as_str().parse::<tonic::metadata::MetadataKey<tonic::metadata::Binary>>() {
                        request.metadata_mut().insert_bin(k, value.clone());
                    }
                }
            }
        }
    }

    /// List services from the loaded schema
    pub fn list_services(&self) -> Vec<String> {
        self.schema.as_ref()
            .and_then(|s| s.grpc_schema())
            .map(|gs| gs.list_services())
            .unwrap_or_default()
    }

    /// List methods for a service from the loaded schema
    pub fn list_methods(&self, service: &str) -> Vec<String> {
        self.schema.as_ref()
            .and_then(|s| s.grpc_schema())
            .map(|gs| gs.list_methods(service))
            .unwrap_or_default()
    }

    /// Describe a service from the loaded schema
    pub fn describe_service(&self, service: &str) -> Option<String> {
        self.schema.as_ref()
            .and_then(|s| s.grpc_schema())
            .and_then(|gs| gs.describe_service(service))
    }

    /// Describe a message type from the loaded schema
    pub fn describe_message(&self, message: &str) -> Option<String> {
        self.schema.as_ref()
            .and_then(|s| s.grpc_schema())
            .and_then(|gs| gs.describe_message(message))
    }
}

/// A gRPC request
#[derive(Debug, Clone)]
pub struct GrpcRequest {
    pub service: String,
    pub method: String,
    pub path: String,
    pub body: Vec<u8>,
    pub metadata: MetadataMap,
}

impl GrpcRequest {
    /// Create a new request
    pub fn new(service: impl Into<String>, method: impl Into<String>) -> Self {
        let service = service.into();
        let method = method.into();
        let path = format!("/{}/{}", service, method);

        Self {
            service,
            method,
            path,
            body: Vec::new(),
            metadata: MetadataMap::new(),
        }
    }

    /// Set the request body from JSON
    pub fn with_json(mut self, json: &JsonValue) -> Result<Self, QuicpulseError> {
        self.body = serde_json::to_vec(json)
            .map_err(|e| QuicpulseError::Argument(format!("Failed to serialize: {}", e)))?;
        Ok(self)
    }

    /// Add a metadata header
    pub fn with_metadata(mut self, key: &str, value: &str) -> Result<Self, QuicpulseError> {
        let key: MetadataKey<Ascii> = key.parse()
            .map_err(|_| QuicpulseError::Argument(format!("Invalid key: {}", key)))?;
        let val = value.parse::<AsciiMetadataValue>()
            .map_err(|_| QuicpulseError::Argument(format!("Invalid value: {}", value)))?;
        self.metadata.insert(key, val);
        Ok(self)
    }
}

/// A gRPC response
#[derive(Debug, Clone)]
pub struct GrpcResponse {
    pub status: Status,
    pub body: Vec<u8>,
    pub metadata: MetadataMap,
    pub trailing_metadata: MetadataMap,
}

impl GrpcResponse {
    /// Check if the response is successful
    pub fn is_ok(&self) -> bool {
        self.status.code() == tonic::Code::Ok
    }

    /// Get the status code
    pub fn code(&self) -> tonic::Code {
        self.status.code()
    }

    /// Get the status message
    pub fn message(&self) -> &str {
        self.status.message()
    }

    /// Try to parse the body as JSON
    pub fn json(&self) -> Result<JsonValue, QuicpulseError> {
        serde_json::from_slice(&self.body)
            .map_err(|e| QuicpulseError::Argument(format!("Failed to parse response: {}", e)))
    }

    /// Format for display
    pub fn format_display(&self) -> String {
        let mut output = String::new();

        // Status line
        output.push_str(&format!("Status: {:?} ({})\n", self.code(), self.status.message()));

        // Metadata
        if !self.metadata.is_empty() {
            output.push_str("\nMetadata:\n");
            for key_value in self.metadata.iter() {
                match key_value {
                    tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
                        if let Ok(v) = value.to_str() {
                            output.push_str(&format!("  {}: {}\n", key.as_str(), v));
                        }
                    }
                    tonic::metadata::KeyAndValueRef::Binary(key, value) => {
                        output.push_str(&format!("  {}: <binary {} bytes>\n", key.as_str(), value.as_ref().len()));
                    }
                }
            }
        }

        // Body
        if !self.body.is_empty() {
            output.push_str("\nBody:\n");
            if let Ok(json) = self.json() {
                if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                    output.push_str(&pretty);
                } else {
                    output.push_str(&String::from_utf8_lossy(&self.body));
                }
            } else {
                output.push_str(&format!("<binary {} bytes>", self.body.len()));
            }
        }

        output
    }
}

/// A gRPC streaming response
pub struct GrpcStreamingResponse {
    pub status: Status,
    pub metadata: MetadataMap,
    pub stream: Pin<Box<dyn Stream<Item = Result<JsonValue, QuicpulseError>> + Send>>,
}

impl GrpcStreamingResponse {
    /// Check if the initial response status is successful
    pub fn is_ok(&self) -> bool {
        self.status.code() == tonic::Code::Ok
    }

    /// Get the status code
    pub fn code(&self) -> tonic::Code {
        self.status.code()
    }

    /// Get the status message
    pub fn message(&self) -> &str {
        self.status.message()
    }

    /// Take the stream out of the response
    pub fn into_stream(self) -> Pin<Box<dyn Stream<Item = Result<JsonValue, QuicpulseError>> + Send>> {
        self.stream
    }
}

/// Bug #2 fix: Build a ClientTlsConfig from SslConfig
/// This enables gRPC to use the same TLS settings as HTTP (--verify, --cert, --cert-key)
fn build_grpc_tls_config(ssl_config: Option<&SslConfig>) -> Result<ClientTlsConfig, QuicpulseError> {
    let mut tls_config = ClientTlsConfig::new();

    if let Some(config) = ssl_config {
        // Handle certificate verification
        if !config.verify {
            // Note: tonic doesn't have a direct way to disable cert verification like reqwest
            // For gRPC with self-signed certs, users should provide the CA bundle via --verify=/path/to/ca.pem
            eprintln!("Warning: --verify=no is not fully supported for gRPC. \
                       For self-signed certificates, use --verify=/path/to/ca.pem instead.");
        }

        // Load custom CA bundle if specified
        if let Some(ca_bundle) = &config.ca_bundle {
            let ca_data = std::fs::read(ca_bundle)
                .map_err(|e| QuicpulseError::Ssl(format!(
                    "Failed to read CA bundle '{}': {}", ca_bundle.display(), e
                )))?;

            let ca_cert = tonic::transport::Certificate::from_pem(ca_data);
            tls_config = tls_config.ca_certificate(ca_cert);
        }

        // Handle client certificate (mTLS)
        if config.client_cert.is_configured() {
            if let Some(cert_path) = &config.client_cert.cert_file {
                let cert_data = std::fs::read(cert_path)
                    .map_err(|e| QuicpulseError::Ssl(format!(
                        "Failed to read client cert '{}': {}", cert_path.display(), e
                    )))?;

                let key_data = if let Some(key_path) = &config.client_cert.key_file {
                    std::fs::read(key_path)
                        .map_err(|e| QuicpulseError::Ssl(format!(
                            "Failed to read client key '{}': {}", key_path.display(), e
                        )))?
                } else {
                    // Key might be in the same file as cert
                    cert_data.clone()
                };

                let identity = tonic::transport::Identity::from_pem(cert_data, key_data);
                tls_config = tls_config.identity(identity);
            }
        }
    }

    Ok(tls_config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_request_new() {
        let req = GrpcRequest::new("mypackage.MyService", "MyMethod");
        assert_eq!(req.service, "mypackage.MyService");
        assert_eq!(req.method, "MyMethod");
        assert_eq!(req.path, "/mypackage.MyService/MyMethod");
    }

    #[test]
    fn test_grpc_request_with_json() {
        let json = serde_json::json!({"name": "test"});
        let req = GrpcRequest::new("Test", "Method")
            .with_json(&json)
            .unwrap();

        assert!(!req.body.is_empty());
    }
}
