//! Integration tests for live HTTP/1.1 and HTTP/2 services in Docker
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::time::Duration;

const HTTP_SERVER_SCRIPT: &str = r#"
import http.server, socketserver, json, urllib.parse, gzip, io, base64

class RequestHandler(http.server.BaseHTTPRequestHandler):
    protocol_version = 'HTTP/1.1'

    def _send_json(self, status, data, headers=None):
        body = json.dumps(data).encode('utf-8')
        self.send_response(status)
        self.send_header('Content-Type', 'application/json; charset=utf-8')
        self.send_header('Content-Length', str(len(body)))
        if headers:
            for k, v in headers.items():
                self.send_header(k, v)
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        query = urllib.parse.parse_qs(parsed.query)

        if path == '/get':
            self._send_json(200, {
                'url': self.path,
                'args': query,
                'headers': dict(self.headers),
                'method': 'GET'
            })
        elif path.startswith('/status/'):
            code = int(path.split('/')[-1])
            self._send_json(code, {'status': code})
        elif path.startswith('/redirect/'):
            count = int(path.split('/')[-1])
            if count <= 1:
                self.send_response(302)
                self.send_header('Location', '/get')
                self.end_headers()
            else:
                self.send_response(302)
                self.send_header('Location', f'/redirect/{count-1}')
                self.end_headers()
        elif path == '/cookies':
            self._send_json(200, {'cookies': self.headers.get('Cookie', '')})
        elif path == '/cookies/set':
            cookie_hdr = parsed.query
            self.send_response(302)
            self.send_header('Set-Cookie', cookie_hdr)
            self.send_header('Location', '/cookies')
            self.end_headers()
        elif path.startswith('/basic-auth/'):
            parts = path.split('/')[2:]
            user, pwd = parts[0], parts[1]
            auth = self.headers.get('Authorization', '')
            expected = 'Basic ' + base64.b64encode(f'{user}:{pwd}'.encode()).decode()
            if auth == expected:
                self._send_json(200, {'authenticated': True, 'user': user})
            else:
                self.send_response(401)
                self.send_header('WWW-Authenticate', 'Basic realm="Test"')
                self.end_headers()
        elif path == '/bearer':
            auth = self.headers.get('Authorization', '')
            if auth.startswith('Bearer '):
                self._send_json(200, {'authenticated': True, 'token': auth[7:]})
            else:
                self.send_response(401)
                self.end_headers()
        elif path == '/gzip':
            data = json.dumps({'gzipped': True, 'headers': dict(self.headers)}).encode('utf-8')
            buf = io.BytesIO()
            with gzip.GzipFile(fileobj=buf, mode='w') as gz:
                gz.write(data)
            gzipped_body = buf.getvalue()
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Encoding', 'gzip')
            self.send_header('Content-Length', str(len(gzipped_body)))
            self.end_headers()
            self.wfile.write(gzipped_body)
        elif path == '/health':
            self.send_response(200)
            self.send_header('Content-Length', '2')
            self.end_headers()
            self.wfile.write(b'OK')
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        self._handle_with_body('POST')

    def do_PUT(self):
        self._handle_with_body('PUT')

    def do_PATCH(self):
        self._handle_with_body('PATCH')

    def do_DELETE(self):
        self._handle_with_body('DELETE')

    def _handle_with_body(self, method):
        parsed = urllib.parse.urlparse(self.path)
        length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(length).decode('utf-8', errors='replace')
        json_data = None
        try:
            json_data = json.loads(body)
        except Exception:
            pass

        self._send_json(200, {
            'method': method,
            'url': self.path,
            'headers': dict(self.headers),
            'data': body,
            'json': json_data
        })

class ThreadedHTTPServer(socketserver.ThreadingMixIn, http.server.HTTPServer):
    allow_reuse_address = True

server = ThreadedHTTPServer(('0.0.0.0', 8080), RequestHandler)
server.serve_forever()
"#;

