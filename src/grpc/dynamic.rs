//! Dynamic gRPC client using prost-reflect
//!
//! This module enables making gRPC calls using JSON payloads, with automatic
//! conversion to/from protobuf wire format using prost-reflect for proper
//! schema-based encoding/decoding.

use bytes::{Bytes, Buf, BufMut};
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor, FieldDescriptor, Kind, Value, ReflectMessage};
use serde_json::{Value as JsonValue, Map as JsonMap, Number as JsonNumber};
use std::path::Path;
use std::sync::Arc;

use crate::errors::QuicpulseError;

/// Schema holder for gRPC reflection-based calls
#[derive(Debug, Clone)]
pub struct GrpcSchema {
    pool: Arc<DescriptorPool>,
}

impl GrpcSchema {
    /// Create a new schema from a .proto file
    pub fn from_proto_file(path: &Path) -> Result<Self, QuicpulseError> {
        // Get the directory containing the proto file for imports
        let include_dir = path.parent().unwrap_or(Path::new("."));

        // Compile the proto file using protox
        let file_descriptor_set = protox::compile([path], [include_dir])
            .map_err(|e| QuicpulseError::Parse(format!("Failed to compile proto file: {}", e)))?;

        // Create descriptor pool
        let pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set)
            .map_err(|e| QuicpulseError::Parse(format!("Failed to create descriptor pool: {}", e)))?;

