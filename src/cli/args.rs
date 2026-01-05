//! CLI argument definitions using clap
//!
//! This module defines all command-line arguments for QuicPulse.

use clap::{Parser, ValueEnum, ArgAction};
use std::fmt;
use std::path::PathBuf;

/// A string that redacts its value in Debug output to prevent credential leakage
#[derive(Clone, Default)]
pub struct SecretString(pub String);

impl SecretString {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "SecretString(\"\")")
        } else {
            write!(f, "SecretString(\"[REDACTED]\")")
        }
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "")
        } else {
            write!(f, "[REDACTED]")
        }
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        SecretString(s)
    }
}

impl std::str::FromStr for SecretString {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SecretString(s.to_string()))
    }
}

impl AsRef<str> for SecretString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for SecretString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Default)]
pub struct SensitiveUrl(pub String);

impl SensitiveUrl {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    /// Redact credentials from URL for debug output
    fn redacted(&self) -> String {
        // Try to parse as URL and redact credentials
        if let Ok(mut url) = url::Url::parse(&self.0) {
            if !url.username().is_empty() || url.password().is_some() {
                // Has credentials - redact them
                let _ = url.set_username("[REDACTED]");
                let _ = url.set_password(None);
                return url.to_string();
            }
        }
        // Not a parseable URL or no credentials - return as-is
        self.0.clone()
    }
}

impl fmt::Debug for SensitiveUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SensitiveUrl(\"{}\")", self.redacted())
    }
}

impl fmt::Display for SensitiveUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.redacted())
    }
}

impl From<String> for SensitiveUrl {
    fn from(s: String) -> Self {
        SensitiveUrl(s)
    }
}

impl std::str::FromStr for SensitiveUrl {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SensitiveUrl(s.to_string()))
    }
}

impl AsRef<str> for SensitiveUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for SensitiveUrl {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// QuicPulse - A user-friendly HTTP client for the command line
#[derive(Parser, Debug, Clone)]
#[command(name = "http", version, about, long_about = None)]
#[command(disable_help_flag = true)]  // We use -h for --headers
pub struct Args {
    // Custom help flag (since we disabled the auto one)
    /// Print help information
    #[arg(long = "help", action = ArgAction::Help)]
    pub help: Option<bool>,
    // =========================================================================
    // POSITIONAL ARGUMENTS
    // =========================================================================
    
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    /// Defaults to GET, or POST if data is present
    #[arg(value_name = "METHOD")]
    pub method: Option<String>,

    /// The URL to request (http:// prefix optional)
    /// Supports localhost shorthand: :3000/foo → http://localhost:3000/foo
    #[arg(value_name = "URL")]
    pub url: Option<String>,

    /// Request items: headers (:), query params (==), data (=), JSON (:=), files (@)
    #[arg(value_name = "REQUEST_ITEM")]
    pub request_items: Vec<String>,

    // =========================================================================
    // PREDEFINED CONTENT TYPES
    // =========================================================================
    
    /// (default) Serialize data as JSON object
    #[arg(short = 'j', long = "json", action = ArgAction::SetTrue)]
    pub json: bool,

    /// Serialize data as form fields (application/x-www-form-urlencoded)
    #[arg(short = 'f', long = "form", action = ArgAction::SetTrue)]
    pub form: bool,

    /// Force multipart/form-data encoding
    #[arg(long = "multipart", action = ArgAction::SetTrue)]
    pub multipart: bool,

    /// Custom boundary for multipart requests
    #[arg(long = "boundary", value_name = "BOUNDARY")]
    pub boundary: Option<String>,

    /// Raw request body from string
    #[arg(long = "raw", value_name = "RAW")]
    pub raw: Option<String>,

    // =========================================================================
    // CONTENT PROCESSING
    // =========================================================================
    
    /// Compress request body with deflate. Use -xx to force compression
    #[arg(short = 'x', long = "compress", action = ArgAction::Count)]
    pub compress: u8,

    /// Use chunked transfer encoding
    #[arg(long = "chunked", action = ArgAction::SetTrue)]
    pub chunked: bool,

    // =========================================================================
    // OUTPUT PROCESSING
    // =========================================================================
    
    /// Output formatting: all, colors, format, none
    #[arg(long = "pretty", value_name = "STYLE", value_enum)]
    pub pretty: Option<PrettyOption>,

    /// Color scheme for syntax highlighting
    /// Available: auto, solarized-dark, solarized-light, monokai, onedark, dracula, nord, gruvbox-dark, gruvbox-light
    #[arg(short = 's', long = "style", alias = "theme", value_name = "STYLE")]
    pub style: Option<String>,

    /// Pipe output through a pager (less, more, etc.)
    /// Uses $PAGER environment variable, or 'less -R' by default
    #[arg(long = "pager", action = ArgAction::SetTrue)]
    pub pager: bool,

    /// Custom pager command (overrides $PAGER)
    #[arg(long = "pager-cmd", value_name = "COMMAND")]
    pub pager_cmd: Option<String>,

    /// Disable automatic paging
    #[arg(long = "no-pager", action = ArgAction::SetTrue)]
    pub no_pager: bool,

