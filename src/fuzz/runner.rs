//! Fuzz testing runner
//!
//! Executes fuzzing payloads against API endpoints and reports anomalies.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use reqwest::{Client, Method, header::HeaderMap};
use serde_json::Value as JsonValue;
use tokio::sync::Semaphore;
use std::sync::Arc;

use crate::errors::QuicpulseError;
use crate::output::terminal::{self, colors, RESET};
use super::payloads::{FuzzPayload, PayloadCategory, generate_payloads};

/// Maximum number of test cases to prevent memory explosion
const MAX_TEST_CASES: usize = 10_000;

/// Body format for fuzzing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FuzzBodyFormat {
    /// JSON body (application/json)
    #[default]
    Json,
    /// Form-urlencoded body (application/x-www-form-urlencoded)
    Form,
}

/// Options for fuzzing
#[derive(Debug, Clone)]
pub struct FuzzOptions {
    /// Maximum concurrent requests
    pub concurrency: usize,
    /// Timeout per request
    pub timeout: Duration,
    /// Categories to fuzz (None = all)
    pub categories: Option<Vec<PayloadCategory>>,
    /// Show verbose output
    pub verbose: bool,
    /// Only show anomalies (5xx errors, timeouts)
    pub anomalies_only: bool,
    /// Stop on first anomaly
    pub stop_on_anomaly: bool,
    /// Minimum risk level to test (1-5)
    pub min_risk_level: u8,
    /// Proxy URL (http://host:port or socks5://host:port)
    pub proxy: Option<String>,
    /// Skip SSL certificate verification
    pub insecure: bool,
    /// Custom CA certificate path
    pub ca_cert: Option<String>,
    /// Body format (JSON or Form)
    pub body_format: FuzzBodyFormat,
    /// Custom payloads from dictionary file or CLI
    pub custom_payloads: Vec<FuzzPayload>,
}

impl Default for FuzzOptions {
    fn default() -> Self {
        Self {
            concurrency: 10,
            timeout: Duration::from_secs(10),
            categories: None,
            verbose: false,
            anomalies_only: false,
            stop_on_anomaly: false,
            min_risk_level: 1,
            proxy: None,
            insecure: false,
            ca_cert: None,
            body_format: FuzzBodyFormat::default(),
            custom_payloads: Vec::new(),
        }
    }
}

/// Result of a single fuzz request
#[derive(Debug, Clone)]
pub struct FuzzResult {
    /// The payload that was sent
    pub payload: FuzzPayload,
    /// The field that was fuzzed
    pub field: String,
    /// HTTP status code (None if request failed)
    pub status_code: Option<u16>,
    /// Response time
    pub response_time: Duration,
    /// Error message if request failed
    pub error: Option<String>,
    /// Whether this is considered an anomaly
    pub is_anomaly: bool,
    /// Anomaly reason
    pub anomaly_reason: Option<String>,
}

impl FuzzResult {
    /// Check if this result indicates a potential vulnerability
    pub fn is_potential_vulnerability(&self) -> bool {
        self.is_anomaly && self.payload.risk_level >= 3
    }
}

/// Summary of fuzz testing
#[derive(Debug)]
pub struct FuzzSummary {
    /// Total requests sent
    pub total_requests: usize,
    /// Successful requests (2xx/3xx)
    pub successful: usize,
    /// Client errors (4xx)
    pub client_errors: usize,
    /// Server errors (5xx) - potential bugs
    pub server_errors: usize,
    /// Timeouts
    pub timeouts: usize,
    /// Connection errors
    pub connection_errors: usize,
    /// Total anomalies found
    pub anomalies: usize,
    /// Results grouped by category
    pub by_category: HashMap<PayloadCategory, CategorySummary>,
    /// Duration of the fuzz test
    pub duration: Duration,
}

/// Summary for a single category
#[derive(Debug, Default, Clone)]
pub struct CategorySummary {
    pub total: usize,
    pub anomalies: usize,
    pub server_errors: usize,
}

/// Fuzz testing runner
pub struct FuzzRunner {
    client: Client,
    options: FuzzOptions,
}

