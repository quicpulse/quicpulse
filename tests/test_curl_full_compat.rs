use quicpulse::cli::Args;
use quicpulse::devexp::curl::generate_curl_command;
use quicpulse::devexp::curl_import::{import_curl, parse_curl_command};

#[test]
fn test_parse_curl_post_json() {
    let cmd = "curl -X POST -H 'Content-Type: application/json' -H 'Authorization: Bearer token123' -d '{\"name\":\"Alice\"}' https://api.example.com/users";
    let parsed = parse_curl_command(cmd).unwrap();

    assert_eq!(parsed.method, Some("POST".to_string()));
    assert_eq!(
        parsed.url,
        Some("https://api.example.com/users".to_string())
    );
    assert_eq!(parsed.data, Some("{\"name\":\"Alice\"}".to_string()));
    assert_eq!(parsed.bearer_token, Some("token123".to_string()));
    assert_eq!(parsed.headers.len(), 1);
}

#[test]
fn test_parse_curl_flags_redirect_insecure_auth() {
    let cmd = "curl -L -k -u admin:secret --compressed https://example.com/login";
    let parsed = parse_curl_command(cmd).unwrap();

    assert!(parsed.follow_redirects);
    assert!(parsed.insecure);
    assert!(parsed.compressed);
    assert_eq!(parsed.user, Some("admin:secret".to_string()));
}

#[test]
fn test_import_curl_to_args() {
    let cmd = "curl -X PUT https://api.example.com/item/1 -d 'data'";
    let args = import_curl(cmd).unwrap();

    assert_eq!(args.method, Some("PUT".to_string()));
    assert_eq!(args.url, Some("https://api.example.com/item/1".to_string()));
}

#[test]
fn test_generate_curl_command() {
    let args = Args {
        method: Some("DELETE".to_string()),
        url: Some("https://api.example.com/v1/resource/42".to_string()),
        follow: true,
        ..Default::default()
    };

    let processed = quicpulse::cli::process_args(&args).unwrap();
    let curl_cmd = generate_curl_command(&args, &processed);

    assert!(curl_cmd.starts_with("curl"));
    assert!(curl_cmd.contains("-X DELETE") || curl_cmd.contains("DELETE"));
    assert!(curl_cmd.contains("https://api.example.com/v1/resource/42"));
    assert!(curl_cmd.contains("-L") || curl_cmd.contains("--location"));
}