    /// Disable sorting of headers and JSON keys
    #[arg(long = "unsorted", action = ArgAction::SetTrue)]
    pub unsorted: bool,

    /// Enable sorting of headers and JSON keys
    #[arg(long = "sorted", action = ArgAction::SetTrue)]
    pub sorted: bool,

    /// Override response character encoding
    #[arg(long = "response-charset", value_name = "ENCODING")]
    pub response_charset: Option<String>,

    /// Override response MIME type
    #[arg(long = "response-mime", value_name = "MIME")]
    pub response_mime: Option<String>,

    /// Format options (e.g., json.indent:4)
    #[arg(long = "format-options", value_name = "OPTIONS")]
    pub format_options: Vec<String>,

    // =========================================================================
    // OUTPUT OPTIONS
    // =========================================================================
    
    /// What to print: H(eaders), B(ody), h(response headers), b(response body), m(eta)
    #[arg(short = 'p', long = "print", value_name = "WHAT")]
    pub print: Option<String>,

    /// Print only response headers (shortcut for -p h)
    #[arg(short = 'h', long = "headers", action = ArgAction::SetTrue, conflicts_with = "body")]
    pub headers_only: bool,

    /// Print only response metadata (shortcut for -p m)
    #[arg(short = 'm', long = "meta", action = ArgAction::SetTrue)]
    pub meta: bool,

    /// Print only response body (shortcut for -p b)
    #[arg(short = 'b', long = "body", action = ArgAction::SetTrue, conflicts_with = "headers_only")]
    pub body: bool,

    /// Verbose output. Use -vv for even more verbose
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    pub verbose: u8,

    /// Show intermediary requests/responses (redirects)
    #[arg(long = "all", action = ArgAction::SetTrue)]
    pub all: bool,

    /// Stream response body line by line
    #[arg(short = 'S', long = "stream", action = ArgAction::SetTrue)]
    pub stream: bool,

    /// Output file
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Download mode: save response body to file
    #[arg(short = 'd', long = "download", action = ArgAction::SetTrue)]
    pub download: bool,

    /// Resume partial download
    #[arg(short = 'c', long = "continue", action = ArgAction::SetTrue)]
    pub continue_download: bool,

    /// Suppress output. Use -qq for even more quiet
    #[arg(short = 'q', long = "quiet", action = ArgAction::Count)]
    pub quiet: u8,

    // =========================================================================
    // SESSIONS
    // =========================================================================
    
    /// Named session to create or update
    #[arg(long = "session", value_name = "NAME")]
    pub session: Option<String>,

    /// Named session to read (without updating)
    #[arg(long = "session-read-only", value_name = "NAME")]
    pub session_read_only: Option<String>,

    // =========================================================================
    // AUTHENTICATION
    // =========================================================================
    
    /// Authentication credentials (user:password or token)
    /// Uses SecretString to redact value in debug output
    #[arg(short = 'a', long = "auth", value_name = "CREDENTIALS")]
    pub auth: Option<SecretString>,

    /// Authentication type
    #[arg(short = 'A', long = "auth-type", value_name = "TYPE", value_enum)]
    pub auth_type: Option<AuthType>,

    /// Skip credentials from .netrc
    #[arg(long = "ignore-netrc", action = ArgAction::SetTrue)]
    pub ignore_netrc: bool,

    /// AWS region for SigV4 signing (e.g., us-east-1)
    #[arg(long = "aws-region", value_name = "REGION")]
    pub aws_region: Option<String>,

    /// AWS service name for SigV4 signing (e.g., execute-api, s3)
    #[arg(long = "aws-service", value_name = "SERVICE")]
    pub aws_service: Option<String>,

    /// AWS profile name from ~/.aws/credentials or ~/.aws/config
    #[arg(long = "aws-profile", value_name = "PROFILE")]
    pub aws_profile: Option<String>,

    /// OAuth 2.0 token endpoint URL
    #[arg(long = "oauth-token-url", value_name = "URL")]
    pub oauth_token_url: Option<String>,

    /// OAuth 2.0 authorization endpoint URL (for authorization code flow)
    #[arg(long = "oauth-auth-url", value_name = "URL")]
    pub oauth_auth_url: Option<String>,

    /// OAuth 2.0 device authorization endpoint URL (for device flow)
    #[arg(long = "oauth-device-url", value_name = "URL")]
    pub oauth_device_url: Option<String>,

    /// OAuth 2.0 redirect port for authorization code flow (default: 8080)
    #[arg(long = "oauth-redirect-port", value_name = "PORT", default_value = "8080")]
    pub oauth_redirect_port: u16,

    /// Use PKCE (Proof Key for Code Exchange) with authorization code flow
    #[arg(long = "oauth-pkce", action = ArgAction::SetTrue)]
    pub oauth_pkce: bool,

    /// OAuth 2.0 scope (can be used multiple times)
    #[arg(long = "oauth-scope", value_name = "SCOPE")]
    pub oauth_scopes: Vec<String>,

    // =========================================================================
    // PROTOCOL OPTIONS
    // =========================================================================

