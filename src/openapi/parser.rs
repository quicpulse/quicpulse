//! OpenAPI/Swagger specification parser
//!
//! Supports both OpenAPI 2.0 (Swagger) and OpenAPI 3.x formats.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::errors::QuicpulseError;

/// Maximum spec file size (16 MB)
const MAX_SPEC_FILE_SIZE: u64 = 16 * 1024 * 1024;

/// Unified representation of an OpenAPI specification
#[derive(Debug, Clone)]
pub struct OpenApiSpec {
    /// API title
    pub title: String,
    /// API description
    pub description: Option<String>,
    /// API version
    pub version: String,
    /// Base URL(s) for the API
    pub servers: Vec<Server>,
    /// All available endpoints
    pub endpoints: Vec<Endpoint>,
    /// Security definitions
    pub security_schemes: HashMap<String, SecurityScheme>,
    /// Global security requirements
    pub security: Vec<HashMap<String, Vec<String>>>,
    /// Component schemas (for reference)
    pub schemas: HashMap<String, Schema>,
}

/// Server information
#[derive(Debug, Clone)]
pub struct Server {
    pub url: String,
    pub description: Option<String>,
    pub variables: HashMap<String, ServerVariable>,
}

/// Server variable for templated URLs
#[derive(Debug, Clone)]
pub struct ServerVariable {
    pub default: String,
    pub description: Option<String>,
    pub enum_values: Vec<String>,
}

/// An API endpoint (operation)
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Path template (e.g., /users/{id})
    pub path: String,
    /// Operation ID (unique identifier)
    pub operation_id: Option<String>,
    /// Summary
    pub summary: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Tags for grouping
    pub tags: Vec<String>,
    /// Path parameters
    pub path_params: Vec<Parameter>,
    /// Query parameters
    pub query_params: Vec<Parameter>,
    /// Header parameters
    pub header_params: Vec<Parameter>,
    /// Request body schema
    pub request_body: Option<RequestBody>,
    /// Response schemas by status code
    pub responses: HashMap<String, Response>,
    /// Security requirements for this endpoint
    pub security: Vec<HashMap<String, Vec<String>>>,
    /// Whether this endpoint is deprecated
    pub deprecated: bool,
}

/// Parameter definition
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
    pub schema: Option<Schema>,
    pub example: Option<Value>,
}

/// Request body definition
#[derive(Debug, Clone)]
pub struct RequestBody {
    pub description: Option<String>,
    pub required: bool,
    pub content: HashMap<String, MediaType>,
}

/// Media type content
#[derive(Debug, Clone)]
pub struct MediaType {
    pub schema: Option<Schema>,
    pub example: Option<Value>,
    pub examples: HashMap<String, Value>,
}

/// Response definition
#[derive(Debug, Clone)]
pub struct Response {
    pub description: String,
    pub content: HashMap<String, MediaType>,
    pub headers: HashMap<String, Parameter>,
}

/// Schema definition (simplified)
#[derive(Debug, Clone)]
pub struct Schema {
    pub schema_type: Option<String>,
    pub format: Option<String>,
    pub description: Option<String>,
    pub properties: HashMap<String, Schema>,
    pub required: Vec<String>,
    pub items: Option<Box<Schema>>,
    pub enum_values: Vec<Value>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub min_length: Option<u64>,
    pub max_length: Option<u64>,
    pub pattern: Option<String>,
    pub example: Option<Value>,
    pub default: Option<Value>,
    pub nullable: bool,
    pub ref_path: Option<String>,
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            schema_type: None,
            format: None,
            description: None,
            properties: HashMap::new(),
            required: Vec::new(),
            items: None,
            enum_values: Vec::new(),
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            example: None,
            default: None,
            nullable: false,
            ref_path: None,
        }
    }
}

/// Security scheme definition
#[derive(Debug, Clone)]
pub struct SecurityScheme {
    pub scheme_type: String,
    pub description: Option<String>,
    pub name: Option<String>,
    pub location: Option<String>,
    pub scheme: Option<String>,
    pub bearer_format: Option<String>,
    pub flows: Option<OAuthFlows>,
}

/// OAuth flows
#[derive(Debug, Clone)]
pub struct OAuthFlows {
    pub implicit: Option<OAuthFlow>,
    pub password: Option<OAuthFlow>,
    pub client_credentials: Option<OAuthFlow>,
    pub authorization_code: Option<OAuthFlow>,
}