impl FuzzRunner {
    /// Create a new fuzz runner
    pub fn new(options: FuzzOptions) -> Result<Self, QuicpulseError> {
        let mut builder = Client::builder()
            .timeout(options.timeout);

        // Apply security settings (Bug #4 fix)
        if options.insecure {
            builder = builder.danger_accept_invalid_certs(true);
        }

        if let Some(ref proxy_url) = options.proxy {
            let proxy = reqwest::Proxy::all(proxy_url)
                .map_err(|e| QuicpulseError::Argument(format!("Invalid proxy URL: {}", e)))?;
            builder = builder.proxy(proxy);
        }

        if let Some(ref ca_path) = options.ca_cert {
            let cert_data = std::fs::read(ca_path)
                .map_err(|e| QuicpulseError::Io(e))?;
            let cert = reqwest::Certificate::from_pem(&cert_data)
                .map_err(|e| QuicpulseError::Argument(format!("Invalid CA certificate: {}", e)))?;
            builder = builder.add_root_certificate(cert);
        }

        let client = builder
            .build()
            .map_err(|e| QuicpulseError::Request(e))?;

        Ok(Self { client, options })
    }

    /// Run fuzzing against a URL with specific fields to fuzz
    pub async fn run(
        &self,
        method: Method,
        url: &str,
        base_body: Option<&JsonValue>,
        fields_to_fuzz: &[String],
        headers: HeaderMap,
    ) -> Result<(Vec<FuzzResult>, FuzzSummary), QuicpulseError> {
        let start = Instant::now();

        // Generate payloads (built-in + custom)
        let mut payloads = generate_payloads(self.options.categories.as_deref())
            .into_iter()
            .filter(|p| p.risk_level >= self.options.min_risk_level)
            .collect::<Vec<_>>();

        // Add custom payloads (always include, regardless of risk level filter)
        let custom_count = self.options.custom_payloads.len();
        payloads.extend(self.options.custom_payloads.clone());

        if self.options.verbose {
            if custom_count > 0 {
                eprintln!("Generated {} payloads ({} built-in, {} custom) across {} categories",
                    payloads.len(),
                    payloads.len() - custom_count,
                    custom_count,
                    PayloadCategory::all().len());
            } else {
                eprintln!("Generated {} payloads across {} categories",
                    payloads.len(),
                    PayloadCategory::all().len());
            }
        }

        // Generate all test cases (with memory limit - Bug #38 fix)
        let total_potential_cases = fields_to_fuzz.len() * payloads.len();
        if total_potential_cases > MAX_TEST_CASES {
            eprintln!("\x1b[33mWarning: Test case limit exceeded ({} > {})\x1b[0m",
                total_potential_cases, MAX_TEST_CASES);
            eprintln!("Limiting to {} test cases. Use --fuzz-risk to filter by risk level.", MAX_TEST_CASES);
        }

        let mut test_cases = Vec::with_capacity(total_potential_cases.min(MAX_TEST_CASES));
        'outer: for field in fields_to_fuzz {
            for payload in &payloads {
                if test_cases.len() >= MAX_TEST_CASES {
                    break 'outer;
                }
                test_cases.push((field.clone(), payload.clone()));
            }
        }

        if self.options.verbose {
            eprintln!("Running {} test cases with concurrency {}",
                test_cases.len(), self.options.concurrency);
        }

        // Run tests concurrently
        let semaphore = Arc::new(Semaphore::new(self.options.concurrency));
        let mut handles = Vec::new();