    /// Use HTTP/3 (QUIC) protocol
    #[arg(long = "http3", action = ArgAction::SetTrue)]
    pub http3: bool,

    /// Preferred HTTP version (1.0, 1.1, 2, 3)
    #[arg(long = "http-version", value_name = "VERSION")]
    pub http_version: Option<String>,

    // =========================================================================
    // GRAPHQL
    // =========================================================================

    /// GraphQL mode: wrap request as GraphQL query
    #[arg(short = 'G', long = "graphql", action = ArgAction::SetTrue)]
    pub graphql: bool,

    /// GraphQL query string (alternative to query= request item)
    #[arg(long = "graphql-query", value_name = "QUERY")]
    pub graphql_query: Option<String>,

    /// GraphQL operation name
    #[arg(long = "graphql-operation", value_name = "NAME")]
    pub graphql_operation: Option<String>,

    /// Fetch GraphQL schema via introspection
    #[arg(long = "graphql-schema", action = ArgAction::SetTrue)]
    pub graphql_schema: bool,

    // =========================================================================
    // GRPC
    // =========================================================================

    /// gRPC mode: send gRPC request
    #[arg(long = "grpc", action = ArgAction::SetTrue)]
    pub grpc: bool,

    /// Path to .proto file for gRPC
    #[arg(long = "proto", value_name = "FILE")]
    pub proto: Option<PathBuf>,

    /// List available gRPC services (via reflection)
    #[arg(long = "grpc-list", action = ArgAction::SetTrue)]
    pub grpc_list: bool,

    /// Describe a gRPC service or method
    #[arg(long = "grpc-describe", value_name = "SERVICE")]
    pub grpc_describe: Option<String>,

    /// Interactive gRPC REPL mode for exploring services
    #[arg(long = "grpc-interactive", action = ArgAction::SetTrue)]
    pub grpc_interactive: bool,

    /// Use plaintext HTTP/2 (h2c) for gRPC without TLS
    #[arg(long = "grpc-plaintext", action = ArgAction::SetTrue)]
    pub grpc_plaintext: bool,

    // =========================================================================
    // WEBSOCKET
    // =========================================================================

    /// WebSocket mode (auto-detected for ws:// URLs)
    #[arg(long = "ws", action = ArgAction::SetTrue)]
    pub ws: bool,

    /// WebSocket subprotocol to request
    #[arg(long = "ws-subprotocol", value_name = "PROTOCOL")]
    pub ws_subprotocol: Option<String>,

    /// Send message and disconnect
    #[arg(long = "ws-send", value_name = "MESSAGE")]
    pub ws_send: Option<String>,

    /// Interactive WebSocket REPL mode
    #[arg(long = "ws-interactive", action = ArgAction::SetTrue)]
    pub ws_interactive: bool,

    /// Listen mode - receive messages only
    #[arg(long = "ws-listen", action = ArgAction::SetTrue)]
    pub ws_listen: bool,

    /// Binary mode: 'hex' or 'base64'
    #[arg(long = "ws-binary", value_name = "MODE")]
    pub ws_binary: Option<String>,

    /// Enable permessage-deflate compression
    #[arg(long = "ws-compress", action = ArgAction::SetTrue)]
    pub ws_compress: bool,

    /// Maximum messages to receive (0 = unlimited)
    #[arg(long = "ws-max-messages", value_name = "NUM", default_value = "0")]
    pub ws_max_messages: usize,

    /// Ping interval in seconds
    #[arg(long = "ws-ping-interval", value_name = "SECONDS")]
    pub ws_ping_interval: Option<u64>,

    // =========================================================================
    // NETWORK
    // =========================================================================

    /// Build request but don't send it
    #[arg(long = "offline", action = ArgAction::SetTrue)]
    pub offline: bool,

    /// Connect via Unix domain socket (e.g., /var/run/docker.sock)
    #[arg(long = "unix-socket", value_name = "PATH")]
    pub unix_socket: Option<PathBuf>,

    /// Proxy URL (protocol:url format)
    /// Bug #7 fix: Uses SensitiveUrl to redact credentials in debug output
    #[arg(long = "proxy", value_name = "PROXY")]
    pub proxy: Vec<SensitiveUrl>,

    /// SOCKS proxy URL (socks4://host:port, socks5://host:port)
    /// Supports SOCKS4, SOCKS4a, SOCKS5, and SOCKS5h protocols
    #[arg(long = "socks", alias = "socks-proxy", value_name = "URL")]
    pub socks_proxy: Option<SensitiveUrl>,

    // =========================================================================
    // LOW-LEVEL NETWORK CONTROLS
    // =========================================================================

    /// Custom DNS resolution (HOST:PORT:ADDRESS)
    /// Example: --resolve example.com:443:127.0.0.1
    #[arg(long = "resolve", value_name = "HOST:PORT:ADDRESS")]
    pub resolve: Vec<String>,

    /// Bind to network interface
    /// Example: --interface eth0 or --interface 192.168.1.100
    #[arg(long = "interface", value_name = "INTERFACE")]
    pub interface: Option<String>,