/// OAuth flow details
#[derive(Debug, Clone)]
pub struct OAuthFlow {
    pub authorization_url: Option<String>,
    pub token_url: Option<String>,
    pub refresh_url: Option<String>,
    pub scopes: HashMap<String, String>,
}

/// Parse an OpenAPI specification from a file
pub fn parse_spec(path: &Path) -> Result<OpenApiSpec, QuicpulseError> {
    // Check file size
    let metadata = fs::metadata(path)
        .map_err(|e| QuicpulseError::Io(e))?;

    if metadata.len() > MAX_SPEC_FILE_SIZE {
        return Err(QuicpulseError::Argument(format!(
            "OpenAPI spec file too large: {} bytes (max {} bytes)",
            metadata.len(), MAX_SPEC_FILE_SIZE
        )));
    }

    // Read file content
    let content = fs::read_to_string(path)
        .map_err(|e| QuicpulseError::Io(e))?;

    // Detect format and parse
    let value: Value = if path.extension().map_or(false, |e| e == "yaml" || e == "yml") {
        serde_yaml::from_str(&content)
            .map_err(|e| QuicpulseError::Argument(format!("Failed to parse YAML: {}", e)))?
    } else if path.extension().map_or(false, |e| e == "json") {
        serde_json::from_str(&content)
            .map_err(|e| QuicpulseError::Argument(format!("Failed to parse JSON: {}", e)))?
    } else {
        // Try JSON first, then YAML
        serde_json::from_str(&content)
            .or_else(|_| serde_yaml::from_str(&content))
            .map_err(|e| QuicpulseError::Argument(format!("Failed to parse spec: {}", e)))?
    };

    // Detect OpenAPI version
    if value.get("openapi").is_some() {
        parse_openapi_3(&value)
    } else if value.get("swagger").is_some() {
        parse_swagger_2(&value)
    } else {
        Err(QuicpulseError::Argument(
            "Unknown spec format: missing 'openapi' or 'swagger' field".to_string()
        ))
    }
}

/// Parse OpenAPI 3.x specification
fn parse_openapi_3(value: &Value) -> Result<OpenApiSpec, QuicpulseError> {
    let info = value.get("info")
        .ok_or_else(|| QuicpulseError::Argument("Missing 'info' field".to_string()))?;

    let title = info.get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled API")
        .to_string();

    let description = info.get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let version = info.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();

    // Parse servers
    let servers = parse_servers_v3(value.get("servers"));

    // Parse components/schemas
    let schemas = parse_schemas_v3(value.get("components").and_then(|c| c.get("schemas")));

    // Parse security schemes
    let security_schemes = parse_security_schemes_v3(
        value.get("components").and_then(|c| c.get("securitySchemes"))
    );

    // Parse global security
    let security = parse_security_requirements(value.get("security"));

    // Parse paths/endpoints
    let endpoints = parse_paths_v3(value.get("paths"), &schemas)?;

    Ok(OpenApiSpec {
        title,
        description,
        version,
        servers,
        endpoints,
        security_schemes,
        security,
        schemas,
    })
}

/// Parse Swagger 2.0 specification
fn parse_swagger_2(value: &Value) -> Result<OpenApiSpec, QuicpulseError> {
    let info = value.get("info")
        .ok_or_else(|| QuicpulseError::Argument("Missing 'info' field".to_string()))?;

    let title = info.get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled API")
        .to_string();

    let description = info.get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let version = info.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();

    // Build server URL from host, basePath, schemes
    let servers = parse_servers_v2(value);

    // Parse definitions (schemas)
    let schemas = parse_schemas_v2(value.get("definitions"));

    // Parse security definitions
    let security_schemes = parse_security_schemes_v2(value.get("securityDefinitions"));

    // Parse global security
    let security = parse_security_requirements(value.get("security"));

    // Parse paths/endpoints
    let endpoints = parse_paths_v2(value.get("paths"), &schemas)?;

    Ok(OpenApiSpec {
        title,
        description,
        version,
        servers,
        endpoints,
        security_schemes,
        security,
        schemas,
    })
}

