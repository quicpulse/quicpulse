//! Workflow Generator from OpenAPI Specifications
//!
//! Generates QuicPulse workflow YAML files from parsed OpenAPI specs.
//! Features:
//! - Automatic magic value generation based on schema types
//! - Dependency chaining (extract ID from POST, use in GET)
//! - Schema-based assertions
//! - CRUD ordering (Create -> Read -> Update -> Delete)

use super::parser::{OpenApiSpec, Endpoint, Schema, RequestBody, Parameter};
use super::schema_mapper::SchemaMapper;
use crate::pipeline::workflow::{Workflow, WorkflowStep, StepAssertions, StatusAssertion};
use std::collections::HashMap;
use once_cell::sync::Lazy;
use serde_json::Value;
use regex::Regex;

/// SIMD-optimized cached regex for camelCase to Title Case conversion
static CAMEL_CASE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([a-z])([A-Z])").expect("Invalid camelCase regex")
});

/// Options for workflow generation
#[derive(Debug, Clone)]
pub struct GeneratorOptions {
    /// Base URL to use (overrides spec servers)
    pub base_url: Option<String>,
    /// Include deprecated endpoints
    pub include_deprecated: bool,
    /// Generate fuzz test steps
    pub include_fuzz: bool,
    /// Only include endpoints matching these tags
    pub filter_tags: Vec<String>,
    /// Exclude endpoints matching these tags
    pub exclude_tags: Vec<String>,
    /// Only include these HTTP methods
    pub filter_methods: Vec<String>,
    /// Maximum latency assertion (e.g., "500ms")
    pub max_latency: Option<String>,
    /// Generate extraction variables for chaining
    pub enable_chaining: bool,
    /// Group steps by tag
    pub group_by_tag: bool,
    /// Add comments to workflow
    pub include_comments: bool,
}

impl Default for GeneratorOptions {
    fn default() -> Self {
        Self {
            base_url: None,
            include_deprecated: false,
            include_fuzz: false,
            filter_tags: Vec::new(),
            exclude_tags: Vec::new(),
            filter_methods: Vec::new(),
            max_latency: Some("500ms".to_string()),
            enable_chaining: true,
            group_by_tag: false,
            include_comments: true,
        }
    }
}

/// Generate a workflow from an OpenAPI spec
pub fn generate_workflow(spec: &OpenApiSpec, options: &GeneratorOptions) -> Workflow {
    let base_url = options.base_url.clone()
        .or_else(|| spec.servers.first().map(|s| s.url.clone()))
        .unwrap_or_else(|| "http://localhost".to_string());

    let mut variables: HashMap<String, Value> = HashMap::new();
    variables.insert("base_url".to_string(), Value::String(base_url));

    // Add auth placeholder if security is defined
    if !spec.security_schemes.is_empty() || !spec.security.is_empty() {
        variables.insert("auth_token".to_string(), Value::String("YOUR_TOKEN_HERE".to_string()));
    }

    // Filter and sort endpoints
    let mut endpoints: Vec<&Endpoint> = spec.endpoints.iter()
        .filter(|e| options.include_deprecated || !e.deprecated)
        .filter(|e| {
            if options.filter_tags.is_empty() {
                true
            } else {
                e.tags.iter().any(|t| options.filter_tags.contains(t))
            }
        })
        .filter(|e| {
            if options.exclude_tags.is_empty() {
                true
            } else {
                !e.tags.iter().any(|t| options.exclude_tags.contains(t))
            }
        })
        .filter(|e| {
            if options.filter_methods.is_empty() {
                true
            } else {
                options.filter_methods.iter().any(|m| m.eq_ignore_ascii_case(&e.method))
            }
        })
        .collect();

    // Sort by CRUD order within each path
    endpoints.sort_by(|a, b| {
        // First by path
        let path_cmp = a.path.cmp(&b.path);
        if path_cmp != std::cmp::Ordering::Equal {
            return path_cmp;
        }
        // Then by CRUD order
        let method_order = |m: &str| match m {
            "POST" => 0,   // Create
            "GET" => 1,    // Read
            "PUT" => 2,    // Update (full)
            "PATCH" => 3,  // Update (partial)
            "DELETE" => 4, // Delete
            _ => 5,
        };
        method_order(&a.method).cmp(&method_order(&b.method))
    });

    // Track extracted variables for chaining
    let mut extracted_vars: HashMap<String, String> = HashMap::new();

    // Generate steps
    let steps: Vec<WorkflowStep> = endpoints.iter()
        .map(|endpoint| {
            generate_step(endpoint, &spec.schemas, options, &mut extracted_vars, &spec.security_schemes)
        })
        .collect();

    Workflow {
        name: format!("{} API Test Suite", spec.title),
        description: spec.description.clone().unwrap_or_else(|| {
            format!("Auto-generated test suite for {} v{}", spec.title, spec.version)
        }),
        base_url: Some("{{base_url}}".to_string()),
        variables,
        environments: HashMap::new(), // Could generate staging/prod variants
        headers: generate_global_headers(&spec.security_schemes, &spec.security),
        session: None,
        session_read_only: None,
        dotenv: None,
        plugins: None,
        output: None,
        steps,
    }
}