    /// Local port range for outgoing connections
    /// Example: --local-port 40000-50000
    #[arg(long = "local-port", value_name = "PORT[-PORT]")]
    pub local_port: Option<String>,

    /// Enable TCP Fast Open (TFO)
    #[arg(long = "tcp-fastopen", action = ArgAction::SetTrue)]
    pub tcp_fastopen: bool,

    /// Set local address to bind to
    /// Example: --local-address 192.168.1.100
    #[arg(long = "local-address", value_name = "ADDRESS")]
    pub local_address: Option<String>,

    /// Follow redirects
    #[arg(short = 'F', long = "follow", action = ArgAction::SetTrue)]
    pub follow: bool,

    /// Maximum number of redirects (default: 30)
    #[arg(long = "max-redirects", value_name = "NUM", default_value = "30")]
    pub max_redirects: u32,

    /// Maximum number of headers to accept
    #[arg(long = "max-headers", value_name = "NUM")]
    pub max_headers: Option<u32>,

    /// Connection timeout in seconds
    #[arg(long = "timeout", value_name = "SECONDS")]
    pub timeout: Option<f64>,

    /// Exit with error on HTTP error status codes (4xx, 5xx)
    #[arg(long = "check-status", action = ArgAction::SetTrue)]
    pub check_status: bool,

    /// Don't normalize URL path (keep .., etc.)
    #[arg(long = "path-as-is", action = ArgAction::SetTrue)]
    pub path_as_is: bool,

    // =========================================================================
    // SSL
    // =========================================================================
    
    /// SSL certificate verification: yes/no/path-to-CA-bundle
    #[arg(long = "verify", value_name = "VERIFY", default_value = "yes")]
    pub verify: String,

    /// Minimum TLS version (tls1, tls1.1, tls1.2, tls1.3)
    #[arg(long = "ssl", value_name = "VERSION")]
    pub ssl: Option<String>,

    /// Cipher suite specification
    #[arg(long = "ciphers", value_name = "CIPHERS")]
    pub ciphers: Option<String>,

    /// Client certificate file
    #[arg(long = "cert", value_name = "FILE")]
    pub cert: Option<PathBuf>,

    /// Client private key file
    #[arg(long = "cert-key", value_name = "FILE")]
    pub cert_key: Option<PathBuf>,

    /// Passphrase for encrypted client key
    #[arg(long = "cert-key-pass", value_name = "PASS")]
    pub cert_key_pass: Option<String>,

    // =========================================================================
    // CI/CD & AUTOMATION
    // =========================================================================

    /// Force disable colors in output
    #[arg(long = "no-color", action = ArgAction::SetTrue)]
    pub no_color: bool,

    /// Fail if stdout is not a TTY (for safety in CI environments)
    #[arg(long = "strict-tty", action = ArgAction::SetTrue)]
    pub strict_tty: bool,

    /// Output format for structured logging: json (JSON Lines) or text (default)
    #[arg(long = "log-format", value_name = "FORMAT", value_enum)]
    pub log_format: Option<LogFormat>,

    // =========================================================================
    // TROUBLESHOOTING
    // =========================================================================

    /// Don't read stdin (useful for scripting)
    #[arg(short = 'I', long = "ignore-stdin", action = ArgAction::SetTrue)]
    pub ignore_stdin: bool,

    /// Default URL scheme when not specified (http or https)
    #[arg(long = "default-scheme", value_name = "SCHEME", default_value = "http")]
    pub default_scheme: String,

    /// Show traceback on error
    #[arg(long = "traceback", action = ArgAction::SetTrue)]
    pub traceback: bool,

    /// Debug mode (implies --traceback)
    #[arg(long = "debug", action = ArgAction::SetTrue)]
    pub debug: bool,

    // =========================================================================
    // SELF-UPDATE
    // =========================================================================

    /// Update quicpulse to the latest version
    #[arg(long = "update", action = ArgAction::SetTrue)]
    pub update: bool,

    // =========================================================================
    // BENCHMARKING (Phase 11)
    // =========================================================================

    /// Enable benchmarking mode: send multiple concurrent requests
    #[arg(long = "bench", action = ArgAction::SetTrue)]
    pub bench: bool,

    /// Number of requests to send in benchmark mode (default: 100)
    #[arg(long = "requests", value_name = "NUM", default_value = "100")]
    pub bench_requests: u32,

    /// Number of concurrent requests in benchmark mode (default: 10)
    #[arg(long = "concurrency", value_name = "NUM", default_value = "10")]
    pub bench_concurrency: u32,

    // =========================================================================
    // DATA FILTERING & FORMATTING (Phase 11)
    // =========================================================================

    /// JQ filter expression to apply to JSON response
    #[arg(long = "filter", short = 'J', value_name = "EXPR")]
    pub filter: Option<String>,

    /// Output JSON array as ASCII table
    #[arg(long = "table", action = ArgAction::SetTrue)]
    pub table: bool,

    /// Output JSON array as CSV
    #[arg(long = "csv", action = ArgAction::SetTrue)]
    pub csv: bool,

    // =========================================================================
    // ASSERTIONS (Phase 12)
    // =========================================================================

