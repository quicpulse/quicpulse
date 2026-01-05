//! cURL command import/parsing
//!
//! Parses curl commands and converts them to QuicPulse arguments for execution.
//!
//! # Example
//!
//! ```bash
//! quicpulse --import-curl "curl -X POST -H 'Content-Type: application/json' -d '{\"name\":\"John\"}' https://api.example.com/users"
//! ```

use crate::cli::Args;
use crate::errors::QuicpulseError;
use std::path::PathBuf;

/// Parsed curl command structure
#[derive(Debug, Default)]
pub struct ParsedCurl {
    pub method: Option<String>,
    pub url: Option<String>,
    pub headers: Vec<(String, String)>,
    pub data: Option<String>,
    pub data_binary: Option<Vec<u8>>,
    pub data_raw: Option<String>,
    pub form_fields: Vec<(String, String)>,
    pub user: Option<String>,
    pub basic_auth: bool,
    pub digest_auth: bool,
    pub bearer_token: Option<String>,
    pub follow_redirects: bool,
    pub max_redirects: Option<u32>,
    pub timeout: Option<f64>,
    pub connect_timeout: Option<f64>,
    pub insecure: bool,
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    pub cacert: Option<PathBuf>,
    pub proxy: Option<String>,
    pub compressed: bool,
    pub verbose: bool,
    pub silent: bool,
    pub output: Option<PathBuf>,
    pub head_only: bool,
    pub include_headers: bool,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub cookie: Option<String>,
    pub cookie_jar: Option<PathBuf>,
    pub http_version: Option<String>,
    pub location_trusted: bool,
}

/// Parse a curl command string into a ParsedCurl structure
pub fn parse_curl_command(cmd: &str) -> Result<ParsedCurl, QuicpulseError> {
    let tokens = tokenize_curl(cmd)?;
    parse_tokens(&tokens)
}

