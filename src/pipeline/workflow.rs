//! Workflow definition and parsing
//!
//! Supports YAML and TOML workflow files for API automation.

use std::collections::HashMap;
use std::path::Path;
use std::fs;
use serde::{Deserialize, Serialize};
use crate::errors::QuicpulseError;

/// Maximum workflow file size (1 MB) - prevents OOM from malicious files
/// YAML/JSON parsers can expand memory 10-20x, so limit input size
const MAX_WORKFLOW_FILE_SIZE: u64 = 1 * 1024 * 1024;

/// A workflow containing multiple steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Name of the workflow
    pub name: String,

    /// Description of the workflow
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Base URL for all requests (can use variables)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Global variables
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub variables: HashMap<String, serde_json::Value>,

    /// Environment-specific variables
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub environments: HashMap<String, HashMap<String, serde_json::Value>>,

    /// Default headers for all requests
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,

    /// Session name for cookie/header persistence
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,

    /// Read-only session (don't save changes)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_read_only: Option<bool>,

    /// Load environment variables from .env file
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dotenv: Option<String>,

    /// Global plugins to apply to all steps
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<String>>,

    /// Global output configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputConfig>,

    /// Workflow steps
    pub steps: Vec<WorkflowStep>,
}

/// A single step in a workflow
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step name for logging
    pub name: String,

    /// Tags for filtering (e.g., "smoke", "auth", "slow")
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Steps that must complete before this step runs
    /// Enables explicit dependency ordering beyond sequential execution
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,

    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    #[serde(default = "default_method")]
    pub method: String,

    /// URL or path (combined with base_url if relative)
    pub url: String,

    /// Query parameters (appended to URL)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub query: HashMap<String, String>,

    /// Request headers (merged with global headers)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,

    /// Request body (for POST/PUT/PATCH)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,

    /// Raw body string (alternative to body)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,

    /// Form data (for application/x-www-form-urlencoded)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<HashMap<String, String>>,

    /// Multipart form data with file uploads
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multipart: Option<Vec<MultipartField>>,

    /// Authentication configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<StepAuth>,

    /// Extract values from response into variables
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extract: HashMap<String, String>,

    /// Assertions to check
    #[serde(default, skip_serializing_if = "StepAssertions::is_empty")]
    pub assert: StepAssertions,

    /// Skip this step if condition is false
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_if: Option<String>,

    /// Delay before executing this step
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,

    /// Timeout for this step (e.g., "30s", "5m")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    /// Number of retries on failure
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retries: Option<u32>,

    /// Delay between retries (e.g., "1s", "500ms")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_delay: Option<String>,

    /// Retry only on specific status codes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retry_on: Vec<u16>,

    // =========================================================================
    // Control Flow
    // =========================================================================

    /// Repeat this step N times
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<u32>,

    /// Loop over an array from variables (e.g., "{{users}}")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreach: Option<String>,

    /// Variable name for current loop item (default: "item")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreach_var: Option<String>,

    /// While condition - repeat while expression is true
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub while_condition: Option<String>,

    /// Maximum iterations for while loop
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,

    /// Run this step in parallel with other steps marked parallel
    #[serde(default)]
    pub parallel: bool,

    /// Stop workflow if this step fails (default: true)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fail_fast: Option<bool>,

    // =========================================================================
    // Network Options
    // =========================================================================

    /// Follow redirects for this step
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_redirects: Option<bool>,

    /// Maximum number of redirects to follow
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_redirects: Option<u32>,

    /// Proxy URL (e.g., "http://proxy:8080", "socks5://localhost:1080")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,

    // =========================================================================
    // SSL/TLS Options
    // =========================================================================

    /// Skip SSL certificate verification
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insecure: Option<bool>,

    /// Path to CA certificate bundle
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_cert: Option<String>,

    /// Path to client certificate
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_cert: Option<String>,

    /// Path to client private key
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_key: Option<String>,

    // =========================================================================
    // Protocol Options
    // =========================================================================

    /// Use HTTP/2
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http2: Option<bool>,

    /// GraphQL configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graphql: Option<GraphQLConfig>,

    /// gRPC configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grpc: Option<GrpcConfig>,

    /// WebSocket configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub websocket: Option<WebSocketConfig>,

    // =========================================================================
    // Compression
    // =========================================================================

    /// Compress request body (deflate)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compress: Option<bool>,

    // =========================================================================
    // Scripting (Rune)
    // =========================================================================

    /// Script to run before the request (can modify request)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_script: Option<ScriptConfig>,

    /// Script to run after the response (can extract/transform data)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_script: Option<ScriptConfig>,

    /// Script-based assertion (returns bool)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_assert: Option<ScriptConfig>,

    // =========================================================================
    // Advanced Features
    // =========================================================================

    /// Security fuzzing configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuzz: Option<FuzzConfig>,

    /// Benchmarking/load testing configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bench: Option<BenchConfig>,

    /// Download response to file
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download: Option<DownloadConfig>,

    /// Use HTTP/3 (QUIC)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http3: Option<bool>,

    // =========================================================================
    // Additional Module Integrations
    // =========================================================================

    /// HAR (HTTP Archive) replay
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub har: Option<HarConfig>,

    /// OpenAPI-driven step configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openapi: Option<OpenApiConfig>,

    /// Plugin execution for this step
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<PluginConfig>>,

    /// Upload configuration (chunked, compression)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload: Option<UploadConfig>,

    /// Output configuration for this step
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputConfig>,

    /// Filter configuration for request/response
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<FilterConfig>,

    /// Save response to file
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save: Option<SaveConfig>,

    /// Generate curl command (debugging)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curl: Option<bool>,
}

