//! HAR replay runner
//!
//! Replays HTTP requests from HAR entries.

use std::time::Duration;
use reqwest::{Client, Method, Response};
use crate::errors::QuicpulseError;
use super::types::{Har, HarEntry, HarRequest};

fn truncate_url(s: &str, max_len: usize) -> String {
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

/// Options for HAR replay
#[derive(Debug, Clone)]
pub struct HarReplayOptions {
    /// Delay between requests
    pub delay: Option<Duration>,

    /// Timeout for each request
    pub timeout: Option<Duration>,

    /// Follow redirects
    pub follow_redirects: bool,

    /// Verbose output
    pub verbose: bool,

    /// Dry run (don't actually send requests)
    pub dry_run: bool,
}

impl Default for HarReplayOptions {
    fn default() -> Self {
        Self {
            delay: None,
            timeout: Some(Duration::from_secs(30)),
            follow_redirects: true,
            verbose: false,
            dry_run: false,
        }
    }
}

/// Result of replaying a single HAR entry
#[derive(Debug)]
pub struct HarReplayResult {
    /// Entry index (1-based)
    pub index: usize,

    /// Request method
    pub method: String,

    /// Request URL
    pub url: String,

    /// Response status code (None if failed)
    pub status: Option<u16>,

    /// Response time in milliseconds
    pub time_ms: u128,

    /// Error message if failed
    pub error: Option<String>,

    /// Whether status matches original HAR
    pub status_match: bool,
}

/// HAR replay runner
pub struct HarRunner {
    client: Client,
    options: HarReplayOptions,
}

impl HarRunner {
    /// Create a new HAR runner
    pub fn new(options: HarReplayOptions) -> Result<Self, QuicpulseError> {
        let mut builder = Client::builder();

        if let Some(timeout) = options.timeout {
            builder = builder.timeout(timeout);
        }

        if !options.follow_redirects {
            builder = builder.redirect(reqwest::redirect::Policy::none());
        }

        let client = builder.build().map_err(|e| {
            QuicpulseError::Request(e)
        })?;

        Ok(Self { client, options })
    }

    /// Replay all entries in a HAR file
    pub async fn replay_all(&self, har: &Har) -> Vec<HarReplayResult> {
        let mut results = Vec::new();

        for (idx, entry) in har.log.entries.iter().enumerate() {
            let result = self.replay_entry(idx + 1, entry).await;
            results.push(result);

            // Add delay between requests
            if let Some(delay) = self.options.delay {
                if idx < har.log.entries.len() - 1 {
                    tokio::time::sleep(delay).await;
                }
            }
        }

        results
    }

    /// Replay a single HAR entry
    pub async fn replay_entry(&self, index: usize, entry: &HarEntry) -> HarReplayResult {
        let start = std::time::Instant::now();
        let original_status = entry.response.status;

        if self.options.dry_run {
            return HarReplayResult {
                index,
                method: entry.request.method.clone(),
                url: entry.request.url.clone(),
                status: None,
                time_ms: 0,
                error: Some("Dry run - request not sent".to_string()),
                status_match: false,
            };
        }

        match self.send_request(&entry.request).await {
            Ok(response) => {
                let status = response.status().as_u16();
                let time_ms = start.elapsed().as_millis();

                HarReplayResult {
                    index,
                    method: entry.request.method.clone(),
                    url: entry.request.url.clone(),
                    status: Some(status),
                    time_ms,
                    error: None,
                    status_match: status as i32 == original_status,
                }
            }
            Err(e) => {
                HarReplayResult {
                    index,
                    method: entry.request.method.clone(),
                    url: entry.request.url.clone(),
                    status: None,
                    time_ms: start.elapsed().as_millis(),
                    error: Some(e.to_string()),
                    status_match: false,
                }
            }
        }
    }

    /// Send an HTTP request based on HAR request data
    async fn send_request(&self, har_request: &HarRequest) -> Result<Response, QuicpulseError> {
        // Parse method
        let method = har_request.method.parse::<Method>().map_err(|_| {
            QuicpulseError::Parse(format!("Invalid HTTP method: {}", har_request.method))
        })?;

        // Build request
        let mut request = self.client.request(method, &har_request.url);

        // Add headers (skip certain browser-specific headers)
        for header in &har_request.headers {
            // Skip headers that should not be replayed
            let name_lower = header.name.to_lowercase();
            if SKIP_HEADERS.contains(&name_lower.as_str()) {
                continue;
            }

            request = request.header(&header.name, &header.value);
        }

        // Add cookies
        if !har_request.cookies.is_empty() {
            let cookie_str: String = har_request.cookies.iter()
                .map(|c| format!("{}={}", c.name, c.value))
                .collect::<Vec<_>>()
                .join("; ");
            request = request.header("Cookie", cookie_str);
        }

        // Add body
        if let Some(ref post_data) = har_request.post_data {
            if let Some(ref text) = post_data.text {
                request = request.body(text.clone());

                // Set content-type if not already set
                if har_request.get_header("content-type").is_none() {
                    request = request.header("Content-Type", &post_data.mime_type);
                }
            } else if let Some(ref params) = post_data.params {
                // Check mime_type to determine form encoding
                let is_multipart = post_data.mime_type.to_lowercase().contains("multipart/form-data");

                if is_multipart {
                    // Build multipart form to preserve original encoding
                    let mut form = reqwest::multipart::Form::new();
                    for param in params {
                        if let Some(ref value) = param.value {
                            // Check if it's a file parameter
                            if param.file_name.is_some() || param.content_type.is_some() {
                                let mut part = reqwest::multipart::Part::text(value.clone());
                                if let Some(ref filename) = param.file_name {
                                    part = part.file_name(filename.clone());
                                }
                                let final_part = if let Some(ref ct) = param.content_type {
                                    // mime_str consumes self, so use match to handle both cases
                                    match part.mime_str(ct) {
                                        Ok(p) => p,
                                        Err(_) => {
                                            // If mime_str fails, recreate the part without mime type
                                            let mut new_part = reqwest::multipart::Part::text(value.clone());
                                            if let Some(ref filename) = param.file_name {
                                                new_part = new_part.file_name(filename.clone());
                                            }
                                            new_part
                                        }
                                    }
                                } else {
                                    part
                                };
                                form = form.part(param.name.clone(), final_part);
                            } else {
                                form = form.text(param.name.clone(), value.clone());
                            }
                        }
                    }
                    request = request.multipart(form);
                } else {
                    // URL-encoded form data
                    let form_data: Vec<(String, String)> = params.iter()
                        .filter_map(|p| {
                            p.value.as_ref().map(|v| (p.name.clone(), v.clone()))
                        })
                        .collect();
                    request = request.form(&form_data);
                }
            }
        }

        // Send request
        request.send().await.map_err(QuicpulseError::Request)
    }
}

/// Headers that should not be replayed from HAR
const SKIP_HEADERS: &[&str] = &[
    "host",
    "connection",
    "content-length",
    "accept-encoding",
    "transfer-encoding",
    "cookie",  // Skip since we reconstruct from har_request.cookies to avoid duplication
    ":method",
    ":path",
    ":scheme",
    ":authority",
];

/// Format HAR replay results for display
pub fn format_replay_results(results: &[HarReplayResult]) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    // Header
    writeln!(output, "\n{}", "=".repeat(80)).unwrap();
    writeln!(output, "HAR REPLAY RESULTS").unwrap();
    writeln!(output, "{}\n", "=".repeat(80)).unwrap();

    // Results table
    writeln!(output, "{:<5} {:<7} {:<40} {:<8} {:<10}",
        "#", "Method", "URL", "Status", "Time").unwrap();
    writeln!(output, "{}", "-".repeat(80)).unwrap();

    let mut success_count = 0;
    let mut failure_count = 0;
    let mut match_count = 0;
    let mut total_time: u128 = 0;

    for result in results {
        let url_display = truncate_url(&result.url, 38);

        let status_display = match result.status {
            Some(s) => {
                if result.status_match {
                    match_count += 1;
                }
                success_count += 1;
                format!("{}", s)
            }
            None => {
                failure_count += 1;
                "ERR".to_string()
            }
        };

        let time_display = format!("{}ms", result.time_ms);
        total_time += result.time_ms;

        let match_indicator = if result.status_match { "✓" } else { " " };

        writeln!(output, "{:<5} {:<7} {:<40} {:<8} {:<10} {}",
            result.index,
            result.method,
            url_display,
            status_display,
            time_display,
            match_indicator
        ).unwrap();

        if let Some(ref error) = result.error {
            writeln!(output, "      Error: {}", error).unwrap();
        }
    }

    // Summary
    writeln!(output, "\n{}", "-".repeat(80)).unwrap();
    writeln!(output, "SUMMARY").unwrap();
    writeln!(output, "  Total requests:  {}", results.len()).unwrap();
    writeln!(output, "  Successful:      {}", success_count).unwrap();
    writeln!(output, "  Failed:          {}", failure_count).unwrap();
    writeln!(output, "  Status matches:  {} (same as original HAR)", match_count).unwrap();
    writeln!(output, "  Total time:      {}ms", total_time).unwrap();
    if !results.is_empty() {
        writeln!(output, "  Avg time:        {}ms", total_time / results.len() as u128).unwrap();
    }

    output
}