        Ok(Self { pool: Arc::new(pool) })
    }

    /// Create a new schema from proto content string
    pub fn from_proto_content(content: &str, filename: &str) -> Result<Self, QuicpulseError> {
        let unique_id = uuid::Uuid::new_v4();
        let unique_filename = format!("{}_{}", unique_id, filename);
        
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(&unique_filename);
        std::fs::write(&temp_path, content)
            .map_err(|e| QuicpulseError::Io(e))?;

        let result = Self::from_proto_file(&temp_path);
        let _ = std::fs::remove_file(&temp_path);
        result
    }

    /// Create from a file descriptor set (binary compiled proto)
    pub fn from_file_descriptor_set(fds: prost_types::FileDescriptorSet) -> Result<Self, QuicpulseError> {
        let pool = DescriptorPool::from_file_descriptor_set(fds)
            .map_err(|e| QuicpulseError::Parse(format!("Failed to create descriptor pool: {}", e)))?;

        Ok(Self { pool: Arc::new(pool) })
    }

    /// Get the descriptor pool
    pub fn pool(&self) -> &DescriptorPool {
        &self.pool
    }

    /// List all services in the schema
    pub fn list_services(&self) -> Vec<String> {
        self.pool.services()
            .map(|s| s.full_name().to_string())
            .collect()
    }

    /// List methods for a service
    pub fn list_methods(&self, service_name: &str) -> Vec<String> {
        if let Some(service) = self.pool.get_service_by_name(service_name) {
            service.methods()
                .map(|m| m.name().to_string())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get the input message descriptor for a method
    pub fn get_method_input(&self, service_name: &str, method_name: &str) -> Option<MessageDescriptor> {
        let service = self.pool.get_service_by_name(service_name)?;
        let method = service.methods().find(|m| m.name() == method_name)?;
        Some(method.input())
    }

    /// Get the output message descriptor for a method
    pub fn get_method_output(&self, service_name: &str, method_name: &str) -> Option<MessageDescriptor> {
        let service = self.pool.get_service_by_name(service_name)?;
        let method = service.methods().find(|m| m.name() == method_name)?;
        Some(method.output())
    }

    /// Get a message descriptor by full name
    pub fn get_message(&self, name: &str) -> Option<MessageDescriptor> {
        self.pool.get_message_by_name(name)
    }

    /// Encode a JSON value to protobuf bytes using the schema
    pub fn encode_message(&self, message_name: &str, json: &JsonValue) -> Result<Bytes, QuicpulseError> {
        let descriptor = self.get_message(message_name)
            .ok_or_else(|| QuicpulseError::Parse(format!("Message type not found: {}", message_name)))?;

        let msg = json_to_dynamic_message(&descriptor, json)?;
        let mut buf = Vec::new();
        msg.encode(&mut buf)
            .map_err(|e| QuicpulseError::Parse(format!("Failed to encode message: {}", e)))?;

        Ok(Bytes::from(buf))
    }

    /// Encode using method input type
    pub fn encode_request(&self, service: &str, method: &str, json: &JsonValue) -> Result<Bytes, QuicpulseError> {
        let descriptor = self.get_method_input(service, method)
            .ok_or_else(|| QuicpulseError::Parse(format!("Method not found: {}/{}", service, method)))?;

        let msg = json_to_dynamic_message(&descriptor, json)?;
        let mut buf = Vec::new();
        msg.encode(&mut buf)
            .map_err(|e| QuicpulseError::Parse(format!("Failed to encode request: {}", e)))?;

        Ok(Bytes::from(buf))
    }

    /// Decode protobuf bytes to JSON using the schema
    pub fn decode_message(&self, message_name: &str, data: &[u8]) -> Result<JsonValue, QuicpulseError> {
        let descriptor = self.get_message(message_name)
            .ok_or_else(|| QuicpulseError::Parse(format!("Message type not found: {}", message_name)))?;

        let msg = DynamicMessage::decode(descriptor, data)
            .map_err(|e| QuicpulseError::Parse(format!("Failed to decode message: {}", e)))?;

        Ok(dynamic_message_to_json(&msg))
    }

    /// Decode using method output type
    pub fn decode_response(&self, service: &str, method: &str, data: &[u8]) -> Result<JsonValue, QuicpulseError> {
        let descriptor = self.get_method_output(service, method)
            .ok_or_else(|| QuicpulseError::Parse(format!("Method not found: {}/{}", service, method)))?;

        let msg = DynamicMessage::decode(descriptor, data)
            .map_err(|e| QuicpulseError::Parse(format!("Failed to decode response: {}", e)))?;

        Ok(dynamic_message_to_json(&msg))
    }

    /// Describe a service (for --grpc-describe)
    pub fn describe_service(&self, service_name: &str) -> Option<String> {
        let service = self.pool.get_service_by_name(service_name)?;
        let mut output = format!("service {} {{\n", service.name());

        for method in service.methods() {
            let input_msg = method.input();
            let output_msg = method.output();
            let input = input_msg.full_name().to_string();
            let output_type = output_msg.full_name().to_string();
            let stream_prefix = if method.is_client_streaming() { "stream " } else { "" };
            let stream_suffix = if method.is_server_streaming() { "stream " } else { "" };

            output.push_str(&format!(
                "  rpc {}({}{}) returns ({}{});\n",
                method.name(),
                stream_prefix, input,
                stream_suffix, output_type
            ));
        }

        output.push_str("}\n");
        Some(output)
    }

    /// Describe a message type
    pub fn describe_message(&self, message_name: &str) -> Option<String> {
        let msg = self.pool.get_message_by_name(message_name)?;
        Some(format_message_descriptor(&msg, 0))
    }

    /// Get method streaming info
    pub fn get_method_info(&self, service_name: &str, method_name: &str) -> Option<MethodInfo> {
        let service = self.pool.get_service_by_name(service_name)?;
        let method = service.methods().find(|m| m.name() == method_name)?;
        Some(MethodInfo {
            name: method.name().to_string(),
            client_streaming: method.is_client_streaming(),
            server_streaming: method.is_server_streaming(),
        })
    }
}

/// Information about a gRPC method
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
}

impl MethodInfo {
    /// Check if this is a unary call (no streaming)
    pub fn is_unary(&self) -> bool {
        !self.client_streaming && !self.server_streaming
    }

    /// Check if this is server streaming only
    pub fn is_server_streaming(&self) -> bool {
        !self.client_streaming && self.server_streaming
    }

