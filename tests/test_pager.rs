//! Tests for the pager module

use std::io::Cursor;

mod common;

#[test]
fn test_pager_config_default() {
    use quicpulse::output::pager::PagerConfig;

    let config = PagerConfig::default();
    assert!(!config.enabled);
    assert!(config.command.is_none());
}

#[test]
fn test_pager_config_custom() {
    use quicpulse::output::pager::PagerConfig;

    let config = PagerConfig {
        enabled: true,
        command: Some("less -R".to_string()),
    };
    assert!(config.enabled);
    assert_eq!(config.command, Some("less -R".to_string()));
}

#[test]
fn test_get_pager_command_default() {
    use quicpulse::output::pager::get_pager_command;

    // When PAGER is not set, should return default
    let cmd = get_pager_command();
    assert!(!cmd.is_empty());
    // Default should contain "less"
    assert!(cmd.contains("less") || std::env::var("PAGER").is_ok());
}

#[test]
fn test_should_page_forced() {
    use quicpulse::output::pager::should_page;

    // Forced paging should always return true
    assert!(should_page("short", false, true));
    assert!(should_page("short", true, true));
    assert!(should_page("", false, true));
}

#[test]
fn test_should_page_not_tty() {
    use quicpulse::output::pager::should_page;

    // Not a TTY should never page (unless forced)
    let long_content = "line\n".repeat(100);
    assert!(!should_page(&long_content, false, false));
    assert!(!should_page("short", false, false));
}

#[test]
fn test_should_page_short_content() {
    use quicpulse::output::pager::should_page;

    // Short content should not page even on TTY
    assert!(!should_page("short", true, false));
    assert!(!should_page("line\n", true, false));
}

#[test]
fn test_write_with_pager_disabled() {
    use quicpulse::output::pager::{PagerConfig, write_with_pager};

    let config = PagerConfig {
        enabled: false,
        command: None,
    };

    let content = "Hello, World!";
    let mut output = Cursor::new(Vec::new());

    let result = write_with_pager(&mut output, content, &config, false);
    assert!(result.is_ok());

    // Content should be written directly
    let written = String::from_utf8(output.into_inner()).unwrap();
    assert_eq!(written, content);
}

#[test]
fn test_write_with_pager_not_tty() {
    use quicpulse::output::pager::{PagerConfig, write_with_pager};

    let config = PagerConfig {
        enabled: true,
        command: None,
    };

    let content = "Hello, World!";
    let mut output = Cursor::new(Vec::new());

    // Not a TTY, so pager should be bypassed
    let result = write_with_pager(&mut output, content, &config, false);
    assert!(result.is_ok());

    let written = String::from_utf8(output.into_inner()).unwrap();
    assert_eq!(written, content);
}

#[test]
fn test_pager_writer_empty_command() {
    use quicpulse::output::pager::PagerWriter;

    // Empty command should fail
    let result = PagerWriter::with_command("");
    assert!(result.is_err());
}

#[test]
fn test_pager_writer_invalid_command() {
    use quicpulse::output::pager::PagerWriter;

    // Non-existent command should fail
    let result = PagerWriter::with_command("nonexistent_command_12345");
    assert!(result.is_err());
}

#[test]
fn test_pager_writer_echo_command() {
    use quicpulse::output::pager::PagerWriter;
    use std::io::Write;

    // Use cat as a simple pager that just outputs
    let result = PagerWriter::with_command("cat");
    if let Ok(mut writer) = result {
        let write_result = writer.write_all(b"test content");
        assert!(write_result.is_ok());
        let wait_result = writer.wait();
        assert!(wait_result.is_ok());
    }
}