/// Format HAR entries list for display
pub fn format_har_list(har: &Har) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    writeln!(output, "\n{}", "=".repeat(80)).unwrap();
    writeln!(output, "HAR FILE ENTRIES").unwrap();
    writeln!(output, "{}\n", "=".repeat(80)).unwrap();

    writeln!(output, "{:<5} {:<7} {:<50} {:<8} {:<10}",
        "#", "Method", "URL", "Status", "Time").unwrap();
    writeln!(output, "{}", "-".repeat(80)).unwrap();

    for (idx, entry) in har.log.entries.iter().enumerate() {
        let url_display = truncate_url(&entry.request.url, 48);

        writeln!(output, "{:<5} {:<7} {:<50} {:<8} {:.0}ms",
            idx + 1,
            entry.request.method,
            url_display,
            entry.response.status,
            entry.time
        ).unwrap();
    }

    // Summary
    let summary = super::parser::HarSummary::from_har(har);

    writeln!(output, "\n{}", "-".repeat(80)).unwrap();
    writeln!(output, "SUMMARY").unwrap();
    writeln!(output, "  Total entries: {}", summary.total_entries).unwrap();
    writeln!(output, "  Total time:    {:.0}ms", summary.total_time_ms).unwrap();

    if !summary.domains.is_empty() {
        writeln!(output, "  Domains:").unwrap();
        let mut domains: Vec<_> = summary.domains.iter().collect();
        domains.sort_by(|a, b| b.1.cmp(a.1));
        for (domain, count) in domains.iter().take(5) {
            writeln!(output, "    - {} ({})", domain, count).unwrap();
        }
        if domains.len() > 5 {
            writeln!(output, "    - ... and {} more", domains.len() - 5).unwrap();
        }
    }

    output
}

