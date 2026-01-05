//! Proto file parser for extracting message schemas
//!
//! Parses .proto files to extract message definitions with field numbers,
//! enabling correct JSON to protobuf encoding.
//!
//! Uses prost-reflect and protox for proper protobuf compilation when possible,
//! with fallback to regex-based parsing for simple cases.

use std::collections::HashMap;
use std::path::Path;
use std::fs;
use once_cell::sync::Lazy;
use regex::Regex;
use crate::errors::QuicpulseError;
use super::dynamic::GrpcSchema;

// SIMD-optimized cached regexes for proto parsing
// These are compiled once and reused for all parsing operations

/// Package declaration regex
static PACKAGE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"package\s+([a-zA-Z_][a-zA-Z0-9_.]*)\s*;").expect("Invalid package regex")
});

/// Import statement regex
static IMPORT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"import\s+(?:public\s+|weak\s+)?"([^"]+)"\s*;"#).expect("Invalid import regex")
});

/// Enum definition regex
static ENUM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"enum\s+(\w+)\s*\{([^}]*)\}").expect("Invalid enum regex")
});

/// Enum value regex
static ENUM_VALUE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\w+)\s*=\s*(-?\d+)").expect("Invalid enum value regex")
});

/// Field definition regex
static FIELD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?:(?P<modifier>optional|repeated|required)\s+)?(?:map\s*<\s*(?P<map_key>\w+)\s*,\s*(?P<map_value>[\w.]+)\s*>|(?P<type>[\w.]+))\s+(?P<name>\w+)\s*=\s*(?P<number>\d+)"
    ).expect("Invalid field regex")
});

/// Service definition regex
static SERVICE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"service\s+(\w+)\s*\{([^}]*)\}").expect("Invalid service regex")
});

/// RPC method regex
static RPC_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"rpc\s+(\w+)\s*\(\s*(stream\s+)?([^)]+)\s*\)\s+returns\s*\(\s*(stream\s+)?([^)]+)\s*\)"
    ).expect("Invalid rpc regex")
});

/// A parsed proto schema containing all message and service definitions
#[derive(Debug, Clone, Default)]
pub struct ProtoSchema {
    /// Package name
    pub package: String,
    /// Message definitions by full name
    pub messages: HashMap<String, ProtoMessage>,
    /// Enum definitions by full name
    pub enums: HashMap<String, ProtoEnum>,
    /// Service definitions
    pub services: Vec<ProtoService>,
    /// Import paths
    pub imports: Vec<String>,
    /// Compiled schema using prost-reflect (when available)
    grpc_schema: Option<GrpcSchema>,
}

/// A message definition
#[derive(Debug, Clone)]
pub struct ProtoMessage {
    /// Short name
    pub name: String,
    /// Full name including package
    pub full_name: String,
    /// Fields by name
    pub fields: HashMap<String, ProtoField>,
    /// Nested message types
    pub nested_messages: Vec<String>,
    /// Nested enum types
    pub nested_enums: Vec<String>,
}

/// A field in a message
#[derive(Debug, Clone)]
pub struct ProtoField {
    /// Field name
    pub name: String,
    /// Field number (tag)
    pub number: u32,
    /// Field type (e.g., "string", "int32", "MyMessage")
    pub field_type: String,
    /// Is this a repeated field
    pub repeated: bool,
    /// Is this a map field
    pub map: bool,
    /// Map key type (if map)
    pub map_key_type: Option<String>,
    /// Map value type (if map)
    pub map_value_type: Option<String>,
    /// Is optional (proto3 optional keyword)
    pub optional: bool,
}

/// An enum definition
#[derive(Debug, Clone)]
pub struct ProtoEnum {
    /// Enum name
    pub name: String,
    /// Full name including package
    pub full_name: String,
    /// Values: name -> number
    pub values: HashMap<String, i32>,
}

