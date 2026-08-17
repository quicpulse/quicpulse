//! Integration tests for Unix Domain Socket against live Docker daemon
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, docker_socket_path, is_docker_available};
use common::ExitStatus;

#[test]
fn test_docker_unix_socket_ping() {
    if !is_docker_available() {
        eprintln!("Skipping Docker test: Docker is not available or disabled in CI");
        return;
    }

    let socket_path = match docker_socket_path() {
        Some(p) => p,
        None => {
            eprintln!("Skipping Docker test: Docker socket not found");
            return;
        }
    };

    let socket_str = socket_path.to_string_lossy().to_string();

    let response = docker_http(&["--unix-socket", &socket_str, "http://localhost/_ping"]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Ping over Unix socket should succeed. Stderr: {}",
        response.stderr
    );
    assert!(
        response.stdout.contains("OK") || response.stdout.contains("200 OK"),
        "Should receive OK from Docker ping endpoint. Stdout: {}",
        response.stdout
    );
}

#[test]
fn test_docker_unix_socket_version_json() {
    if !is_docker_available() {
        eprintln!("Skipping Docker test: Docker is not available or disabled in CI");
        return;
    }

    let socket_path = match docker_socket_path() {
        Some(p) => p,
        None => return,
    };

    let socket_str = socket_path.to_string_lossy().to_string();

    let response = docker_http(&[
        "--unix-socket",
        &socket_str,
        "--print=b",
        "http://localhost/v1.41/version",
    ]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Version query over Unix socket should succeed. Stderr: {}",
        response.stderr
    );

    // Verify response body is valid JSON with docker version fields
    let body = response.body().unwrap_or(&response.stdout);
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(body.trim()) {
        assert!(
            json_val.get("Version").is_some() || json_val.get("ApiVersion").is_some(),
            "JSON response should contain Version or ApiVersion fields. Got: {json_val:?}"
        );
    } else {
        assert!(
            body.contains("Version") || body.contains("ApiVersion"),
            "Response should contain version info: {body}"
        );
    }
}

#[test]
fn test_docker_unix_socket_containers_list() {
    if !is_docker_available() {
        eprintln!("Skipping Docker test: Docker is not available or disabled in CI");
        return;
    }

    let socket_path = match docker_socket_path() {
        Some(p) => p,
        None => return,
    };

    let socket_str = socket_path.to_string_lossy().to_string();

    let response = docker_http(&[
        "--unix-socket",
        &socket_str,
        "http://localhost/containers/json",
        "all==true",
    ]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Listing containers over Unix socket should succeed. Stderr: {}",
        response.stderr
    );
}
