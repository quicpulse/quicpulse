//! Unit tests for binary data detection and human-readable byte formatting

use quicpulse::binary::{format_bytes, is_binary};

#[test]
fn test_is_binary_detection() {
    assert!(is_binary(&[0x89, 0x50, 0x4E, 0x47])); // PNG signature
    assert!(is_binary(&[0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00])); // Null bytes binary stream

    assert!(!is_binary(b"Hello, World!"));
    assert!(!is_binary(b"{\"status\":\"ok\",\"code\":200}"));
    assert!(!is_binary(
        b"<html><head><title>Test</title></head><body><h1>Hello</h1></body></html>"
    ));
}

#[test]
fn test_format_bytes_human_readable() {
    assert_eq!(format_bytes(0, 1), "0.0 B");
    assert_eq!(format_bytes(512, 0), "512 B");
    assert_eq!(format_bytes(1024, 2), "1.00 KiB");
    assert_eq!(format_bytes(1048576, 2), "1.00 MiB");
    assert_eq!(format_bytes(1073741824, 2), "1.00 GiB");
}
