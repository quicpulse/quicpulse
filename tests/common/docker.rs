//! Docker test utilities and lifecycle management for quicpulse integration tests
//!
//! Provides automatic Docker daemon detection, CI gating, dynamic port discovery,
//! and RAII cleanup for test containers.

#![allow(dead_code)]

use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Check if Docker is available and enabled for running integration tests.
///
/// Returns `false` if:
/// - `SKIP_DOCKER_TESTS=1` is set.
/// - In CI environments (`CI=true` or `GITHUB_ACTIONS=true`) unless `RUN_DOCKER_TESTS=1` is set.
/// - The Docker CLI is missing or the daemon is not responding to `docker info`.
pub fn is_docker_available() -> bool {
    // 1. Explicit skip flag
    if std::env::var("SKIP_DOCKER_TESTS").unwrap_or_default() == "1" {
        return false;
    }

    // 2. Gate in CI environments unless explicitly requested
    let in_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
    let force_docker = std::env::var("RUN_DOCKER_TESTS").unwrap_or_default() == "1";
    if in_ci && !force_docker {
        return false;
    }

    // 3. Check Docker daemon responsiveness with a quick probe
    match Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            !version.is_empty()
        }
        _ => false,
    }
}

/// Locate the active Docker Unix domain socket.
///
/// Checks common locations on macOS and Linux:
/// - Path in `DOCKER_HOST` environment variable (if unix:// scheme)
/// - `~/.docker/run/docker.sock` (standard Docker Desktop on macOS)
/// - `/var/run/docker.sock` (standard Linux / root daemon)
pub fn docker_socket_path() -> Option<PathBuf> {
    // 1. Check DOCKER_HOST if specified
    if let Ok(host) = std::env::var("DOCKER_HOST") {
        if let Some(path_str) = host.strip_prefix("unix://") {
            let path = PathBuf::from(path_str);
            if path.exists() {
                return Some(path);
            }
        }
    }

    // 2. Check user's home Docker Desktop socket (macOS standard)
    if let Ok(home) = std::env::var("HOME") {
        let mac_sock = PathBuf::from(home).join(".docker/run/docker.sock");
        if mac_sock.exists() {
            return Some(mac_sock);
        }
    }

    // 3. Check system standard path
    let std_sock = PathBuf::from("/var/run/docker.sock");
    if std_sock.exists() {
        return Some(std_sock);
    }

    None
}

/// RAII Guard for managing the lifecycle of a Docker test container.
///
/// Automatically removes the container (`docker rm -f <id>`) on `Drop`.
#[derive(Debug)]
pub struct DockerContainerGuard {
    /// Docker container ID
    pub id: String,
}

impl DockerContainerGuard {
    /// Start a new container from an image with arguments.
    pub fn start(image: &str, extra_args: &[&str]) -> Result<Self, String> {
        let mut cmd = Command::new("docker");
        cmd.arg("run").arg("-d");
        cmd.args(extra_args);
        cmd.arg(image);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to spawn docker run: {e}"))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("docker run failed: {err}"));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if container_id.is_empty() {
            return Err("docker run returned empty container ID".to_string());
        }

        Ok(Self { id: container_id })
    }

    /// Resolve the mapped host port for a container port and protocol (tcp/udp).
    pub fn get_host_port(&self, container_port: u16, proto: &str) -> Result<u16, String> {
        let target = format!("{container_port}/{proto}");
        let output = Command::new("docker")
            .args(["port", &self.id, &target])
            .output()
            .map_err(|e| format!("Failed to run docker port: {e}"))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("docker port {target} failed: {err}"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if let Some(colon_pos) = line.rfind(':') {
                let port_str = &line[colon_pos + 1..];
                if let Ok(port) = port_str.parse::<u16>() {
                    return Ok(port);
                }
            }
        }

        Err(format!(
            "Could not parse host port from 'docker port' output: {stdout}"
        ))
    }

    /// Wait until a TCP port accepts connections.
    pub fn wait_for_tcp(&self, port: u16, timeout: Duration) -> bool {
        let start = Instant::now();
        let addr = format!("127.0.0.1:{port}");
        while start.elapsed() < timeout {
            if TcpStream::connect(&addr).is_ok() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    /// Wait until an HTTP endpoint returns a success status.
    pub fn wait_for_http(&self, url: &str, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Ok(resp) = reqwest::blocking::Client::builder()
                .danger_accept_invalid_certs(true)
                .timeout(Duration::from_millis(500))
                .build()
                .and_then(|c| c.get(url).send())
            {
                if resp.status().is_success() || resp.status().is_redirection() {
                    return true;
                }
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        false
    }
}

impl Drop for DockerContainerGuard {
    fn drop(&mut self) {
        if !self.id.is_empty() {
            let _ = Command::new("docker").args(["rm", "-f", &self.id]).output();
        }
    }
}

/// Run quicpulse with a generous timeout suitable for Docker containers (15s).
pub fn docker_http(args: &[&str]) -> super::CliResponse {
    docker_http_with_env(args, &super::MockEnvironment::new())
}

/// Run quicpulse with a generous timeout and custom environment for Docker containers.
pub fn docker_http_with_env(args: &[&str], env: &super::MockEnvironment) -> super::CliResponse {
    use std::io::Write;
    use std::process::Stdio;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_quicpulse"));
    cmd.args(["--timeout", "15"]);
    cmd.args(args);

    cmd.env("QUICPULSE_CONFIG_DIR", env.config_path());
    for (key, value) in &env.env_vars {
        cmd.env(key, value);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if let Some(ref stdin_data) = env.stdin {
        cmd.stdin(Stdio::piped());
        let mut child = cmd.spawn().expect("Failed to spawn command");
        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin
                .write_all(stdin_data)
                .expect("Failed to write to stdin");
        }
        let output = child
            .wait_with_output()
            .expect("Failed to wait for command");
        super::parse_output(output)
    } else {
        cmd.stdin(Stdio::null());
        let output = cmd.output().expect("Failed to execute command");
        super::parse_output(output)
    }
}