    /// Assert response status code (e.g., 200, 2xx, 200-299)
    #[arg(long = "assert-status", value_name = "CODE")]
    pub assert_status: Option<String>,

    /// Assert response time (e.g., "<500ms", "<2s")
    #[arg(long = "assert-time", value_name = "DURATION")]
    pub assert_time: Option<String>,

    /// Assert response body contains pattern (JQ expression or literal)
    #[arg(long = "assert-body", value_name = "PATTERN")]
    pub assert_body: Option<String>,

    /// Assert response header exists and optionally matches value
    #[arg(long = "assert-header", value_name = "HEADER[:VALUE]")]
    pub assert_header: Vec<String>,

    #[arg(long = "script-allow-dir", value_name = "DIR")]
    pub script_allow_dirs: Vec<PathBuf>,

    // =========================================================================
    // WORKFLOW PIPELINES (Phase 12)
    // =========================================================================

    /// Run a workflow file (YAML/TOML)
    #[arg(long = "run", value_name = "FILE")]
    pub run_workflow: Option<PathBuf>,

    /// Environment/profile to use for workflow variables
    #[arg(long = "env", value_name = "NAME")]
    pub workflow_env: Option<String>,

    /// Set workflow variable (can be used multiple times)
    #[arg(long = "var", value_name = "NAME=VALUE")]
    pub workflow_vars: Vec<String>,

    /// Dry-run workflow (show steps without executing)
    #[arg(long = "dry-run", action = ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Continue workflow execution even if a step fails
    #[arg(long = "continue-on-failure", action = ArgAction::SetTrue)]
    pub continue_on_failure: bool,

    /// Number of retries for failed workflow steps
    #[arg(long = "workflow-retries", value_name = "NUM", default_value = "0")]
    pub workflow_retries: u32,

    /// Show verbose progress during workflow execution
    #[arg(long = "workflow-verbose", action = ArgAction::SetTrue)]
    pub workflow_verbose: bool,

    /// Validate workflow file without executing
    #[arg(long = "validate", action = ArgAction::SetTrue)]
    pub validate_workflow: bool,

    /// Run only steps with these tags (comma-separated)
    #[arg(long = "tags", value_delimiter = ',', value_name = "TAGS")]
    pub workflow_step_tags: Vec<String>,

    /// Include only these steps by name (comma-separated)
    #[arg(long = "include", value_delimiter = ',', value_name = "STEPS")]
    pub workflow_include: Vec<String>,

    /// Exclude steps matching these patterns (comma-separated, supports regex)
    #[arg(long = "exclude", value_delimiter = ',', value_name = "PATTERNS")]
    pub workflow_exclude: Vec<String>,

    /// Save response data from each step to this directory
    /// Filename template: {step_name}_{status}_{timestamp}.json
    #[arg(long = "save-responses", value_name = "DIR")]
    pub save_responses: Option<PathBuf>,

    /// Generate JUnit XML report (for CI/CD integration)
    #[arg(long = "report-junit", value_name = "FILE")]
    pub report_junit: Option<PathBuf>,

    /// Generate JSON report
    #[arg(long = "report-json", value_name = "FILE")]
    pub report_json: Option<PathBuf>,

    /// Generate TAP (Test Anything Protocol) report
    #[arg(long = "report-tap", value_name = "FILE")]
    pub report_tap: Option<PathBuf>,

    // =========================================================================
    // WORKFLOW SHARING & COLLABORATION
    // =========================================================================

    /// List available workflows (local and remote)
    #[arg(long = "workflow-list", action = ArgAction::SetTrue)]
    pub workflow_list: bool,

    /// Pull a workflow from remote registry
    #[arg(long = "workflow-pull", value_name = "URL_OR_NAME")]
    pub workflow_pull: Option<String>,

    /// Push a workflow to remote registry
    #[arg(long = "workflow-push", value_name = "FILE")]
    pub workflow_push: Option<PathBuf>,

    /// Publish workflow publicly (with --workflow-push)
    #[arg(long = "workflow-public", action = ArgAction::SetTrue)]
    pub workflow_public: bool,

    /// Tags for published workflow (comma-separated)
    #[arg(long = "workflow-tags", value_name = "TAGS")]
    pub workflow_tags: Option<String>,

    /// Description for published workflow
    #[arg(long = "workflow-description", value_name = "DESC")]
    pub workflow_description: Option<String>,

    /// Remote registry URL (default: GitHub gists)
    #[arg(long = "workflow-registry", value_name = "URL")]
    pub workflow_registry: Option<String>,

    /// Search remote workflows
    #[arg(long = "workflow-search", value_name = "QUERY")]
    pub workflow_search: Option<String>,

    // =========================================================================
    // SECURITY FUZZING (Phase 13)
    // =========================================================================

    /// Enable fuzzing mode: send mutated payloads to test for vulnerabilities
    #[arg(long = "fuzz", action = ArgAction::SetTrue)]
    pub fuzz: bool,

    /// Fields to fuzz (can be used multiple times, default: all data fields)
    #[arg(long = "fuzz-field", value_name = "FIELD")]
    pub fuzz_fields: Vec<String>,