/// A service definition
#[derive(Debug, Clone)]
pub struct ProtoService {
    /// Service name
    pub name: String,
    /// Full name including package
    pub full_name: String,
    /// RPC methods
    pub methods: Vec<ProtoMethod>,
}

/// An RPC method
#[derive(Debug, Clone)]
pub struct ProtoMethod {
    /// Method name
    pub name: String,
    /// Input message type
    pub input_type: String,
    /// Output message type
    pub output_type: String,
    /// Is client streaming
    pub client_streaming: bool,
    /// Is server streaming
    pub server_streaming: bool,
}

impl ProtoSchema {
    /// Parse a proto file from a path
    /// Tries prost-reflect/protox first for proper compilation, falls back to regex parsing
    pub fn from_file(path: &Path) -> Result<Self, QuicpulseError> {
        let content = fs::read_to_string(path)
            .map_err(|e| QuicpulseError::Io(e))?;

        // Try to compile with protox first (proper protobuf parsing)
        let grpc_schema = GrpcSchema::from_proto_file(path).ok();

        // Also do regex parsing for backward compatibility / additional info
        let mut schema = Self::parse_content(&content)?;
        schema.grpc_schema = grpc_schema;

        Ok(schema)
    }

    /// Parse proto content from a string
    pub fn parse(content: &str) -> Result<Self, QuicpulseError> {
        // Try to compile with protox
        let grpc_schema = GrpcSchema::from_proto_content(content, "input.proto").ok();

        let mut schema = Self::parse_content(content)?;
        schema.grpc_schema = grpc_schema;

        Ok(schema)
    }

    /// Internal: Parse proto content using regex
    fn parse_content(content: &str) -> Result<Self, QuicpulseError> {
        let mut schema = ProtoSchema::default();

        // Remove comments
        let content = remove_comments(content);

        // Parse package
        if let Some(pkg) = parse_package(&content) {
            schema.package = pkg;
        }

        // Parse imports
        schema.imports = parse_imports(&content);

        // Parse enums (before messages, as messages may reference them)
        schema.enums = parse_enums(&content, &schema.package);

        // Parse messages
        schema.messages = parse_messages(&content, &schema.package);

        // Parse services
        schema.services = parse_services(&content, &schema.package);

        Ok(schema)
    }

    /// Get the compiled GrpcSchema if available
    pub fn grpc_schema(&self) -> Option<&GrpcSchema> {
        self.grpc_schema.as_ref()
    }

    /// Check if we have a compiled schema
    pub fn has_grpc_schema(&self) -> bool {
        self.grpc_schema.is_some()
    }

    /// Set the compiled GrpcSchema (Bug #6 fix: for reflection-loaded schemas)
    pub fn set_grpc_schema(&mut self, schema: GrpcSchema) {
        self.grpc_schema = Some(schema);
    }

    /// Get field numbers for a message type
    pub fn get_field_numbers(&self, message_name: &str) -> Option<HashMap<String, u32>> {
        // Try exact match first
        if let Some(msg) = self.messages.get(message_name) {
            return Some(msg.fields.iter()
                .map(|(name, field)| (name.clone(), field.number))
                .collect());
        }

        // Try with package prefix
        let full_name = format!("{}.{}", self.package, message_name);
        if let Some(msg) = self.messages.get(&full_name) {
            return Some(msg.fields.iter()
                .map(|(name, field)| (name.clone(), field.number))
                .collect());
        }

        // Try partial match
        for (key, msg) in &self.messages {
            if key.ends_with(&format!(".{}", message_name)) || key == message_name {
                return Some(msg.fields.iter()
                    .map(|(name, field)| (name.clone(), field.number))
                    .collect());
            }
        }

        None
    }

    /// Get a message definition by name
    pub fn get_message(&self, name: &str) -> Option<&ProtoMessage> {
        // Try exact match
        if let Some(msg) = self.messages.get(name) {
            return Some(msg);
        }

        // Try with package prefix
        let full_name = format!("{}.{}", self.package, name);
        if let Some(msg) = self.messages.get(&full_name) {
            return Some(msg);
        }

        // Try partial match
        for (key, msg) in &self.messages {
            if key.ends_with(&format!(".{}", name)) {
                return Some(msg);
            }
        }

        None
    }

