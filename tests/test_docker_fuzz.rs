//! Integration tests for live API Fuzzing (--fuzz) against Docker HTTP endpoints
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::time::Duration;

const FUZZ_SERVER_SCRIPT: &str = r#"
import http.server, socketserver

class FuzzHandler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get('Content-Length', 0))
        _body = self.rfile.read(length)
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(b'{"status":"ok"}')

    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Length', '2')
        self.end_headers()
        self.wfile.write(b'OK')

    def log_message(self, format, *args):
        pass

class ThreadedHTTPServer(socketserver.ThreadingMixIn, http.server.HTTPServer):
    allow_reuse_address = True

server = ThreadedHTTPServer(('0.0.0.0', 8080), FuzzHandler)
server.serve_forever()
"#;

fn start_fuzz_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:8080", "python3", "-c", FUZZ_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start fuzz container: {e}");
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
        eprintln!("Fuzz server did not start in time");
        return None;
    }

    Some((guard, host_port))
}

#[test]
fn test_docker_fuzz_live_endpoint() {
    let (_container, port) = match start_fuzz_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("http://127.0.0.1:{port}/api/fuzz");
    let response = docker_http(&["--fuzz", "POST", &url, "name=admin", "role=user"]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Fuzz testing against live endpoint should succeed. Stderr: {}",
        response.stderr
    );

    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(
        output.contains("Fuzz")
            || output.contains("fuzz")
            || output.contains("requests")
            || output.contains("payload")
            || output.contains("OK"),
        "Output should show fuzzing progress/summary. Output: {}",
        output
    );
}