    /// Fuzz categories to use (sql, xss, cmd, path, boundary, type, format, int, unicode, nosql)
    #[arg(long = "fuzz-category", value_name = "CATEGORY")]
    pub fuzz_categories: Vec<String>,

    /// Concurrency for fuzz requests (default: 10)
    #[arg(long = "fuzz-concurrency", value_name = "NUM", default_value = "10")]
    pub fuzz_concurrency: usize,

    /// Minimum risk level for payloads (1-5, default: 1)
    #[arg(long = "fuzz-risk", value_name = "LEVEL", default_value = "1")]
    pub fuzz_risk: u8,

    /// Only show anomalies (5xx errors, timeouts)
    #[arg(long = "fuzz-anomalies-only", action = ArgAction::SetTrue)]
    pub fuzz_anomalies_only: bool,

    /// Stop fuzzing on first anomaly found
    #[arg(long = "fuzz-stop-on-anomaly", action = ArgAction::SetTrue)]
    pub fuzz_stop_on_anomaly: bool,

    /// Custom fuzzing dictionary file (one payload per line)
    #[arg(long = "fuzz-dict", value_name = "FILE")]
    pub fuzz_dict: Option<PathBuf>,

    /// Custom fuzzing payload (can be used multiple times)
    #[arg(long = "fuzz-payload", value_name = "PAYLOAD")]
    pub fuzz_payloads: Vec<String>,

    // =========================================================================
    // HAR REPLAY (Phase 15)
    // =========================================================================

    /// Import and replay requests from a HAR file (from browser DevTools)
    #[arg(long = "import-har", value_name = "FILE")]
    pub import_har: Option<PathBuf>,

    /// Interactive mode: select which requests to replay from HAR
    #[arg(long = "har-interactive", action = ArgAction::SetTrue)]
    pub har_interactive: bool,

    /// Filter HAR requests by URL pattern (regex)
    #[arg(long = "har-filter", value_name = "PATTERN")]
    pub har_filter: Option<String>,

    /// Delay between replayed HAR requests (e.g., "100ms", "1s")
    #[arg(long = "har-delay", value_name = "DURATION")]
    pub har_delay: Option<String>,

    /// Only show HAR entries without replaying (list mode)
    #[arg(long = "har-list", action = ArgAction::SetTrue)]
    pub har_list: bool,

    /// Replay specific request by index (1-based, can be used multiple times)
    #[arg(long = "har-index", value_name = "INDEX")]
    pub har_indices: Vec<usize>,

    // =========================================================================
    // OPENAPI IMPORT (Phase 18)
    // =========================================================================

    /// Import OpenAPI/Swagger specification and generate workflow
    #[arg(long = "import-openapi", value_name = "FILE")]
    pub import_openapi: Option<PathBuf>,

    /// Output file for generated workflow (default: stdout)
    #[arg(long = "generate-workflow", value_name = "FILE")]
    pub generate_workflow: Option<PathBuf>,

    /// Base URL override for generated workflow
    #[arg(long = "openapi-base-url", value_name = "URL")]
    pub openapi_base_url: Option<String>,

    /// Include deprecated endpoints in generated workflow
    #[arg(long = "openapi-include-deprecated", action = ArgAction::SetTrue)]
    pub openapi_include_deprecated: bool,

    /// Filter endpoints by tag (can be used multiple times)
    #[arg(long = "openapi-tag", value_name = "TAG")]
    pub openapi_tags: Vec<String>,

    /// Exclude endpoints by tag (can be used multiple times)
    #[arg(long = "openapi-exclude-tag", value_name = "TAG")]
    pub openapi_exclude_tags: Vec<String>,

    /// Include fuzz test payloads in generated workflow
    #[arg(long = "openapi-fuzz", action = ArgAction::SetTrue)]
    pub openapi_fuzz: bool,

    /// List all endpoints in the OpenAPI spec without generating workflow
    #[arg(long = "openapi-list", action = ArgAction::SetTrue)]
    pub openapi_list: bool,

    // =========================================================================
    // DEVELOPER EXPERIENCE (Phase 16)
    // =========================================================================

    /// Print equivalent curl command instead of sending request
    #[arg(long = "curl", action = ArgAction::SetTrue)]
    pub curl: bool,

    /// Import and execute a curl command
    #[arg(long = "import-curl", value_name = "COMMAND")]
    pub import_curl: Option<String>,

    /// Import and execute requests from a .http/.rest file
    #[arg(long = "http-file", value_name = "FILE")]
    pub http_file: Option<PathBuf>,

    /// Run specific request(s) from .http file by name or index (1-based)
    #[arg(long = "http-request", value_name = "NAME|INDEX")]
    pub http_request: Option<String>,

    /// List all requests in a .http file
    #[arg(long = "http-list", action = ArgAction::SetTrue)]
    pub http_list: bool,

    /// Generate code snippet in specified language (python, node, go, java, php, rust, ruby)
    #[arg(long = "generate", value_name = "LANGUAGE")]
    pub generate_code: Option<String>,

