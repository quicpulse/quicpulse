//! Benchmarking module for load testing

pub mod runner;
pub mod stats;

pub use runner::{BenchmarkRunner, BenchmarkConfig, BenchmarkResult};

use crate::cli::Args;
use crate::cli::parser::ProcessedArgs;
use crate::context::Environment;
use crate::errors::QuicpulseError;
use crate::status::ExitStatus;
use crate::output::terminal::{self, colors, RESET};

/// Check if benchmark mode is enabled
pub fn is_benchmark_mode(args: &Args) -> bool {
    args.bench
}

/// Format benchmark results for display
pub fn format_results(result: &BenchmarkResult) -> String {
    use terminal::protocol::http_method;

    let mut output = String::new();

    let header_line = terminal::colorize("═══════════════════════════════════════════════════════════════════", colors::GREY);
    let section_line = terminal::colorize("───────────────────────────────────────────────────────────────────", colors::GREY);

    output.push_str("\n");
    output.push_str(&header_line);
    output.push_str("\n");
    output.push_str(&format!("{}                        BENCHMARK RESULTS{}\n", terminal::bold_fg(colors::WHITE), RESET));
    output.push_str(&header_line);
    output.push_str("\n\n");

    // Summary
    output.push_str(&format!("  {}              {}\n", terminal::label("URL:"), terminal::colorize(&result.url, colors::AQUA)));
    output.push_str(&format!("  {}           {}{}{}\n", terminal::label("Method:"), http_method(&result.method), result.method, RESET));
    output.push_str(&format!("  {}         {}\n", terminal::label("Requests:"), terminal::number(&result.total_requests.to_string())));
    output.push_str(&format!("  {}      {}\n", terminal::label("Concurrency:"), terminal::number(&result.concurrency.to_string())));
    output.push_str(&format!("  {}         {}\n", terminal::label("Duration:"), terminal::number(&format!("{:.2}s", result.duration.as_secs_f64()))));
    output.push_str("\n");

    // Throughput
    output.push_str(&section_line);
    output.push_str("\n");
    output.push_str(&format!("  {}\n", terminal::bold("THROUGHPUT", colors::WHITE)));
    output.push_str(&section_line);
    output.push_str("\n");
    output.push_str(&format!("  {}     {}\n", terminal::label("Requests/sec:"), terminal::number(&format!("{:.2}", result.stats.requests_per_second))));
    output.push_str(&format!("  {}        {}\n", terminal::label("Bytes/sec:"), terminal::number(&crate::utils::format_bytes(result.stats.bytes_per_second as u64, 2))));
    output.push_str(&format!("  {}      {}\n", terminal::label("Total bytes:"), terminal::number(&crate::utils::format_bytes(result.stats.total_bytes, 2))));
    output.push_str("\n");

    // Latency
    output.push_str(&section_line);
    output.push_str("\n");
    output.push_str(&format!("  {}\n", terminal::bold("LATENCY", colors::WHITE)));
    output.push_str(&section_line);
    output.push_str("\n");
    output.push_str(&format!("  {}              {}\n", terminal::label("Min:"), terminal::colorize(&format!("{:.2}ms", result.stats.latency.min_ms), colors::GREEN)));
    output.push_str(&format!("  {}             {}\n", terminal::label("Mean:"), terminal::number(&format!("{:.2}ms", result.stats.latency.mean_ms))));
    output.push_str(&format!("  {}              {}\n", terminal::label("Max:"), terminal::colorize(&format!("{:.2}ms", result.stats.latency.max_ms), colors::ORANGE)));
    output.push_str(&format!("  {}          {}\n", terminal::label("Std Dev:"), terminal::muted(&format!("{:.2}ms", result.stats.latency.stddev_ms))));
    output.push_str("\n");
    output.push_str(&format!("  {}:\n", terminal::label("Percentiles")));
    output.push_str(&format!("    {}            {}\n", terminal::muted("p50:"), terminal::number(&format!("{:.2}ms", result.stats.latency.p50_ms))));
    output.push_str(&format!("    {}            {}\n", terminal::muted("p75:"), terminal::number(&format!("{:.2}ms", result.stats.latency.p75_ms))));
    output.push_str(&format!("    {}            {}\n", terminal::muted("p90:"), terminal::number(&format!("{:.2}ms", result.stats.latency.p90_ms))));
    output.push_str(&format!("    {}            {}\n", terminal::muted("p95:"), terminal::number(&format!("{:.2}ms", result.stats.latency.p95_ms))));
    output.push_str(&format!("    {}            {}\n", terminal::muted("p99:"), terminal::number(&format!("{:.2}ms", result.stats.latency.p99_ms))));
    output.push_str("\n");

    // Status codes
    output.push_str(&section_line);
    output.push_str("\n");
    output.push_str(&format!("  {}\n", terminal::bold("STATUS CODES", colors::WHITE)));
    output.push_str(&section_line);
    output.push_str("\n");
    let success_color = if result.stats.success_rate >= 0.95 { colors::GREEN } else if result.stats.success_rate >= 0.5 { colors::YELLOW } else { colors::RED };
    output.push_str(&format!("  {}       {} ({})\n",
        terminal::label("Successful:"),
        terminal::colorize(&result.stats.successful_requests.to_string(), success_color),
        terminal::colorize(&format!("{:.1}%", result.stats.success_rate * 100.0), success_color)
    ));
    let fail_color = if result.stats.failed_requests == 0 { colors::GREEN } else { colors::RED };
    output.push_str(&format!("  {}           {}\n", terminal::label("Failed:"), terminal::colorize(&result.stats.failed_requests.to_string(), fail_color)));

    if !result.stats.status_codes.is_empty() {
        output.push_str(&format!("\n  {}:\n", terminal::label("Status breakdown")));
        let mut codes: Vec<_> = result.stats.status_codes.iter().collect();
        codes.sort_by_key(|(code, _)| *code);
        for (code, count) in codes {
            let pct = (*count as f64 / result.total_requests as f64) * 100.0;
            let status_color = match code / 100 {
                2 => colors::GREEN,
                3 => colors::YELLOW,
                4 => colors::ORANGE,
                5 => colors::RED,
                _ => colors::GREY,
            };
            output.push_str(&format!("    {}:              {} ({})\n",
                terminal::colorize(&code.to_string(), status_color),
                terminal::number(&count.to_string()),
                terminal::muted(&format!("{:.1}%", pct))
            ));
        }
    }

    if !result.stats.errors.is_empty() {
        output.push_str(&format!("\n  {}:\n", terminal::error("Error breakdown")));
        for (error, count) in &result.stats.errors {
            output.push_str(&format!("    {}: {}\n", terminal::colorize(error, colors::RED), terminal::number(&count.to_string())));
        }
    }

    output.push_str("\n");
    output.push_str(&header_line);
    output.push_str("\n");

    output
}

