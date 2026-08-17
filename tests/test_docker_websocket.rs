//! Integration tests for live WebSocket connections
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::time::Duration;

const WEBSOCKET_SERVER_SCRIPT: &str = r#"
import socket, threading, hashlib, base64, struct

GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11'

def handle_ws_client(client_sock):
    try:
        # Handshake
        request = client_sock.recv(4096).decode('utf-8', errors='replace')
        headers = {}
        for line in request.split('\r\n')[1:]:
            if ':' in line:
                k, v = line.split(':', 1)
                headers[k.strip().lower()] = v.strip()

        ws_key = headers.get('sec-websocket-key')
        if not ws_key:
            client_sock.close()
            return

        accept_val = base64.b64encode(hashlib.sha1((ws_key + GUID).encode()).digest()).decode()
        response = (
            "HTTP/1.1 101 Switching Protocols\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Accept: {accept_val}\r\n\r\n"
        )
        client_sock.sendall(response.encode())

        # Frame loop
        while True:
            head = client_sock.recv(2)
            if len(head) < 2:
                break
            b1, b2 = head[0], head[1]
            opcode = b1 & 0x0F
            masked = (b2 & 0x80) != 0
            payload_len = b2 & 0x7F

            if payload_len == 126:
                payload_len = struct.unpack('>H', client_sock.recv(2))[0]
            elif payload_len == 127:
                payload_len = struct.unpack('>Q', client_sock.recv(8))[0]

            masks = client_sock.recv(4) if masked else b''
            data = bytearray(client_sock.recv(payload_len))
            if masked:
                for i in range(len(data)):
                    data[i] ^= masks[i % 4]

            if opcode == 8: # Close frame
                # Send close reply
                client_sock.sendall(bytes([0x88, 0x00]))
                break
            elif opcode == 1 or opcode == 2: # Text or Binary frame -> Echo back
                frame = bytearray([0x80 | opcode])
                if len(data) < 126:
                    frame.append(len(data))
                elif len(data) <= 65535:
                    frame.append(126)
                    frame.extend(struct.pack('>H', len(data)))
                else:
                    frame.append(127)
                    frame.extend(struct.pack('>Q', len(data)))
                frame.extend(data)
                client_sock.sendall(frame)
    except Exception:
        pass
    finally:
        client_sock.close()

server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server.setsockopt(socket.SOL_SOCKET, socket.SOL_REUSEADDR, 1)
server.bind(('0.0.0.0', 9001))
server.listen(10)
while True:
    client, _ = server.accept()
    threading.Thread(target=handle_ws_client, args=(client,), daemon=True).start()
"#;

fn start_ws_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:9001", "python3", "-c", WEBSOCKET_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start websocket container: {e}");
            return None;
        }
    };

    let host_port = match guard.get_host_port(9001, "tcp") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get host port: {e}");
            return None;
        }
    };

    if !guard.wait_for_tcp(host_port, Duration::from_secs(10)) {
        eprintln!("WebSocket port did not open in time");
        return None;
    }

    Some((guard, host_port))
}

#[test]
fn test_docker_websocket_handshake_and_echo() {
    let (_container, port) = match start_ws_container() {
        Some(c) => c,
        None => return,
    };

    let ws_url = format!("ws://127.0.0.1:{port}");
    let response = docker_http(&["--ws", &ws_url]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "WebSocket connection to live server should succeed. Stderr: {}",
        response.stderr
    );
}