    /// Load environment variables from .env file
    #[arg(long = "env-file", value_name = "FILE")]
    pub env_file: Option<PathBuf>,

    /// Disable auto-loading of .env file from current directory
    #[arg(long = "no-env", action = ArgAction::SetTrue)]
    pub no_env: bool,

    // =========================================================================
    // MOCK SERVER
    // =========================================================================

    /// Start a mock HTTP server
    #[arg(long = "mock", alias = "serve", action = ArgAction::SetTrue)]
    pub mock_server: bool,

    /// Mock server config file (YAML, JSON, or TOML)
    #[arg(long = "mock-config", value_name = "FILE")]
    pub mock_config: Option<PathBuf>,

    /// Mock server port (default: 8080)
    #[arg(long = "mock-port", value_name = "PORT")]
    pub mock_port: Option<u16>,

    /// Mock server route: METHOD:PATH:BODY (can be used multiple times)
    #[arg(long = "mock-route", value_name = "ROUTE")]
    pub mock_routes: Vec<String>,

    /// Enable CORS on mock server
    #[arg(long = "mock-cors", action = ArgAction::SetTrue)]
    pub mock_cors: bool,

    /// Simulate latency (min-max ms, e.g., "50-200")
    #[arg(long = "mock-latency", value_name = "MS")]
    pub mock_latency: Option<String>,

    /// Mock server bind host (default: 127.0.0.1)
    #[arg(long = "mock-host", value_name = "HOST")]
    pub mock_host: Option<String>,

    /// Log mock server requests to stderr
    #[arg(long = "mock-log", action = ArgAction::SetTrue)]
    pub mock_log: bool,

    /// Record requests to HAR file
    #[arg(long = "mock-record", value_name = "FILE")]
    pub mock_record: Option<PathBuf>,

    /// TLS certificate for HTTPS mock server
    #[arg(long = "mock-tls-cert", value_name = "FILE")]
    pub mock_tls_cert: Option<PathBuf>,

    /// TLS private key for HTTPS mock server
    #[arg(long = "mock-tls-key", value_name = "FILE")]
    pub mock_tls_key: Option<PathBuf>,

    /// Proxy unmatched requests to another server
    #[arg(long = "mock-proxy", value_name = "URL")]
    pub mock_proxy: Option<String>,

    // =========================================================================
    // PLUGIN ECOSYSTEM
    // =========================================================================

    /// List installed plugins
    #[arg(long = "plugin-list", alias = "plugins", action = ArgAction::SetTrue)]
    pub plugin_list: bool,

    /// Install a plugin from registry or git URL
    #[arg(long = "plugin-install", value_name = "NAME_OR_URL")]
    pub plugin_install: Option<String>,

    /// Uninstall a plugin
    #[arg(long = "plugin-uninstall", value_name = "NAME")]
    pub plugin_uninstall: Option<String>,

    /// Search for plugins in registry
    #[arg(long = "plugin-search", value_name = "QUERY")]
    pub plugin_search: Option<String>,

    /// Update installed plugins
    #[arg(long = "plugin-update", action = ArgAction::SetTrue)]
    pub plugin_update: bool,

    /// Plugin directory
    #[arg(long = "plugin-dir", value_name = "DIR")]
    pub plugin_dir: Option<PathBuf>,

    /// Enable specific plugin(s) for this request (comma-separated or repeated)
    #[arg(long = "plugin", value_name = "NAME")]
    pub enabled_plugins: Vec<String>,

    // =========================================================================
    // GENERATION (hidden)
    // =========================================================================

    /// Generate shell completions for the specified shell
    #[arg(long = "generate-completions", value_name = "SHELL", value_enum, hide = true)]
    pub generate_completions: Option<Shell>,