/// Script configuration for workflow steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    /// Inline script code
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Path to external script file
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,

    /// Script language: "rune" (default), "javascript" or "js"
    /// If not specified, detected from file extension (.js = JavaScript, .rn = Rune)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// GraphQL request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLConfig {
    /// GraphQL query string
    pub query: String,

    /// GraphQL variables
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<serde_json::Value>,

    /// Operation name (for multi-operation documents)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,

    /// Run introspection query instead of custom query
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub introspection: Option<bool>,
}

/// gRPC request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    /// Service name (e.g., "UserService")
    pub service: String,

    /// Method name (e.g., "GetUser")
    pub method: String,

    /// Request message as JSON (for unary and server streaming)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_json::Value>,

    /// Multiple request messages for client/bidi streaming (array of JSON objects)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<serde_json::Value>>,

    /// Path to .proto file (optional, for reflection)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proto_file: Option<String>,

    /// Additional proto import paths
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_paths: Option<Vec<String>>,

    /// Use TLS for gRPC connection
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,

    /// Streaming mode override (auto-detected from proto if not specified)
    /// Values: "unary", "server", "client", "bidi"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub streaming: Option<String>,

    /// gRPC metadata (headers) - alternative to step headers
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

/// WebSocket request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Message to send (text string or JSON)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Multiple messages to send in sequence
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<String>>,

    /// Binary message in hex or base64 encoding
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,

    /// Binary encoding mode: "hex" or "base64" (default: "hex")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_mode: Option<String>,

    /// WebSocket subprotocol to request
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subprotocol: Option<String>,

    /// Operation mode: "send" (send and disconnect), "listen" (receive only), 
    /// "interactive" (REPL), "stream" (send from messages array)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,

    /// Maximum number of messages to receive (0 = unlimited)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_messages: Option<usize>,

    /// Ping interval in seconds (for keep-alive)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ping_interval: Option<u64>,

    /// Wait for a response after sending (in milliseconds)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_response: Option<u64>,

    /// Enable permessage-deflate compression
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compress: Option<bool>,
}

/// Security fuzzing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzConfig {
    /// Fields to fuzz (defaults to all body fields)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,

    /// Fuzz categories: sql, xss, cmd, path, boundary, type, format, int, unicode, nosql
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,

    /// Minimum risk level (1-5, default: 1)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_level: Option<u8>,

    /// Concurrent requests (default: 10)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<usize>,

    /// Only report anomalies (5xx, timeouts)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anomalies_only: Option<bool>,

    /// Stop on first anomaly
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_on_anomaly: Option<bool>,
}

/// Benchmarking/load testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchConfig {
    /// Total number of requests to send
    pub requests: u32,

    /// Number of concurrent connections (default: 10)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<u32>,

    /// Target requests per second (0 = unlimited)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<u32>,

    /// Warmup requests before measurement
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warmup: Option<u32>,
}

/// Download configuration for saving responses to file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    /// Output file path (supports variables)
    pub path: String,

    /// Resume partial download if possible
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<bool>,

    /// Overwrite existing file
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overwrite: Option<bool>,
}

/// HAR (HTTP Archive) replay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarConfig {
    /// Path to HAR file
    pub file: String,

    /// Specific entry index to replay (0-based)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_index: Option<usize>,

    /// Entry indices to skip
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_entries: Option<Vec<usize>>,
}

