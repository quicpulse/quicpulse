//! Tests for .netrc credential lookup, GCP, and Azure CLI authentication configurations

mod common;

use common::http;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_netrc_parsing_with_custom_file() {
    let dir = TempDir::new().unwrap();
    let netrc_file = dir.path().join(".netrc");
    fs::write(
        &netrc_file,
        r#"
# Sample .netrc configuration
machine api.example.com login alice password secretpassword
machine gitlab.com login git password token123

default login guest password anonymous
"#,
    )
    .unwrap();

    // Netrc parsing can be tested via file load
    let netrc = quicpulse::auth::netrc::Netrc::load_from(&netrc_file).unwrap();
    assert_eq!(
        netrc.get_credentials("api.example.com"),
        Some(("alice".to_string(), "secretpassword".to_string()))
    );
    assert_eq!(
        netrc.get_credentials("api.example.com:443"),
        Some(("alice".to_string(), "secretpassword".to_string()))
    );
    assert_eq!(
        netrc.get_credentials("gitlab.com"),
        Some(("git".to_string(), "token123".to_string()))
    );
    assert_eq!(
        netrc.get_credentials("unknown-site.org"),
        Some(("guest".to_string(), "anonymous".to_string()))
    );
}

#[test]
fn test_ignore_netrc_flag_offline() {
    let r = http(&["--offline", "--ignore-netrc", "http://example.com/api"]);
    assert_eq!(r.exit_code, 0);
}