// Local humanize_bytes removed in favor of crate::utils::format_bytes

pub async fn run_benchmark(
    args: Args,
    processed: ProcessedArgs,
    _env: Environment,
) -> Result<ExitStatus, QuicpulseError> {
    use crate::input::InputItem;

    let config = BenchmarkConfig::from_args(&args, &processed);

    use terminal::protocol::http_method;
    eprintln!(
        "{} {}{}{} {} (requests: {}, concurrency: {})",
        terminal::info("Benchmarking"),
        http_method(&config.method), config.method, RESET,
        terminal::colorize(&config.url, colors::AQUA),
        terminal::number(&config.total_requests.to_string()),
        terminal::number(&config.concurrency.to_string())
    );
    eprintln!("{}\n", terminal::muted("Running..."));

    let runner = BenchmarkRunner::new(config, &args)?;

    let runner = if processed.has_data {
        let mut body = serde_json::json!({});
        for item in &processed.items {
            match item {
                InputItem::DataField { key, value } => {
                    if let Some(map) = body.as_object_mut() {
                        map.insert(key.clone(), serde_json::Value::String(value.clone()));
                    }
                }
                InputItem::JsonField { key, value } => {
                    if let Some(map) = body.as_object_mut() {
                        map.insert(key.clone(), value.clone());
                    }
                }
                _ => {}
            }
        }
        runner.with_body(body.to_string().into_bytes())
    } else {
        runner
    };

    let mut headers = reqwest::header::HeaderMap::new();
    for item in &processed.items {
        match item {
            InputItem::Header { name, value } => {
                if let Ok(header_name) = reqwest::header::HeaderName::try_from(name.as_str()) {
                    if let Ok(header_value) = reqwest::header::HeaderValue::from_str(value) {
                        headers.insert(header_name, header_value);
                    }
                }
            }
            InputItem::EmptyHeader { name } => {
                if let Ok(header_name) = reqwest::header::HeaderName::try_from(name.as_str()) {
                    if let Ok(header_value) = reqwest::header::HeaderValue::from_str("") {
                        headers.insert(header_name, header_value);
                    }
                }
            }
            InputItem::HeaderFile { name, path } => {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(header_name) = reqwest::header::HeaderName::try_from(name.as_str()) {
                        if let Ok(header_value) = reqwest::header::HeaderValue::from_str(content.trim()) {
                            headers.insert(header_name, header_value);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let runner = if !headers.is_empty() {
        runner.with_headers(headers)
    } else {
        runner
    };

    let result = runner.run().await?;

    print!("{}", format_results(&result));

    if result.stats.success_rate >= 0.5 {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::stats::{BenchmarkStats, LatencyStats};
    use std::time::Duration;
    use std::collections::HashMap;

    #[test]
    fn test_format_bytes() {
        use crate::utils::format_bytes;
        // humansize crate outputs consistent format with decimal places
        assert_eq!(format_bytes(500, 2), "500.00 B");
        assert_eq!(format_bytes(1536, 2), "1.50 KiB");
        assert_eq!(format_bytes(1572864, 2), "1.50 MiB");
    }

    #[test]
    fn test_format_results() {
        let result = BenchmarkResult {
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            total_requests: 100,
            concurrency: 10,
            duration: Duration::from_secs(5),
            stats: BenchmarkStats {
                successful_requests: 95,
                failed_requests: 5,
                success_rate: 0.95,
                requests_per_second: 20.0,
                bytes_per_second: 10240.0,
                total_bytes: 51200,
                status_codes: HashMap::from([(200, 95), (500, 5)]),
                errors: HashMap::new(),
                latency: LatencyStats {
                    min_ms: 10.0,
                    max_ms: 500.0,
                    mean_ms: 100.0,
                    stddev_ms: 50.0,
                    p50_ms: 90.0,
                    p75_ms: 120.0,
                    p90_ms: 200.0,
                    p95_ms: 300.0,
                    p99_ms: 450.0,
                },
            },
        };

        let output = format_results(&result);
        // Check that key sections are present (ignoring ANSI color codes)
        assert!(output.contains("BENCHMARK RESULTS"));
        assert!(output.contains("THROUGHPUT"));
        assert!(output.contains("LATENCY"));
        assert!(output.contains("STATUS CODES"));
        assert!(output.contains("example.com"));
        assert!(output.contains("p95"));
    }
}
