//! Core data types and type aliases
//!
//! This module defines the primary data structures used throughout the application.
//!
//! # Why IndexMap?
//!
//! We use [`IndexMap`] for all user-facing dictionaries to preserve insertion order.
//! While HTTP semantics don't require ordering for query params or form data,
//! users expect their specified order to be preserved in request output and
//! serialization. This provides predictable, reproducible behavior.

use indexmap::IndexMap;
use serde_json::Value as JsonValue;
use std::path::PathBuf;

// =============================================================================
// TYPE ALIASES
// =============================================================================
// Note: IndexMap preserves insertion order, which provides better UX for
// user-specified data even when not strictly required by protocol.

/// Headers dictionary - header name to list of values (for multiple occurrences)
/// Uses IndexMap because HTTP header order can affect behavior in some servers.
pub type HeadersDict = IndexMap<String, Vec<String>>;

/// Query parameters dictionary - preserves user-specified order for predictable output
pub type QueryParamsDict = IndexMap<String, String>;

/// Request data dictionary (for form data) - preserves user-specified order
pub type RequestDataDict = IndexMap<String, String>;

/// Request JSON dictionary - preserves key order for consistent serialization
pub type RequestJsonDict = IndexMap<String, JsonValue>;

/// Multipart value type
#[derive(Debug, Clone)]
pub enum MultipartValue {
    /// Text field
    Text(String),
    /// File upload (filename, content_type, data)
    File {
        filename: String,
        content_type: String,
        path: PathBuf,
    },
}

/// Multipart data dictionary
pub type MultipartDict = IndexMap<String, MultipartValue>;

// =============================================================================
// MESSAGE TYPES
// =============================================================================

/// Type of HTTP message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMessageKind {
    Request,
    Response,
}

// =============================================================================
// OUTPUT OPTIONS
// =============================================================================

/// Print option characters for --print flag (H=request headers, B=request body, etc.)
pub const PRINT_REQUEST_HEADERS: char = 'H';
pub const PRINT_REQUEST_BODY: char = 'B';
pub const PRINT_RESPONSE_HEADERS: char = 'h';
pub const PRINT_RESPONSE_BODY: char = 'b';
pub const PRINT_RESPONSE_META: char = 'm';

// =============================================================================
// REQUEST TYPE
// =============================================================================

/// Request body type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestType {
    /// JSON body (application/json)
    #[default]
    Json,
    /// Form body (application/x-www-form-urlencoded)
    Form,
    /// Multipart body (multipart/form-data)
    Multipart,
}

// =============================================================================
// LOGGING
// =============================================================================

/// Log level for messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    /// Get the string prefix for this log level
    pub fn prefix(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
        }
    }
}

// =============================================================================
// PROCESSING OPTIONS
// =============================================================================

/// Processing options for output formatting
#[derive(Debug, Clone)]
pub struct ProcessingOptions {
    /// Stream output (line by line)
    pub stream: bool,
    /// Force JSON interpretation (--json flag)
    pub json: bool,
    /// Color style name
    pub style: String,
    /// Override response MIME type
    pub response_mime: Option<String>,
    /// Override response character set
    pub response_charset: Option<String>,
    /// Format-specific options
    pub format_options: FormatOptions,
    /// Show tracebacks on error
    pub show_traceback: bool,
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            stream: false,
            json: false,
            style: "auto".to_string(),
            response_mime: None,
            response_charset: None,
            format_options: FormatOptions::default(),
            show_traceback: false,
        }
    }
}

/// Format-specific options
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Sort headers alphabetically
    pub headers_sort: bool,
    /// Format JSON output
    pub json_format: bool,
    /// JSON indentation level
    pub json_indent: u8,
    /// Sort JSON keys
    pub json_sort_keys: bool,
    /// Format XML output
    pub xml_format: bool,
    /// XML indentation level
    pub xml_indent: u8,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            headers_sort: false,
            json_format: true,
            json_indent: 4,
            json_sort_keys: true,
            xml_format: true,
            xml_indent: 2,
        }
    }
}

// =============================================================================
// HTTP MESSAGE
// =============================================================================

/// A prepared HTTP request
#[derive(Debug, Clone)]
pub struct PreparedRequest {
    /// HTTP method
    pub method: String,
    /// Request URL
    pub url: String,
    /// Request headers
    pub headers: IndexMap<String, String>,
    /// Request body (if any)
    pub body: Option<Vec<u8>>,
}

/// HTTP response data
#[derive(Debug, Clone)]
pub struct ResponseData {
    /// HTTP status code
    pub status: u16,
    /// Status reason phrase
    pub reason: String,
    /// Response headers
    pub headers: IndexMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
    /// Response encoding
    pub encoding: Option<String>,
}

// =============================================================================
// HTTP VERSION
// =============================================================================

/// HTTP version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpVersion {
    Http10,
    #[default]
    Http11,
    Http2,
    Http3,
}

impl HttpVersion {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "1.0" | "http/1.0" => Some(HttpVersion::Http10),
            "1.1" | "http/1.1" => Some(HttpVersion::Http11),
            "2" | "2.0" | "http/2" | "http/2.0" => Some(HttpVersion::Http2),
            "3" | "http/3" => Some(HttpVersion::Http3),
            _ => None,
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpVersion::Http10 => "HTTP/1.0",
            HttpVersion::Http11 => "HTTP/1.1",
            HttpVersion::Http2 => "HTTP/2",
            HttpVersion::Http3 => "HTTP/3",
        }
    }
}