/// Parse servers from OpenAPI 3.x
fn parse_servers_v3(servers: Option<&Value>) -> Vec<Server> {
    let Some(servers) = servers.and_then(|s| s.as_array()) else {
        return vec![Server {
            url: "http://localhost".to_string(),
            description: None,
            variables: HashMap::new(),
        }];
    };

    servers.iter().filter_map(|s| {
        let url = s.get("url")?.as_str()?.to_string();
        let description = s.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());

        let mut variables = HashMap::new();
        if let Some(vars) = s.get("variables").and_then(|v| v.as_object()) {
            for (name, var) in vars {
                if let Some(default) = var.get("default").and_then(|d| d.as_str()) {
                    variables.insert(name.clone(), ServerVariable {
                        default: default.to_string(),
                        description: var.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                        enum_values: var.get("enum")
                            .and_then(|e| e.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                            .unwrap_or_default(),
                    });
                }
            }
        }

        Some(Server { url, description, variables })
    }).collect()
}

/// Parse servers from Swagger 2.0
fn parse_servers_v2(value: &Value) -> Vec<Server> {
    let host = value.get("host")
        .and_then(|h| h.as_str())
        .unwrap_or("localhost");

    let base_path = value.get("basePath")
        .and_then(|b| b.as_str())
        .unwrap_or("");

    let schemes = value.get("schemes")
        .and_then(|s| s.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["http"]);

    schemes.into_iter().map(|scheme| {
        Server {
            url: format!("{}://{}{}", scheme, host, base_path),
            description: None,
            variables: HashMap::new(),
        }
    }).collect()
}

/// Parse schemas from OpenAPI 3.x components
fn parse_schemas_v3(schemas: Option<&Value>) -> HashMap<String, Schema> {
    let Some(schemas) = schemas.and_then(|s| s.as_object()) else {
        return HashMap::new();
    };

    schemas.iter().map(|(name, schema)| {
        (name.clone(), parse_schema(schema))
    }).collect()
}

/// Parse schemas from Swagger 2.0 definitions
fn parse_schemas_v2(definitions: Option<&Value>) -> HashMap<String, Schema> {
    parse_schemas_v3(definitions)
}

/// Parse a single schema
fn parse_schema(value: &Value) -> Schema {
    let mut schema = Schema::default();

    // Handle $ref
    if let Some(ref_path) = value.get("$ref").and_then(|r| r.as_str()) {
        schema.ref_path = Some(ref_path.to_string());
        return schema;
    }

    schema.schema_type = value.get("type").and_then(|t| t.as_str()).map(|s| s.to_string());
    schema.format = value.get("format").and_then(|f| f.as_str()).map(|s| s.to_string());
    schema.description = value.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());
    schema.example = value.get("example").cloned();
    schema.default = value.get("default").cloned();
    schema.nullable = value.get("nullable").and_then(|n| n.as_bool()).unwrap_or(false);
    schema.pattern = value.get("pattern").and_then(|p| p.as_str()).map(|s| s.to_string());
    schema.minimum = value.get("minimum").and_then(|m| m.as_f64());
    schema.maximum = value.get("maximum").and_then(|m| m.as_f64());
    schema.min_length = value.get("minLength").and_then(|m| m.as_u64());
    schema.max_length = value.get("maxLength").and_then(|m| m.as_u64());

    // Parse enum
    if let Some(enum_arr) = value.get("enum").and_then(|e| e.as_array()) {
        schema.enum_values = enum_arr.clone();
    }

    // Parse properties (for objects)
    if let Some(props) = value.get("properties").and_then(|p| p.as_object()) {
        for (name, prop) in props {
            schema.properties.insert(name.clone(), parse_schema(prop));
        }
    }

    // Parse required fields
    if let Some(required) = value.get("required").and_then(|r| r.as_array()) {
        schema.required = required.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }

    // Parse items (for arrays)
    if let Some(items) = value.get("items") {
        schema.items = Some(Box::new(parse_schema(items)));
    }

    schema
}

/// Parse security schemes from OpenAPI 3.x
fn parse_security_schemes_v3(schemes: Option<&Value>) -> HashMap<String, SecurityScheme> {
    let Some(schemes) = schemes.and_then(|s| s.as_object()) else {
        return HashMap::new();
    };

    schemes.iter().filter_map(|(name, scheme)| {
        let scheme_type = scheme.get("type")?.as_str()?.to_string();

        Some((name.clone(), SecurityScheme {
            scheme_type,
            description: scheme.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
            name: scheme.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()),
            location: scheme.get("in").and_then(|i| i.as_str()).map(|s| s.to_string()),
            scheme: scheme.get("scheme").and_then(|s| s.as_str()).map(|s| s.to_string()),
            bearer_format: scheme.get("bearerFormat").and_then(|b| b.as_str()).map(|s| s.to_string()),
            flows: parse_oauth_flows(scheme.get("flows")),
        }))
    }).collect()
}

