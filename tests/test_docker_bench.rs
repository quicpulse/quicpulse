//! Integration tests for live Benchmarking (--bench) against Docker HTTP endpoints
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::time::Duration;

const BENCH_SERVER_SCRIPT: &str = r#"
import http.server, socketserver

class FastHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'text/plain')
        self.send_header('Content-Length', '2')
        self.end_headers()
        self.wfile.write(b'OK')

    def log_message(self, format, *args):
        pass

class ThreadedHTTPServer(socketserver.ThreadingMixIn, http.server.HTTPServer):
    allow_reuse_address = True

server = ThreadedHTTPServer(('0.0.0.0', 8080), FastHandler)
server.serve_forever()
"#;

fn start_bench_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:8080", "python3", "-c", BENCH_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start benchmark container: {e}");
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
        eprintln!("Benchmark server did not start in time");
        return None;
    }

    Some((guard, host_port))
}

#[test]
fn test_docker_bench_live_server() {
    let (_container, port) = match start_bench_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("http://127.0.0.1:{port}/");
    let response = docker_http(&["--bench", "--requests", "30", "--concurrency", "5", &url]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Benchmark run against live server should succeed. Stderr: {}",
        response.stderr
    );

    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(
        output.contains("requests")
            || output.contains("latency")
            || output.contains("Benchmark")
            || output.contains("req/s")
            || output.contains("p50")
            || output.contains("p95"),
        "Output should show benchmark metrics. Output: {}",
        output
    );
}
