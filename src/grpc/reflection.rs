//! gRPC reflection support
//!
//! This module provides gRPC server reflection client functionality
//! for discovering available services and methods.

use tonic::transport::Channel;
use serde::{Deserialize, Serialize};
use crate::errors::QuicpulseError;

/// Standard gRPC reflection service name
pub const REFLECTION_SERVICE_V1: &str = "grpc.reflection.v1.ServerReflection";
pub const REFLECTION_SERVICE_V1ALPHA: &str = "grpc.reflection.v1alpha.ServerReflection";

/// Service descriptor from reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDescriptor {
    pub name: String,
    pub full_name: String,
    pub methods: Vec<MethodDescriptor>,
    pub description: Option<String>,
}

impl ServiceDescriptor {
    /// Format for display
    pub fn format_display(&self) -> String {
        let mut output = format!("service {} {{\n", self.name);

        for method in &self.methods {
            let _stream_prefix = match (method.client_streaming, method.server_streaming) {
                (true, true) => "stream ",
                (true, false) => "client-stream ",
                (false, true) => "stream ",
                (false, false) => "",
            };

            output.push_str(&format!(
                "  rpc {}({}{}) returns ({}{});\n",
                method.name,
                if method.client_streaming { "stream " } else { "" },
                method.input_type,
                if method.server_streaming { "stream " } else { "" },
                method.output_type
            ));

            if let Some(ref desc) = method.description {
                output.push_str(&format!("    // {}\n", desc));
            }
        }

        output.push_str("}\n");
        output
    }
}

/// Method descriptor from reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodDescriptor {
    pub name: String,
    pub full_name: String,
    pub input_type: String,
    pub output_type: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
    pub description: Option<String>,
}

impl MethodDescriptor {
    /// Get the full method path for gRPC calls
    pub fn path(&self) -> String {
        format!("/{}", self.full_name.replace('.', "/"))
    }

    /// Format for display
    pub fn format_display(&self) -> String {
        let stream_info = match (self.client_streaming, self.server_streaming) {
            (true, true) => " (bidirectional streaming)",
            (true, false) => " (client streaming)",
            (false, true) => " (server streaming)",
            (false, false) => "",
        };

        format!(
            "{}\n  Input: {}\n  Output: {}{}",
            self.name,
            self.input_type,
            self.output_type,
            stream_info
        )
    }
}

/// Message type descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDescriptor {
    pub name: String,
    pub full_name: String,
    pub fields: Vec<FieldDescriptor>,
}

impl MessageDescriptor {
    /// Format as JSON schema hint
    pub fn to_json_template(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();

        for field in &self.fields {
            let value = match field.type_name.as_str() {
                "string" | "bytes" => serde_json::json!(""),
                "int32" | "int64" | "uint32" | "uint64" | "sint32" | "sint64" |
                "fixed32" | "fixed64" | "sfixed32" | "sfixed64" => serde_json::json!(0),
                "float" | "double" => serde_json::json!(0.0),
                "bool" => serde_json::json!(false),
                _ => {
                    if field.is_repeated {
                        serde_json::json!([])
                    } else if field.is_map {
                        serde_json::json!({})
                    } else {
                        serde_json::json!({})
                    }
                }
            };

            obj.insert(field.name.clone(), if field.is_repeated {
                serde_json::json!([value])
            } else {
                value
            });
        }

        serde_json::Value::Object(obj)
    }
}

/// Field descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDescriptor {
    pub name: String,
    pub number: i32,
    pub type_name: String,
    pub is_repeated: bool,
    pub is_map: bool,
    pub is_optional: bool,
}

/// Reflection client for service discovery
pub struct ReflectionClient {
    channel: Channel,
}

impl ReflectionClient {
    /// Create a new reflection client
    pub fn new(channel: Channel) -> Self {
        Self { channel }
    }

    /// List all available services using gRPC reflection
    pub async fn list_services(&self) -> Result<Vec<String>, QuicpulseError> {
        use super::dynamic::{RawMessage, RawCodec, decode_to_json_schemaless};
        use super::codec::WireEncoder;
        use tonic::client::Grpc;

        // Try v1 first, then v1alpha
        for service_name in &[REFLECTION_SERVICE_V1, REFLECTION_SERVICE_V1ALPHA] {
            let mut client = Grpc::new(self.channel.clone());

            // Build ListServices request manually
            // Field 3 = list_services (string), empty string for listing all services
            let mut encoder = WireEncoder::new();
            encoder.write_string(3, ""); // list_services = ""
            let request_bytes = encoder.finish();

            let request = tonic::Request::new(RawMessage(request_bytes));
            let path: http::uri::PathAndQuery = format!("/{}/ServerReflectionInfo", service_name)
                .parse()
                .map_err(|e| QuicpulseError::Argument(format!("Invalid path: {}", e)))?;

            // For streaming, we need to handle it differently
            // ServerReflectionInfo is a bidirectional streaming RPC
            // For simplicity, try a unary call first (some servers support this)
            match client.unary(request, path.clone(), RawCodec).await {
                Ok(resp) => {
                    let (_, body, _) = resp.into_parts();
                    let json = decode_to_json_schemaless(&body.0)?;

                    // Parse the response to extract service names
                    // The response contains list_services_response with service fields
                    return extract_service_list(&json);
                }
                Err(_) => {
                    // Try streaming approach or continue to next version
                    continue;
                }
            }
        }

        Err(QuicpulseError::Argument(
            "gRPC reflection not available. Server may not have reflection enabled. \
             Try using a proto file with --proto instead.".to_string()
        ))
    }