/// Parse security schemes from Swagger 2.0
fn parse_security_schemes_v2(definitions: Option<&Value>) -> HashMap<String, SecurityScheme> {
    let Some(defs) = definitions.and_then(|d| d.as_object()) else {
        return HashMap::new();
    };

    defs.iter().filter_map(|(name, def)| {
        let def_type = def.get("type")?.as_str()?;

        // Convert Swagger 2.0 types to OpenAPI 3.x types
        let scheme_type = match def_type {
            "basic" => "http".to_string(),
            "apiKey" => "apiKey".to_string(),
            "oauth2" => "oauth2".to_string(),
            other => other.to_string(),
        };

        Some((name.clone(), SecurityScheme {
            scheme_type,
            description: def.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
            name: def.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()),
            location: def.get("in").and_then(|i| i.as_str()).map(|s| s.to_string()),
            scheme: if def_type == "basic" { Some("basic".to_string()) } else { None },
            bearer_format: None,
            flows: None, // Swagger 2.0 OAuth is different, simplified here
        }))
    }).collect()
}

/// Parse OAuth flows
fn parse_oauth_flows(flows: Option<&Value>) -> Option<OAuthFlows> {
    let flows = flows?;

    Some(OAuthFlows {
        implicit: parse_oauth_flow(flows.get("implicit")),
        password: parse_oauth_flow(flows.get("password")),
        client_credentials: parse_oauth_flow(flows.get("clientCredentials")),
        authorization_code: parse_oauth_flow(flows.get("authorizationCode")),
    })
}

/// Parse a single OAuth flow
fn parse_oauth_flow(flow: Option<&Value>) -> Option<OAuthFlow> {
    let flow = flow?;

    let mut scopes = HashMap::new();
    if let Some(scope_obj) = flow.get("scopes").and_then(|s| s.as_object()) {
        for (name, desc) in scope_obj {
            if let Some(desc_str) = desc.as_str() {
                scopes.insert(name.clone(), desc_str.to_string());
            }
        }
    }

    Some(OAuthFlow {
        authorization_url: flow.get("authorizationUrl").and_then(|u| u.as_str()).map(|s| s.to_string()),
        token_url: flow.get("tokenUrl").and_then(|u| u.as_str()).map(|s| s.to_string()),
        refresh_url: flow.get("refreshUrl").and_then(|u| u.as_str()).map(|s| s.to_string()),
        scopes,
    })
}

/// Parse security requirements
fn parse_security_requirements(security: Option<&Value>) -> Vec<HashMap<String, Vec<String>>> {
    let Some(security) = security.and_then(|s| s.as_array()) else {
        return Vec::new();
    };

    security.iter().filter_map(|req| {
        let obj = req.as_object()?;
        let mut result = HashMap::new();
        for (name, scopes) in obj {
            let scope_vec = scopes.as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            result.insert(name.clone(), scope_vec);
        }
        Some(result)
    }).collect()
}

/// Parse paths/endpoints from OpenAPI 3.x
fn parse_paths_v3(paths: Option<&Value>, schemas: &HashMap<String, Schema>) -> Result<Vec<Endpoint>, QuicpulseError> {
    let Some(paths) = paths.and_then(|p| p.as_object()) else {
        return Ok(Vec::new());
    };

    let mut endpoints = Vec::new();

    for (path, path_item) in paths {
        // Skip path-level parameters for now
        let methods = ["get", "post", "put", "patch", "delete", "head", "options", "trace"];

        for method in methods {
            if let Some(operation) = path_item.get(method) {
                let endpoint = parse_operation_v3(method, path, operation, schemas)?;
                endpoints.push(endpoint);
            }
        }
    }

    Ok(endpoints)
}