    /// Generate man page to stdout
    #[arg(long = "generate-manpage", action = ArgAction::SetTrue, hide = true)]
    pub generate_manpage: bool,
}

/// Shell types for completion generation
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

/// Log format for structured output (CI/CD)
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum LogFormat {
    /// Plain text output (default)
    #[default]
    Text,
    /// JSON Lines format for parsing
    Json,
}

// Note: PrettyOption is defined in output::options and re-exported from output module
pub use crate::output::PrettyOption;

/// Authentication type
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum AuthType {
    /// HTTP Basic authentication
    Basic,
    /// HTTP Digest authentication
    Digest,
    /// Bearer token authentication
    Bearer,
    /// AWS Signature Version 4
    #[value(name = "aws-sigv4", alias = "aws")]
    AwsSigv4,
    /// Google Cloud Platform (gcloud auth print-access-token)
    #[value(name = "gcp", alias = "google")]
    Gcp,
    /// Azure CLI (az account get-access-token)
    #[value(name = "azure", alias = "az")]
    Azure,
    /// OAuth 2.0 Client Credentials flow
    #[value(name = "oauth2", alias = "oauth")]
    OAuth2,
    /// OAuth 2.0 Authorization Code flow (with optional PKCE)
    #[value(name = "oauth2-auth-code", alias = "oauth-code")]
    OAuth2AuthCode,
    /// OAuth 2.0 Device Authorization flow
    #[value(name = "oauth2-device", alias = "oauth-device")]
    OAuth2Device,
    /// NTLM authentication (Windows Integrated Auth)
    Ntlm,
    /// Negotiate authentication (Kerberos/NTLM auto-select)
    Negotiate,
    /// Kerberos authentication
    Kerberos,
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::Basic
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            help: None,
            method: None,
            url: None,
            request_items: Vec::new(),
            json: false,
            form: false,
            multipart: false,
            boundary: None,
            raw: None,
            compress: 0,
            chunked: false,
            pretty: None,
            style: None,
            pager: false,
            pager_cmd: None,
            no_pager: false,
            unsorted: false,
            sorted: false,
            response_charset: None,
            response_mime: None,
            format_options: Vec::new(),
            print: None,
            headers_only: false,
            meta: false,
            body: false,
            verbose: 0,
            all: false,
            stream: false,
            output: None,
            download: false,
            continue_download: false,
            quiet: 0,
            session: None,
            session_read_only: None,
            auth: None,
            auth_type: None,
            ignore_netrc: false,
            aws_region: None,
            aws_service: None,
            aws_profile: None,
            oauth_token_url: None,
            oauth_auth_url: None,
            oauth_device_url: None,
            oauth_redirect_port: 8080,
            oauth_pkce: false,
            oauth_scopes: Vec::new(),
            http3: false,
            http_version: None,
            graphql: false,
            graphql_query: None,
            graphql_operation: None,
            graphql_schema: false,
            grpc: false,
            proto: None,
            grpc_list: false,
            grpc_describe: None,
            grpc_interactive: false,
            grpc_plaintext: false,
            ws: false,
            ws_subprotocol: None,
            ws_send: None,
            ws_interactive: false,
            ws_listen: false,
            ws_binary: None,
            ws_compress: false,
            ws_max_messages: 0,
            ws_ping_interval: None,
            offline: false,
            unix_socket: None,
            proxy: Vec::new(), // Vec<SensitiveUrl> - defaults to empty
            socks_proxy: None,
            resolve: Vec::new(),
            interface: None,
            local_port: None,
            tcp_fastopen: false,
            local_address: None,
            follow: false,
            max_redirects: 30,
            max_headers: None,
            timeout: None,
            check_status: false,
            path_as_is: false,
            verify: "yes".to_string(),
            ssl: None,
            ciphers: None,
            cert: None,
            cert_key: None,
            cert_key_pass: None,
            no_color: false,
            strict_tty: false,
            log_format: None,
            ignore_stdin: false,
            default_scheme: "http".to_string(),
            traceback: false,
            debug: false,
            update: false,
            bench: false,
            bench_requests: 100,
            bench_concurrency: 10,
            filter: None,
            table: false,
            csv: false,
            assert_status: None,
            assert_time: None,
            assert_body: None,
            assert_header: Vec::new(),
            script_allow_dirs: Vec::new(),
            run_workflow: None,
            workflow_env: None,
            workflow_vars: Vec::new(),
            dry_run: false,
            continue_on_failure: false,
            workflow_retries: 0,
            workflow_verbose: false,
            validate_workflow: false,
            workflow_step_tags: Vec::new(),
            workflow_include: Vec::new(),
            workflow_exclude: Vec::new(),
            save_responses: None,
            report_junit: None,
            report_json: None,
            report_tap: None,
            workflow_list: false,
            workflow_pull: None,
            workflow_push: None,
            workflow_public: false,
            workflow_tags: None,
            workflow_description: None,
            workflow_registry: None,
            workflow_search: None,
            fuzz: false,
            fuzz_fields: Vec::new(),
            fuzz_categories: Vec::new(),
            fuzz_concurrency: 10,
            fuzz_risk: 1,
            fuzz_anomalies_only: false,
            fuzz_stop_on_anomaly: false,
            fuzz_dict: None,
            fuzz_payloads: Vec::new(),
            import_har: None,
            har_interactive: false,
            har_filter: None,
            har_delay: None,
            har_list: false,
            har_indices: Vec::new(),
            import_openapi: None,
            generate_workflow: None,
            openapi_base_url: None,
            openapi_include_deprecated: false,
            openapi_tags: Vec::new(),
            openapi_exclude_tags: Vec::new(),
            openapi_fuzz: false,
            openapi_list: false,
            curl: false,
            import_curl: None,
            http_file: None,
            http_request: None,
            http_list: false,
            generate_code: None,
            env_file: None,
            no_env: false,
            mock_server: false,
            mock_config: None,
            mock_port: None,
            mock_routes: Vec::new(),
            mock_cors: false,
            mock_latency: None,
            mock_host: None,
            mock_log: false,
            mock_record: None,
            mock_tls_cert: None,
            mock_tls_key: None,
            mock_proxy: None,
            plugin_list: false,
            plugin_install: None,
            plugin_uninstall: None,
            plugin_search: None,
            plugin_update: false,
            plugin_dir: None,
            enabled_plugins: Vec::new(),
            generate_completions: None,
            generate_manpage: false,
        }
    }
}