/// Interactive request selection (for --har-interactive)
pub fn select_requests_interactive(har: &Har) -> Result<Vec<usize>, QuicpulseError> {
    use std::io::{self, Write};

    println!("\n{}", format_har_list(har));
    println!("\nEnter request numbers to replay (comma-separated, or 'all' for all, 'q' to quit):");
    print!("> ");
    io::stdout().flush().map_err(|e| QuicpulseError::Io(e))?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| QuicpulseError::Io(e))?;

    let input = input.trim().to_lowercase();

    if input == "q" || input == "quit" {
        return Ok(vec![]);
    }

    if input == "all" || input == "*" {
        return Ok((1..=har.log.entries.len()).collect());
    }

    // Parse comma-separated numbers
    let indices: Vec<usize> = input
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .filter(|&n| n > 0 && n <= har.log.entries.len())
        .collect();

    if indices.is_empty() {
        return Err(QuicpulseError::Parse("No valid request numbers entered".to_string()));
    }

    Ok(indices)
}

/// Parse delay string to Duration (e.g., "100ms", "1s", "500")
pub fn parse_delay(s: &str) -> Result<Duration, QuicpulseError> {
    let s = s.trim().to_lowercase();

    if let Some(ms_str) = s.strip_suffix("ms") {
        let ms: u64 = ms_str.trim().parse().map_err(|_| {
            QuicpulseError::Parse(format!("Invalid delay: {}", s))
        })?;
        return Ok(Duration::from_millis(ms));
    }

    if let Some(s_str) = s.strip_suffix('s') {
        let secs: f64 = s_str.trim().parse().map_err(|_| {
            QuicpulseError::Parse(format!("Invalid delay: {}", s))
        })?;
        return Ok(Duration::from_secs_f64(secs));
    }

    // Default: treat as milliseconds
    let ms: u64 = s.parse().map_err(|_| {
        QuicpulseError::Parse(format!("Invalid delay: {}", s))
    })?;
    Ok(Duration::from_millis(ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_delay() {
        assert_eq!(parse_delay("100ms").unwrap(), Duration::from_millis(100));
        assert_eq!(parse_delay("1s").unwrap(), Duration::from_secs(1));
        assert_eq!(parse_delay("500").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_delay("1.5s").unwrap(), Duration::from_secs_f64(1.5));
    }

    #[test]
    fn test_skip_headers() {
        assert!(SKIP_HEADERS.contains(&"host"));
        assert!(SKIP_HEADERS.contains(&"content-length"));
        assert!(!SKIP_HEADERS.contains(&"authorization"));
    }
}