    /// Get input message type for a service method
    pub fn get_method_input_type(&self, service: &str, method: &str) -> Option<String> {
        for svc in &self.services {
            if svc.name == service || svc.full_name == service {
                for m in &svc.methods {
                    if m.name == method {
                        return Some(m.input_type.clone());
                    }
                }
            }
        }
        None
    }
}

/// Remove comments from proto content
fn remove_comments(content: &str) -> String {
    let mut result = String::new();
    let mut in_block_comment = false;

    for line in content.lines() {
        if in_block_comment {
            if let Some(idx) = line.find("*/") {
                in_block_comment = false;
                result.push_str(&line[idx + 2..]);
                result.push('\n');
            }
            continue;
        }

        // Check for block comment start
        if let Some(idx) = line.find("/*") {
            let before = &line[..idx];
            result.push_str(before);
            if let Some(end_idx) = line[idx..].find("*/") {
                result.push_str(&line[idx + end_idx + 2..]);
            } else {
                in_block_comment = true;
            }
            result.push('\n');
            continue;
        }

        // Remove line comments
        let line = if let Some(idx) = line.find("//") {
            &line[..idx]
        } else {
            line
        };

        result.push_str(line);
        result.push('\n');
    }

    result
}

/// Parse package declaration (uses cached SIMD-optimized regex)
fn parse_package(content: &str) -> Option<String> {
    PACKAGE_RE.captures(content)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

/// Parse import statements (uses cached SIMD-optimized regex)
fn parse_imports(content: &str) -> Vec<String> {
    IMPORT_RE.captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

/// Parse enum definitions (uses cached SIMD-optimized regex)
fn parse_enums(content: &str, package: &str) -> HashMap<String, ProtoEnum> {
    let mut enums = HashMap::new();

    for cap in ENUM_RE.captures_iter(content) {
        let name = cap.get(1).unwrap().as_str().to_string();
        let body = cap.get(2).unwrap().as_str();

        let mut values = HashMap::new();
        for val_cap in ENUM_VALUE_RE.captures_iter(body) {
            let val_name = val_cap.get(1).unwrap().as_str().to_string();
            let val_num: i32 = val_cap.get(2).unwrap().as_str().parse().unwrap_or(0);
            values.insert(val_name, val_num);
        }

        let full_name = if package.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", package, name)
        };

        enums.insert(full_name.clone(), ProtoEnum {
            name,
            full_name,
            values,
        });
    }

    enums
}

/// Parse message definitions
fn parse_messages(content: &str, package: &str) -> HashMap<String, ProtoMessage> {
    let mut messages = HashMap::new();
    parse_messages_recursive(content, package, "", &mut messages);
    messages
}

/// Recursively parse message definitions (handles nested messages)
fn parse_messages_recursive(
    content: &str,
    package: &str,
    parent: &str,
    messages: &mut HashMap<String, ProtoMessage>,
) {
    // Find all message definitions at this level
    let mut depth = 0;
    let mut msg_start: Option<(usize, String)> = None;
    let mut msg_body_start = 0;

    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Look for "message" keyword
        if depth == 0 && i + 7 < chars.len() {
            let slice: String = chars[i..i + 7].iter().collect();
            if slice == "message" && (i == 0 || !chars[i - 1].is_alphanumeric()) {
                // Find message name
                let mut j = i + 7;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                let name_start = j;
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                let name: String = chars[name_start..j].iter().collect();

                if !name.is_empty() {
                    msg_start = Some((i, name));
                }
            }
        }

        if chars[i] == '{' {
            if depth == 0 && msg_start.is_some() {
                msg_body_start = i + 1;
            }
            depth += 1;
        } else if chars[i] == '}' {
            depth -= 1;
            if depth == 0 {
                if let Some((_, ref name)) = msg_start {
                    let body: String = chars[msg_body_start..i].iter().collect();

                    let full_name = if parent.is_empty() {
                        if package.is_empty() {
                            name.clone()
                        } else {
                            format!("{}.{}", package, name)
                        }
                    } else {
                        format!("{}.{}", parent, name)
                    };

                    // Parse fields
                    let fields = parse_fields(&body);

                    // Create message
                    let msg = ProtoMessage {
                        name: name.clone(),
                        full_name: full_name.clone(),
                        fields,
                        nested_messages: Vec::new(),
                        nested_enums: Vec::new(),
                    };

                    messages.insert(full_name.clone(), msg);

                    // Parse nested messages
                    parse_messages_recursive(&body, package, &full_name, messages);

                    msg_start = None;
                }
            }
        }

        i += 1;
    }
}