/// Tokenize a curl command, handling quoted strings
fn tokenize_curl(cmd: &str) -> Result<Vec<String>, QuicpulseError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;
    let mut chars = cmd.chars().peekable();

    while let Some(c) = chars.next() {
        if escape_next {
            current.push(c);
            escape_next = false;
            continue;
        }

        match c {
            '\\' if !in_single_quote => {
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' | '\n' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    if in_single_quote || in_double_quote {
        return Err(QuicpulseError::Parse("Unterminated quote in curl command".to_string()));
    }

    Ok(tokens)
}

/// Parse tokenized curl command
fn parse_tokens(tokens: &[String]) -> Result<ParsedCurl, QuicpulseError> {
    let mut parsed = ParsedCurl::default();
    let mut i = 0;

    // Skip "curl" if present
    if tokens.first().map(|s| s.to_lowercase()) == Some("curl".to_string()) {
        i = 1;
    }

    while i < tokens.len() {
        let token = &tokens[i];

        if token.starts_with('-') {
            // Handle combined short flags like -sSL
            if token.starts_with('-') && !token.starts_with("--") && token.len() > 2 {
                // Check if it's a combined flag (all characters are valid short flags without args)
                let flags_without_args = ['s', 'S', 'L', 'v', 'k', 'I', 'i', 'O', 'J', 'f', 'g', 'G', 'N', '0', '1', '2', '3', '4', '6'];
                let rest = &token[1..];
                let all_flags = rest.chars().all(|c| flags_without_args.contains(&c));

                if all_flags {
                    for c in rest.chars() {
                        match c {
                            's' => parsed.silent = true,
                            'S' => {} // Show errors (default behavior)
                            'L' => parsed.follow_redirects = true,
                            'v' => parsed.verbose = true,
                            'k' => parsed.insecure = true,
                            'I' => parsed.head_only = true,
                            'i' => parsed.include_headers = true,
                            'O' | 'J' => {} // Output to file (handled separately)
                            'f' => {} // Fail silently
                            'g' | 'G' | 'N' => {} // Globbing, GET mode, no buffering
                            '0' => parsed.http_version = Some("1.0".to_string()),
                            '1' | '2' | '3' | '4' | '6' => {} // HTTP version and IP version
                            _ => {}
                        }
                    }
                    i += 1;
                    continue;
                }
            }

            match token.as_str() {
                "-X" | "--request" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.method = Some(tokens[i].to_uppercase());
                    }
                }
                "-H" | "--header" => {
                    i += 1;
                    if i < tokens.len() {
                        if let Some((name, value)) = parse_header(&tokens[i]) {
                            // Check for Authorization header
                            if name.eq_ignore_ascii_case("Authorization") {
                                if value.to_lowercase().starts_with("bearer ") {
                                    parsed.bearer_token = Some(value[7..].trim().to_string());
                                } else {
                                    parsed.headers.push((name, value));
                                }
                            } else {
                                parsed.headers.push((name, value));
                            }
                        }
                    }
                }
                "-d" | "--data" | "--data-ascii" => {
                    i += 1;
                    if i < tokens.len() {
                        let data = &tokens[i];
                        if let Some(existing) = &mut parsed.data {
                            existing.push('&');
                            existing.push_str(data);
                        } else {
                            parsed.data = Some(data.clone());
                        }
                    }
                }
                "--data-raw" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.data_raw = Some(tokens[i].clone());
                    }
                }
                "--data-binary" => {
                    i += 1;
                    if i < tokens.len() {
                        let data = &tokens[i];
                        if data.starts_with('@') {
                            // File reference
                            parsed.data_raw = Some(data.clone());
                        } else {
                            parsed.data = Some(data.clone());
                        }
                    }
                }
                "--data-urlencode" => {
                    i += 1;
                    if i < tokens.len() {
                        // Data that should be URL encoded
                        let data = &tokens[i];
                        if let Some(existing) = &mut parsed.data {
                            existing.push('&');
                            existing.push_str(&urlencoding::encode(data));
                        } else {
                            parsed.data = Some(urlencoding::encode(data).into_owned());
                        }
                    }
                }
                "-F" | "--form" => {
                    i += 1;
                    if i < tokens.len() {
                        if let Some((key, value)) = parse_form_field(&tokens[i]) {
                            parsed.form_fields.push((key, value));
                        }
                    }
                }
                "-u" | "--user" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.user = Some(tokens[i].clone());
                    }
                }
                "--basic" => {
                    parsed.basic_auth = true;
                }
                "--digest" => {
                    parsed.digest_auth = true;
                }
                "-L" | "--location" => {
                    parsed.follow_redirects = true;
                }
                "--location-trusted" => {
                    parsed.follow_redirects = true;
                    parsed.location_trusted = true;
                }
                "--max-redirs" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.max_redirects = tokens[i].parse().ok();
                    }
                }
                "-m" | "--max-time" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.timeout = tokens[i].parse().ok();
                    }
                }
                "--connect-timeout" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.connect_timeout = tokens[i].parse().ok();
                    }
                }
                "-k" | "--insecure" => {
                    parsed.insecure = true;
                }
                "--cert" | "-E" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.cert = Some(PathBuf::from(&tokens[i]));
                    }
                }
                "--key" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.key = Some(PathBuf::from(&tokens[i]));
                    }
                }
                "--cacert" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.cacert = Some(PathBuf::from(&tokens[i]));
                    }
                }
                "-x" | "--proxy" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.proxy = Some(tokens[i].clone());
                    }
                }
                "--compressed" => {
                    parsed.compressed = true;
                }
                "-v" | "--verbose" => {
                    parsed.verbose = true;
                }
                "-s" | "--silent" => {
                    parsed.silent = true;
                }
                "-o" | "--output" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.output = Some(PathBuf::from(&tokens[i]));
                    }
                }
                "-I" | "--head" => {
                    parsed.head_only = true;
                    parsed.method = Some("HEAD".to_string());
                }
                "-i" | "--include" => {
                    parsed.include_headers = true;
                }
                "-A" | "--user-agent" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.user_agent = Some(tokens[i].clone());
                    }
                }
                "-e" | "--referer" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.referer = Some(tokens[i].clone());
                    }
                }
                "-b" | "--cookie" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.cookie = Some(tokens[i].clone());
                    }
                }
                "-c" | "--cookie-jar" => {
                    i += 1;
                    if i < tokens.len() {
                        parsed.cookie_jar = Some(PathBuf::from(&tokens[i]));
                    }
                }
                "--http1.0" | "-0" => {
                    parsed.http_version = Some("1.0".to_string());
                }
                "--http1.1" => {
                    parsed.http_version = Some("1.1".to_string());
                }
                "--http2" => {
                    parsed.http_version = Some("2".to_string());
                }
                "--http3" => {
                    parsed.http_version = Some("3".to_string());
                }
                "-G" | "--get" => {
                    // Data as query params
                    if parsed.method.is_none() {
                        parsed.method = Some("GET".to_string());
                    }
                }
                // Skip unknown flags with arguments
                opt if opt.starts_with("--") => {
                    // Check if next token is a value (not starting with -)
                    if i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') {
                        i += 1;
                    }
                }
                opt if opt.starts_with('-') && opt.len() == 2 => {
                    // Single char flag that might have an argument
                    let flag_char = opt.chars().nth(1).unwrap_or('_');
                    let flags_with_args = ['o', 'O', 'T', 'u', 'A', 'e', 'b', 'c', 'x', 'd', 'F', 'H', 'm', 'E', 'r', 'w'];
                    if flags_with_args.contains(&flag_char) && i + 1 < tokens.len() {
                        i += 1;
                    }
                }
                _ => {}
            }
        } else if parsed.url.is_none() && !token.starts_with('-') {
            // URL (first non-flag argument)
            parsed.url = Some(token.clone());
        }

        i += 1;
    }

    Ok(parsed)
}