    /// Check if this is client streaming only
    pub fn is_client_streaming(&self) -> bool {
        self.client_streaming && !self.server_streaming
    }

    /// Check if this is bidirectional streaming
    pub fn is_bidi_streaming(&self) -> bool {
        self.client_streaming && self.server_streaming
    }
}

/// Format a message descriptor as proto-like syntax
fn format_message_descriptor(msg: &MessageDescriptor, indent: usize) -> String {
    let indent_str = "  ".repeat(indent);
    let mut output = format!("{}message {} {{\n", indent_str, msg.name());

    for field in msg.fields() {
        let field_indent = "  ".repeat(indent + 1);
        let type_name = format_field_type(&field);
        let repeated = if field.is_list() { "repeated " } else { "" };

        output.push_str(&format!(
            "{}{}{} {} = {};\n",
            field_indent, repeated, type_name, field.name(), field.number()
        ));
    }

    // Nested messages
    for nested in msg.child_messages() {
        output.push_str(&format_message_descriptor(&nested, indent + 1));
    }

    output.push_str(&format!("{}}}\n", indent_str));
    output
}

/// Format a field type
fn format_field_type(field: &FieldDescriptor) -> String {
    match field.kind() {
        Kind::Double => "double".to_string(),
        Kind::Float => "float".to_string(),
        Kind::Int64 => "int64".to_string(),
        Kind::Uint64 => "uint64".to_string(),
        Kind::Int32 => "int32".to_string(),
        Kind::Fixed64 => "fixed64".to_string(),
        Kind::Fixed32 => "fixed32".to_string(),
        Kind::Bool => "bool".to_string(),
        Kind::String => "string".to_string(),
        Kind::Bytes => "bytes".to_string(),
        Kind::Uint32 => "uint32".to_string(),
        Kind::Sfixed32 => "sfixed32".to_string(),
        Kind::Sfixed64 => "sfixed64".to_string(),
        Kind::Sint32 => "sint32".to_string(),
        Kind::Sint64 => "sint64".to_string(),
        Kind::Message(m) => m.full_name().to_string(),
        Kind::Enum(e) => e.full_name().to_string(),
    }
}

/// Convert JSON to a DynamicMessage
/// Bug #9 fix: Now tracks depth to prevent stack overflow from deeply nested messages
fn json_to_dynamic_message(descriptor: &MessageDescriptor, json: &JsonValue) -> Result<DynamicMessage, QuicpulseError> {
    json_to_dynamic_message_with_depth(descriptor, json, 0)
}

/// Internal function with depth tracking to prevent stack overflow
fn json_to_dynamic_message_with_depth(
    descriptor: &MessageDescriptor,
    json: &JsonValue,
    depth: usize,
) -> Result<DynamicMessage, QuicpulseError> {
    if depth > MAX_ENCODE_DEPTH {
        return Err(QuicpulseError::Parse(format!(
            "Maximum recursion depth ({}) exceeded in protobuf encoding. \
            The message is too deeply nested.",
            MAX_ENCODE_DEPTH
        )));
    }

    let mut msg = DynamicMessage::new(descriptor.clone());

    if let JsonValue::Object(map) = json {
        for (key, value) in map {
            if let Some(field) = descriptor.get_field_by_name(key) {
                let proto_value = json_to_proto_value_with_depth(&field, value, depth)?;
                msg.set_field(&field, proto_value);
            }
            // Silently ignore unknown fields
        }
    }

    Ok(msg)
}

/// Convert a JSON value to a prost-reflect Value
fn json_to_proto_value(field: &FieldDescriptor, json: &JsonValue) -> Result<Value, QuicpulseError> {
    json_to_proto_value_with_depth(field, json, 0)
}

