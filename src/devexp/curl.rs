//! Curl command generation
//!
//! Converts QuicPulse requests to equivalent curl commands for sharing
//! and debugging.

use crate::cli::Args;
use crate::cli::parser::ProcessedArgs;
use crate::input::InputItem;

/// Generate an equivalent curl command from the request
pub fn generate_curl_command(args: &Args, processed: &ProcessedArgs) -> String {
    let mut parts: Vec<String> = vec!["curl".to_string()];

    // Add method if not GET
    if processed.method != "GET" {
        parts.push("-X".to_string());
        parts.push(processed.method.clone());
    }

    // Add headers
    for item in &processed.items {
        match item {
            InputItem::Header { name, value } => {
                parts.push("-H".to_string());
                parts.push(shell_escape(&format!("{}: {}", name, value)));
            }
            InputItem::EmptyHeader { name } => {
                parts.push("-H".to_string());
                parts.push(shell_escape(&format!("{}:", name)));
            }
            InputItem::HeaderFile { name, path } => {
                if let Ok(content) = std::fs::read_to_string(path) {
                    parts.push("-H".to_string());
                    parts.push(shell_escape(&format!("{}: {}", name, content.trim())));
                }
            }
            _ => {}
        }
    }

    // Add default headers that QuicPulse sends
    parts.push("-H".to_string());
    parts.push(shell_escape("Accept: application/json, */*;q=0.5"));

    parts.push("-H".to_string());
    parts.push(shell_escape(&format!("User-Agent: QuicPulse/0.1.0")));

    // Add body based on request type
    if let Some(body) = build_body(args, processed) {
        // Add content-type header
        let content_type = if args.form {
            "application/x-www-form-urlencoded"
        } else {
            "application/json"
        };
        parts.push("-H".to_string());
        parts.push(shell_escape(&format!("Content-Type: {}", content_type)));

        // Add data
        parts.push("-d".to_string());
        parts.push(shell_escape(&body));
    }

    // Add authentication
    if let Some(ref auth) = args.auth {
        match args.auth_type {
            Some(crate::cli::args::AuthType::Bearer) => {
                parts.push("-H".to_string());
                parts.push(shell_escape(&format!("Authorization: Bearer {}", auth)));
            }
            Some(crate::cli::args::AuthType::Digest) => {
                parts.push("--digest".to_string());
                parts.push("-u".to_string());
                parts.push(shell_escape(auth));
            }
            _ => {
                // Basic auth (default)
                parts.push("-u".to_string());
                parts.push(shell_escape(auth));
            }
        }
    }

    // Add timeout
    if let Some(timeout) = args.timeout {
        parts.push("--max-time".to_string());
        parts.push(format!("{}", timeout));
    }

    // Add follow redirects
    if args.follow {
        parts.push("-L".to_string());
        if args.max_redirects != 30 {
            parts.push("--max-redirs".to_string());
            parts.push(format!("{}", args.max_redirects));
        }
    }

    // Add SSL options
    if args.verify == "no" {
        parts.push("-k".to_string());
    } else if args.verify != "yes" {
        parts.push("--cacert".to_string());
        parts.push(shell_escape(&args.verify));
    }

    if let Some(ref cert) = args.cert {
        parts.push("--cert".to_string());
        parts.push(shell_escape(&cert.display().to_string()));
    }

    if let Some(ref key) = args.cert_key {
        parts.push("--key".to_string());
        parts.push(shell_escape(&key.display().to_string()));
    }

    // Add proxy
    for proxy in &args.proxy {
        parts.push("-x".to_string());
        parts.push(shell_escape(proxy));
    }

    // Add compressed
    if args.compress > 0 {
        parts.push("--compressed".to_string());
    }

    // Add verbose
    if args.verbose > 0 {
        parts.push("-v".to_string());
    }

    // Add URL (always last)
    parts.push(shell_escape(&processed.url));

    parts.join(" ")
}

