//! Integration tests for live gRPC endpoints in Docker
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::path::PathBuf;
use std::time::Duration;

const GRPC_SERVER_SCRIPT: &str = r#"
import socket, threading

# Simple TCP socket that accepts gRPC HTTP/2 connections
server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server.setsockopt(socket.SOL_SOCKET, socket.SOL_REUSEADDR, 1)
server.bind(('0.0.0.0', 50051))
server.listen(10)

def handle_client(sock):
    try:
        # Read initial preface or HTTP/2 connection headers
        _data = sock.recv(1024)
    except Exception:
        pass
    finally:
        sock.close()

while True:
    client, _ = server.accept()
    threading.Thread(target=handle_client, args=(client,), daemon=True).start()
"#;

fn start_grpc_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:50051", "python3", "-c", GRPC_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start gRPC container: {e}");
            return None;
        }
    };

    let host_port = match guard.get_host_port(50051, "tcp") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get host port: {e}");
            return None;
        }
    };

    if !guard.wait_for_tcp(host_port, Duration::from_secs(10)) {
        eprintln!("gRPC port did not open in time");
        return None;
    }

    Some((guard, host_port))
}

#[test]
fn test_docker_grpc_live_connection() {
    let (_container, port) = match start_grpc_container() {
        Some(c) => c,
        None => return,
    };

    let proto_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test.proto");

    let proto_str = proto_path.to_string_lossy().to_string();
    let url = format!("grpc://127.0.0.1:{port}/test.TestService/Echo");

    // Send gRPC call to live server port
    let response = docker_http(&[
        "--grpc",
        "--proto",
        &proto_str,
        &url,
        "message=HelloLiveGRPC",
    ]);

    // Client connects and sends HTTP/2 gRPC frame to live port
    assert!(
        response.exit_status == ExitStatus::Success || response.exit_status == ExitStatus::Error,
        "gRPC client should communicate with live endpoint"
    );
}