    /// Get file descriptor containing a symbol
    pub async fn file_containing_symbol(&self, symbol: &str) -> Result<Vec<u8>, QuicpulseError> {
        use super::dynamic::{RawMessage, RawCodec};
        use super::codec::WireEncoder;
        use tonic::client::Grpc;

        for service_name in &[REFLECTION_SERVICE_V1, REFLECTION_SERVICE_V1ALPHA] {
            let mut client = Grpc::new(self.channel.clone());

            // Build FileContainingSymbol request
            // Field 4 = file_containing_symbol (string)
            let mut encoder = WireEncoder::new();
            encoder.write_string(4, symbol);
            let request_bytes = encoder.finish();

            let request = tonic::Request::new(RawMessage(request_bytes));
            let path: http::uri::PathAndQuery = format!("/{}/ServerReflectionInfo", service_name)
                .parse()
                .map_err(|e| QuicpulseError::Argument(format!("Invalid path: {}", e)))?;

            match client.unary(request, path, RawCodec).await {
                Ok(resp) => {
                    let (_, body, _) = resp.into_parts();
                    return Ok(body.0.to_vec());
                }
                Err(_) => continue,
            }
        }

        Err(QuicpulseError::Argument(
            format!("Could not get file descriptor for symbol '{}'", symbol)
        ))
    }
}

/// Extract service list from reflection response JSON
fn extract_service_list(json: &serde_json::Value) -> Result<Vec<String>, QuicpulseError> {
    let mut services = Vec::new();

    // Response structure varies, try to find service names in various places
    if let Some(obj) = json.as_object() {
        for (_, value) in obj {
            if let Some(arr) = value.as_array() {
                for item in arr {
                    if let Some(name) = item.as_str() {
                        if !name.is_empty() && !name.starts_with("grpc.reflection") {
                            services.push(name.to_string());
                        }
                    } else if let Some(obj) = item.as_object() {
                        // Nested service object
                        if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                            if !name.starts_with("grpc.reflection") {
                                services.push(name.to_string());
                            }
                        }
                    }
                }
            } else if let Some(s) = value.as_str() {
                if !s.is_empty() && !s.starts_with("grpc.reflection") {
                    services.push(s.to_string());
                }
            }
        }
    }

    if services.is_empty() {
        Err(QuicpulseError::Argument(
            "No services found in reflection response".to_string()
        ))
    } else {
        Ok(services)
    }
}

/// Discover services using gRPC reflection
pub async fn discover_services(channel: Channel) -> Result<Vec<ServiceDescriptor>, QuicpulseError> {
    let client = ReflectionClient::new(channel);
    let service_names = client.list_services().await?;

    // For now, return basic descriptors without methods
    // Full implementation would fetch file descriptors and parse them
    Ok(service_names.into_iter().map(|name| {
        let parts: Vec<&str> = name.rsplitn(2, '.').collect();
        let short_name = parts.first().unwrap_or(&name.as_str()).to_string();

        ServiceDescriptor {
            name: short_name,
            full_name: name,
            methods: Vec::new(), // Would be populated from file descriptor
            description: None,
        }
    }).collect())
}

/// List services from a reflection-enabled server
pub async fn list_services(channel: Channel) -> Result<Vec<String>, QuicpulseError> {
    let client = ReflectionClient::new(channel);
    client.list_services().await
}

