//! Integration tests for multi-step Workflows (--run) against live Docker endpoints
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

const WORKFLOW_SERVER_SCRIPT: &str = r#"
import http.server, socketserver, json

class WorkflowHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/api/init':
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'token': 'token_xyz123', 'status': 'ready'}).encode('utf-8'))
        else:
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(b'{"status":"ok"}')

    def do_POST(self):
        length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(length).decode('utf-8')
        auth = self.headers.get('Authorization', '')

        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps({'created': True, 'auth': auth, 'body': body}).encode('utf-8'))

server = socketserver.TCPServer(('0.0.0.0', 8080), WorkflowHandler)
server.serve_forever()
"#;

fn start_workflow_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:8080", "python3", "-c", WORKFLOW_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start workflow container: {e}");
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

    let health_url = format!("http://127.0.0.1:{host_port}/api/init");
    if !guard.wait_for_http(&health_url, Duration::from_secs(10)) {
        eprintln!("Workflow server did not start in time");
        return None;
    }

    Some((guard, host_port))
}

#[test]
fn test_docker_workflow_multi_step_live() {
    let (_container, port) = match start_workflow_container() {
        Some(c) => c,
        None => return,
    };

    let temp_dir = TempDir::new().unwrap();
    let workflow_file = temp_dir.path().join("live_workflow.yaml");

    let yaml_content = r#"
name: Live Docker Workflow
steps:
  - name: Step 1 Init
    method: GET
    url: "{{ base_url }}/api/init"
    assert:
      status: 200

  - name: Step 2 Create Resource
    method: POST
    url: "{{ base_url }}/api/create"
    headers:
      Content-Type: application/json
    body: '{"action":"create_item"}'
    assert:
      status: 200
"#;

    fs::write(&workflow_file, yaml_content).unwrap();
    let workflow_str = workflow_file.to_string_lossy().to_string();
    let base_url = format!("base_url=http://127.0.0.1:{port}");

    let response = docker_http(&["--run", &workflow_str, "--var", &base_url]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Workflow execution against live container should succeed. Stderr: {}",
        response.stderr
    );
    assert!(
        response.stdout.contains("Step 1 Init")
            || response.stdout.contains("Step 2 Create")
            || response.stdout.contains("passed")
            || response.exit_code == 0
    );
}
