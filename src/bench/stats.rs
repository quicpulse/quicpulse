//! Statistics collection and computation for benchmarks
//!
//! Uses HDR Histogram for accurate latency percentile calculation.

use std::collections::HashMap;
use std::time::Duration;
use hdrhistogram::Histogram;

/// Latency statistics in milliseconds
#[derive(Debug, Clone)]
pub struct LatencyStats {
    pub min_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub stddev_ms: f64,
    pub p50_ms: f64,
    pub p75_ms: f64,
    pub p90_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

impl Default for LatencyStats {
    fn default() -> Self {
        Self {
            min_ms: 0.0,
            max_ms: 0.0,
            mean_ms: 0.0,
            stddev_ms: 0.0,
            p50_ms: 0.0,
            p75_ms: 0.0,
            p90_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
        }
    }
}

/// Complete benchmark statistics
#[derive(Debug, Clone)]
pub struct BenchmarkStats {
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub success_rate: f64,
    pub requests_per_second: f64,
    pub bytes_per_second: f64,
    pub total_bytes: u64,
    pub status_codes: HashMap<u16, u64>,
    pub errors: HashMap<String, u64>,
    pub latency: LatencyStats,
}

impl Default for BenchmarkStats {
    fn default() -> Self {
        Self {
            successful_requests: 0,
            failed_requests: 0,
            success_rate: 0.0,
            requests_per_second: 0.0,
            bytes_per_second: 0.0,
            total_bytes: 0,
            status_codes: HashMap::new(),
            errors: HashMap::new(),
            latency: LatencyStats::default(),
        }
    }
}

/// Collects statistics during benchmark execution
pub struct StatsCollector {
    /// HDR Histogram for latency tracking (in microseconds)
    histogram: Histogram<u64>,
    /// Status code counts
    status_codes: HashMap<u16, u64>,
    /// Error counts
    errors: HashMap<String, u64>,
    /// Total successful requests
    successful: u64,
    /// Total failed requests
    failed: u64,
    /// Total bytes received
    total_bytes: u64,
}

const MAX_LATENCY_US: u64 = 300_000_000; // 5 minutes

impl StatsCollector {
    /// Create a new stats collector
    pub fn new() -> Self {
        let histogram = Histogram::new_with_bounds(1, MAX_LATENCY_US, 3)
            .expect("Failed to create histogram");

        Self {
            histogram,
            status_codes: HashMap::new(),
            errors: HashMap::new(),
            successful: 0,
            failed: 0,
            total_bytes: 0,
        }
    }

    /// Record a request result
    pub fn record(
        &mut self,
        status_code: Option<u16>,
        latency: Duration,
        bytes: usize,
        error: Option<String>,
    ) {
        let latency_us = latency.as_micros() as u64;
        let clamped_latency = latency_us.max(1).min(MAX_LATENCY_US);
        let _ = self.histogram.record(clamped_latency);

        if let Some(code) = status_code {
            *self.status_codes.entry(code).or_insert(0) += 1;

            if code >= 200 && code < 400 {
                self.successful += 1;
            } else {
                self.failed += 1;
            }

            self.total_bytes += bytes as u64;
        } else {
            self.failed += 1;

            if let Some(err) = error {
                *self.errors.entry(err).or_insert(0) += 1;
            }
        }
    }

    /// Finalize and compute statistics
    pub fn finalize(self, duration: Duration) -> BenchmarkStats {
        let total = self.successful + self.failed;
        let duration_secs = duration.as_secs_f64();

        let latency = if self.histogram.len() > 0 {
            LatencyStats {
                min_ms: self.histogram.min() as f64 / 1000.0,
                max_ms: self.histogram.max() as f64 / 1000.0,
                mean_ms: self.histogram.mean() / 1000.0,
                stddev_ms: self.histogram.stdev() / 1000.0,
                p50_ms: self.histogram.value_at_percentile(50.0) as f64 / 1000.0,
                p75_ms: self.histogram.value_at_percentile(75.0) as f64 / 1000.0,
                p90_ms: self.histogram.value_at_percentile(90.0) as f64 / 1000.0,
                p95_ms: self.histogram.value_at_percentile(95.0) as f64 / 1000.0,
                p99_ms: self.histogram.value_at_percentile(99.0) as f64 / 1000.0,
            }
        } else {
            LatencyStats::default()
        };

        BenchmarkStats {
            successful_requests: self.successful,
            failed_requests: self.failed,
            success_rate: if total > 0 { self.successful as f64 / total as f64 } else { 0.0 },
            requests_per_second: if duration_secs > 0.0 { total as f64 / duration_secs } else { 0.0 },
            bytes_per_second: if duration_secs > 0.0 { self.total_bytes as f64 / duration_secs } else { 0.0 },
            total_bytes: self.total_bytes,
            status_codes: self.status_codes,
            errors: self.errors,
            latency,
        }
    }
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_collector() {
        let mut collector = StatsCollector::new();

        // Record some successful requests
        collector.record(Some(200), Duration::from_millis(100), 1024, None);
        collector.record(Some(200), Duration::from_millis(150), 2048, None);
        collector.record(Some(200), Duration::from_millis(200), 512, None);

        // Record a failed request
        collector.record(Some(500), Duration::from_millis(50), 100, None);

        // Record an error
        collector.record(None, Duration::from_millis(1000), 0, Some("Timeout".to_string()));

        let stats = collector.finalize(Duration::from_secs(1));

        assert_eq!(stats.successful_requests, 3);
        assert_eq!(stats.failed_requests, 2);
        assert_eq!(stats.total_bytes, 1024 + 2048 + 512 + 100);
        assert!(stats.latency.mean_ms > 0.0);
    }

    #[test]
    fn test_latency_percentiles() {
        let mut collector = StatsCollector::new();

        // Add 100 requests with increasing latencies
        for i in 1..=100 {
            collector.record(Some(200), Duration::from_millis(i * 10), 100, None);
        }

        let stats = collector.finalize(Duration::from_secs(10));

        // p50 should be around 500ms
        assert!(stats.latency.p50_ms >= 450.0 && stats.latency.p50_ms <= 550.0);

        // p99 should be around 990ms
        assert!(stats.latency.p99_ms >= 950.0 && stats.latency.p99_ms <= 1010.0);
    }

    #[test]
    fn test_empty_collector() {
        let collector = StatsCollector::new();
        let stats = collector.finalize(Duration::from_secs(1));

        assert_eq!(stats.successful_requests, 0);
        assert_eq!(stats.failed_requests, 0);
        assert_eq!(stats.latency.mean_ms, 0.0);
    }
}
