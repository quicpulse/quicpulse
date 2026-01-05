//! Session management for persistent cookies, headers, and auth
//!
//! Sessions allow persisting request state across HTTP calls.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::NamedTempFile;

use crate::config::Config;
use crate::errors::QuicpulseError;
use crate::sessions::cookies::domain_matches;
use crate::utils::is_localhost;

/// Session file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Metadata about the session
    #[serde(rename = "session_info")]
    pub meta: SessionMeta,
    
    /// Persisted headers
    #[serde(default)]
    pub headers: Vec<SessionHeader>,
    
    /// Persisted cookies
    #[serde(default)]
    pub cookies: Vec<SessionCookie>,
    
    /// Persisted authentication
    #[serde(default)]
    pub auth: Option<SessionAuth>,
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    /// Version of QuicPulse that created this session
    pub client_version: String,
    /// Help URL
    #[serde(default = "default_help_url")]
    pub help: String,
    /// About string
    #[serde(default = "default_about")]
    pub about: String,
}

fn default_help_url() -> String {
    "https://github.com/my-org/quicpulse".to_string()
}

fn default_about() -> String {
    "QuicPulse session file".to_string()
}

/// A session header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeader {
    pub name: String,
    pub value: String,
}

/// A session cookie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCookie {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub expires: Option<i64>,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub http_only: bool,
}

/// Session authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAuth {
    #[serde(rename = "type")]
    pub auth_type: String,
    pub credentials: String,
}

/// Headers that should NOT be persisted in sessions (explicit list approach)
/// These are request-specific headers that shouldn't carry over between requests
const SESSION_EXCLUDED_HEADERS: &[&str] = &[
    "content-type",
    "content-length",
    "content-encoding",
    "content-disposition",
    "content-range",
    "if-match",
    "if-none-match",
    "if-modified-since",
    "if-unmodified-since",
    "if-range",
    "transfer-encoding",
    "host",
    "connection",
    "keep-alive",
];

impl Session {
    /// Create a new empty session
    pub fn new() -> Self {
        Self {
            meta: SessionMeta {
                client_version: env!("CARGO_PKG_VERSION").to_string(),
                help: default_help_url(),
                about: default_about(),
            },
            headers: Vec::new(),
            cookies: Vec::new(),
            auth: None,
        }
    }

    /// Load a session from a file path
    pub fn load(path: &Path) -> Result<Self, QuicpulseError> {
        let content = fs::read_to_string(path)
            .map_err(|e| QuicpulseError::Session(format!("Failed to read session: {}", e)))?;
        
        serde_json::from_str(&content)
            .map_err(|e| QuicpulseError::Session(format!("Failed to parse session: {}", e)))
    }

    /// Load a named session for a host
    pub fn load_named(name: &str, host: &str, config: &Config) -> Result<Self, QuicpulseError> {
        let path = Self::session_path(name, host, config);
        
        if path.exists() {
            Self::load(&path)
        } else {
            // Return a new session if it doesn't exist yet
            Ok(Self::new())
        }
    }

    /// Save the session to a file path
    /// Uses atomic write (write to temp file, then rename) to prevent corruption
    pub fn save(&self, path: &Path) -> Result<(), QuicpulseError> {
        // Ensure parent directory exists
        let parent = path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(parent)
            .map_err(|e| QuicpulseError::Session(format!("Failed to create session directory: {}", e)))?;

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| QuicpulseError::Session(format!("Failed to serialize session: {}", e)))?;

        // Use tempfile for atomic writes - handles temp file creation, cleanup on error,
        // and atomic rename via persist()
        let mut temp = NamedTempFile::new_in(parent)
            .map_err(|e| QuicpulseError::Session(format!("Failed to create temp file: {}", e)))?;

        temp.write_all(content.as_bytes())
            .map_err(|e| QuicpulseError::Session(format!("Failed to write session: {}", e)))?;

        temp.persist(path)
            .map_err(|e| QuicpulseError::Session(format!("Failed to save session: {}", e)))?;