/// Generate a single workflow step from an endpoint
fn generate_step(
    endpoint: &Endpoint,
    schemas: &HashMap<String, Schema>,
    options: &GeneratorOptions,
    extracted_vars: &mut HashMap<String, String>,
    security_schemes: &HashMap<String, super::parser::SecurityScheme>,
) -> WorkflowStep {
    let name = generate_step_name(endpoint);

    // Build URL with path parameters
    let url = build_url(endpoint, extracted_vars, schemas);

    // Generate request body
    let body = generate_request_body(&endpoint.request_body, schemas);

    // Generate extractions for chaining
    let extract = if options.enable_chaining {
        generate_extractions(endpoint, schemas, extracted_vars)
    } else {
        HashMap::new()
    };

    // Generate assertions
    let assert = generate_assertions(endpoint, options);

    // Generate headers for this step
    let headers = generate_step_headers(endpoint, security_schemes);

    WorkflowStep {
        name,
        tags: Vec::new(),
        depends_on: Vec::new(),
        method: endpoint.method.clone(),
        url,
        query: HashMap::new(),
        headers,
        body,
        raw: None,
        form: None,
        multipart: None,
        auth: None,
        extract,
        assert,
        skip_if: None,
        delay: None,
        timeout: None,
        retries: None,
        retry_delay: None,
        retry_on: Vec::new(),
        repeat: None,
        foreach: None,
        foreach_var: None,
        while_condition: None,
        max_iterations: None,
        parallel: false,
        fail_fast: None,
        follow_redirects: None,
        max_redirects: None,
        proxy: None,
        insecure: None,
        ca_cert: None,
        client_cert: None,
        client_key: None,
        http2: None,
        graphql: None,
        grpc: None,
        websocket: None,
        compress: None,
        pre_script: None,
        post_script: None,
        script_assert: None,
        fuzz: None,
        bench: None,
        download: None,
        http3: None,
        har: None,
        openapi: None,
        plugins: None,
        upload: None,
        output: None,
        filter: None,
        save: None,
        curl: None,
    }
}