/// Bug #9 fix: Convert a JSON value to a prost-reflect Value with depth tracking
fn json_to_proto_value_with_depth(
    field: &FieldDescriptor,
    json: &JsonValue,
    depth: usize,
) -> Result<Value, QuicpulseError> {
    // Handle repeated fields
    if field.is_list() {
        if let JsonValue::Array(arr) = json {
            let values: Result<Vec<Value>, _> = arr.iter()
                .map(|v| json_to_proto_scalar_with_depth(field, v, depth))
                .collect();
            return Ok(Value::List(values?));
        } else {
            // Single value for repeated field - wrap in list
            let value = json_to_proto_scalar_with_depth(field, json, depth)?;
            return Ok(Value::List(vec![value]));
        }
    }

    // Handle map fields
    if field.is_map() {
        if let JsonValue::Object(obj) = json {
            let map_entry = field.kind();
            if let Kind::Message(entry_desc) = map_entry {
                let key_field = entry_desc.get_field_by_name("key")
                    .ok_or_else(|| QuicpulseError::Parse("Invalid map entry: missing key field".to_string()))?;
                let value_field = entry_desc.get_field_by_name("value")
                    .ok_or_else(|| QuicpulseError::Parse("Invalid map entry: missing value field".to_string()))?;

                let entries: Result<Vec<(prost_reflect::MapKey, Value)>, QuicpulseError> = obj.iter()
                    .map(|(k, v)| {
                        let key = json_string_to_map_key(&key_field, k)?;
                        let value = json_to_proto_scalar_with_depth(&value_field, v, depth)?;
                        Ok((key, value))
                    })
                    .collect();

                return Ok(Value::Map(entries?.into_iter().collect()));
            }
        }
    }

    json_to_proto_scalar_with_depth(field, json, depth)
}

/// Convert a JSON string to a map key
fn json_string_to_map_key(field: &FieldDescriptor, s: &str) -> Result<prost_reflect::MapKey, QuicpulseError> {
    match field.kind() {
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => {
            let v: i32 = s.parse().map_err(|_| QuicpulseError::Parse(format!("Invalid int32 map key: {}", s)))?;
            Ok(prost_reflect::MapKey::I32(v))
        }
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => {
            let v: i64 = s.parse().map_err(|_| QuicpulseError::Parse(format!("Invalid int64 map key: {}", s)))?;
            Ok(prost_reflect::MapKey::I64(v))
        }
        Kind::Uint32 | Kind::Fixed32 => {
            let v: u32 = s.parse().map_err(|_| QuicpulseError::Parse(format!("Invalid uint32 map key: {}", s)))?;
            Ok(prost_reflect::MapKey::U32(v))
        }
        Kind::Uint64 | Kind::Fixed64 => {
            let v: u64 = s.parse().map_err(|_| QuicpulseError::Parse(format!("Invalid uint64 map key: {}", s)))?;
            Ok(prost_reflect::MapKey::U64(v))
        }
        Kind::Bool => {
            let v: bool = s.parse().map_err(|_| QuicpulseError::Parse(format!("Invalid bool map key: {}", s)))?;
            Ok(prost_reflect::MapKey::Bool(v))
        }
        Kind::String => Ok(prost_reflect::MapKey::String(s.to_string())),
        _ => Err(QuicpulseError::Parse(format!("Unsupported map key type: {:?}", field.kind()))),
    }
}

/// Convert a JSON value to a scalar proto value
fn json_to_proto_scalar(field: &FieldDescriptor, json: &JsonValue) -> Result<Value, QuicpulseError> {
    json_to_proto_scalar_with_depth(field, json, 0)
}

