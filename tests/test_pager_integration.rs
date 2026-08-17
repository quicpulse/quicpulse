//! Integration tests for Pager CLI options

mod common;
use common::http;

#[test]
fn test_no_pager_flag_offline() {
    let r = http(&["--offline", "--no-pager", "http://example.com"]);
    assert_eq!(r.exit_code, 0);
}

#[test]
fn test_pager_cmd_flag_offline() {
    let r = http(&["--offline", "--pager-cmd=cat", "http://example.com"]);
    assert_eq!(r.exit_code, 0);
}
