//! Curl command generation
//!
//! Converts QuicPulse requests to equivalent curl commands for sharing
//! and debugging.

use crate::cli::parser::ProcessedArgs;
use crate::cli::Args;
use crate::input::InputItem;
use crate::strings::form_urlencode;

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
    parts.push(shell_escape("User-Agent: QuicPulse/0.1.0"));

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
                // as_str(), not Display: SecretString's Display renders
                // "[REDACTED]", which would emit an unusable curl command.
                parts.push(shell_escape(&format!(
                    "Authorization: Bearer {}",
                    auth.as_str()
                )));
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
    parts.push(shell_escape(&build_url(processed)));

    parts.join(" ")
}

/// Fold query-parameter items into the URL.
///
/// Query params live in the item list rather than in `processed.url` (they are
/// normally applied when the request is built), so the exported command has to
/// append them itself or they would be silently dropped. Uses the same
/// form-style encoding and separator logic as the real request path.
fn build_url(processed: &ProcessedArgs) -> String {
    let pairs: Vec<String> = processed
        .items
        .iter()
        .filter_map(|item| match item {
            InputItem::QueryParam { name, value } => Some(format!(
                "{}={}",
                form_urlencode(name),
                form_urlencode(value)
            )),
            InputItem::QueryParamFile { name, path } => std::fs::read_to_string(path)
                .ok()
                .map(|v| format!("{}={}", form_urlencode(name), form_urlencode(v.trim()))),
            _ => None,
        })
        .collect();

    if pairs.is_empty() {
        return processed.url.clone();
    }

    let separator = if processed.url.contains('?') {
        '&'
    } else {
        '?'
    };
    format!("{}{}{}", processed.url, separator, pairs.join("&"))
}