/// Generate a descriptive step name (uses cached SIMD-optimized regex)
fn generate_step_name(endpoint: &Endpoint) -> String {
    if let Some(summary) = &endpoint.summary {
        summary.clone()
    } else if let Some(op_id) = &endpoint.operation_id {
        // Convert camelCase/snake_case to Title Case using cached regex
        let spaced = CAMEL_CASE_RE.replace_all(op_id, "$1 $2");
        spaced.replace('_', " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        format!("{} {}", endpoint.method, endpoint.path)
    }
}

/// Build URL with path parameter substitution
fn build_url(
    endpoint: &Endpoint,
    extracted_vars: &HashMap<String, String>,
    schemas: &HashMap<String, Schema>,
) -> String {
    let mut url = format!("{{{{base_url}}}}{}", endpoint.path);

    // Substitute path parameters
    for param in &endpoint.path_params {
        let placeholder = format!("{{{}}}", param.name);

        // Check if we have an extracted variable for this parameter
        let var_name = format!("{}_id", param.name.trim_end_matches("Id").trim_end_matches("_id"));
        let value = if let Some(var) = extracted_vars.get(&var_name) {
            format!("{{{{{}}}}}", var)
        } else if let Some(var) = extracted_vars.get(&param.name) {
            format!("{{{{{}}}}}", var)
        } else {
            // Generate magic value based on schema
            generate_param_value(param, schemas)
        };

        url = url.replace(&placeholder, &value);
    }

    // Add query parameters
    if !endpoint.query_params.is_empty() {
        let query_parts: Vec<String> = endpoint.query_params.iter()
            .filter(|p| p.required)
            .map(|p| {
                let value = generate_param_value(p, schemas);
                format!("{}={}", p.name, value)
            })
            .collect();

        if !query_parts.is_empty() {
            url = format!("{}?{}", url, query_parts.join("&"));
        }
    }

    url
}

/// Generate a parameter value based on its schema
fn generate_param_value(param: &Parameter, schemas: &HashMap<String, Schema>) -> String {
    // Use example if provided
    if let Some(example) = &param.example {
        return value_to_template(example);
    }

    // Use schema to generate value
    if let Some(schema) = &param.schema {
        // Handle $ref
        if let Some(ref_path) = &schema.ref_path {
            let ref_name = ref_path.rsplit('/').next().unwrap_or("");
            if let Some(ref_schema) = schemas.get(ref_name) {
                return SchemaMapper::schema_to_magic(ref_schema);
            }
        }
        return SchemaMapper::schema_to_magic(schema);
    }

    // Default to random string
    "{random_string:10}".to_string()
}

/// Convert a JSON value to a template string
fn value_to_template(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()),
    }
}

/// Generate request body from schema
fn generate_request_body(
    request_body: &Option<RequestBody>,
    schemas: &HashMap<String, Schema>,
) -> Option<Value> {
    let rb = request_body.as_ref()?;

    // Prefer JSON content type
    let media_type = rb.content.get("application/json")
        .or_else(|| rb.content.values().next())?;

    let schema = media_type.schema.as_ref()?;

    // Use example if provided
    if let Some(example) = &media_type.example {
        return Some(example.clone());
    }

    // Generate from schema
    Some(SchemaMapper::generate_request_body(schema, schemas))
}

/// Generate extraction rules for response chaining
fn generate_extractions(
    endpoint: &Endpoint,
    schemas: &HashMap<String, Schema>,
    extracted_vars: &mut HashMap<String, String>,
) -> HashMap<String, String> {
    let mut extractions = HashMap::new();

    // Only extract from successful responses (2xx)
    let success_responses: Vec<_> = endpoint.responses.iter()
        .filter(|(status, _)| {
            status.starts_with('2') || status.as_str() == "default"
        })
        .collect();

    for (_, response) in success_responses {
        if let Some(media_type) = response.content.get("application/json") {
            if let Some(schema) = &media_type.schema {
                // Look for id fields to extract (with visited tracking to prevent infinite recursion)
                let mut visited = std::collections::HashSet::new();
                extract_id_fields(schema, schemas, &mut extractions, "", endpoint, extracted_vars, &mut visited, 0);
            }
        }
    }

    extractions
}

/// Maximum recursion depth for schema traversal (prevents stack overflow)
const MAX_SCHEMA_DEPTH: usize = 20;