/// OpenAPI-driven step configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiConfig {
    /// Path to OpenAPI spec file (YAML or JSON)
    pub spec: String,

    /// Operation ID to execute
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,

    /// API path (alternative to operation_id)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// HTTP method (used with path)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

/// Plugin execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin name
    pub name: String,

    /// Plugin-specific configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

/// Upload configuration for file uploads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadConfig {
    /// File to upload
    pub file: String,

    /// Use chunked transfer encoding
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunked: Option<bool>,

    /// Chunk size (e.g., "1MB", "512KB")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_size: Option<String>,

    /// Compress upload body ("gzip", "deflate", "br")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compress: Option<String>,

    /// Field name for multipart uploads
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_name: Option<String>,

    /// Content type override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// Output configuration for step display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Show verbose output (request + response)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,

    /// Show only response headers
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers_only: Option<bool>,

    /// Show only response body
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_only: Option<bool>,

    /// Output format ("json", "xml", "raw", "table")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    /// Enable/disable colors
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colors: Option<bool>,

    /// Print options (Standard style: "hHbB")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print: Option<String>,

    /// Pretty print output
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pretty: Option<bool>,
}

/// Filter configuration for request/response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// Headers to include (glob patterns)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_headers: Option<Vec<String>>,

    /// Headers to exclude (glob patterns)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_headers: Option<Vec<String>>,

    /// Body fields to include (JSONPath)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_body: Option<Vec<String>>,

    /// Body fields to exclude (JSONPath)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_body: Option<Vec<String>>,
}

/// Save response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveConfig {
    /// Output file path
    pub path: String,

    /// What to save: "response", "headers", "body", "all"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub what: Option<String>,

    /// Output format: "raw", "json", "har"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    /// Append to file instead of overwriting
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append: Option<bool>,
}

/// Authentication configuration for a step
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepAuth {
    /// Basic HTTP authentication
    Basic {
        username: String,
        password: String,
    },
    /// Bearer token authentication
    Bearer {
        token: String,
    },
    /// Digest authentication
    Digest {
        username: String,
        password: String,
    },
    /// AWS Signature Version 4 authentication
    #[serde(rename = "aws_sigv4")]
    AwsSigV4 {
        access_key: String,
        secret_key: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_token: Option<String>,
        region: String,
        service: String,
    },
    /// Google Cloud Platform authentication (uses gcloud CLI)
    #[serde(rename = "gcp")]
    Gcp {
        /// Optional: specific project ID
        #[serde(default, skip_serializing_if = "Option::is_none")]
        project_id: Option<String>,
        /// Optional: service account to impersonate
        #[serde(default, skip_serializing_if = "Option::is_none")]
        service_account: Option<String>,
        /// Optional: target scopes (comma-separated or array)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scopes: Option<String>,
    },
    /// Azure CLI authentication
    #[serde(rename = "azure")]
    Azure {
        /// Optional: tenant ID
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tenant_id: Option<String>,
        /// Optional: subscription ID
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subscription_id: Option<String>,
        /// Optional: resource for token (e.g., https://management.azure.com/)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resource: Option<String>,
    },
    /// OAuth2 Client Credentials flow
    OAuth2 {
        token_url: String,
        client_id: String,
        client_secret: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scope: Option<String>,
    },
}

/// Multipart form field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartField {
    /// Field name
    pub name: String,
    /// Field value (for text fields)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// File path (for file fields)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Content type override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// Assertions for a workflow step
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepAssertions {
    /// Expected status code
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusAssertion>,

    /// Expected response time
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency: Option<String>,

    /// Body assertions
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub body: HashMap<String, serde_json::Value>,

    /// Header assertions
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
}

impl StepAssertions {
    /// Check if all assertion fields are empty/none
    pub fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.latency.is_none()
            && self.body.is_empty()
            && self.headers.is_empty()
    }
}

/// Status code assertion (can be number or range)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StatusAssertion {
    Exact(u16),
    Range(String),
}

fn default_method() -> String {
    "GET".to_string()
}