/// Build the request body for curl
fn build_body(args: &Args, processed: &ProcessedArgs) -> Option<String> {
    // Check for raw body first
    if let Some(ref raw) = args.raw {
        return Some(raw.clone());
    }

    // Collect data items
    let data_items: Vec<&InputItem> = processed.items.iter().filter(|i| i.is_data()).collect();

    if data_items.is_empty() {
        return None;
    }

    if args.form {
        // URL-encoded form data
        let pairs: Vec<String> = data_items
            .iter()
            .filter_map(|item| match item {
                InputItem::DataField { key, value } => {
                    Some(format!("{}={}", percent_encode(key), percent_encode(value)))
                }
                InputItem::DataFieldFile { key, path } => std::fs::read_to_string(path)
                    .ok()
                    .map(|v| format!("{}={}", percent_encode(key), percent_encode(v.trim()))),
                _ => None,
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
                (
                    key.clone(),
                    serde_json::Value::String(content.trim().to_string()),
                )
            }
            InputItem::JsonField { key, value } => (key.clone(), value.clone()),
            InputItem::JsonFieldFile { key, path } => {
                let content = std::fs::read_to_string(path).unwrap_or_default();
                let json_val =
                    serde_json::from_str(&content).unwrap_or(serde_json::Value::String(content));
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
        matches!(
            c,
            ' ' | '\''
                | '"'
                | '\\'
                | '$'
                | '`'
                | '!'
                | '*'
                | '?'
                | '['
                | ']'
                | '{'
                | '}'
                | '('
                | ')'
                | '<'
                | '>'
                | '|'
                | '&'
                | ';'
                | '\n'
                | '\t'
        )
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

    // ---- shell_escape / percent_encode extras ----

    #[test]
    fn test_shell_escape_empty_string_is_quoted() {
        // An unquoted empty argument would vanish from the command line.
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn test_shell_escape_covers_every_metacharacter() {
        for c in [
            ' ', '\'', '"', '\\', '$', '`', '!', '*', '?', '[', ']', '{', '}', '(', ')', '<', '>',
            '|', '&', ';', '\n', '\t',
        ] {
            let input = format!("a{c}b");
            let escaped = shell_escape(&input);
            assert!(
                escaped.starts_with('\'') && escaped.ends_with('\''),
                "{c:?} should force quoting, got {escaped}"
            );
        }
    }

    #[test]
    fn test_shell_escape_leaves_safe_strings_bare() {
        for safe in ["abc", "a-b_c.d", "https://example.com/p?", "1234"] {
            if !safe.contains('?') {
                assert_eq!(shell_escape(safe), safe, "should not quote {safe}");
            }
        }
        assert_eq!(shell_escape("application/json"), "application/json");
    }

    #[test]
    fn test_shell_escape_multiple_single_quotes() {
        assert_eq!(shell_escape("a'b'c"), "'a'\"'\"'b'\"'\"'c'");
    }

    #[test]
    fn test_percent_encode_reserved_and_unicode() {
        assert_eq!(percent_encode("a&b"), "a%26b");
        assert_eq!(percent_encode("a/b"), "a%2Fb");
        assert_eq!(percent_encode("é"), "%C3%A9");
        assert_eq!(percent_encode(""), "");
    }

    // ---- format_curl_pretty ----

    #[test]
    fn test_format_curl_pretty_highlights_command_flags_and_strings() {
        let out = format_curl_pretty("curl -X POST 'https://example.com'");
        assert!(
            out.contains("\x1b[1;33mcurl\x1b[0m"),
            "command not bold yellow: {out:?}"
        );
        assert!(out.contains("\x1b[36m-X\x1b[0m"), "flag not cyan: {out:?}");
        assert!(out.contains("\x1b[32m'"), "string not green: {out:?}");
    }

    #[test]
    fn test_format_curl_pretty_preserves_the_plain_text() {
        let cmd = "curl -X POST -H 'A: b' 'https://example.com'";
        let out = format_curl_pretty(cmd);
        let mut plain = String::new();
        let mut chars = out.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for c in chars.by_ref() {
                    if c == 'm' {
                        break;
                    }
                }
            } else {
                plain.push(c);
            }
        }
        assert_eq!(plain, cmd);
    }

    #[test]
    fn test_format_curl_pretty_handles_long_flags_and_empty_input() {
        let out = format_curl_pretty("curl --max-time 30");
        assert!(out.contains("\x1b[36m--max-time\x1b[0m"), "got: {out:?}");
        assert_eq!(format_curl_pretty(""), "");
    }

    // ---- generate_curl_command ----

    use crate::cli::process::process_args;
    use clap::Parser;

    /// Build a curl command the same way the CLI does: parse real argv, run it
    /// through process_args, then render.
    fn curl_for(argv: &[&str]) -> String {
        let mut full = vec!["quicpulse"];
        full.extend_from_slice(argv);
        let args = Args::try_parse_from(full).expect("args should parse");
        let processed = process_args(&args).expect("args should process");
        generate_curl_command(&args, &processed)
    }

    #[test]
    fn test_generated_command_starts_with_curl_and_ends_with_the_url() {
        let cmd = curl_for(&["https://example.com/api"]);
        assert!(cmd.starts_with("curl "), "got: {cmd}");
        assert!(
            cmd.trim_end().ends_with("https://example.com/api"),
            "URL must come last, got: {cmd}"
        );
    }

    #[test]
    fn test_get_does_not_emit_an_explicit_method() {
        let cmd = curl_for(&["https://example.com"]);
        assert!(!cmd.contains("-X"), "GET should be implicit, got: {cmd}");
    }

    #[test]
    fn test_non_get_methods_are_emitted_explicitly() {
        let cmd = curl_for(&["DELETE", "https://example.com"]);
        assert!(cmd.contains("-X DELETE"), "got: {cmd}");
    }

    #[test]
    fn test_default_headers_are_always_included() {
        let cmd = curl_for(&["https://example.com"]);
        assert!(cmd.contains("Accept: application/json"), "got: {cmd}");
        assert!(cmd.contains("User-Agent: QuicPulse"), "got: {cmd}");
    }

    #[test]
    fn test_custom_header_is_forwarded_and_escaped() {
        let cmd = curl_for(&["https://example.com", "X-Token:abc123"]);
        assert!(cmd.contains("'X-Token: abc123'"), "got: {cmd}");
    }

    #[test]
    fn test_json_data_fields_become_a_json_body() {
        let cmd = curl_for(&["POST", "https://example.com", "name=alex", "role=admin"]);
        assert!(cmd.contains("-d "), "got: {cmd}");
        assert!(cmd.contains("Content-Type: application/json"), "got: {cmd}");
        // Field order comes from a HashMap, so check both keys individually.
        assert!(cmd.contains("alex"), "got: {cmd}");
        assert!(cmd.contains("admin"), "got: {cmd}");
    }

    #[test]
    fn test_form_mode_uses_urlencoded_body() {
        let cmd = curl_for(&["--form", "POST", "https://example.com", "a=hello world"]);
        assert!(
            cmd.contains("Content-Type: application/x-www-form-urlencoded"),
            "got: {cmd}"
        );
        assert!(cmd.contains("a=hello%20world"), "got: {cmd}");
    }

    #[test]
    fn test_raw_body_is_passed_through_verbatim() {
        let cmd = curl_for(&["--raw", "{\"k\":1}", "POST", "https://example.com"]);
        assert!(cmd.contains(r#"{"k":1}"#), "got: {cmd}");
    }

    #[test]
    fn test_no_data_means_no_body_flags() {
        let cmd = curl_for(&["https://example.com"]);
        assert!(!cmd.contains("-d "), "got: {cmd}");
        assert!(!cmd.contains("Content-Type:"), "got: {cmd}");
    }

    #[test]
    fn test_basic_auth_uses_dash_u() {
        let cmd = curl_for(&["--auth", "user:pass", "https://example.com"]);
        assert!(cmd.contains("-u user:pass"), "got: {cmd}");
        assert!(!cmd.contains("--digest"), "got: {cmd}");
    }

    #[test]
    fn test_bearer_auth_becomes_an_authorization_header() {
        let cmd = curl_for(&[
            "--auth-type",
            "bearer",
            "--auth",
            "tok123",
            "https://example.com",
        ]);
        // Regression: formatting the SecretString via Display emitted
        // "Bearer [REDACTED]", producing a curl command that could not run.
        assert!(cmd.contains("'Authorization: Bearer tok123'"), "got: {cmd}");
        assert!(!cmd.contains("REDACTED"), "got: {cmd}");
    }

    #[test]
    fn test_digest_auth_adds_the_digest_flag() {
        let cmd = curl_for(&[
            "--auth-type",
            "digest",
            "--auth",
            "user:pass",
            "https://example.com",
        ]);
        assert!(cmd.contains("--digest"), "got: {cmd}");
        assert!(cmd.contains("-u user:pass"), "got: {cmd}");
    }

    #[test]
    fn test_timeout_maps_to_max_time() {
        let cmd = curl_for(&["--timeout", "15", "https://example.com"]);
        assert!(cmd.contains("--max-time 15"), "got: {cmd}");
    }

    #[test]
    fn test_follow_redirects_maps_to_dash_l() {
        let cmd = curl_for(&["--follow", "https://example.com"]);
        assert!(cmd.contains("-L"), "got: {cmd}");
        // The default redirect cap is not spelled out.
        assert!(!cmd.contains("--max-redirs"), "got: {cmd}");
    }

    #[test]
    fn test_non_default_redirect_limit_is_emitted() {
        let cmd = curl_for(&["--follow", "--max-redirects", "5", "https://example.com"]);
        assert!(cmd.contains("--max-redirs 5"), "got: {cmd}");
    }

    #[test]
    fn test_verify_no_maps_to_insecure_flag() {
        let cmd = curl_for(&["--verify", "no", "https://example.com"]);
        assert!(cmd.contains(" -k"), "got: {cmd}");
        assert!(!cmd.contains("--cacert"), "got: {cmd}");
    }

    #[test]
    fn test_verify_yes_adds_nothing() {
        let cmd = curl_for(&["--verify", "yes", "https://example.com"]);
        assert!(!cmd.contains("-k"), "got: {cmd}");
        assert!(!cmd.contains("--cacert"), "got: {cmd}");
    }

    #[test]
    fn test_verify_path_becomes_cacert() {
        let cmd = curl_for(&["--verify", "/tmp/ca.pem", "https://example.com"]);
        assert!(cmd.contains("--cacert /tmp/ca.pem"), "got: {cmd}");
    }

    #[test]
    fn test_client_certificate_and_key() {
        let cmd = curl_for(&[
            "--cert",
            "/tmp/c.pem",
            "--cert-key",
            "/tmp/k.pem",
            "https://example.com",
        ]);
        assert!(cmd.contains("--cert /tmp/c.pem"), "got: {cmd}");
        assert!(cmd.contains("--key /tmp/k.pem"), "got: {cmd}");
    }

    #[test]
    fn test_proxies_map_to_dash_x() {
        let cmd = curl_for(&[
            "--proxy",
            "http:http://127.0.0.1:8080",
            "https://example.com",
        ]);
        assert!(cmd.contains("-x http:http://127.0.0.1:8080"), "got: {cmd}");
    }

    #[test]
    fn test_verbose_maps_to_dash_v() {
        let cmd = curl_for(&["--verbose", "https://example.com"]);
        assert!(cmd.contains(" -v"), "got: {cmd}");
    }

    #[test]
    fn test_header_from_file_is_inlined() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tok.txt");
        std::fs::write(&path, "secret-token\n").unwrap();

        let spec = format!("X-Token:@{}", path.display());
        let cmd = curl_for(&["https://example.com", &spec]);
        // The file's contents are trimmed and inlined into the header.
        assert!(cmd.contains("'X-Token: secret-token'"), "got: {cmd}");
    }

    #[test]
    fn test_empty_header_renders_with_a_bare_colon() {
        let cmd = curl_for(&["https://example.com", "X-Drop;"]);
        assert!(cmd.contains("X-Drop:"), "got: {cmd}");
    }

    #[test]
    fn test_data_field_from_file_is_inlined_into_the_body() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.txt");
        std::fs::write(&path, "from-file\n").unwrap();

        let spec = format!("key=@{}", path.display());
        let cmd = curl_for(&["POST", "https://example.com", &spec]);
        assert!(cmd.contains("from-file"), "got: {cmd}");
    }

    #[test]
    fn test_form_data_field_from_file_is_percent_encoded() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.txt");
        std::fs::write(&path, "a b\n").unwrap();

        let spec = format!("key=@{}", path.display());
        let cmd = curl_for(&["--form", "POST", "https://example.com", &spec]);
        assert!(cmd.contains("key=a%20b"), "got: {cmd}");
    }

    #[test]
    fn test_typed_json_field_keeps_its_json_type() {
        let cmd = curl_for(&["POST", "https://example.com", "count:=42", "ok:=true"]);
        assert!(cmd.contains("42") && !cmd.contains("\"42\""), "got: {cmd}");
        assert!(
            cmd.contains("true") && !cmd.contains("\"true\""),
            "got: {cmd}"
        );
    }

    #[test]
    fn test_body_is_valid_json_and_shell_safe() {
        let cmd = curl_for(&["POST", "https://example.com", "msg=hello world"]);

        // Pull the single-quoted argument that follows -d and re-parse it.
        let after = cmd.split("-d ").nth(1).expect("a -d argument");
        let body = after.trim_start_matches('\'');
        let end = body.find('\'').expect("closing quote");
        let json: serde_json::Value = serde_json::from_str(&body[..end]).expect("valid JSON body");
        assert_eq!(json["msg"], "hello world");
    }

    #[test]
    fn test_query_params_are_not_treated_as_body_data() {
        let cmd = curl_for(&["https://example.com", "q==search term"]);
        assert!(
            !cmd.contains("-d "),
            "query params are not body data: {cmd}"
        );
    }

    #[test]
    fn test_query_params_are_appended_to_the_exported_url() {
        // Regression: query params live in the item list, not processed.url, and
        // were previously dropped from the exported command entirely.
        let cmd = curl_for(&["https://example.com", "q==searchterm"]);
        assert!(
            cmd.contains("https://example.com?q=searchterm"),
            "got: {cmd}"
        );
    }

    #[test]
    fn test_query_params_use_form_style_encoding() {
        // Must match the real request path: space becomes '+', not %20.
        let cmd = curl_for(&["https://example.com", "q==search term"]);
        assert!(cmd.contains("q=search+term"), "got: {cmd}");
    }

    #[test]
    fn test_query_param_reserved_characters_are_percent_encoded() {
        let cmd = curl_for(&["https://example.com", "filter==a&b=c"]);
        assert!(cmd.contains("filter=a%26b%3Dc"), "got: {cmd}");
    }

    #[test]
    fn test_query_param_unreserved_characters_pass_through() {
        let cmd = curl_for(&["https://example.com", "k==a-b_c.d~e"]);
        assert!(cmd.contains("k=a-b_c.d~e"), "got: {cmd}");
    }

    #[test]
    fn test_multiple_query_params_are_joined_with_ampersands() {
        let cmd = curl_for(&["https://example.com", "a==1", "b==2"]);
        assert!(cmd.contains("a=1&b=2"), "got: {cmd}");
    }

    #[test]
    fn test_query_params_merge_into_a_url_that_already_has_some() {
        // An existing query string means the separator must be '&', not '?'.
        let cmd = curl_for(&["https://example.com/p?existing=1", "added==2"]);
        assert!(cmd.contains("existing=1&added=2"), "got: {cmd}");
        assert_eq!(cmd.matches('?').count(), 1, "only one '?' allowed: {cmd}");
    }

    #[test]
    fn test_query_params_from_file_are_appended_and_trimmed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("q.txt");
        std::fs::write(&path, "from file\n").unwrap();

        let spec = format!("q==@{}", path.display());
        let cmd = curl_for(&["https://example.com", &spec]);
        assert!(cmd.contains("q=from+file"), "got: {cmd}");
    }

    #[test]
    fn test_url_with_query_params_is_shell_quoted() {
        // '?' and '&' are shell metacharacters, so the URL must be quoted or the
        // shell would glob or background part of the command.
        let cmd = curl_for(&["https://example.com", "a==1", "b==2"]);
        assert!(
            cmd.contains("'https://example.com?a=1&b=2'"),
            "URL must be quoted: {cmd}"
        );
    }

    #[test]
    fn test_url_without_query_params_is_unchanged() {
        let cmd = curl_for(&["https://example.com/api"]);
        assert!(
            cmd.trim_end().ends_with("https://example.com/api"),
            "got: {cmd}"
        );
        assert!(!cmd.contains('?'), "got: {cmd}");
    }

    /// Extract just the single-quoted argument that follows `-d`.
    fn body_arg(cmd: &str) -> String {
        let after = cmd.split("-d ").nth(1).expect("a -d argument");
        let inner = after.trim_start_matches('\'');
        let end = inner.find('\'').expect("closing quote");
        inner[..end].to_string()
    }

    #[test]
    fn test_query_params_and_body_data_coexist() {
        let cmd = curl_for(&["POST", "https://example.com", "q==find", "field=value"]);
        assert!(cmd.contains("q=find"), "query missing: {cmd}");
        // The query belongs to the URL, not the body.
        let body = body_arg(&cmd);
        assert!(!body.contains("q=find"), "query leaked into body: {body}");
        assert!(body.contains("field"), "body missing its field: {body}");
    }

    #[test]
    fn test_exported_query_string_matches_the_real_request_url() {
        // The whole point of the exporter is fidelity: the query string it emits
        // must equal what core builds for the actual request.
        let args = Args::try_parse_from([
            "quicpulse",
            "https://example.com/p",
            "q==search term",
            "n==a&b",
        ])
        .unwrap();
        let processed = process_args(&args).unwrap();

        let exported = build_url(&processed);
        assert_eq!(exported, "https://example.com/p?q=search+term&n=a%26b");
    }

    #[test]
    fn test_combined_options_produce_one_well_formed_command() {
        let cmd = curl_for(&[
            "--follow",
            "--timeout",
            "10",
            "--verify",
            "no",
            "--auth",
            "u:p",
            "PUT",
            "https://example.com/x",
            "X-A:1",
            "f=v",
        ]);

        assert!(cmd.starts_with("curl -X PUT"), "got: {cmd}");
        assert!(cmd.contains("'X-A: 1'"), "got: {cmd}");
        assert!(cmd.contains("-u u:p"), "got: {cmd}");
        assert!(cmd.contains("--max-time 10"), "got: {cmd}");
        assert!(cmd.contains("-L"), "got: {cmd}");
        assert!(cmd.contains(" -k"), "got: {cmd}");
        assert!(
            cmd.trim_end().ends_with("https://example.com/x"),
            "got: {cmd}"
        );
    }
}