/// Parse paths/endpoints from Swagger 2.0
fn parse_paths_v2(paths: Option<&Value>, schemas: &HashMap<String, Schema>) -> Result<Vec<Endpoint>, QuicpulseError> {
    let Some(paths) = paths.and_then(|p| p.as_object()) else {
        return Ok(Vec::new());
    };

    let mut endpoints = Vec::new();

    for (path, path_item) in paths {
        let methods = ["get", "post", "put", "patch", "delete", "head", "options"];

        for method in methods {
            if let Some(operation) = path_item.get(method) {
                let endpoint = parse_operation_v2(method, path, operation, schemas)?;
                endpoints.push(endpoint);
            }
        }
    }

    Ok(endpoints)
}

/// Parse an operation from OpenAPI 3.x
fn parse_operation_v3(method: &str, path: &str, operation: &Value, _schemas: &HashMap<String, Schema>) -> Result<Endpoint, QuicpulseError> {
    let operation_id = operation.get("operationId")
        .and_then(|o| o.as_str())
        .map(|s| s.to_string());

    let summary = operation.get("summary")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let description = operation.get("description")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());

    let tags = operation.get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let deprecated = operation.get("deprecated")
        .and_then(|d| d.as_bool())
        .unwrap_or(false);

    // Parse parameters
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();
    let mut header_params = Vec::new();

    if let Some(params) = operation.get("parameters").and_then(|p| p.as_array()) {
        for param in params {
            let parsed = parse_parameter_v3(param);
            if let Some(p) = parsed {
                match param.get("in").and_then(|i| i.as_str()) {
                    Some("path") => path_params.push(p),
                    Some("query") => query_params.push(p),
                    Some("header") => header_params.push(p),
                    _ => {}
                }
            }
        }
    }

    // Parse request body (OpenAPI 3.x)
    let request_body = operation.get("requestBody").map(|rb| {
        let content = parse_content_v3(rb.get("content"));
        RequestBody {
            description: rb.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
            required: rb.get("required").and_then(|r| r.as_bool()).unwrap_or(false),
            content,
        }
    });

    // Parse responses
    let responses = parse_responses_v3(operation.get("responses"));

    // Parse security
    let security = parse_security_requirements(operation.get("security"));

    Ok(Endpoint {
        method: method.to_uppercase(),
        path: path.to_string(),
        operation_id,
        summary,
        description,
        tags,
        path_params,
        query_params,
        header_params,
        request_body,
        responses,
        security,
        deprecated,
    })
}

/// Parse an operation from Swagger 2.0
fn parse_operation_v2(method: &str, path: &str, operation: &Value, _schemas: &HashMap<String, Schema>) -> Result<Endpoint, QuicpulseError> {
    let operation_id = operation.get("operationId")
        .and_then(|o| o.as_str())
        .map(|s| s.to_string());

    let summary = operation.get("summary")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let description = operation.get("description")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());

    let tags = operation.get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let deprecated = operation.get("deprecated")
        .and_then(|d| d.as_bool())
        .unwrap_or(false);

    // Parse parameters (Swagger 2.0 includes body as parameter)
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();
    let mut header_params = Vec::new();
    let mut request_body = None;

    if let Some(params) = operation.get("parameters").and_then(|p| p.as_array()) {
        for param in params {
            let location = param.get("in").and_then(|i| i.as_str());

            if location == Some("body") {
                // Convert to request body
                let schema = param.get("schema").map(|s| parse_schema(s));
                let mut content = HashMap::new();
                content.insert("application/json".to_string(), MediaType {
                    schema,
                    example: param.get("example").cloned(),
                    examples: HashMap::new(),
                });
                request_body = Some(RequestBody {
                    description: param.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                    required: param.get("required").and_then(|r| r.as_bool()).unwrap_or(false),
                    content,
                });
            } else {
                let parsed = parse_parameter_v2(param);
                if let Some(p) = parsed {
                    match location {
                        Some("path") => path_params.push(p),
                        Some("query") => query_params.push(p),
                        Some("header") => header_params.push(p),
                        _ => {}
                    }
                }
            }
        }
    }

    // Parse responses
    let responses = parse_responses_v2(operation.get("responses"));

    // Parse security
    let security = parse_security_requirements(operation.get("security"));

    Ok(Endpoint {
        method: method.to_uppercase(),
        path: path.to_string(),
        operation_id,
        summary,
        description,
        tags,
        path_params,
        query_params,
        header_params,
        request_body,
        responses,
        security,
        deprecated,
    })
}

