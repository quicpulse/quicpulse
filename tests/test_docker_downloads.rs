//! Integration tests for live file downloads and resuming (--download, --continue)
//!
//! Gated by Docker daemon availability and skipped in CI environments.

mod common;

use common::docker::{docker_http, is_docker_available, DockerContainerGuard};
use common::ExitStatus;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

const DOWNLOAD_SERVER_SCRIPT: &str = r#"
import http.server, socketserver

DATA = b"QUICPULSE_DOWNLOAD_TEST_DATA_" * 1000  # ~30 KB

class DownloadHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        range_header = self.headers.get('Range')
        if range_header and range_header.startswith('bytes='):
            parts = range_header[6:].split('-')
            start = int(parts[0]) if parts[0] else 0
            end = int(parts[1]) if parts[1] else len(DATA) - 1
            chunk = DATA[start:end+1]

            self.send_response(206)
            self.send_header('Content-Type', 'application/octet-stream')
            self.send_header('Content-Range', f'bytes {start}-{end}/{len(DATA)}')
            self.send_header('Content-Length', str(len(chunk)))
            self.end_headers()
            self.wfile.write(chunk)
        else:
            self.send_response(200)
            self.send_header('Content-Type', 'application/octet-stream')
            self.send_header('Content-Disposition', 'attachment; filename="sample.bin"')
            self.send_header('Content-Length', str(len(DATA)))
            self.end_headers()
            self.wfile.write(DATA)

server = socketserver.TCPServer(('0.0.0.0', 8080), DownloadHandler)
server.serve_forever()
"#;

fn start_download_container() -> Option<(DockerContainerGuard, u16)> {
    if !is_docker_available() {
        return None;
    }

    let guard = match DockerContainerGuard::start(
        "python:3.11-alpine",
        &["-p", "0:8080", "python3", "-c", DOWNLOAD_SERVER_SCRIPT],
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to start download container: {e}");
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

    let health_url = format!("http://127.0.0.1:{host_port}/sample.bin");
    if !guard.wait_for_http(&health_url, Duration::from_secs(10)) {
        eprintln!("Download server did not start in time");
        return None;
    }

    Some((guard, host_port))
}

#[test]
fn test_docker_download_file_to_disk() {
    let (_container, port) = match start_download_container() {
        Some(c) => c,
        None => return,
    };

    let temp_dir = TempDir::new().unwrap();
    let out_file = temp_dir.path().join("downloaded.bin");
    let out_str = out_file.to_string_lossy().to_string();

    let url = format!("http://127.0.0.1:{port}/sample.bin");
    let response = docker_http(&["--download", "--output", &out_str, &url]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Download should succeed. Stderr: {}",
        response.stderr
    );

    assert!(out_file.exists(), "Downloaded file should exist on disk");
    let content = fs::read(&out_file).unwrap();
    assert_eq!(content.len(), 30000, "File size should match full payload");
}

#[test]
fn test_docker_download_resume() {
    let (_container, port) = match start_download_container() {
        Some(c) => c,
        None => return,
    };

    let temp_dir = TempDir::new().unwrap();
    let out_file = temp_dir.path().join("resumed.bin");

    // Write partial 10KB first
    let partial = vec![b'X'; 10000];
    fs::write(&out_file, &partial).unwrap();
    let out_str = out_file.to_string_lossy().to_string();

    let url = format!("http://127.0.0.1:{port}/sample.bin");
    let response = docker_http(&["--download", "--continue", "--output", &out_str, &url]);

    assert_eq!(
        response.exit_status,
        ExitStatus::Success,
        "Resumed download should succeed. Stderr: {}",
        response.stderr
    );
    assert!(out_file.exists());
}
