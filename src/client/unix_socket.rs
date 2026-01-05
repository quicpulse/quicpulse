//! Unix Domain Socket HTTP Client
//!
//! Provides HTTP request capabilities over Unix domain sockets.
//! This is commonly used for local services like Docker, PostgreSQL, etc.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::timeout;

use crate::errors::QuicpulseError;

/// Response from a Unix socket HTTP request
#[derive(Debug, Clone)]
pub struct UnixSocketResponse {
    /// HTTP status code
    pub status: u16,
    /// HTTP status message
    pub status_text: String,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
    /// HTTP version (e.g., "HTTP/1.1")
    pub http_version: String,
}

impl UnixSocketResponse {
    /// Get header value (case-insensitive)
    pub fn header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.headers.iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }

    /// Get Content-Type header
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    /// Get body as string (assumes UTF-8)
    pub fn text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body.clone())
    }
}

/// Send an HTTP request over a Unix domain socket
pub async fn send_request(
    socket_path: &Path,
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
    request_timeout: Option<Duration>,
) -> Result<UnixSocketResponse, QuicpulseError> {
    // Connect to the Unix socket
    let stream = UnixStream::connect(socket_path).await
        .map_err(|e| QuicpulseError::Connection(format!(
            "Failed to connect to Unix socket '{}': {}",
            socket_path.display(), e
        )))?;

    // Build the HTTP request
    let mut request = format!("{} {} HTTP/1.1\r\n", method, path);

    // Add Host header (required for HTTP/1.1)
    let mut has_host = false;
    for (name, value) in headers {
        if name.to_lowercase() == "host" {
            has_host = true;
        }
        request.push_str(&format!("{}: {}\r\n", name, value));
    }

    if !has_host {
        // Use localhost as default host for Unix sockets
        request.push_str("Host: localhost\r\n");
    }

    // Add Content-Length if body is present
    if let Some(body_bytes) = body {
        let has_content_length = headers.iter()
            .any(|(k, _)| k.to_lowercase() == "content-length");
        if !has_content_length {
            request.push_str(&format!("Content-Length: {}\r\n", body_bytes.len()));
        }
    }

    // Add Connection: close to simplify response handling
    request.push_str("Connection: close\r\n");

    // End headers
    request.push_str("\r\n");

    // Send request with optional timeout
    let result = if let Some(t) = request_timeout {
        timeout(t, send_and_receive(stream, request, body)).await
            .map_err(|_| QuicpulseError::Timeout(t.as_secs_f64()))?
    } else {
        send_and_receive(stream, request, body).await
    };

    result
}

/// Send request and receive response
async fn send_and_receive(
    mut stream: UnixStream,
    request: String,
    body: Option<&[u8]>,
) -> Result<UnixSocketResponse, QuicpulseError> {
    // Write request headers
    stream.write_all(request.as_bytes()).await
        .map_err(|e| QuicpulseError::Io(e))?;

    // Write body if present
    if let Some(body_bytes) = body {
        stream.write_all(body_bytes).await
            .map_err(|e| QuicpulseError::Io(e))?;
    }

    // Flush the stream
    stream.flush().await
        .map_err(|e| QuicpulseError::Io(e))?;

    // Read the entire response
    let mut response_data = Vec::new();
    stream.read_to_end(&mut response_data).await
        .map_err(|e| QuicpulseError::Io(e))?;

    // Parse the HTTP response
    parse_response(&response_data)
}