/// Build the request body for curl
fn build_body(args: &Args, processed: &ProcessedArgs) -> Option<String> {
    // Check for raw body first
    if let Some(ref raw) = args.raw {
        return Some(raw.clone());
    }

    // Collect data items
    let data_items: Vec<&InputItem> = processed.items.iter()
        .filter(|i| i.is_data())
        .collect();

    if data_items.is_empty() {
        return None;
    }

    if args.form {
        // URL-encoded form data
        let pairs: Vec<String> = data_items.iter()
            .filter_map(|item| {
                match item {
                    InputItem::DataField { key, value } => {
                        Some(format!("{}={}", percent_encode(key), percent_encode(value)))
                    }
                    InputItem::DataFieldFile { key, path } => {
                        std::fs::read_to_string(path).ok().map(|v| {
                            format!("{}={}", percent_encode(key), percent_encode(v.trim()))
                        })
                    }
                    _ => None,
                }
            })
            .collect();
        Some(pairs.join("&"))
    } else {
        // JSON body
        build_json_body(&data_items)
    }
}

/// Build JSON body from data items
fn build_json_body(items: &[&InputItem]) -> Option<String> {
    use std::collections::HashMap;

    if items.is_empty() {
        return None;
    }

    let mut map: HashMap<String, serde_json::Value> = HashMap::new();

    for item in items {
        let (key, value) = match item {
            InputItem::DataField { key, value } => {
                (key.clone(), serde_json::Value::String(value.clone()))
            }
            InputItem::DataFieldFile { key, path } => {
                let content = std::fs::read_to_string(path).unwrap_or_default();
                (key.clone(), serde_json::Value::String(content.trim().to_string()))
            }
            InputItem::JsonField { key, value } => {
                (key.clone(), value.clone())
            }
            InputItem::JsonFieldFile { key, path } => {
                let content = std::fs::read_to_string(path).unwrap_or_default();
                let json_val = serde_json::from_str(&content)
                    .unwrap_or(serde_json::Value::String(content));
                (key.clone(), json_val)
            }
            _ => continue,
        };
        map.insert(key, value);
    }

    Some(serde_json::to_string(&map).unwrap_or_default())
}

/// Shell-escape a string for safe inclusion in a command
fn shell_escape(s: &str) -> String {
    // Check if escaping is needed
    let needs_escaping = s.chars().any(|c| {
        matches!(c, ' ' | '\'' | '"' | '\\' | '$' | '`' | '!' | '*' | '?' |
                    '[' | ']' | '{' | '}' | '(' | ')' | '<' | '>' | '|' |
                    '&' | ';' | '\n' | '\t')
    });

    if !needs_escaping && !s.is_empty() {
        return s.to_string();
    }

    // Use single quotes and escape any single quotes within
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

/// Percent-encode a string for URL/form data
fn percent_encode(s: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

/// Format curl command with syntax highlighting for terminal
pub fn format_curl_pretty(cmd: &str) -> String {
    // Simple colorization using ANSI codes
    let mut result = String::new();
    let mut in_string = false;
    let mut chars = cmd.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\'' && !in_string {
            in_string = true;
            result.push_str("\x1b[32m'"); // Green for strings
        } else if c == '\'' && in_string {
            in_string = false;
            result.push_str("'\x1b[0m");
        } else if !in_string && (c == '-') {
            // Check if it's a flag
            result.push_str("\x1b[36m"); // Cyan for flags
            result.push(c);
            while let Some(&next) = chars.peek() {
                if next.is_alphanumeric() || next == '-' {
                    result.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            result.push_str("\x1b[0m");
        } else if !in_string && c == 'c' && cmd.starts_with("curl") && result.is_empty() {
            // "curl" command
            result.push_str("\x1b[1;33mcurl\x1b[0m"); // Bold yellow
            // Skip "url"
            chars.next(); // u
            chars.next(); // r
            chars.next(); // l
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape_simple() {
        assert_eq!(shell_escape("hello"), "hello");
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn test_shell_escape_quotes() {
        assert_eq!(shell_escape("it's"), "'it'\"'\"'s'");
    }

    #[test]
    fn test_shell_escape_special_chars() {
        assert_eq!(shell_escape("$HOME"), "'$HOME'");
        assert_eq!(shell_escape("a & b"), "'a & b'");
    }

    #[test]
    fn test_percent_encode() {
        assert_eq!(percent_encode("hello"), "hello");
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("a=b"), "a%3Db");
    }
}