/// Parse a header string "Name: Value"
fn parse_header(header: &str) -> Option<(String, String)> {
    let colon_pos = header.find(':')?;
    let name = header[..colon_pos].trim().to_string();
    let value = header[colon_pos + 1..].trim().to_string();
    Some((name, value))
}

/// Parse a form field "key=value"
fn parse_form_field(field: &str) -> Option<(String, String)> {
    let eq_pos = field.find('=')?;
    let key = field[..eq_pos].to_string();
    let value = field[eq_pos + 1..].to_string();
    Some((key, value))
}

/// Convert ParsedCurl to QuicPulse Args
pub fn curl_to_args(parsed: ParsedCurl) -> Result<Args, QuicpulseError> {
    let mut args = Args::default();

    // URL
    args.url = parsed.url.clone();

    // Method
    if let Some(method) = parsed.method {
        args.method = Some(method);
    } else if parsed.data.is_some() || parsed.data_raw.is_some() || !parsed.form_fields.is_empty() {
        args.method = Some("POST".to_string());
    } else {
        args.method = Some("GET".to_string());
    }

    // Headers as request items
    for (name, value) in &parsed.headers {
        args.request_items.push(format!("{}:{}", name, value));
    }

    // User-Agent
    if let Some(ua) = parsed.user_agent {
        args.request_items.push(format!("User-Agent:{}", ua));
    }

    // Referer
    if let Some(referer) = parsed.referer {
        args.request_items.push(format!("Referer:{}", referer));
    }

    // Cookie
    if let Some(cookie) = parsed.cookie {
        args.request_items.push(format!("Cookie:{}", cookie));
    }

    // Data handling
    if !parsed.form_fields.is_empty() {
        args.form = true;
        for (key, value) in &parsed.form_fields {
            if value.starts_with('@') {
                // File upload
                args.request_items.push(format!("{}@{}", key, &value[1..]));
            } else {
                args.request_items.push(format!("{}={}", key, value));
            }
        }
    } else if let Some(data) = parsed.data.or(parsed.data_raw) {
        // Check if it's JSON
        if data.trim().starts_with('{') || data.trim().starts_with('[') {
            args.raw = Some(data);
        } else if data.starts_with('@') {
            // File reference
            let file_path = &data[1..];
            if let Ok(content) = std::fs::read_to_string(file_path) {
                args.raw = Some(content);
            } else {
                return Err(QuicpulseError::Parse(format!("Cannot read file: {}", file_path)));
            }
        } else {
            // Form-encoded data
            args.form = true;
            // Parse key=value pairs
            for pair in data.split('&') {
                if let Some(eq_pos) = pair.find('=') {
                    let key = &pair[..eq_pos];
                    let value = &pair[eq_pos + 1..];
                    args.request_items.push(format!("{}={}", key, value));
                }
            }
        }
    }

    // Authentication
    if let Some(bearer) = parsed.bearer_token {
        args.auth = Some(crate::cli::args::SecretString::from(bearer));
        args.auth_type = Some(crate::cli::args::AuthType::Bearer);
    } else if let Some(user) = parsed.user {
        args.auth = Some(crate::cli::args::SecretString::from(user));
        if parsed.digest_auth {
            args.auth_type = Some(crate::cli::args::AuthType::Digest);
        } else {
            args.auth_type = Some(crate::cli::args::AuthType::Basic);
        }
    }

    // Redirects
    args.follow = parsed.follow_redirects;
    if let Some(max) = parsed.max_redirects {
        args.max_redirects = max;
    }

    // Timeout
    if let Some(timeout) = parsed.timeout {
        args.timeout = Some(timeout);
    }

    // SSL/TLS
    if parsed.insecure {
        args.verify = "no".to_string();
    }
    if let Some(cert) = parsed.cert {
        args.cert = Some(cert);
    }
    if let Some(key) = parsed.key {
        args.cert_key = Some(key);
    }
    if let Some(cacert) = parsed.cacert {
        args.verify = cacert.display().to_string();
    }

    // Proxy
    if let Some(proxy) = parsed.proxy {
        args.proxy.push(crate::cli::args::SensitiveUrl::from(proxy));
    }

    // Verbose
    if parsed.verbose {
        args.verbose = 1;
    }

    // Quiet
    if parsed.silent {
        args.quiet = 1;
    }

    // Output
    if let Some(output) = parsed.output {
        args.output = Some(output);
        args.download = true;
    }

    // HTTP version
    if let Some(version) = parsed.http_version {
        args.http_version = Some(version.clone());
        if version == "3" {
            args.http3 = true;
        }
    }

    Ok(args)
}

