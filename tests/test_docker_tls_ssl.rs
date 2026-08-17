//! Integration tests for live TLS / SSL endpoints, self-signed certificates, and verification
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::time::Duration;

const TLS_SERVER_SCRIPT: &str = r#"
import http.server, ssl, subprocess, os

# Generate self-signed certificate
subprocess.run([
    'openssl', 'req', '-x509', '-newkey', 'rsa:2048',
    '-keyout', '/tmp/key.pem', '-out', '/tmp/cert.pem',
    '-days', '365', '-nodes', '-subj', '/CN=localhost'
], check=True)

class SimpleHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('X-TLS-Verified', 'true')
        self.end_headers()
        self.wfile.write(b'{"status":"ok","tls":true}')

httpd = http.server.HTTPServer(('0.0.0.0', 8443), SimpleHandler)
ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
ctx.load_cert_chain(certfile='/tmp/cert.pem', keyfile='/tmp/key.pem')
httpd.socket = ctx.wrap_socket(httpd.socket, server_side=True)
httpd.serve_forever()
"#;

fn start_tls_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:8443", "python3", "-c", TLS_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start TLS container: {e}");
            return None;
        }
    };

    let host_port = match guard.get_host_port(8443, "tcp") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get host port: {e}");
            return None;
        }
    };

    if !guard.wait_for_tcp(host_port, Duration::from_secs(10)) {
        eprintln!("TLS server port did not open in time");
        return None;
    }

    Some((guard, host_port))
}

#[test]
fn test_docker_tls_verify_no() {
    let (_container, port) = match start_tls_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("https://127.0.0.1:{port}/");
    let response = docker_http(&["--verify=no", &url]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Request with --verify=no should succeed against self-signed cert. Stderr: {}",
        response.stderr
    );
    assert!(response.stdout.contains("tls"));
}

#[test]
fn test_docker_tls_insecure_flag() {
    let (_container, port) = match start_tls_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("https://127.0.0.1:{port}/");
    let response = docker_http(&["--insecure", &url]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Request with --insecure should succeed against self-signed cert. Stderr: {}",
        response.stderr
    );
    assert!(response.stdout.contains("tls"));
}