/// Bug #9 fix: Convert a JSON value to a scalar proto value with depth tracking
/// This is the key function that recurses for nested messages
fn json_to_proto_scalar_with_depth(
    field: &FieldDescriptor,
    json: &JsonValue,
    depth: usize,
) -> Result<Value, QuicpulseError> {
    match field.kind() {
        Kind::Double => {
            let v = json_to_f64(json)?;
            Ok(Value::F64(v))
        }
        Kind::Float => {
            let v = json_to_f64(json)? as f32;
            Ok(Value::F32(v))
        }
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => {
            let v = json_to_i64(json)?;
            Ok(Value::I64(v))
        }
        Kind::Uint64 | Kind::Fixed64 => {
            let v = json_to_u64(json)?;
            Ok(Value::U64(v))
        }
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => {
            let v = json_to_i64(json)? as i32;
            Ok(Value::I32(v))
        }
        Kind::Uint32 | Kind::Fixed32 => {
            let v = json_to_u64(json)? as u32;
            Ok(Value::U32(v))
        }
        Kind::Bool => {
            let v = json_to_bool(json)?;
            Ok(Value::Bool(v))
        }
        Kind::String => {
            let v = json_to_string(json)?;
            Ok(Value::String(v))
        }
        Kind::Bytes => {
            let v = json_to_bytes(json)?;
            Ok(Value::Bytes(v.into()))
        }
        Kind::Message(msg_desc) => {
            // Bug #9 fix: Increment depth when recursing into nested message
            let msg = json_to_dynamic_message_with_depth(&msg_desc, json, depth + 1)?;
            Ok(Value::Message(msg))
        }
        Kind::Enum(enum_desc) => {
            let v = match json {
                JsonValue::String(s) => {
                    enum_desc.get_value_by_name(s)
                        .map(|e| e.number())
                        .unwrap_or(0)
                }
                JsonValue::Number(n) => n.as_i64().unwrap_or(0) as i32,
                _ => 0,
            };
            Ok(Value::EnumNumber(v))
        }
    }
}

fn json_to_f64(json: &JsonValue) -> Result<f64, QuicpulseError> {
    match json {
        JsonValue::Number(n) => Ok(n.as_f64().unwrap_or(0.0)),
        JsonValue::String(s) => s.parse().map_err(|_| QuicpulseError::Parse(format!("Invalid number: {}", s))),
        _ => Ok(0.0),
    }
}

fn json_to_i64(json: &JsonValue) -> Result<i64, QuicpulseError> {
    match json {
        JsonValue::Number(n) => Ok(n.as_i64().unwrap_or(0)),
        JsonValue::String(s) => s.parse().map_err(|_| QuicpulseError::Parse(format!("Invalid integer: {}", s))),
        _ => Ok(0),
    }
}

fn json_to_u64(json: &JsonValue) -> Result<u64, QuicpulseError> {
    match json {
        JsonValue::Number(n) => Ok(n.as_u64().unwrap_or(0)),
        JsonValue::String(s) => s.parse().map_err(|_| QuicpulseError::Parse(format!("Invalid unsigned integer: {}", s))),
        _ => Ok(0),
    }
}

fn json_to_bool(json: &JsonValue) -> Result<bool, QuicpulseError> {
    match json {
        JsonValue::Bool(b) => Ok(*b),
        JsonValue::String(s) => Ok(s == "true" || s == "1"),
        JsonValue::Number(n) => Ok(n.as_i64().unwrap_or(0) != 0),
        _ => Ok(false),
    }
}

fn json_to_string(json: &JsonValue) -> Result<String, QuicpulseError> {
    match json {
        JsonValue::String(s) => Ok(s.clone()),
        JsonValue::Number(n) => Ok(n.to_string()),
        JsonValue::Bool(b) => Ok(b.to_string()),
        JsonValue::Null => Ok(String::new()),
        _ => Ok(json.to_string()),
    }
}