/// Parse an HTTP response from raw bytes
fn parse_response(data: &[u8]) -> Result<UnixSocketResponse, QuicpulseError> {
    let mut reader = BufReader::new(data);
    let mut status_line = String::new();

    // Read status line
    reader.read_line(&mut status_line)
        .map_err(|e| QuicpulseError::Parse(format!("Failed to read status line: {}", e)))?;

    // Parse status line (e.g., "HTTP/1.1 200 OK")
    let parts: Vec<&str> = status_line.trim().splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(QuicpulseError::Parse(format!(
            "Invalid status line: {}", status_line.trim()
        )));
    }

    let http_version = parts[0].to_string();
    let status: u16 = parts[1].parse()
        .map_err(|_| QuicpulseError::Parse(format!(
            "Invalid status code: {}", parts[1]
        )))?;
    let status_text = parts.get(2).unwrap_or(&"").to_string();

    // Parse headers
    let mut headers = HashMap::new();
    let mut content_length: Option<usize> = None;
    let mut chunked = false;

    loop {
        let mut line = String::new();
        reader.read_line(&mut line)
            .map_err(|e| QuicpulseError::Parse(format!("Failed to read header: {}", e)))?;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }

        if let Some((name, value)) = trimmed.split_once(':') {
            let name = name.trim().to_string();
            let value = value.trim().to_string();

            if name.to_lowercase() == "content-length" {
                content_length = value.parse().ok();
            }
            if name.to_lowercase() == "transfer-encoding" && value.to_lowercase().contains("chunked") {
                chunked = true;
            }

            headers.insert(name, value);
        }
    }

    // Read body
    let body = if chunked {
        // Handle chunked transfer encoding
        read_chunked_body(&mut reader)?
    } else if let Some(len) = content_length {
        let mut body = vec![0u8; len];
        use std::io::Read;
        reader.read_exact(&mut body)
            .map_err(|e| QuicpulseError::Parse(format!("Failed to read body: {}", e)))?;
        body
    } else {
        // Read remaining data
        let mut body = Vec::new();
        use std::io::Read;
        reader.read_to_end(&mut body)
            .map_err(|e| QuicpulseError::Io(e))?;
        body
    };

    Ok(UnixSocketResponse {
        status,
        status_text,
        headers,
        body,
        http_version,
    })
}

/// Read chunked transfer encoding body
fn read_chunked_body(reader: &mut BufReader<&[u8]>) -> Result<Vec<u8>, QuicpulseError> {
    let mut body = Vec::new();

    loop {
        let mut size_line = String::new();
        use std::io::BufRead;
        reader.read_line(&mut size_line)
            .map_err(|e| QuicpulseError::Parse(format!("Failed to read chunk size: {}", e)))?;

        let size_str = size_line.trim().split(';').next().unwrap_or("0");
        let chunk_size = usize::from_str_radix(size_str, 16)
            .map_err(|_| QuicpulseError::Parse(format!("Invalid chunk size: {}", size_str)))?;

        if chunk_size == 0 {
            // Read trailing CRLF
            let mut _trailer = String::new();
            reader.read_line(&mut _trailer).ok();
            break;
        }

        let mut chunk = vec![0u8; chunk_size];
        use std::io::Read;
        reader.read_exact(&mut chunk)
            .map_err(|e| QuicpulseError::Parse(format!("Failed to read chunk: {}", e)))?;
        body.extend_from_slice(&chunk);

        // Read trailing CRLF after chunk
        let mut _crlf = String::new();
        reader.read_line(&mut _crlf).ok();
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_response() {
        // Note: Content-Length must match actual body length exactly
        let body = b"{\"ok\":true}";
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             \r\n",
            body.len()
        );
        let mut full_response = response.into_bytes();
        full_response.extend_from_slice(body);

        let parsed = parse_response(&full_response).unwrap();
        assert_eq!(parsed.status, 200);
        assert_eq!(parsed.status_text, "OK");
        assert_eq!(parsed.http_version, "HTTP/1.1");
        assert_eq!(parsed.header("content-type"), Some("application/json"));
        assert_eq!(parsed.body, body);
    }

    #[test]
    fn test_parse_no_body() {
        let response = b"HTTP/1.1 204 No Content\r\n\
            \r\n";

        let parsed = parse_response(response).unwrap();
        assert_eq!(parsed.status, 204);
        assert!(parsed.body.is_empty());
    }

    #[test]
    fn test_unix_socket_response_helpers() {
        let response = UnixSocketResponse {
            status: 200,
            status_text: "OK".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "text/plain".to_string());
                h
            },
            body: b"Hello".to_vec(),
            http_version: "HTTP/1.1".to_string(),
        };

        assert_eq!(response.content_type(), Some("text/plain"));
        assert_eq!(response.text().unwrap(), "Hello");
    }
}