/// Recursively extract ID fields from a schema
/// Tracks visited schemas to prevent infinite recursion on circular references
fn extract_id_fields(
    schema: &Schema,
    schemas: &HashMap<String, Schema>,
    extractions: &mut HashMap<String, String>,
    prefix: &str,
    endpoint: &Endpoint,
    extracted_vars: &mut HashMap<String, String>,
    visited: &mut std::collections::HashSet<String>,
    depth: usize,
) {
    // Prevent stack overflow from deeply nested or circular schemas
    if depth > MAX_SCHEMA_DEPTH {
        return;
    }

    // Handle $ref
    if let Some(ref_path) = &schema.ref_path {
        let ref_name = ref_path.rsplit('/').next().unwrap_or("");

        // Skip if we've already visited this schema (circular reference)
        if visited.contains(ref_name) {
            return;
        }
        visited.insert(ref_name.to_string());

        if let Some(ref_schema) = schemas.get(ref_name) {
            extract_id_fields(ref_schema, schemas, extractions, prefix, endpoint, extracted_vars, visited, depth + 1);
        }
        return;
    }

    // Only POST/PUT typically return created resources
    if endpoint.method != "POST" && endpoint.method != "PUT" {
        return;
    }

    // Extract id field
    for (name, _prop_schema) in &schema.properties {
        if name == "id" || name.ends_with("Id") || name.ends_with("_id") {
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}.{}", prefix, name)
            };

            // Generate a unique variable name based on the resource
            let resource_name = extract_resource_name(&endpoint.path);
            let var_name = if name == "id" {
                format!("{}_id", resource_name)
            } else {
                name.clone()
            };

            extractions.insert(var_name.clone(), format!("response.body.{}", path));
            extracted_vars.insert(var_name.clone(), var_name);
        }
    }
}

/// Extract resource name from path (e.g., /users/{id} -> user)
fn extract_resource_name(path: &str) -> String {
    // Split path and find the last non-parameter segment
    let segments: Vec<&str> = path.split('/')
        .filter(|s| !s.is_empty() && !s.starts_with('{'))
        .collect();

    if let Some(last) = segments.last() {
        // Singularize if ends with 's'
        let name = last.to_string();
        if name.ends_with('s') && name.len() > 1 {
            name[..name.len()-1].to_string()
        } else {
            name
        }
    } else {
        "resource".to_string()
    }
}

/// Generate assertions based on endpoint responses
fn generate_assertions(endpoint: &Endpoint, options: &GeneratorOptions) -> StepAssertions {
    let mut assertions = StepAssertions::default();

    // Find expected success status code
    let success_status = endpoint.responses.keys()
        .filter(|s| s.starts_with('2'))
        .min()
        .map(|s| s.parse::<u16>().ok())
        .flatten();

    if let Some(status) = success_status {
        assertions.status = Some(StatusAssertion::Exact(status));
    } else {
        // Default to 2xx range
        assertions.status = Some(StatusAssertion::Range("2xx".to_string()));
    }

    // Add latency assertion
    if let Some(latency) = &options.max_latency {
        assertions.latency = Some(format!("<{}", latency));
    }

    // Add body assertions based on response schema
    if let Some(response) = endpoint.responses.get("200")
        .or_else(|| endpoint.responses.get("201"))
        .or_else(|| endpoint.responses.get("default"))
    {
        if let Some(media_type) = response.content.get("application/json") {
            if let Some(schema) = &media_type.schema {
                // Add type assertion for response
                if schema.schema_type.as_deref() == Some("object") {
                    for required_field in &schema.required {
                        let escaped = escape_jq_field(required_field);
                        assertions.body.insert(
                            format!(".[\"{}\"] | type", escaped),
                            Value::String("!= \"null\"".to_string()),
                        );
                    }
                }
            }
        }
    }

    assertions
}

