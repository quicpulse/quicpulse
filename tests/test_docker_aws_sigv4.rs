//! Integration tests for AWS SigV4 authentication against a live server
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::time::Duration;

const AWS_SERVER_SCRIPT: &str = r#"
import http.server, socketserver, json

class AWSMockHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        auth = self.headers.get('Authorization', '')
        amz_date = self.headers.get('x-amz-date', '')
        
        valid = 'AWS4-HMAC-SHA256' in auth and 'Credential=' in auth and 'Signature=' in auth
        
        if valid:
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({
                'authenticated': True,
                'auth_header': auth,
                'amz_date': amz_date
            }).encode('utf-8'))
        else:
            self.send_response(403)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({
                'error': 'Missing or invalid AWS SigV4 signature',
                'received_auth': auth
            }).encode('utf-8'))

server = socketserver.TCPServer(('0.0.0.0', 9000), AWSMockHandler)
server.serve_forever()
"#;

fn start_aws_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:9000", "python3", "-c", AWS_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start AWS container: {e}");
            return None;
        }
    };

    let host_port = match guard.get_host_port(9000, "tcp") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get host port: {e}");
            return None;
        }
    };

    if !guard.wait_for_tcp(host_port, Duration::from_secs(10)) {
        eprintln!("AWS server port did not open in time");
        return None;
    }

    Some((guard, host_port))
}

#[test]
fn test_docker_aws_sigv4_request() {
    let (_container, port) = match start_aws_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("http://127.0.0.1:{port}/my-bucket/object.txt");
    let response = docker_http(&[
        "--auth-type=aws-sigv4",
        "-a",
        "AKIAIOSFODNN7EXAMPLE:wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        "--aws-region=us-east-1",
        "--aws-service=s3",
        &url,
    ]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "AWS SigV4 request should succeed against live server. Stderr: {}",
        response.stderr
    );
    assert!(response.stdout.contains("AWS4-HMAC-SHA256"));
    assert!(response.stdout.contains("AKIAIOSFODNN7EXAMPLE"));
}
