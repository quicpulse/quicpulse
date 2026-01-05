//! HAR (HTTP Archive) data structures
//!
//! Based on the HAR 1.2 specification: http://www.softwareishard.com/blog/har-12-spec/

use serde::{Deserialize, Serialize};

fn truncate_url(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    if max_len <= 3 {
        return "...".to_string();
    }
    let target_len = max_len - 3;
    let mut truncate_at = target_len.min(s.len());
    while truncate_at > 0 && !s.is_char_boundary(truncate_at) {
        truncate_at -= 1;
    }
    format!("{}...", &s[..truncate_at])
}

/// Root HAR structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Har {
    pub log: HarLog,
}

/// HAR log containing all entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarLog {
    /// HAR format version (e.g., "1.2")
    #[serde(default)]
    pub version: String,

    /// Creator application info
    #[serde(default)]
    pub creator: Option<HarCreator>,

    /// Browser info
    #[serde(default)]
    pub browser: Option<HarBrowser>,

    /// List of HTTP request/response entries
    #[serde(default)]
    pub entries: Vec<HarEntry>,

    /// List of pages (optional)
    #[serde(default)]
    pub pages: Option<Vec<HarPage>>,

    /// Comment (optional)
    #[serde(default)]
    pub comment: Option<String>,
}

/// Creator application info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarCreator {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub comment: Option<String>,
}

/// Browser info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarBrowser {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub comment: Option<String>,
}

/// Page info (for multi-page archives)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarPage {
    #[serde(rename = "startedDateTime")]
    pub started_date_time: String,
    pub id: String,
    pub title: String,
    #[serde(rename = "pageTimings", default)]
    pub page_timings: Option<HarPageTimings>,
    #[serde(default)]
    pub comment: Option<String>,
}

/// Page timings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarPageTimings {
    #[serde(rename = "onContentLoad", default)]
    pub on_content_load: Option<f64>,
    #[serde(rename = "onLoad", default)]
    pub on_load: Option<f64>,
    #[serde(default)]
    pub comment: Option<String>,
}

/// A single HTTP request/response entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarEntry {
    /// Reference to parent page
    #[serde(default)]
    pub pageref: Option<String>,

    /// Request start time (ISO 8601)
    #[serde(rename = "startedDateTime")]
    pub started_date_time: String,

    /// Total time in milliseconds
    #[serde(default)]
    pub time: f64,

    /// Request details
    pub request: HarRequest,

    /// Response details
    pub response: HarResponse,

    /// Cache info
    #[serde(default)]
    pub cache: Option<HarCache>,

    /// Timing breakdown
    #[serde(default)]
    pub timings: Option<HarTimings>,

    /// Server IP address
    #[serde(rename = "serverIPAddress", default)]
    pub server_ip_address: Option<String>,

    /// Connection ID
    #[serde(default)]
    pub connection: Option<String>,

    /// Comment
    #[serde(default)]
    pub comment: Option<String>,
}

/// HTTP request details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarRequest {
    /// HTTP method (GET, POST, etc.)
    pub method: String,

    /// Full URL
    pub url: String,

    /// HTTP version (e.g., "HTTP/1.1")
    #[serde(rename = "httpVersion")]
    pub http_version: String,

    /// Request cookies
    #[serde(default)]
    pub cookies: Vec<HarCookie>,

    /// Request headers
    #[serde(default)]
    pub headers: Vec<HarHeader>,

    /// Query string parameters
    #[serde(rename = "queryString", default)]
    pub query_string: Vec<HarQueryParam>,

    /// POST data
    #[serde(rename = "postData", default)]
    pub post_data: Option<HarPostData>,

    /// Headers size in bytes (-1 if unknown)
    #[serde(rename = "headersSize", default)]
    pub headers_size: i64,

    /// Body size in bytes (-1 if unknown)
    #[serde(rename = "bodySize", default)]
    pub body_size: i64,

    /// Comment
    #[serde(default)]
    pub comment: Option<String>,
}

/// HTTP response details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarResponse {
    /// HTTP status code
    pub status: i32,

    /// Status text
    #[serde(rename = "statusText")]
    pub status_text: String,

    /// HTTP version
    #[serde(rename = "httpVersion")]
    pub http_version: String,

    /// Response cookies
    #[serde(default)]
    pub cookies: Vec<HarCookie>,

    /// Response headers
    #[serde(default)]
    pub headers: Vec<HarHeader>,

    /// Response content
    pub content: HarContent,

    /// Redirect URL
    #[serde(rename = "redirectURL", default)]
    pub redirect_url: String,

    /// Headers size in bytes
    #[serde(rename = "headersSize", default)]
    pub headers_size: i64,

    /// Body size in bytes
    #[serde(rename = "bodySize", default)]
    pub body_size: i64,

    /// Comment
    #[serde(default)]
    pub comment: Option<String>,
}

/// HTTP header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarHeader {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub comment: Option<String>,
}

/// Cookie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarCookie {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub expires: Option<String>,
    #[serde(rename = "httpOnly", default)]
    pub http_only: Option<bool>,
    #[serde(default)]
    pub secure: Option<bool>,
    #[serde(default)]
    pub comment: Option<String>,
}

/// Query parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarQueryParam {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub comment: Option<String>,
}

/// POST data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarPostData {
    /// MIME type
    #[serde(rename = "mimeType")]
    pub mime_type: String,

    /// Posted text (for non-multipart)
    #[serde(default)]
    pub text: Option<String>,

    /// Posted parameters (for form data)
    #[serde(default)]
    pub params: Option<Vec<HarPostParam>>,

    /// Comment
    #[serde(default)]
    pub comment: Option<String>,
}