fn json_to_bytes(json: &JsonValue) -> Result<Vec<u8>, QuicpulseError> {
    match json {
        JsonValue::String(s) => {
            // For protobuf bytes fields, strings should be base64 encoded
            // Don't silently fall back to UTF-8 as that hides encoding errors
            if s.is_empty() {
                return Ok(Vec::new());
            }

            // Check if string looks like base64 (contains only base64 chars)
            let is_likely_base64 = s.chars().all(|c| {
                c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' || c.is_whitespace()
            });

            if is_likely_base64 {
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s)
                    .map_err(|e| QuicpulseError::Parse(format!(
                        "Invalid base64 for bytes field: {}. \
                        Bytes fields require base64-encoded strings or array of numbers.",
                        e
                    )))
            } else {
                // String contains non-base64 characters, likely intended as raw text
                // Return an error to inform the user about the expected format
                Err(QuicpulseError::Parse(format!(
                    "Invalid format for bytes field. Expected base64-encoded string or array of numbers, got: '{}'...",
                    s.chars().take(50).collect::<String>()
                )))
            }
        }
        JsonValue::Array(arr) => {
            let bytes: Result<Vec<u8>, _> = arr.iter()
                .map(|v| v.as_u64().map(|n| n as u8).ok_or_else(|| QuicpulseError::Parse("Invalid byte value".to_string())))
                .collect();
            bytes
        }
        _ => Ok(Vec::new()),
    }
}

/// Convert a DynamicMessage to JSON
fn dynamic_message_to_json(msg: &DynamicMessage) -> JsonValue {
    let mut map = JsonMap::new();

    for field in msg.descriptor().fields() {
        if msg.has_field(&field) {
            let value = msg.get_field(&field);
            let json_value = proto_value_to_json(&value);
            map.insert(field.name().to_string(), json_value);
        }
    }

    JsonValue::Object(map)
}

/// Convert a prost-reflect Value to JSON
fn proto_value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Bool(b) => JsonValue::Bool(*b),
        Value::I32(n) => JsonValue::Number(JsonNumber::from(*n)),
        Value::I64(n) => JsonValue::Number(JsonNumber::from(*n)),
        Value::U32(n) => JsonValue::Number(JsonNumber::from(*n)),
        Value::U64(n) => JsonValue::Number(JsonNumber::from(*n)),
        Value::F32(f) => {
            JsonNumber::from_f64(*f as f64)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null)
        }
        Value::F64(f) => {
            JsonNumber::from_f64(*f)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null)
        }
        Value::String(s) => JsonValue::String(s.clone()),
        Value::Bytes(b) => {
            JsonValue::String(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b))
        }
        Value::EnumNumber(n) => JsonValue::Number(JsonNumber::from(*n)),
        Value::Message(m) => dynamic_message_to_json(m),
        Value::List(list) => {
            JsonValue::Array(list.iter().map(proto_value_to_json).collect())
        }
        Value::Map(map) => {
            let mut obj = JsonMap::new();
            for (k, v) in map {
                let key = match k {
                    prost_reflect::MapKey::Bool(b) => b.to_string(),
                    prost_reflect::MapKey::I32(n) => n.to_string(),
                    prost_reflect::MapKey::I64(n) => n.to_string(),
                    prost_reflect::MapKey::U32(n) => n.to_string(),
                    prost_reflect::MapKey::U64(n) => n.to_string(),
                    prost_reflect::MapKey::String(s) => s.clone(),
                };
                obj.insert(key, proto_value_to_json(v));
            }
            JsonValue::Object(obj)
        }
    }
}

/// Bug #9 fix: Maximum recursion depth for encoding/decoding to prevent stack overflow
/// Applies to both encoder and decoder to prevent DoS from deeply nested messages
const MAX_ENCODE_DEPTH: usize = 50;
const MAX_DECODE_DEPTH: usize = 50;

/// Decode protobuf wire format to JSON without schema (best-effort)
/// This is used when no schema is available
pub fn decode_to_json_schemaless(data: &[u8]) -> Result<JsonValue, QuicpulseError> {
    decode_to_json_schemaless_with_depth(data, 0)
}

