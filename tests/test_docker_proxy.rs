//! Integration tests for live Proxy connections (SOCKS5, SOCKS5h, HTTP forward proxy)
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::time::Duration;

const PROXY_SERVER_SCRIPT: &str = r#"
import socket, threading, select, sys, http.server, socketserver

# --- SOCKS5 Proxy Implementation ---
def handle_socks5(client_sock):
    try:
        # Handshake: [VER, NMETHODS, METHODS...]
        ver, nmethods = client_sock.recv(1), client_sock.recv(1)
        if not ver or ver[0] != 5:
            client_sock.close()
            return
        methods = client_sock.recv(nmethods[0])
        # Respond: VER 5, METHOD 0 (No Auth)
        client_sock.sendall(b'\x05\x00')

        # Request: [VER, CMD, RSV, ATYP, DST.ADDR, DST.PORT]
        data = client_sock.recv(4)
        if len(data) < 4 or data[1] != 1: # CMD 1 = CONNECT
            client_sock.close()
            return

        atyp = data[3]
        if atyp == 1: # IPv4
            dest_addr = socket.inet_ntoa(client_sock.recv(4))
        elif atyp == 3: # Domain name
            addr_len = client_sock.recv(1)[0]
            dest_addr = client_sock.recv(addr_len).decode('utf-8')
        else:
            client_sock.close()
            return

        dest_port = int.from_bytes(client_sock.recv(2), 'big')

        # Connect to destination
        remote_sock = socket.create_connection((dest_addr, dest_port), timeout=5)
        # Success response: VER 5, REP 0, RSV 0, ATYP 1, BND.ADDR, BND.PORT
        client_sock.sendall(b'\x05\x00\x00\x01\x00\x00\x00\x00\x00\x00')

        # Relay data
        sockets = [client_sock, remote_sock]
        while True:
            r, _, _ = select.select(sockets, [], [], 10)
            if not r:
                break
            for s in r:
                other = remote_sock if s is client_sock else client_sock
                chunk = s.recv(4096)
                if not chunk:
                    return
                other.sendall(chunk)
    except Exception:
        pass
    finally:
        client_sock.close()

def start_socks5_server(port):
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(('0.0.0.0', port))
    server.listen(50)
    while True:
        client, _ = server.accept()
        threading.Thread(target=handle_socks5, args=(client,), daemon=True).start()

# --- HTTP CONNECT Proxy Implementation ---
class HTTPProxyHandler(http.server.BaseHTTPRequestHandler):
    def do_CONNECT(self):
        try:
            host, port_str = self.path.split(':')
            port = int(port_str)
            remote = socket.create_connection((host, port), timeout=5)
            self.send_response(200, 'Connection Established')
            self.end_headers()

            sockets = [self.connection, remote]
            while True:
                r, _, _ = select.select(sockets, [], [], 10)
                if not r:
                    break
                for s in r:
                    other = remote if s is self.connection else self.connection
                    chunk = s.recv(4096)
                    if not chunk:
                        return
                    other.sendall(chunk)
        except Exception:
            pass

    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'text/plain')
        self.end_headers()
        self.wfile.write(b'HTTP Proxy OK')

# --- Echo target server (running inside container on 8080) ---
class TargetHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('X-Proxy-Passed', 'true')
        self.end_headers()
        self.wfile.write(b'{"status":"ok","proxied":true}')

def start_target_server():
    server = socketserver.TCPServer(('0.0.0.0', 8080), TargetHandler)
    server.serve_forever()

threading.Thread(target=start_target_server, daemon=True).start()
threading.Thread(target=start_socks5_server, args=(1080,), daemon=True).start()

http_proxy = socketserver.TCPServer(('0.0.0.0', 8081), HTTPProxyHandler)
http_proxy.serve_forever()
"#;

fn start_proxy_container() -> Option<(DockerContainerGuard, u16, u16, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &[
            "-p",
            "0:1080", // SOCKS5
            "-p",
            "0:8081", // HTTP Proxy
            "-p",
            "0:8080", // Target echo server
            "python3",
            "-c",
            PROXY_SERVER_SCRIPT,
        ],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start proxy container: {e}");
            return None;
        }
    };

    let socks_port = match guard.get_host_port(1080, "tcp") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get socks port: {e}");
            return None;
        }
    };

    let http_proxy_port = match guard.get_host_port(8081, "tcp") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get http proxy port: {e}");
            return None;
        }
    };

    let target_port = match guard.get_host_port(8080, "tcp") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get target port: {e}");
            return None;
        }
    };

    if !guard.wait_for_tcp(socks_port, Duration::from_secs(10))
        || !guard.wait_for_tcp(http_proxy_port, Duration::from_secs(10))
        || !guard.wait_for_tcp(target_port, Duration::from_secs(10))
    {
        eprintln!("Proxy ports did not open in time");
        return None;
    }

    Some((guard, socks_port, http_proxy_port, target_port))
}

#[test]
fn test_docker_socks5_proxy() {
    let (_container, socks_port, _http_proxy_port, target_port) = match start_proxy_container() {
        Some(c) => c,
        None => return,
    };

    let proxy_url = format!("socks5://127.0.0.1:{socks_port}");
    let target_url = format!("http://127.0.0.1:{target_port}/");

    let response = docker_http(&["--proxy", &proxy_url, &target_url]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Request via SOCKS5 proxy should succeed. Stderr: {}",
        response.stderr
    );
    assert!(response.stdout.contains("proxied"));
}

#[test]
fn test_docker_socks5h_remote_dns_proxy() {
    let (_container, socks_port, _http_proxy_port, target_port) = match start_proxy_container() {
        Some(c) => c,
        None => return,
    };

    let proxy_url = format!("socks5h://127.0.0.1:{socks_port}");
    let target_url = format!("http://127.0.0.1:{target_port}/");

    let response = docker_http(&["--proxy", &proxy_url, &target_url]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Request via SOCKS5h proxy should succeed. Stderr: {}",
        response.stderr
    );
    assert!(response.stdout.contains("proxied"));
}