/// Parse fields from a message body (uses cached SIMD-optimized regex)
fn parse_fields(body: &str) -> HashMap<String, ProtoField> {
    let mut fields = HashMap::new();

    // Match field definitions using cached regex
    // Patterns:
    // - optional TYPE NAME = NUMBER;
    // - repeated TYPE NAME = NUMBER;
    // - TYPE NAME = NUMBER;
    // - map<KEY, VALUE> NAME = NUMBER;
    for cap in FIELD_RE.captures_iter(body) {
        let name = cap.name("name").unwrap().as_str().to_string();
        let number: u32 = cap.name("number").unwrap().as_str().parse().unwrap_or(0);

        let modifier = cap.name("modifier").map(|m| m.as_str());
        let repeated = modifier == Some("repeated");
        let optional = modifier == Some("optional");

        let (field_type, map, map_key_type, map_value_type) = if let Some(map_key) = cap.name("map_key") {
            let key = map_key.as_str().to_string();
            let value = cap.name("map_value").unwrap().as_str().to_string();
            (format!("map<{},{}>", key, value), true, Some(key), Some(value))
        } else {
            let typ = cap.name("type").unwrap().as_str().to_string();
            (typ, false, None, None)
        };

        fields.insert(name.clone(), ProtoField {
            name,
            number,
            field_type,
            repeated,
            map,
            map_key_type,
            map_value_type,
            optional,
        });
    }

    fields
}

