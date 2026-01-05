//! Benchmark runner implementation
//!
//! Handles concurrent HTTP request execution and result collection.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Semaphore};
use reqwest::{Client, Method};
use url::Url;

use crate::cli::Args;
use crate::cli::parser::ProcessedArgs;
use crate::errors::QuicpulseError;
use super::stats::{BenchmarkStats, StatsCollector};

/// Configuration for a benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub total_requests: u32,
    pub concurrency: u32,
    pub url: String,
    pub method: String,
}

impl BenchmarkConfig {
    pub fn from_args(args: &Args, processed: &ProcessedArgs) -> Self {
        Self {
            total_requests: args.bench_requests,
            concurrency: args.bench_concurrency,
            url: processed.url.clone(),
            method: processed.method.clone(),
        }
    }
}

/// Result of a single request
#[derive(Debug)]
struct RequestResult {
    status_code: Option<u16>,
    latency: Duration,
    bytes: usize,
    error: Option<String>,
}

/// Final benchmark results
#[derive(Debug)]
pub struct BenchmarkResult {
    pub url: String,
    pub method: String,
    pub total_requests: u32,
    pub concurrency: u32,
    pub duration: Duration,
    pub stats: BenchmarkStats,
}

/// Runs HTTP benchmarks with concurrent requests
pub struct BenchmarkRunner {
    pub config: BenchmarkConfig,
    pub client: Client,
    pub body: Option<Vec<u8>>,
    pub headers: reqwest::header::HeaderMap,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner
    pub fn new(
        config: BenchmarkConfig,
        args: &Args,
    ) -> Result<Self, QuicpulseError> {
        let client = build_client(args)?;

        Ok(Self {
            config,
            client,
            body: None,
            headers: reqwest::header::HeaderMap::new(),
        })
    }

    /// Set request body
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// Set request headers
    pub fn with_headers(mut self, headers: reqwest::header::HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    /// Run the benchmark
    pub async fn run(self) -> Result<BenchmarkResult, QuicpulseError> {
        let config = self.config.clone();
        let start = Instant::now();

        // Parse URL
        let url: Url = config.url.parse()
            .map_err(|e| QuicpulseError::Argument(format!("Invalid URL: {}", e)))?;

        // Parse method
        let method: Method = config.method.parse()
            .map_err(|e| QuicpulseError::Argument(format!("Invalid method: {}", e)))?;

        // Create channel for collecting results
        let (tx, mut rx) = mpsc::channel::<RequestResult>(config.concurrency as usize * 2);

        // Semaphore to limit concurrency
        let semaphore = Arc::new(Semaphore::new(config.concurrency as usize));

        // Spawn request tasks
        let client = Arc::new(self.client);
        let body = Arc::new(self.body);
        let headers = Arc::new(self.headers);

        let mut handles = Vec::new();

        for _ in 0..config.total_requests {
            let semaphore = semaphore.clone();
            let tx = tx.clone();
            let client = client.clone();
            let url = url.clone();
            let method = method.clone();
            let body = body.clone();
            let headers = headers.clone();

            let handle = tokio::spawn(async move {
                // Acquire permit inside the task to avoid blocking the spawn loop
                let _permit = semaphore.acquire().await.ok();
                let result = Self::execute_request(&client, url, method, &body, &headers).await;
                let _ = tx.send(result).await;
                // _permit drops here, releasing the semaphore
            });

            handles.push(handle);
        }

        // Drop the original sender so the receiver knows when all senders are done
        drop(tx);

        // Collect results
        let mut collector = StatsCollector::new();

        while let Some(result) = rx.recv().await {
            collector.record(result.status_code, result.latency, result.bytes, result.error);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let _ = handle.await;
        }

        let duration = start.elapsed();
        let stats = collector.finalize(duration);

        Ok(BenchmarkResult {
            url: config.url,
            method: config.method,
            total_requests: config.total_requests,
            concurrency: config.concurrency,
            duration,
            stats,
        })
    }

    /// Execute a single request
    async fn execute_request(
        client: &Client,
        url: Url,
        method: Method,
        body: &Option<Vec<u8>>,
        headers: &reqwest::header::HeaderMap,
    ) -> RequestResult {
        let start = Instant::now();

        let mut request = client.request(method, url);

        // Add headers
        for (key, value) in headers.iter() {
            request = request.header(key, value);
        }

        // Add body if present
        if let Some(ref body_bytes) = body {
            request = request.body(body_bytes.clone());
        }

        match request.send().await {
            Ok(response) => {
                let status_code = response.status().as_u16();
                let bytes = match response.bytes().await {
                    Ok(b) => b.len(),
                    Err(_) => 0,
                };
                let latency = start.elapsed();

                RequestResult {
                    status_code: Some(status_code),
                    latency,
                    bytes,
                    error: None,
                }
            }
            Err(e) => {
                let latency = start.elapsed();
                let error_msg = if e.is_timeout() {
                    "Timeout".to_string()
                } else if e.is_connect() {
                    "Connection failed".to_string()
                } else {
                    format!("{}", e)
                };

                RequestResult {
                    status_code: None,
                    latency,
                    bytes: 0,
                    error: Some(error_msg),
                }
            }
        }
    }
}

/// Build the HTTP client for benchmarking
fn build_client(args: &Args) -> Result<Client, QuicpulseError> {
    let mut builder = Client::builder()
        .user_agent(concat!("QuicPulse-Bench/", env!("CARGO_PKG_VERSION")))
        .pool_max_idle_per_host(100)
        .pool_idle_timeout(Duration::from_secs(30));

    // Set timeout if specified
    if let Some(timeout) = args.timeout {
        builder = builder.timeout(Duration::from_secs_f64(timeout));
    }

    // Configure redirects
    if args.follow {
        builder = builder.redirect(reqwest::redirect::Policy::limited(args.max_redirects as usize));
    } else {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }

    // SSL verification
    if args.verify == "no" {
        builder = builder.danger_accept_invalid_certs(true);
    }

    builder.build()
        .map_err(|e| QuicpulseError::Connection(format!("Failed to build client: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_config() {
        let config = BenchmarkConfig {
            total_requests: 100,
            concurrency: 10,
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
        };

        assert_eq!(config.total_requests, 100);
        assert_eq!(config.concurrency, 10);
    }
}