/// Generate global headers based on security schemes
fn generate_global_headers(
    security_schemes: &HashMap<String, super::parser::SecurityScheme>,
    global_security: &[HashMap<String, Vec<String>>],
) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    // Check what security is required globally
    for req in global_security {
        for scheme_name in req.keys() {
            if let Some(scheme) = security_schemes.get(scheme_name) {
                match scheme.scheme_type.as_str() {
                    "http" => {
                        if scheme.scheme.as_deref() == Some("bearer") {
                            headers.insert(
                                "Authorization".to_string(),
                                "Bearer {{auth_token}}".to_string(),
                            );
                        } else if scheme.scheme.as_deref() == Some("basic") {
                            headers.insert(
                                "Authorization".to_string(),
                                "Basic {{auth_token}}".to_string(),
                            );
                        }
                    }
                    "apiKey" => {
                        if scheme.location.as_deref() == Some("header") {
                            if let Some(name) = &scheme.name {
                                headers.insert(name.clone(), "{{auth_token}}".to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    headers
}

/// Generate headers for a specific step
fn generate_step_headers(
    endpoint: &Endpoint,
    security_schemes: &HashMap<String, super::parser::SecurityScheme>,
) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    // Add Content-Type for requests with body
    if endpoint.request_body.is_some() {
        headers.insert("Content-Type".to_string(), "application/json".to_string());
    }

    // Add any header parameters
    for param in &endpoint.header_params {
        if param.required {
            let value = if let Some(example) = &param.example {
                value_to_template(example)
            } else if let Some(schema) = &param.schema {
                SchemaMapper::schema_to_magic(schema)
            } else {
                "{{header_value}}".to_string()
            };
            headers.insert(param.name.clone(), value);
        }
    }

    // Handle endpoint-specific security (if different from global)
    if !endpoint.security.is_empty() {
        for req in &endpoint.security {
            for scheme_name in req.keys() {
                if let Some(scheme) = security_schemes.get(scheme_name) {
                    if scheme.scheme_type == "http" && scheme.scheme.as_deref() == Some("bearer") {
                        headers.insert(
                            "Authorization".to_string(),
                            "Bearer {{auth_token}}".to_string(),
                        );
                    }
                }
            }
        }
    }

    headers
}

fn escape_jq_field(field: &str) -> String {
    field
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

/// Output workflow as YAML string
pub fn workflow_to_yaml(workflow: &Workflow) -> Result<String, serde_yaml::Error> {
    // Add header comment
    let mut yaml = String::from("# Auto-generated by QuicPulse from OpenAPI specification\n");
    yaml.push_str("# Edit variables below to configure for your environment\n\n");
    yaml.push_str(&serde_yaml::to_string(workflow)?);
    Ok(yaml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_resource_name() {
        assert_eq!(extract_resource_name("/users"), "user");
        assert_eq!(extract_resource_name("/users/{id}"), "user");
        assert_eq!(extract_resource_name("/api/v1/products"), "product");
        assert_eq!(extract_resource_name("/api/orders/{orderId}/items"), "item");
    }

    #[test]
    fn test_generate_step_name_from_operation_id() {
        let endpoint = Endpoint {
            method: "GET".to_string(),
            path: "/users".to_string(),
            operation_id: Some("getUserById".to_string()),
            summary: None,
            description: None,
            tags: vec![],
            path_params: vec![],
            query_params: vec![],
            header_params: vec![],
            request_body: None,
            responses: HashMap::new(),
            security: vec![],
            deprecated: false,
        };

        assert_eq!(generate_step_name(&endpoint), "Get User By Id");
    }

    #[test]
    fn test_generate_step_name_from_summary() {
        let endpoint = Endpoint {
            method: "GET".to_string(),
            path: "/users".to_string(),
            operation_id: Some("getUserById".to_string()),
            summary: Some("Get a user by their ID".to_string()),
            description: None,
            tags: vec![],
            path_params: vec![],
            query_params: vec![],
            header_params: vec![],
            request_body: None,
            responses: HashMap::new(),
            security: vec![],
            deprecated: false,
        };

        assert_eq!(generate_step_name(&endpoint), "Get a user by their ID");
    }
}