/// Parse service definitions (uses cached SIMD-optimized regex)
fn parse_services(content: &str, package: &str) -> Vec<ProtoService> {
    let mut services = Vec::new();

    for cap in SERVICE_RE.captures_iter(content) {
        let name = cap.get(1).unwrap().as_str().to_string();
        let body = cap.get(2).unwrap().as_str();

        let full_name = if package.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", package, name)
        };

        let mut methods = Vec::new();
        for rpc_cap in RPC_RE.captures_iter(body) {
            let method_name = rpc_cap.get(1).unwrap().as_str().to_string();
            let client_streaming = rpc_cap.get(2).is_some();
            let input_type = rpc_cap.get(3).unwrap().as_str().trim().to_string();
            let server_streaming = rpc_cap.get(4).is_some();
            let output_type = rpc_cap.get(5).unwrap().as_str().trim().to_string();

            methods.push(ProtoMethod {
                name: method_name,
                input_type,
                output_type,
                client_streaming,
                server_streaming,
            });
        }

        services.push(ProtoService {
            name,
            full_name,
            methods,
        });
    }

    services
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PROTO: &str = r#"
        syntax = "proto3";

        package mypackage;

        message User {
            string name = 1;
            int32 age = 2;
            string email = 3;
            repeated string tags = 4;
            Address address = 5;
        }

        message Address {
            string street = 1;
            string city = 2;
            string country = 3;
            int32 zip_code = 4;
        }

        message GetUserRequest {
            string user_id = 1;
        }

        message CreateUserRequest {
            string name = 1;
            string email = 2;
            optional int32 age = 3;
        }

        enum Status {
            UNKNOWN = 0;
            ACTIVE = 1;
            INACTIVE = 2;
        }

        service UserService {
            rpc GetUser(GetUserRequest) returns (User);
            rpc CreateUser(CreateUserRequest) returns (User);
            rpc ListUsers(ListUsersRequest) returns (stream User);
        }
    "#;

    #[test]
    fn test_parse_package() {
        let schema = ProtoSchema::parse(TEST_PROTO).unwrap();
        assert_eq!(schema.package, "mypackage");
    }

    #[test]
    fn test_parse_messages() {
        let schema = ProtoSchema::parse(TEST_PROTO).unwrap();

        // Should have parsed User, Address, GetUserRequest, CreateUserRequest
        assert!(schema.messages.contains_key("mypackage.User"));
        assert!(schema.messages.contains_key("mypackage.Address"));
        assert!(schema.messages.contains_key("mypackage.GetUserRequest"));
    }

    #[test]
    fn test_field_numbers() {
        let schema = ProtoSchema::parse(TEST_PROTO).unwrap();

        let user = schema.get_message("User").unwrap();
        assert_eq!(user.fields.get("name").unwrap().number, 1);
        assert_eq!(user.fields.get("age").unwrap().number, 2);
        assert_eq!(user.fields.get("email").unwrap().number, 3);
        assert_eq!(user.fields.get("tags").unwrap().number, 4);
        assert!(user.fields.get("tags").unwrap().repeated);
    }

    #[test]
    fn test_get_field_numbers() {
        let schema = ProtoSchema::parse(TEST_PROTO).unwrap();

        let field_nums = schema.get_field_numbers("CreateUserRequest").unwrap();
        assert_eq!(field_nums.get("name"), Some(&1));
        assert_eq!(field_nums.get("email"), Some(&2));
        assert_eq!(field_nums.get("age"), Some(&3));
    }

    #[test]
    fn test_parse_services() {
        let schema = ProtoSchema::parse(TEST_PROTO).unwrap();

        assert_eq!(schema.services.len(), 1);
        let svc = &schema.services[0];
        assert_eq!(svc.name, "UserService");
        assert_eq!(svc.methods.len(), 3);

        let get_user = &svc.methods[0];
        assert_eq!(get_user.name, "GetUser");
        assert_eq!(get_user.input_type, "GetUserRequest");
        assert_eq!(get_user.output_type, "User");
        assert!(!get_user.client_streaming);
        assert!(!get_user.server_streaming);

        let list_users = &svc.methods[2];
        assert_eq!(list_users.name, "ListUsers");
        assert!(list_users.server_streaming);
    }

    #[test]
    fn test_parse_enums() {
        let schema = ProtoSchema::parse(TEST_PROTO).unwrap();

        assert!(schema.enums.contains_key("mypackage.Status"));
        let status_enum = schema.enums.get("mypackage.Status").unwrap();
        assert_eq!(status_enum.values.get("UNKNOWN"), Some(&0));
        assert_eq!(status_enum.values.get("ACTIVE"), Some(&1));
        assert_eq!(status_enum.values.get("INACTIVE"), Some(&2));
    }

    #[test]
    fn test_optional_field() {
        let schema = ProtoSchema::parse(TEST_PROTO).unwrap();

        let msg = schema.get_message("CreateUserRequest").unwrap();
        assert!(msg.fields.get("age").unwrap().optional);
        assert!(!msg.fields.get("name").unwrap().optional);
    }

    #[test]
    fn test_map_field() {
        let proto = r#"
            syntax = "proto3";
            message Config {
                map<string, string> settings = 1;
                map<int32, User> users = 2;
            }
        "#;

        let schema = ProtoSchema::parse(proto).unwrap();
        let msg = schema.get_message("Config").unwrap();

        let settings = msg.fields.get("settings").unwrap();
        assert!(settings.map);
        assert_eq!(settings.map_key_type, Some("string".to_string()));
        assert_eq!(settings.map_value_type, Some("string".to_string()));
    }
}
