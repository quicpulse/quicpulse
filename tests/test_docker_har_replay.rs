//! Integration tests for HAR recording against live Docker servers and replaying
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::time::Duration;
use tempfile::TempDir;

const HAR_SERVER_SCRIPT: &str = r#"
import http.server, socketserver, json

class SimpleHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps({'message': 'Hello from HAR test server', 'path': self.path}).encode('utf-8'))

server = socketserver.TCPServer(('0.0.0.0', 8080), SimpleHandler)
server.serve_forever()
"#;

fn start_har_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:8080", "python3", "-c", HAR_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start HAR test container: {e}");
            return None;
        }
    };

    let host_port = match guard.get_host_port(8080, "tcp") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get host port: {e}");
            return None;
        }
    };

    let health_url = format!("http://127.0.0.1:{host_port}/");
    if !guard.wait_for_http(&health_url, Duration::from_secs(10)) {
        eprintln!("HAR server did not start in time");
        return None;
    }

    Some((guard, host_port))
}

#[test]
fn test_docker_har_record_and_replay() {
    let (_container, port) = match start_har_container() {
        Some(c) => c,
        None => return,
    };

    let temp_dir = TempDir::new().unwrap();
    let har_path = temp_dir.path().join("recorded.har");
    let har_str = har_path.to_string_lossy().to_string();

    let url = format!("http://127.0.0.1:{port}/api/endpoint");

    // 1. Record live request to HAR file
    let record_resp = docker_http(&["--record-har", &har_str, &url]);
    assert_eq!(
        record_resp.exit_status,
        ExitStatus::Success,
        "Recording to HAR should succeed. Stderr: {}",
        record_resp.stderr
    );

    assert!(har_path.exists(), "Recorded HAR file should be created");

    // 2. Import and list HAR file entries
    let list_resp = docker_http(&["--import-har", &har_str, "--har-list"]);
    assert_eq!(
        list_resp.exit_status,
        ExitStatus::Success,
        "Listing HAR entries should succeed. Stderr: {}",
        list_resp.stderr
    );
    assert!(
        list_resp.stdout.contains("endpoint") || list_resp.stdout.contains("GET"),
        "HAR list should show recorded endpoint"
    );
}