fn start_http_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:8080", "python3", "-c", HTTP_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start python HTTP server container: {e}");
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

    // Wait for server health endpoint
    let health_url = format!("http://127.0.0.1:{host_port}/health");
    if !guard.wait_for_http(&health_url, Duration::from_secs(10)) {
        eprintln!("Container did not become healthy in time");
        return None;
    }

    Some((guard, host_port))
}

#[test]
fn test_docker_http_get_query_params() {
    let (_container, port) = match start_http_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("http://127.0.0.1:{port}/get");
    let response = docker_http(&[
        &url,
        "search==rust",
        "limit==10",
        "X-Custom-Header:test-value",
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    assert!(response.stdout.contains("200 OK"));
    assert!(response.stdout.contains("search"));
    assert!(response.stdout.contains("rust"));
    assert!(response.stdout.contains("X-Custom-Header"));
}

#[test]
fn test_docker_http_post_json_payload() {
    let (_container, port) = match start_http_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("http://127.0.0.1:{port}/post");
    let response = docker_http(&[&url, "name=quicpulse", "active:=true", "count:=42"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    assert!(response.stdout.contains("200 OK"));
    assert!(response.stdout.contains("quicpulse"));
    assert!(response.stdout.contains("active"));
}

#[test]
fn test_docker_http_put_patch_delete_verbs() {
    let (_container, port) = match start_http_container() {
        Some(c) => c,
        None => return,
    };

    // PUT
    let put_url = format!("http://127.0.0.1:{port}/put");
    let put_resp = docker_http(&["PUT", &put_url, "action=update"]);
    assert_eq!(put_resp.exit_status, ExitStatus::Success);
    assert!(put_resp.stdout.contains("PUT"));

    // PATCH
    let patch_url = format!("http://127.0.0.1:{port}/patch");
    let patch_resp = docker_http(&["PATCH", &patch_url, "status=patched"]);
    assert_eq!(patch_resp.exit_status, ExitStatus::Success);
    assert!(patch_resp.stdout.contains("PATCH"));

    // DELETE
    let delete_url = format!("http://127.0.0.1:{port}/delete");
    let del_resp = docker_http(&["DELETE", &delete_url]);
    assert_eq!(del_resp.exit_status, ExitStatus::Success);
    assert!(del_resp.stdout.contains("DELETE"));
}

#[test]
fn test_docker_http_redirect_follow() {
    let (_container, port) = match start_http_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("http://127.0.0.1:{port}/redirect/3");
    let response = docker_http(&["--follow", &url]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    assert!(response.stdout.contains("200 OK"));
}

#[test]
fn test_docker_http_basic_and_bearer_auth() {
    let (_container, port) = match start_http_container() {
        Some(c) => c,
        None => return,
    };

    // Basic Auth
    let basic_url = format!("http://127.0.0.1:{port}/basic-auth/admin/secret123");
    let basic_resp = docker_http(&["-a", "admin:secret123", &basic_url]);
    assert_eq!(basic_resp.exit_status, ExitStatus::Success);
    assert!(basic_resp.stdout.contains("authenticated"));

    // Bearer Auth
    let bearer_url = format!("http://127.0.0.1:{port}/bearer");
    let bearer_resp = docker_http(&["-A", "bearer", "-a", "my-secret-jwt-token", &bearer_url]);
    assert_eq!(bearer_resp.exit_status, ExitStatus::Success);
    assert!(bearer_resp.stdout.contains("my-secret-jwt-token"));
}

#[test]
fn test_docker_http_gzip_decompression() {
    let (_container, port) = match start_http_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("http://127.0.0.1:{port}/gzip");
    let response = docker_http(&[&url]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    assert!(response.stdout.contains("gzipped"));
}

#[test]
fn test_docker_http_status_code_exit() {
    let (_container, port) = match start_http_container() {
        Some(c) => c,
        None => return,
    };

    let url = format!("http://127.0.0.1:{port}/status/404");
    let response = docker_http(&["--check-status", &url]);

    // --check-status on 404 should return non-zero exit status
    assert_eq!(response.exit_status, ExitStatus::Error);
}