        for (field, payload) in test_cases {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let client = self.client.clone();
            let method = method.clone();
            let url = url.to_string();
            let headers = headers.clone();
            let base_body = base_body.cloned();
            let verbose = self.options.verbose;

            let body_format = self.options.body_format;
            let handle = tokio::spawn(async move {
                let result = Self::run_single_fuzz(
                    &client,
                    method,
                    &url,
                    base_body.as_ref(),
                    &field,
                    payload,
                    headers,
                    verbose,
                    body_format,
                ).await;
                drop(permit);
                result
            });

            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        let mut should_stop = false;

        for handle in handles {
            if should_stop && self.options.stop_on_anomaly {
                handle.abort();
                continue;
            }

            match handle.await {
                Ok(result) => {
                    if result.is_anomaly && self.options.stop_on_anomaly {
                        should_stop = true;
                    }
                    results.push(result);
                }
                Err(e) => {
                    eprintln!("Task error: {}", e);
                }
            }
        }

        // Build summary
        let summary = Self::build_summary(&results, start.elapsed());

        Ok((results, summary))
    }

    /// Run a single fuzz test
    async fn run_single_fuzz(
        client: &Client,
        method: Method,
        url: &str,
        base_body: Option<&JsonValue>,
        field: &str,
        payload: FuzzPayload,
        headers: HeaderMap,
        verbose: bool,
        body_format: FuzzBodyFormat,
    ) -> FuzzResult {
        // Build the request body with the fuzzed field
        let body = if let Some(base) = base_body {
            let mut body = base.clone();
            if let Some(obj) = body.as_object_mut() {
                obj.insert(field.to_string(), payload.value.clone());
            }
            Some(body)
        } else {
            // Create a simple object with just the fuzzed field
            Some(serde_json::json!({ field: payload.value.clone() }))
        };

        if verbose {
            eprintln!("  Testing {} = {} ({})", field,
                truncate_display(&payload.value.to_string(), 50),
                payload.description);
        }

        let start = Instant::now();

        let mut request = client.request(method, url).headers(headers);
        if let Some(ref body) = body {
            // Use the appropriate body format based on options
            // Bug #10 fix: Form data requests should use form encoding, not JSON
            match body_format {
                FuzzBodyFormat::Json => {
                    request = request.json(body);
                }
                FuzzBodyFormat::Form => {
                    // Convert JSON object to form data
                    if let Some(obj) = body.as_object() {
                        let form_data: HashMap<String, String> = obj.iter()
                            .map(|(k, v)| {
                                let value_str = match v {
                                    JsonValue::String(s) => s.clone(),
                                    _ => v.to_string(),
                                };
                                (k.clone(), value_str)
                            })
                            .collect();
                        request = request.form(&form_data);
                    } else {
                        // Fallback to JSON if body is not an object
                        request = request.json(body);
                    }
                }
            }
        }

        let response = request.send().await;
        let response_time = start.elapsed();

        match response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let (is_anomaly, reason) = Self::detect_anomaly(status, &response_time);

                FuzzResult {
                    payload,
                    field: field.to_string(),
                    status_code: Some(status),
                    response_time,
                    error: None,
                    is_anomaly,
                    anomaly_reason: reason,
                }
            }
            Err(e) => {
                let (is_anomaly, reason) = if e.is_timeout() {
                    (true, Some("Request timed out".to_string()))
                } else if e.is_connect() {
                    (true, Some("Connection failed".to_string()))
                } else {
                    (true, Some(format!("Request error: {}", e)))
                };

                FuzzResult {
                    payload,
                    field: field.to_string(),
                    status_code: None,
                    response_time,
                    error: Some(e.to_string()),
                    is_anomaly,
                    anomaly_reason: reason,
                }
            }
        }
    }

    /// Detect if a response is anomalous
    fn detect_anomaly(status: u16, response_time: &Duration) -> (bool, Option<String>) {
        // Server errors are always anomalies
        if status >= 500 {
            return (true, Some(format!("Server error: {}", status)));
        }

        // Slow responses might indicate issues
        if response_time.as_secs() > 5 {
            return (true, Some(format!("Slow response: {:?}", response_time)));
        }

        (false, None)
    }

    /// Build summary from results
    fn build_summary(results: &[FuzzResult], duration: Duration) -> FuzzSummary {
        let mut summary = FuzzSummary {
            total_requests: results.len(),
            successful: 0,
            client_errors: 0,
            server_errors: 0,
            timeouts: 0,
            connection_errors: 0,
            anomalies: 0,
            by_category: HashMap::new(),
            duration,
        };

        for result in results {
            // Update category summary
            let cat_summary = summary.by_category
                .entry(result.payload.category)
                .or_insert_with(CategorySummary::default);
            cat_summary.total += 1;

            if result.is_anomaly {
                summary.anomalies += 1;
                cat_summary.anomalies += 1;
            }

            match result.status_code {
                Some(status) if status >= 500 => {
                    summary.server_errors += 1;
                    cat_summary.server_errors += 1;
                }
                Some(status) if status >= 400 => {
                    summary.client_errors += 1;
                }
                Some(_) => {
                    summary.successful += 1;
                }
                None => {
                    if result.error.as_ref().map(|e| e.contains("timeout")).unwrap_or(false) {
                        summary.timeouts += 1;
                    } else {
                        summary.connection_errors += 1;
                    }
                }
            }
        }

        summary
    }
}