/// Bug #6 fix: Fetch GrpcSchema from server reflection
/// This allows making gRPC calls without providing a local .proto file
pub async fn fetch_schema_for_service(
    channel: Channel,
    service_name: &str,
) -> Result<super::dynamic::GrpcSchema, QuicpulseError> {
    use super::dynamic::{RawMessage, RawCodec};
    use super::codec::WireEncoder;
    use tonic::client::Grpc;

    // Try both reflection API versions
    for reflection_service in &[REFLECTION_SERVICE_V1, REFLECTION_SERVICE_V1ALPHA] {
        let mut client = Grpc::new(channel.clone());

        // Build FileContainingSymbol request
        // Field 4 = file_containing_symbol (string)
        let mut encoder = WireEncoder::new();
        encoder.write_string(4, service_name);
        let request_bytes = encoder.finish();

        let request = tonic::Request::new(RawMessage(request_bytes));
        let path: http::uri::PathAndQuery = format!("/{}/ServerReflectionInfo", reflection_service)
            .parse()
            .map_err(|e| QuicpulseError::Argument(format!("Invalid path: {}", e)))?;

        match client.unary(request, path, RawCodec).await {
            Ok(resp) => {
                let (_, body, _) = resp.into_parts();
                let response_bytes = body.0.to_vec();

                // Parse the reflection response to extract file descriptors
                if let Some(fds) = parse_file_descriptor_response(&response_bytes) {
                    return super::dynamic::GrpcSchema::from_file_descriptor_set(fds);
                }
            }
            Err(_) => continue,
        }
    }

    Err(QuicpulseError::Argument(format!(
        "Could not fetch schema for service '{}' via reflection. \
         Server may not support reflection or the service doesn't exist.",
        service_name
    )))
}

/// Parse file descriptors from a ServerReflectionResponse
/// Returns a FileDescriptorSet if successful
fn parse_file_descriptor_response(response_bytes: &[u8]) -> Option<prost_types::FileDescriptorSet> {
    use super::codec::{WireDecoder, WireType};
    use bytes::Bytes;

    // ServerReflectionResponse has file_descriptor_response at field 4
    // FileDescriptorResponse has file_descriptor_proto (repeated bytes) at field 1
    let mut decoder = WireDecoder::new(Bytes::copy_from_slice(response_bytes));

    while decoder.has_remaining() {
        let (field_num, wire_type) = match decoder.read_tag() {
            Ok(t) => t,
            Err(_) => break,
        };

        if field_num == 4 && wire_type == WireType::LengthDelimited {
            // Length-delimited field: file_descriptor_response
            if let Ok(fdr_bytes) = decoder.read_length_delimited() {
                // Parse FileDescriptorResponse
                let mut fdr_decoder = WireDecoder::new(fdr_bytes);
                let mut file_descriptors = Vec::new();

                while fdr_decoder.has_remaining() {
                    let (fdr_field, fdr_wire) = match fdr_decoder.read_tag() {
                        Ok(t) => t,
                        Err(_) => break,
                    };

                    if fdr_field == 1 && fdr_wire == WireType::LengthDelimited {
                        // file_descriptor_proto (bytes)
                        if let Ok(fd_bytes) = fdr_decoder.read_length_delimited() {
                            // Parse as FileDescriptorProto
                            if let Ok(fd) = prost::Message::decode(fd_bytes.as_ref()) {
                                file_descriptors.push(fd);
                            }
                        }
                    } else {
                        let _ = fdr_decoder.skip_field(fdr_wire);
                    }
                }

                if !file_descriptors.is_empty() {
                    return Some(prost_types::FileDescriptorSet { file: file_descriptors });
                }
            }
        } else {
            let _ = decoder.skip_field(wire_type);
        }
    }

    None
}

/// Get service details including methods
pub async fn describe_service(
    channel: Channel,
    service_name: &str,
) -> Result<ServiceDescriptor, QuicpulseError> {
    let client = ReflectionClient::new(channel);

    // Try to get file descriptor for the service
    let _descriptor_bytes = client.file_containing_symbol(service_name).await?;

    // Parse the file descriptor to extract service info
    // This would require protobuf descriptor parsing
    // For now, return a basic descriptor
    Ok(ServiceDescriptor {
        name: service_name.rsplit('.').next().unwrap_or(service_name).to_string(),
        full_name: service_name.to_string(),
        methods: Vec::new(),
        description: Some("Use --grpc-proto to see full method details".to_string()),
    })
}

/// Get method details including input/output types
pub async fn describe_method(
    channel: Channel,
    service_name: &str,
    method_name: &str,
) -> Result<MethodDescriptor, QuicpulseError> {
    let full_name = format!("{}.{}", service_name, method_name);
    let client = ReflectionClient::new(channel);

    // Try to get file descriptor
    let _descriptor_bytes = client.file_containing_symbol(&full_name).await?;

    // Return basic descriptor
    Ok(MethodDescriptor {
        name: method_name.to_string(),
        full_name,
        input_type: "Unknown".to_string(),
        output_type: "Unknown".to_string(),
        client_streaming: false,
        server_streaming: false,
        description: Some("Use --grpc-proto to see full type details".to_string()),
    })
}

