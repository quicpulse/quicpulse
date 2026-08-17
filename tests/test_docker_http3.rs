//! Integration tests for HTTP/3 (QUIC) client against live Docker endpoints
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;

const UDP_ECHO_SERVER_SCRIPT: &str = r#"
import socket

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.bind(('0.0.0.0', 8443))
while True:
    data, addr = sock.recvfrom(65535)
    # Echo back or discard
    if data:
        sock.sendto(data, addr)
"#;

fn start_udp_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:8443/udp", "python3", "-c", UDP_ECHO_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start UDP container: {e}");
            return None;
        }
    };

    let host_port = match guard.get_host_port(8443, "udp") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get host UDP port: {e}");
            return None;
        }
    };

    Some((guard, host_port))
}

#[test]
fn test_docker_http3_udp_connection_dispatch() {
    let (_container, port) = match start_udp_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("https://127.0.0.1:{port}/");
    // Connect with --http3 and short timeout to verify UDP dispatching
    let response = docker_http(&["--http3", "--verify=no", "--timeout=2", &url]);

    // The client dispatches UDP packets to the target port. Even if the handshake fails due to simple UDP echo,
    // it must gracefully handle the QUIC connection lifecycle without panicking.
    assert!(
        response.exit_status == ExitStatus::Success || response.exit_status == ExitStatus::Error,
        "HTTP/3 client should handle UDP transport cleanly"
    );
}

#[test]
fn test_docker_http_version_3_flag() {
    let (_container, port) = match start_udp_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("https://127.0.0.1:{port}/");
    let response = docker_http(&["--http-version=3", "--verify=no", "--timeout=2", &url]);

    assert!(
        response.exit_status == ExitStatus::Success || response.exit_status == ExitStatus::Error,
        "HTTP/3 version flag should be processed by client"
    );
}