/// POST parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarPostParam {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(rename = "fileName", default)]
    pub file_name: Option<String>,
    #[serde(rename = "contentType", default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
}

/// Response content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarContent {
    /// Content size in bytes
    #[serde(default)]
    pub size: i64,

    /// Compression savings
    #[serde(default)]
    pub compression: Option<i64>,

    /// MIME type
    #[serde(rename = "mimeType", default)]
    pub mime_type: String,

    /// Response text
    #[serde(default)]
    pub text: Option<String>,

    /// Encoding (e.g., "base64")
    #[serde(default)]
    pub encoding: Option<String>,

    /// Comment
    #[serde(default)]
    pub comment: Option<String>,
}

/// Cache info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarCache {
    #[serde(rename = "beforeRequest", default)]
    pub before_request: Option<HarCacheState>,
    #[serde(rename = "afterRequest", default)]
    pub after_request: Option<HarCacheState>,
    #[serde(default)]
    pub comment: Option<String>,
}

/// Cache state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarCacheState {
    #[serde(default)]
    pub expires: Option<String>,
    #[serde(rename = "lastAccess")]
    pub last_access: String,
    #[serde(rename = "eTag")]
    pub etag: String,
    #[serde(rename = "hitCount")]
    pub hit_count: i32,
    #[serde(default)]
    pub comment: Option<String>,
}

/// Timing breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarTimings {
    /// Time spent in browser queue
    #[serde(default)]
    pub blocked: Option<f64>,

    /// DNS resolution time
    #[serde(default)]
    pub dns: Option<f64>,

    /// Time to establish connection
    #[serde(default)]
    pub connect: Option<f64>,

    /// Time to send request
    #[serde(default)]
    pub send: Option<f64>,

    /// Time waiting for response
    #[serde(default)]
    pub wait: Option<f64>,

    /// Time receiving response
    #[serde(default)]
    pub receive: Option<f64>,

    /// SSL/TLS negotiation time
    #[serde(default)]
    pub ssl: Option<f64>,

    /// Comment
    #[serde(default)]
    pub comment: Option<String>,
}

impl HarEntry {
    /// Get a short description of the entry for display
    pub fn short_description(&self) -> String {
        let url = truncate_url(&self.request.url, 60);
        format!(
            "{} {} → {}",
            self.request.method,
            url,
            self.response.status
        )
    }

    /// Get the content type from request headers
    pub fn content_type(&self) -> Option<String> {
        self.request.headers.iter()
            .find(|h| h.name.eq_ignore_ascii_case("content-type"))
            .map(|h| h.value.clone())
    }
}

impl HarRequest {
    /// Check if this is a GET request
    pub fn is_get(&self) -> bool {
        self.method.eq_ignore_ascii_case("GET")
    }

    /// Check if this is a POST request
    pub fn is_post(&self) -> bool {
        self.method.eq_ignore_ascii_case("POST")
    }

    /// Get header value by name (case-insensitive)
    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
            .map(|h| h.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_har() {
        let json = r#"{
            "log": {
                "version": "1.2",
                "entries": []
            }
        }"#;

        let har: Har = serde_json::from_str(json).unwrap();
        assert_eq!(har.log.version, "1.2");
        assert!(har.log.entries.is_empty());
    }

    #[test]
    fn test_parse_har_entry() {
        let json = r#"{
            "log": {
                "version": "1.2",
                "entries": [{
                    "startedDateTime": "2024-01-01T00:00:00.000Z",
                    "time": 100,
                    "request": {
                        "method": "GET",
                        "url": "https://example.com/api",
                        "httpVersion": "HTTP/1.1",
                        "headers": [
                            {"name": "Accept", "value": "application/json"}
                        ],
                        "queryString": [],
                        "cookies": [],
                        "headersSize": 100,
                        "bodySize": 0
                    },
                    "response": {
                        "status": 200,
                        "statusText": "OK",
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "cookies": [],
                        "content": {
                            "size": 50,
                            "mimeType": "application/json"
                        },
                        "redirectURL": "",
                        "headersSize": 50,
                        "bodySize": 50
                    }
                }]
            }
        }"#;

        let har: Har = serde_json::from_str(json).unwrap();
        assert_eq!(har.log.entries.len(), 1);

        let entry = &har.log.entries[0];
        assert_eq!(entry.request.method, "GET");
        assert_eq!(entry.request.url, "https://example.com/api");
        assert_eq!(entry.response.status, 200);
    }

    #[test]
    fn test_short_description() {
        let entry = HarEntry {
            pageref: None,
            started_date_time: "2024-01-01T00:00:00.000Z".to_string(),
            time: 100.0,
            request: HarRequest {
                method: "POST".to_string(),
                url: "https://api.example.com/users".to_string(),
                http_version: "HTTP/1.1".to_string(),
                cookies: vec![],
                headers: vec![],
                query_string: vec![],
                post_data: None,
                headers_size: 0,
                body_size: 0,
                comment: None,
            },
            response: HarResponse {
                status: 201,
                status_text: "Created".to_string(),
                http_version: "HTTP/1.1".to_string(),
                cookies: vec![],
                headers: vec![],
                content: HarContent {
                    size: 0,
                    compression: None,
                    mime_type: "application/json".to_string(),
                    text: None,
                    encoding: None,
                    comment: None,
                },
                redirect_url: String::new(),
                headers_size: 0,
                body_size: 0,
                comment: None,
            },
            cache: None,
            timings: None,
            server_ip_address: None,
            connection: None,
            comment: None,
        };

        let desc = entry.short_description();
        assert!(desc.contains("POST"));
        assert!(desc.contains("201"));
    }
}