/// Parse a parameter from OpenAPI 3.x
fn parse_parameter_v3(param: &Value) -> Option<Parameter> {
    let name = param.get("name")?.as_str()?.to_string();

    Some(Parameter {
        name,
        description: param.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
        required: param.get("required").and_then(|r| r.as_bool()).unwrap_or(false),
        schema: param.get("schema").map(|s| parse_schema(s)),
        example: param.get("example").cloned(),
    })
}

/// Parse a parameter from Swagger 2.0
fn parse_parameter_v2(param: &Value) -> Option<Parameter> {
    let name = param.get("name")?.as_str()?.to_string();

    // In Swagger 2.0, schema info is inline
    let mut schema = Schema::default();
    schema.schema_type = param.get("type").and_then(|t| t.as_str()).map(|s| s.to_string());
    schema.format = param.get("format").and_then(|f| f.as_str()).map(|s| s.to_string());
    schema.minimum = param.get("minimum").and_then(|m| m.as_f64());
    schema.maximum = param.get("maximum").and_then(|m| m.as_f64());
    if let Some(enum_arr) = param.get("enum").and_then(|e| e.as_array()) {
        schema.enum_values = enum_arr.clone();
    }

    Some(Parameter {
        name,
        description: param.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
        required: param.get("required").and_then(|r| r.as_bool()).unwrap_or(false),
        schema: if schema.schema_type.is_some() { Some(schema) } else { None },
        example: param.get("example").cloned()
            .or_else(|| param.get("x-example").cloned()),
    })
}

/// Parse content from OpenAPI 3.x
fn parse_content_v3(content: Option<&Value>) -> HashMap<String, MediaType> {
    let Some(content) = content.and_then(|c| c.as_object()) else {
        return HashMap::new();
    };

    content.iter().map(|(media_type, media_obj)| {
        let mt = MediaType {
            schema: media_obj.get("schema").map(|s| parse_schema(s)),
            example: media_obj.get("example").cloned(),
            examples: media_obj.get("examples")
                .and_then(|e| e.as_object())
                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
        };
        (media_type.clone(), mt)
    }).collect()
}

/// Parse responses from OpenAPI 3.x
fn parse_responses_v3(responses: Option<&Value>) -> HashMap<String, Response> {
    let Some(responses) = responses.and_then(|r| r.as_object()) else {
        return HashMap::new();
    };

    responses.iter().filter_map(|(status, resp)| {
        let description = resp.get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        let content = parse_content_v3(resp.get("content"));

        let mut headers = HashMap::new();
        if let Some(header_obj) = resp.get("headers").and_then(|h| h.as_object()) {
            for (name, header) in header_obj {
                if let Some(param) = parse_parameter_v3(header) {
                    headers.insert(name.clone(), param);
                }
            }
        }

        Some((status.clone(), Response { description, content, headers }))
    }).collect()
}

/// Parse responses from Swagger 2.0
fn parse_responses_v2(responses: Option<&Value>) -> HashMap<String, Response> {
    let Some(responses) = responses.and_then(|r| r.as_object()) else {
        return HashMap::new();
    };

    responses.iter().filter_map(|(status, resp)| {
        let description = resp.get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        // In Swagger 2.0, schema is directly under the response
        let mut content = HashMap::new();
        if let Some(schema) = resp.get("schema") {
            content.insert("application/json".to_string(), MediaType {
                schema: Some(parse_schema(schema)),
                example: resp.get("examples")
                    .and_then(|e| e.get("application/json"))
                    .cloned(),
                examples: HashMap::new(),
            });
        }

        Some((status.clone(), Response {
            description,
            content,
            headers: HashMap::new(), // Simplified
        }))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_schema() {
        let json = r#"{
            "type": "object",
            "properties": {
                "id": {"type": "integer"},
                "name": {"type": "string"}
            },
            "required": ["id"]
        }"#;

        let value: Value = serde_json::from_str(json).unwrap();
        let schema = parse_schema(&value);

        assert_eq!(schema.schema_type, Some("object".to_string()));
        assert_eq!(schema.properties.len(), 2);
        assert_eq!(schema.required, vec!["id"]);
    }

    #[test]
    fn test_parse_schema_with_ref() {
        let json = r##"{"$ref": "#/components/schemas/User"}"##;
        let value: Value = serde_json::from_str(json).unwrap();
        let schema = parse_schema(&value);

        assert_eq!(schema.ref_path, Some("#/components/schemas/User".to_string()));
    }
}