        // Set restrictive permissions (0600) on Unix to prevent other users from reading
        // session files which may contain cookies and auth credentials
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(path, permissions);
        }

        Ok(())
    }

    /// Save a named session for a host
    pub fn save_named(&self, name: &str, host: &str, config: &Config) -> Result<(), QuicpulseError> {
        let path = Self::session_path(name, host, config);
        self.save(&path)
    }

    /// Get the path for a named session
    fn session_path(name: &str, host: &str, config: &Config) -> PathBuf {
        config.sessions_dir()
            .join(host)
            .join(format!("{}.json", name))
    }

    /// Add or update a header (skipping ignored headers)
    pub fn update_header(&mut self, name: &str, value: &str) {
        // Skip headers that shouldn't be persisted
        if Self::should_ignore_header(name) {
            return;
        }

        // Update existing or add new
        if let Some(header) = self.headers.iter_mut().find(|h| h.name.eq_ignore_ascii_case(name)) {
            header.value = value.to_string();
        } else {
            self.headers.push(SessionHeader {
                name: name.to_string(),
                value: value.to_string(),
            });
        }
    }

    /// Remove a header
    pub fn remove_header(&mut self, name: &str) {
        self.headers.retain(|h| !h.name.eq_ignore_ascii_case(name));
    }

    /// Check if a header should be ignored (not persisted)
    fn should_ignore_header(name: &str) -> bool {
        let name_lower = name.to_lowercase();
        SESSION_EXCLUDED_HEADERS.iter().any(|&h| h == name_lower)
    }

    /// Add or update a cookie
    pub fn update_cookie(&mut self, cookie: SessionCookie) {
        // Remove expired cookies first
        self.remove_expired_cookies();

        // Update existing or add new
        if let Some(existing) = self.cookies.iter_mut().find(|c| {
            c.name == cookie.name && c.domain == cookie.domain && c.path == cookie.path
        }) {
            *existing = cookie;
        } else {
            self.cookies.push(cookie);
        }
    }

    /// Remove a cookie
    pub fn remove_cookie(&mut self, name: &str, domain: Option<&str>, path: Option<&str>) {
        self.cookies.retain(|c| {
            !(c.name == name 
                && c.domain.as_deref() == domain 
                && c.path.as_deref() == path)
        });
    }

    /// Remove expired cookies
    pub fn remove_expired_cookies(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        self.cookies.retain(|c| {
            c.expires.map(|exp| exp > now).unwrap_or(true)
        });
    }

    /// Get cookies that apply to a given URL
    pub fn get_cookies_for_url(&self, domain: &str, path: &str, is_secure: bool) -> Vec<&SessionCookie> {
        self.cookies.iter()
            .filter(|c| {
                // Check domain
                let domain_match = c.domain.as_ref()
                    .map(|d| domain_matches(domain, d))
                    .unwrap_or(true);

                // Check path
                let path_match = c.path.as_ref()
                    .map(|p| path.starts_with(p))
                    .unwrap_or(true);

                // Check secure
                let secure_match = !c.secure || is_secure || is_localhost(domain);

                // Check expiration
                let not_expired = c.expires.map(|exp| {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    exp > now
                }).unwrap_or(true);

                domain_match && path_match && secure_match && not_expired
            })
            .collect()
    }

    /// Set authentication
    pub fn set_auth(&mut self, auth_type: &str, credentials: &str) {
        self.auth = Some(SessionAuth {
            auth_type: auth_type.to_string(),
            credentials: credentials.to_string(),
        });
    }

    /// Clear authentication
    pub fn clear_auth(&mut self) {
        self.auth = None;
    }

    /// Get the Cookie header value for a request
    pub fn get_cookie_header(&self, domain: &str, path: &str, is_secure: bool) -> Option<String> {
        let cookies = self.get_cookies_for_url(domain, path, is_secure);
        
        if cookies.is_empty() {
            return None;
        }

        let cookie_str = cookies.iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ");

        Some(cookie_str)
    }

    /// Parse and add cookies from a Set-Cookie header
    pub fn parse_set_cookie(&mut self, set_cookie: &str, default_domain: &str) {
        let parts: Vec<&str> = set_cookie.split(';').map(|s| s.trim()).collect();
        
        if parts.is_empty() {
            return;
        }

        // Parse name=value
        let name_value = parts[0];
        let (name, value) = match name_value.split_once('=') {
            Some((n, v)) => (n.to_string(), v.to_string()),
            None => return,
        };

        let mut cookie = SessionCookie {
            name,
            value,
            domain: Some(default_domain.to_string()),
            path: Some("/".to_string()),
            expires: None,
            secure: false,
            http_only: false,
        };

        // Parse attributes
        for attr in &parts[1..] {
            let (key, val) = attr.split_once('=')
                .map(|(k, v)| (k.to_lowercase(), Some(v)))
                .unwrap_or_else(|| (attr.to_lowercase(), None));

            match key.as_str() {
                "domain" => cookie.domain = val.map(|s| s.to_string()),
                "path" => cookie.path = val.map(|s| s.to_string()),
                "expires" => {
                    if let Some(exp) = val {
                        cookie.expires = parse_cookie_date(exp);
                    }
                }
                "max-age" => {
                    if let Some(age) = val.and_then(|s| s.parse::<i64>().ok()) {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        cookie.expires = Some(now + age);
                    }
                }
                "secure" => cookie.secure = true,
                "httponly" => cookie.http_only = true,
                _ => {}
            }
        }

        self.update_cookie(cookie);
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a cookie date string to Unix timestamp
fn parse_cookie_date(date_str: &str) -> Option<i64> {
    use chrono::{DateTime, NaiveDateTime};

    // Try various cookie date formats
    let formats = [
        // RFC 1123: "Sun, 06 Nov 1994 08:49:37 GMT"
        "%a, %d %b %Y %H:%M:%S GMT",
        "%a, %d %b %Y %H:%M:%S %Z",
        // RFC 850: "Sunday, 06-Nov-94 08:49:37 GMT"
        "%A, %d-%b-%y %H:%M:%S GMT",
        "%A, %d-%b-%y %H:%M:%S %Z",
        // ANSI C: "Sun Nov  6 08:49:37 1994"
        "%a %b %e %H:%M:%S %Y",
        // ISO 8601
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%dT%H:%M:%S%z",
    ];

    for fmt in &formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, fmt) {
            return Some(dt.and_utc().timestamp());
        }
    }

    // Try parsing as RFC 2822 (common email date format)
    if let Ok(dt) = DateTime::parse_from_rfc2822(date_str) {
        return Some(dt.timestamp());
    }

    // Try parsing as RFC 3339/ISO 8601
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.timestamp());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new();
        assert!(session.headers.is_empty());
        assert!(session.cookies.is_empty());
        assert!(session.auth.is_none());
    }

    #[test]
    fn test_header_persistence() {
        let mut session = Session::new();
        
        // Regular headers should be persisted
        session.update_header("X-Custom", "value");
        assert_eq!(session.headers.len(), 1);
        
        // Content-* headers should be ignored
        session.update_header("Content-Type", "application/json");
        assert_eq!(session.headers.len(), 1);
        
        // If-* headers should be ignored
        session.update_header("If-None-Match", "abc123");
        assert_eq!(session.headers.len(), 1);
    }

    #[test]
    fn test_localhost_secure() {
        use crate::utils::is_localhost;
        assert!(is_localhost("localhost"));
        assert!(is_localhost("app.localhost"));
        assert!(is_localhost("127.0.0.1"));
        assert!(!is_localhost("example.com"));
    }

    #[test]
    fn test_parse_set_cookie() {
        let mut session = Session::new();
        session.parse_set_cookie("session_id=abc123; Path=/; Secure", "example.com");

        assert_eq!(session.cookies.len(), 1);
        assert_eq!(session.cookies[0].name, "session_id");
        assert_eq!(session.cookies[0].value, "abc123");
        assert!(session.cookies[0].secure);
    }

    #[test]
    fn test_parse_cookie_date() {
        // RFC 1123 format
        let ts = parse_cookie_date("Sun, 06 Nov 1994 08:49:37 GMT");
        assert!(ts.is_some());
        assert_eq!(ts.unwrap(), 784111777);

        // RFC 2822 format
        let ts = parse_cookie_date("Sun, 06 Nov 1994 08:49:37 +0000");
        assert!(ts.is_some());

        // ISO 8601 format
        let ts = parse_cookie_date("1994-11-06T08:49:37Z");
        assert!(ts.is_some());

        // Invalid date
        let ts = parse_cookie_date("not a date");
        assert!(ts.is_none());
    }

    #[test]
    fn test_cookie_expiration() {
        let mut session = Session::new();

        // Add a cookie with expiration in the past
        session.cookies.push(SessionCookie {
            name: "expired".to_string(),
            value: "old".to_string(),
            domain: Some("example.com".to_string()),
            path: Some("/".to_string()),
            expires: Some(0), // Unix epoch (expired)
            secure: false,
            http_only: false,
        });

        // Add a cookie with no expiration
        session.cookies.push(SessionCookie {
            name: "session".to_string(),
            value: "current".to_string(),
            domain: Some("example.com".to_string()),
            path: Some("/".to_string()),
            expires: None,
            secure: false,
            http_only: false,
        });

        // Remove expired cookies
        session.remove_expired_cookies();

        // Only the non-expired cookie should remain
        assert_eq!(session.cookies.len(), 1);
        assert_eq!(session.cookies[0].name, "session");
    }

    #[test]
    fn test_get_cookie_header() {
        let mut session = Session::new();

        session.cookies.push(SessionCookie {
            name: "foo".to_string(),
            value: "bar".to_string(),
            domain: Some("example.com".to_string()),
            path: Some("/".to_string()),
            expires: None,
            secure: false,
            http_only: false,
        });

        session.cookies.push(SessionCookie {
            name: "baz".to_string(),
            value: "qux".to_string(),
            domain: Some("example.com".to_string()),
            path: Some("/".to_string()),
            expires: None,
            secure: false,
            http_only: false,
        });

        let cookie_header = session.get_cookie_header("example.com", "/", false);
        assert!(cookie_header.is_some());
        let header = cookie_header.unwrap();
        assert!(header.contains("foo=bar"));
        assert!(header.contains("baz=qux"));
    }

    #[test]
    fn test_session_auth() {
        let mut session = Session::new();

        session.set_auth("basic", "user:pass");
        assert!(session.auth.is_some());
        assert_eq!(session.auth.as_ref().unwrap().auth_type, "basic");
        assert_eq!(session.auth.as_ref().unwrap().credentials, "user:pass");

        session.clear_auth();
        assert!(session.auth.is_none());
    }
}