/// Internal function with depth tracking to prevent stack overflow
fn decode_to_json_schemaless_with_depth(data: &[u8], depth: usize) -> Result<JsonValue, QuicpulseError> {
    if depth > MAX_DECODE_DEPTH {
        return Err(QuicpulseError::Parse(format!(
            "Maximum recursion depth ({}) exceeded in protobuf decoding",
            MAX_DECODE_DEPTH
        )));
    }

    if data.is_empty() {
        return Ok(JsonValue::Object(JsonMap::new()));
    }

    // Use our wire decoder for schemaless decoding
    let mut result = JsonMap::new();
    let mut pos = 0;

    while pos < data.len() {
        // Read tag (varint)
        let (tag, bytes_read) = read_varint(&data[pos..])?;
        pos += bytes_read;

        let field_num = (tag >> 3) as u32;
        let wire_type = (tag & 0x7) as u8;
        let field_name = format!("field_{}", field_num);

        let value = match wire_type {
            0 => { // Varint
                let (v, bytes_read) = read_varint(&data[pos..])?;
                pos += bytes_read;
                if v <= i64::MAX as u64 {
                    JsonValue::Number(JsonNumber::from(v as i64))
                } else {
                    JsonValue::String(v.to_string())
                }
            }
            1 => { // Fixed64
                if pos + 8 > data.len() {
                    return Err(QuicpulseError::Parse("Unexpected end of data".to_string()));
                }
                let v = u64::from_le_bytes(data[pos..pos+8].try_into().unwrap());
                pos += 8;
                JsonValue::Number(JsonNumber::from(v))
            }
            2 => { // Length-delimited
                let (len, bytes_read) = read_varint(&data[pos..])?;
                pos += bytes_read;
                let len = len as usize;
                if pos + len > data.len() {
                    return Err(QuicpulseError::Parse("Unexpected end of data".to_string()));
                }
                let bytes = &data[pos..pos+len];
                pos += len;

                // Try to interpret as string
                if let Ok(s) = std::str::from_utf8(bytes) {
                    if s.chars().all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t') {
                        JsonValue::String(s.to_string())
                    } else {
                        // Try nested message with depth limit
                        decode_to_json_schemaless_with_depth(bytes, depth + 1).unwrap_or_else(|_| {
                            JsonValue::String(base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                bytes
                            ))
                        })
                    }
                } else {
                    // Try nested message with depth limit
                    decode_to_json_schemaless_with_depth(bytes, depth + 1).unwrap_or_else(|_| {
                        JsonValue::String(base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            bytes
                        ))
                    })
                }
            }
            5 => { // Fixed32
                if pos + 4 > data.len() {
                    return Err(QuicpulseError::Parse("Unexpected end of data".to_string()));
                }
                let v = u32::from_le_bytes(data[pos..pos+4].try_into().unwrap());
                pos += 4;
                JsonValue::Number(JsonNumber::from(v))
            }
            _ => {
                // Skip unknown wire types
                continue;
            }
        };

        // Handle repeated fields
        if let Some(existing) = result.get_mut(&field_name) {
            if let JsonValue::Array(arr) = existing {
                arr.push(value);
            } else {
                let old = existing.clone();
                *existing = JsonValue::Array(vec![old, value]);
            }
        } else {
            result.insert(field_name, value);
        }
    }

    Ok(JsonValue::Object(result))
}

/// Read a varint from bytes
fn read_varint(data: &[u8]) -> Result<(u64, usize), QuicpulseError> {
    let mut result: u64 = 0;
    let mut shift = 0;
    let mut pos = 0;

    loop {
        if pos >= data.len() {
            return Err(QuicpulseError::Parse("Unexpected end of varint".to_string()));
        }

        let byte = data[pos];
        pos += 1;

        result |= ((byte & 0x7F) as u64) << shift;

        if byte & 0x80 == 0 {
            break;
        }

        shift += 7;
        if shift >= 64 {
            return Err(QuicpulseError::Parse("Varint too long".to_string()));
        }
    }

    Ok((result, pos))
}

/// Raw message wrapper for untyped gRPC calls
#[derive(Debug, Clone)]
pub struct RawMessage(pub Bytes);

/// Raw codec that passes bytes through without transformation
#[derive(Debug, Clone, Copy, Default)]
pub struct RawCodec;

impl tonic::codec::Codec for RawCodec {
    type Encode = RawMessage;
    type Decode = RawMessage;
    type Encoder = RawEncoder;
    type Decoder = RawDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        RawEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        RawDecoder
    }
}