/// Parse a proto file and extract service descriptors
pub fn parse_proto_file(content: &str) -> Result<Vec<ServiceDescriptor>, QuicpulseError> {
    // Simple regex-based proto parser for basic service extraction
    // For full proto parsing, we'd need a proper protobuf parser

    let mut services = Vec::new();
    let mut current_service: Option<ServiceDescriptor> = None;
    let mut in_service = false;
    let mut brace_depth = 0;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        // Track brace depth
        brace_depth += line.matches('{').count();
        brace_depth -= line.matches('}').count();

        // Detect service definition
        if line.starts_with("service ") {
            let name = line.strip_prefix("service ")
                .and_then(|s| s.split('{').next())
                .map(|s| s.trim())
                .unwrap_or("");

            if !name.is_empty() {
                current_service = Some(ServiceDescriptor {
                    name: name.to_string(),
                    full_name: name.to_string(), // Would need package info for full name
                    methods: Vec::new(),
                    description: None,
                });
                in_service = true;
            }
        }

        // Detect rpc definitions within service
        if in_service && line.starts_with("rpc ") {
            if let Some(ref mut service) = current_service {
                if let Some(method) = parse_rpc_line(line, &service.full_name) {
                    service.methods.push(method);
                }
            }
        }

        // End of service
        if in_service && brace_depth == 0 && line.contains('}') {
            if let Some(service) = current_service.take() {
                services.push(service);
            }
            in_service = false;
        }
    }

    Ok(services)
}

/// Parse an RPC line from a proto file
fn parse_rpc_line(line: &str, service_name: &str) -> Option<MethodDescriptor> {
    // Format: rpc MethodName(InputType) returns (OutputType);
    // Or with streaming: rpc MethodName(stream InputType) returns (stream OutputType);

    let line = line.strip_prefix("rpc ")?.trim();

    // Extract method name
    let paren_idx = line.find('(')?;
    let name = line[..paren_idx].trim().to_string();

    // Extract input type
    let input_start = paren_idx + 1;
    let input_end = line.find(')')?;
    let input_part = &line[input_start..input_end];
    let (client_streaming, input_type) = if input_part.trim().starts_with("stream ") {
        (true, input_part.trim().strip_prefix("stream ")?.trim().to_string())
    } else {
        (false, input_part.trim().to_string())
    };

    // Find returns keyword
    let returns_idx = line.find("returns")?;
    let after_returns = &line[returns_idx + 7..];

    // Extract output type
    let out_start = after_returns.find('(')? + 1;
    let out_end = after_returns.find(')')?;
    let output_part = &after_returns[out_start..out_end];
    let (server_streaming, output_type) = if output_part.trim().starts_with("stream ") {
        (true, output_part.trim().strip_prefix("stream ")?.trim().to_string())
    } else {
        (false, output_part.trim().to_string())
    };

    Some(MethodDescriptor {
        name: name.clone(),
        full_name: format!("{}.{}", service_name, name),
        input_type,
        output_type,
        client_streaming,
        server_streaming,
        description: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rpc_line_simple() {
        let method = parse_rpc_line("rpc GetUser(GetUserRequest) returns (User);", "mypackage.UserService");
        let method = method.unwrap();

        assert_eq!(method.name, "GetUser");
        assert_eq!(method.input_type, "GetUserRequest");
        assert_eq!(method.output_type, "User");
        assert!(!method.client_streaming);
        assert!(!method.server_streaming);
    }

    #[test]
    fn test_parse_rpc_line_streaming() {
        let method = parse_rpc_line("rpc StreamData(stream Request) returns (stream Response);", "Test");
        let method = method.unwrap();

        assert!(method.client_streaming);
        assert!(method.server_streaming);
    }

    #[test]
    fn test_parse_proto_file() {
        let proto = r#"
            syntax = "proto3";

            package mypackage;

            service UserService {
                rpc GetUser(GetUserRequest) returns (User);
                rpc ListUsers(ListUsersRequest) returns (stream User);
            }
        "#;

        let services = parse_proto_file(proto).unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "UserService");
        assert_eq!(services[0].methods.len(), 2);
    }

    #[test]
    fn test_service_descriptor_format() {
        let service = ServiceDescriptor {
            name: "UserService".to_string(),
            full_name: "mypackage.UserService".to_string(),
            methods: vec![
                MethodDescriptor {
                    name: "GetUser".to_string(),
                    full_name: "mypackage.UserService.GetUser".to_string(),
                    input_type: "GetUserRequest".to_string(),
                    output_type: "User".to_string(),
                    client_streaming: false,
                    server_streaming: false,
                    description: None,
                }
            ],
            description: None,
        };

        let output = service.format_display();
        assert!(output.contains("service UserService"));
        assert!(output.contains("rpc GetUser"));
    }
}