/// Parse and convert a curl command to Args in one step
pub fn import_curl(cmd: &str) -> Result<Args, QuicpulseError> {
    let parsed = parse_curl_command(cmd)?;
    curl_to_args(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_get() {
        let parsed = parse_curl_command("curl https://example.com").unwrap();
        assert_eq!(parsed.url, Some("https://example.com".to_string()));
        assert!(parsed.method.is_none()); // Will default to GET
    }

    #[test]
    fn test_post_with_data() {
        let parsed = parse_curl_command("curl -X POST -d 'name=John' https://example.com").unwrap();
        assert_eq!(parsed.url, Some("https://example.com".to_string()));
        assert_eq!(parsed.method, Some("POST".to_string()));
        assert_eq!(parsed.data, Some("name=John".to_string()));
    }

    #[test]
    fn test_headers() {
        let parsed = parse_curl_command(
            "curl -H 'Content-Type: application/json' -H 'Accept: application/json' https://example.com"
        ).unwrap();
        assert_eq!(parsed.headers.len(), 2);
        assert_eq!(parsed.headers[0], ("Content-Type".to_string(), "application/json".to_string()));
    }

    #[test]
    fn test_json_data() {
        let parsed = parse_curl_command(
            r#"curl -X POST -H 'Content-Type: application/json' -d '{"name":"John"}' https://example.com"#
        ).unwrap();
        assert_eq!(parsed.data, Some(r#"{"name":"John"}"#.to_string()));
    }

    #[test]
    fn test_auth() {
        let parsed = parse_curl_command("curl -u user:pass https://example.com").unwrap();
        assert_eq!(parsed.user, Some("user:pass".to_string()));
    }

    #[test]
    fn test_bearer_token() {
        let parsed = parse_curl_command(
            "curl -H 'Authorization: Bearer token123' https://example.com"
        ).unwrap();
        assert_eq!(parsed.bearer_token, Some("token123".to_string()));
    }

    #[test]
    fn test_combined_flags() {
        let parsed = parse_curl_command("curl -sSL https://example.com").unwrap();
        assert!(parsed.silent);
        assert!(parsed.follow_redirects);
    }

    #[test]
    fn test_curl_to_args() {
        let args = import_curl(
            "curl -X POST -H 'Content-Type: application/json' -d '{\"name\":\"John\"}' https://example.com"
        ).unwrap();

        assert_eq!(args.method, Some("POST".to_string()));
        assert_eq!(args.url, Some("https://example.com".to_string()));
        assert!(args.raw.is_some());
    }

    #[test]
    fn test_tokenize_quotes() {
        let tokens = tokenize_curl(r#"curl -H 'Content-Type: application/json' "https://example.com""#).unwrap();
        assert_eq!(tokens, vec!["curl", "-H", "Content-Type: application/json", "https://example.com"]);
    }
}
