//! Integration tests for live GraphQL endpoints in Docker
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::time::Duration;

const GRAPHQL_SERVER_SCRIPT: &str = r#"
import http.server, socketserver, json

class GraphQLHandler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(length).decode('utf-8')
        try:
            payload = json.loads(body)
        except Exception:
            self.send_response(400)
            self.end_headers()
            return

        query = payload.get('query', '')
        variables = payload.get('variables', {})

        if '__schema' in query:
            resp_data = {
                'data': {
                    '__schema': {
                        'types': [{'name': 'Query'}, {'name': 'Mutation'}, {'name': 'User'}]
                    }
                }
            }
        elif 'createUser' in query:
            resp_data = {
                'data': {
                    'createUser': {'id': 'usr_100', 'name': 'Bob', 'status': 'created'}
                }
            }
        elif 'GetUser' in query or '$id' in query:
            user_id = variables.get('id', 1)
            resp_data = {
                'data': {
                    'user': {'id': user_id, 'name': f'User_{user_id}'}
                }
            }
        else:
            resp_data = {
                'data': {
                    'viewer': {'id': 'viewer_1', 'name': 'Alice', 'role': 'admin'}
                }
            }

        resp_bytes = json.dumps(resp_data).encode('utf-8')
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(resp_bytes)))
        self.end_headers()
        self.wfile.write(resp_bytes)

    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Length', '2')
        self.end_headers()
        self.wfile.write(b'OK')

server = socketserver.TCPServer(('0.0.0.0', 8080), GraphQLHandler)
server.serve_forever()
"#;

fn start_graphql_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:8080", "python3", "-c", GRAPHQL_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start GraphQL container: {e}");
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
        eprintln!("GraphQL container did not start in time");
        return None;
    }

    Some((guard, host_port))
}

#[test]
fn test_docker_graphql_query() {
    let (_container, port) = match start_graphql_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("http://127.0.0.1:{port}/graphql");
    let response = docker_http(&["--graphql", &url, "query={ viewer { name role } }"]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "GraphQL query should succeed. Stderr: {}",
        response.stderr
    );
    assert!(response.stdout.contains("Alice"));
    assert!(response.stdout.contains("admin"));
}

#[test]
fn test_docker_graphql_with_variables() {
    let (_container, port) = match start_graphql_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("http://127.0.0.1:{port}/graphql");
    let response = docker_http(&[
        "--graphql",
        &url,
        "query=query GetUser($id: Int!) { user(id: $id) { id name } }",
        "--graphql-var",
        "id=42",
    ]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "GraphQL query with variables should succeed. Stderr: {}",
        response.stderr
    );
    assert!(response.stdout.contains("User_42") || response.stdout.contains("42"));
}

#[test]
fn test_docker_graphql_mutation() {
    let (_container, port) = match start_graphql_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("http://127.0.0.1:{port}/graphql");
    let response = docker_http(&[
        "--graphql",
        &url,
        "query=mutation { createUser(name: \"Bob\") { id name status } }",
    ]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "GraphQL mutation should succeed. Stderr: {}",
        response.stderr
    );
    assert!(response.stdout.contains("Bob"));
    assert!(response.stdout.contains("usr_100"));
}