/// Encoder for raw bytes
#[derive(Debug, Clone, Copy, Default)]
pub struct RawEncoder;

impl tonic::codec::Encoder for RawEncoder {
    type Item = RawMessage;
    type Error = tonic::Status;

    fn encode(&mut self, item: Self::Item, dst: &mut tonic::codec::EncodeBuf<'_>) -> Result<(), Self::Error> {
        dst.put_slice(&item.0);
        Ok(())
    }
}

/// Decoder for raw bytes
#[derive(Debug, Clone, Copy, Default)]
pub struct RawDecoder;

impl tonic::codec::Decoder for RawDecoder {
    type Item = RawMessage;
    type Error = tonic::Status;

    fn decode(&mut self, src: &mut tonic::codec::DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        let len = src.remaining();
        if len == 0 {
            return Ok(Some(RawMessage(Bytes::new())));
        }
        let bytes = src.copy_to_bytes(len);
        Ok(Some(RawMessage(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_schemaless_decode() {
        // Create a simple protobuf message manually
        // field 1 = string "hello", field 2 = int 42
        let mut data = Vec::new();
        // Field 1, wire type 2 (length-delimited) = (1 << 3) | 2 = 10
        data.push(10);
        data.push(5); // length
        data.extend_from_slice(b"hello");
        // Field 2, wire type 0 (varint) = (2 << 3) | 0 = 16
        data.push(16);
        data.push(42); // value

        let json = decode_to_json_schemaless(&data).unwrap();
        assert!(json.is_object());
        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("field_1").unwrap().as_str().unwrap(), "hello");
        assert_eq!(obj.get("field_2").unwrap().as_i64().unwrap(), 42);
    }

    #[test]
    fn test_prost_reflect_encode_decode() {
        // Create a temp proto file
        let proto_content = r#"
            syntax = "proto3";
            package test;

            message HelloRequest {
                string name = 1;
                int32 count = 2;
            }

            message HelloResponse {
                string message = 1;
            }

            service Greeter {
                rpc SayHello (HelloRequest) returns (HelloResponse);
            }
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(proto_content.as_bytes()).unwrap();

        let schema = GrpcSchema::from_proto_file(temp_file.path()).unwrap();

        // Test encode
        let request_json = serde_json::json!({
            "name": "World",
            "count": 42
        });

        let encoded = schema.encode_request("test.Greeter", "SayHello", &request_json).unwrap();
        assert!(!encoded.is_empty());

        // Decode it back (as the input type since we're testing roundtrip)
        let decoded = schema.decode_message("test.HelloRequest", &encoded).unwrap();
        assert_eq!(decoded["name"], "World");
        assert_eq!(decoded["count"], 42);
    }

    #[test]
    fn test_list_services() {
        let proto_content = r#"
            syntax = "proto3";
            package myapp;

            message Request {}
            message Response {}

            service ServiceA {
                rpc MethodOne (Request) returns (Response);
            }

            service ServiceB {
                rpc MethodTwo (Request) returns (Response);
            }
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(proto_content.as_bytes()).unwrap();

        let schema = GrpcSchema::from_proto_file(temp_file.path()).unwrap();
        let services = schema.list_services();

        assert!(services.contains(&"myapp.ServiceA".to_string()));
        assert!(services.contains(&"myapp.ServiceB".to_string()));
    }

    #[test]
    fn test_list_methods() {
        let proto_content = r#"
            syntax = "proto3";
            package myapp;

            message Request {}
            message Response {}

            service TestService {
                rpc GetItem (Request) returns (Response);
                rpc CreateItem (Request) returns (Response);
            }
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(proto_content.as_bytes()).unwrap();

        let schema = GrpcSchema::from_proto_file(temp_file.path()).unwrap();
        let methods = schema.list_methods("myapp.TestService");

        assert!(methods.contains(&"GetItem".to_string()));
        assert!(methods.contains(&"CreateItem".to_string()));
    }
}