/// Format fuzz results for display
pub fn format_fuzz_results(results: &[FuzzResult], summary: &FuzzSummary, anomalies_only: bool) -> String {
    let mut output = String::new();

    let header_line = terminal::colorize("═══════════════════════════════════════════════════════════════════", colors::GREY);
    let section_line = terminal::colorize("───────────────────────────────────────────────────────────────────", colors::GREY);

    output.push_str("\n");
    output.push_str(&header_line);
    output.push_str("\n");
    output.push_str(&format!("{}                        FUZZ TEST RESULTS{}\n", terminal::bold_fg(colors::WHITE), RESET));
    output.push_str(&header_line);
    output.push_str("\n\n");

    // Show anomalies
    let anomalies: Vec<_> = results.iter().filter(|r| r.is_anomaly).collect();

    if anomalies.is_empty() {
        output.push_str(&format!("  {} {}\n\n",
            terminal::colorize("✓", colors::GREEN),
            terminal::success("No anomalies detected!")));
    } else {
        output.push_str(&format!("  {} {} {} anomalies:\n\n",
            terminal::colorize("⚠", colors::ORANGE),
            terminal::warning("Found"),
            terminal::number(&anomalies.len().to_string())));

        for result in &anomalies {
            let status_str = result.status_code
                .map(|s| s.to_string())
                .unwrap_or_else(|| "ERR".to_string());

            let (icon, status_color) = if result.payload.risk_level >= 4 {
                (terminal::colorize("●", colors::RED), colors::RED)
            } else if result.payload.risk_level >= 3 {
                (terminal::colorize("●", colors::ORANGE), colors::ORANGE)
            } else {
                (terminal::colorize("●", colors::YELLOW), colors::YELLOW)
            };

            output.push_str(&format!(
                "  {} {} {}: {} = {}\n",
                icon,
                terminal::colorize(&format!("[{}]", status_str), status_color),
                terminal::label(result.payload.category.as_str()),
                terminal::key(&result.field),
                terminal::muted(&truncate_display(&result.payload.value.to_string(), 40))
            ));
            output.push_str(&format!(
                "      {} {} (Risk: {})\n",
                terminal::muted("Payload:"),
                terminal::info(&result.payload.description),
                terminal::colorize(&format!("{}/5", result.payload.risk_level), status_color)
            ));
            if let Some(ref reason) = result.anomaly_reason {
                output.push_str(&format!("      {} {}\n",
                    terminal::muted("Reason:"),
                    terminal::colorize(reason, colors::ORANGE)));
            }
            output.push('\n');
        }
    }

    // Show non-anomalies if not anomalies_only
    if !anomalies_only {
        let normal: Vec<_> = results.iter().filter(|r| !r.is_anomaly).collect();
        if !normal.is_empty() {
            output.push_str(&format!("  {} {} payloads handled correctly\n\n",
                terminal::colorize("✓", colors::GREEN),
                terminal::number(&normal.len().to_string())));
        }
    }

    // Summary section
    output.push_str(&section_line);
    output.push_str("\n");
    output.push_str(&format!("{}                           SUMMARY{}\n", terminal::bold_fg(colors::WHITE), RESET));
    output.push_str(&section_line);
    output.push_str("\n\n");

    output.push_str(&format!("  {}    {}\n", terminal::label("Total Requests:"), terminal::number(&summary.total_requests.to_string())));
    output.push_str(&format!("  {}          {}\n", terminal::label("Duration:"), terminal::number(&format!("{:?}", summary.duration))));
    output.push_str(&format!("  {}      {}\n\n", terminal::label("Requests/sec:"),
        terminal::number(&format!("{:.1}", summary.total_requests as f64 / summary.duration.as_secs_f64()))));

    output.push_str(&format!("  {}:\n", terminal::label("Response Breakdown")));
    output.push_str(&format!("    {} {} {}\n",
        terminal::colorize("✓", colors::GREEN),
        terminal::muted("Successful (2xx/3xx):"),
        terminal::colorize(&summary.successful.to_string(), colors::GREEN)));
    output.push_str(&format!("    {} {} {}\n",
        terminal::colorize("⊘", colors::ORANGE),
        terminal::muted("Client Errors (4xx):"),
        terminal::colorize(&summary.client_errors.to_string(), colors::ORANGE)));
    let server_err_color = if summary.server_errors > 0 { colors::RED } else { colors::GREEN };
    output.push_str(&format!("    {} {} {}\n",
        terminal::colorize("✗", colors::RED),
        terminal::muted("Server Errors (5xx):"),
        terminal::colorize(&summary.server_errors.to_string(), server_err_color)));
    let timeout_color = if summary.timeouts > 0 { colors::ORANGE } else { colors::GREEN };
    output.push_str(&format!("    {} {} {}\n",
        terminal::colorize("⏱", colors::YELLOW),
        terminal::muted("Timeouts:"),
        terminal::colorize(&summary.timeouts.to_string(), timeout_color)));
    let conn_err_color = if summary.connection_errors > 0 { colors::RED } else { colors::GREEN };
    output.push_str(&format!("    {} {} {}\n\n",
        terminal::colorize("⚡", colors::AQUA),
        terminal::muted("Connection Errors:"),
        terminal::colorize(&summary.connection_errors.to_string(), conn_err_color)));

    // Category breakdown
    if !summary.by_category.is_empty() {
        output.push_str(&format!("  {}:\n", terminal::label("By Category")));
        for (category, cat_summary) in &summary.by_category {
            let anomaly_indicator = if cat_summary.server_errors > 0 {
                terminal::colorize(" ⚠", colors::ORANGE)
            } else {
                String::new()
            };
            output.push_str(&format!(
                "    {}: {} tested, {} anomalies{}\n",
                terminal::label(category.as_str()),
                terminal::number(&cat_summary.total.to_string()),
                terminal::number(&cat_summary.anomalies.to_string()),
                anomaly_indicator
            ));
        }
    }

    output.push_str("\n");
    output.push_str(&header_line);
    output.push_str("\n");

    output
}

/// Truncate a string for display
fn truncate_display(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    if max_len <= 3 {
        return "...".to_string();
    }
    let target_len = max_len - 3;
    let mut truncate_at = target_len.min(s.len());
    while truncate_at > 0 && !s.is_char_boundary(truncate_at) {
        truncate_at -= 1;
    }
    format!("{}...", &s[..truncate_at])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzz_options_default() {
        let opts = FuzzOptions::default();
        assert_eq!(opts.concurrency, 10);
        assert_eq!(opts.timeout, Duration::from_secs(10));
        assert_eq!(opts.min_risk_level, 1);
    }

    #[test]
    fn test_detect_anomaly() {
        let (is_anomaly, _) = FuzzRunner::detect_anomaly(500, &Duration::from_millis(100));
        assert!(is_anomaly);

        let (is_anomaly, _) = FuzzRunner::detect_anomaly(200, &Duration::from_millis(100));
        assert!(!is_anomaly);

        let (is_anomaly, _) = FuzzRunner::detect_anomaly(200, &Duration::from_secs(10));
        assert!(is_anomaly);
    }
}