/// Load a workflow from a file (YAML or TOML)
///
/// # Safety
/// - File size is checked before loading to prevent OOM attacks
/// - Maximum file size is 10 MB (should be more than enough for any workflow)
pub fn load_workflow(path: &Path) -> Result<Workflow, QuicpulseError> {
    // Check file size before loading to prevent OOM
    let metadata = fs::metadata(path)
        .map_err(|e| QuicpulseError::Io(e))?;

    let file_size = metadata.len();
    if file_size > MAX_WORKFLOW_FILE_SIZE {
        return Err(QuicpulseError::Argument(format!(
            "Workflow file too large: {} bytes (max {} bytes). \
             Consider splitting into multiple workflows.",
            file_size, MAX_WORKFLOW_FILE_SIZE
        )));
    }

    // Read file content
    let content = fs::read_to_string(path)
        .map_err(|e| QuicpulseError::Io(e))?;

    let extension = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let workflow = match extension.to_lowercase().as_str() {
        "yaml" | "yml" => {
            serde_yaml::from_str(&content)
                .map_err(|e| QuicpulseError::Argument(format!("Failed to parse YAML workflow: {}", e)))?
        }
        "toml" => {
            toml::from_str(&content)
                .map_err(|e| QuicpulseError::Argument(format!("Failed to parse TOML workflow: {}", e)))?
        }
        _ => {
            // Try YAML first, then TOML
            serde_yaml::from_str(&content)
                .or_else(|_| toml::from_str(&content)
                    .map_err(|e| QuicpulseError::Argument(format!("Failed to parse workflow: {}", e))))?
        }
    };

    // Validate basic structure
    validate_workflow_structure(&workflow)?;

    Ok(workflow)
}

/// Validate basic workflow structure
fn validate_workflow_structure(workflow: &Workflow) -> Result<(), QuicpulseError> {
    if workflow.name.is_empty() {
        return Err(QuicpulseError::Argument("Workflow must have a name".to_string()));
    }

    if workflow.steps.is_empty() {
        return Err(QuicpulseError::Argument("Workflow must have at least one step".to_string()));
    }

    // Validate each step has required fields
    for (i, step) in workflow.steps.iter().enumerate() {
        if step.name.is_empty() {
            return Err(QuicpulseError::Argument(format!(
                "Step {} must have a name", i + 1
            )));
        }
        if step.url.is_empty() {
            return Err(QuicpulseError::Argument(format!(
                "Step {} ({}) must have a URL", i + 1, step.name
            )));
        }
    }

    Ok(())
}

/// Apply environment-specific variables to a workflow
pub fn apply_environment(workflow: &mut Workflow, env_name: &str) -> Result<(), QuicpulseError> {
    if let Some(env_vars) = workflow.environments.get(env_name) {
        for (key, value) in env_vars {
            workflow.variables.insert(key.clone(), value.clone());
        }
        Ok(())
    } else if !workflow.environments.is_empty() {
        Err(QuicpulseError::Argument(format!(
            "Environment '{}' not found. Available: {}",
            env_name,
            workflow.environments.keys().cloned().collect::<Vec<_>>().join(", ")
        )))
    } else {
        Ok(())
    }
}

/// Apply CLI variables to a workflow
pub fn apply_cli_variables(workflow: &mut Workflow, vars: &[String]) -> Result<(), QuicpulseError> {
    for var in vars {
        if let Some((key, value)) = var.split_once('=') {
            // Try to parse as JSON, fall back to string
            let json_value = serde_json::from_str(value)
                .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
            workflow.variables.insert(key.to_string(), json_value);
        } else {
            return Err(QuicpulseError::Argument(format!("Invalid variable format: {}. Use NAME=VALUE", var)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yaml_workflow() {
        let yaml = r#"
name: "Test Workflow"
description: "A test workflow"
variables:
  base_url: "https://api.example.com"
steps:
  - name: "Get Users"
    method: GET
    url: "{{base_url}}/users"
    assert:
      status: 200
  - name: "Create User"
    method: POST
    url: "{{base_url}}/users"
    body:
      name: "Test User"
    extract:
      user_id: "response.body.id"
    assert:
      status: 201
"#;

        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.steps.len(), 2);
        assert_eq!(workflow.steps[0].method, "GET");
        assert_eq!(workflow.steps[1].method, "POST");
    }

    #[test]
    fn test_apply_environment() {
        let yaml = r#"
name: "Test"
variables:
  base_url: "http://localhost"
environments:
  production:
    base_url: "https://api.example.com"
  staging:
    base_url: "https://staging.example.com"
steps:
  - name: "Test"
    url: "{{base_url}}/test"
"#;

        let mut workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(workflow.variables["base_url"], "http://localhost");

        apply_environment(&mut workflow, "production").unwrap();
        assert_eq!(workflow.variables["base_url"], "https://api.example.com");
    }

    #[test]
    fn test_apply_cli_variables() {
        let yaml = r#"
name: "Test"
variables:
  token: "default"
steps:
  - name: "Test"
    url: "/test"
"#;

        let mut workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        apply_cli_variables(&mut workflow, &["token=secret123".to_string()]).unwrap();
        assert_eq!(workflow.variables["token"], "secret123");
    }
}
