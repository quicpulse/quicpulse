# Workflow Benchmarking Reference

Load testing and performance benchmarking integration for workflow steps.

## Overview

Benchmarking allows you to load test API endpoints directly within workflows. When enabled on a step, the benchmark runner will:

1. Send the configured number of concurrent requests
2. Collect response times and success rates
3. Calculate performance statistics (p50, p90, p95, p99)
4. Report throughput and error rates

## Quick Start

```yaml
name: Performance Tests
base_url: https://api.example.com

steps:
  - name: Benchmark Health Endpoint
    method: GET
    url: /health
    bench:
      requests: 1000
      concurrency: 50
```

## Configuration Reference

### BenchConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `requests` | integer | required | Total number of requests to send |
| `concurrency` | integer | 10 | Number of concurrent connections |
| `rate_limit` | integer | 0 | Max requests per second (0 = unlimited) |
| `warmup` | integer | 0 | Warmup requests before measurement |

## Examples

### Basic Load Test

Simple concurrent load test:

```yaml
steps:
  - name: Load Test API
    method: GET
    url: /api/products
    bench:
      requests: 500
      concurrency: 25
```

### High Concurrency Test

Stress test with high parallelism:

```yaml
steps:
  - name: Stress Test
    method: GET
    url: /api/status
    bench:
      requests: 10000
      concurrency: 200
```

### Rate-Limited Benchmark

Control request rate for realistic load patterns:

```yaml
steps:
  - name: Realistic Load Pattern
    method: POST
    url: /api/orders
    body: '{"product_id": "123", "quantity": 1}'
    bench:
      requests: 1000
      concurrency: 50
      rate_limit: 100  # 100 requests/second max
```

### Warmup Phase

Exclude warmup from statistics:

```yaml
steps:
  - name: Benchmark with Warmup
    method: GET
    url: /api/data
    bench:
      requests: 1000
      concurrency: 20
      warmup: 100  # First 100 requests not measured
```

### POST with Body

Benchmark POST endpoints:

```yaml
steps:
  - name: Benchmark Create Endpoint
    method: POST
    url: /api/users
    headers:
      Content-Type: application/json
    body: |
      {
        "name": "Test User",
        "email": "test_{uuid}@example.com"
      }
    bench:
      requests: 500
      concurrency: 20
```

### Authenticated Benchmark

Benchmark protected endpoints:

```yaml
steps:
  - name: Get Token
    method: POST
    url: /auth/token
    body: '{"client_id": "test", "client_secret": "secret"}'
    extract:
      token: body.access_token

  - name: Benchmark Protected API
    method: GET
    url: /api/protected/data
    headers:
      Authorization: "Bearer {{ token }}"
    bench:
      requests: 1000
      concurrency: 50
```

## Output Interpretation

### Sample Output

```
Benchmark: "Load Test API"
URL: https://api.example.com/api/products
Method: GET

Results:
  Total Requests:    1000
  Successful:        998 (99.8%)
  Failed:            2 (0.2%)

  Duration:          12.34s
  Requests/sec:      81.03

Response Times:
  Min:               5ms
  Max:               234ms
  Mean:              48ms
  Median (p50):      42ms
  p90:               89ms
  p95:               112ms
  p99:               198ms

Transfer:
  Total:             15.2 MB
  Rate:              1.23 MB/s
```

### Metrics Explained

| Metric | Description |
|--------|-------------|
| `Requests/sec` | Throughput (higher is better) |
| `p50 (Median)` | 50% of requests faster than this |
| `p90` | 90% of requests faster than this |
| `p95` | 95% of requests faster than this |
| `p99` | 99% of requests faster than this |
| `Mean` | Average response time |

### Interpreting Percentiles

- **p50 close to mean**: Normal distribution, consistent performance
- **p99 >> p90**: Tail latency issues, occasional slow requests
- **High variance**: Inconsistent performance, investigate caching/scaling

## Multi-Endpoint Benchmarks

### Sequential Benchmarks

Test multiple endpoints in sequence:

```yaml
name: API Benchmark Suite
base_url: https://api.example.com

steps:
  - name: Benchmark Read Operations
    method: GET
    url: /api/products
    bench:
      requests: 1000
      concurrency: 50

  - name: Benchmark Write Operations
    method: POST
    url: /api/products
    body: '{"name": "Test", "price": 9.99}'
    bench:
      requests: 500
      concurrency: 20

  - name: Benchmark Search
    method: GET
    url: /api/search
    query:
      q: "test query"
    bench:
      requests: 1000
      concurrency: 30
```

### Environment-Specific Benchmarks

Different loads for different environments:

```yaml
name: Environment Benchmarks
base_url: https://api.example.com

variables:
  bench_requests: 100
  bench_concurrency: 10

environments:
  staging:
    bench_requests: 1000
    bench_concurrency: 50
  production:
    bench_requests: 100
    bench_concurrency: 10

steps:
  - name: Benchmark API
    method: GET
    url: /api/data
    bench:
      requests: "{{ bench_requests }}"
      concurrency: "{{ bench_concurrency }}"
```

## Combining with Assertions

### Performance SLA Validation

Combine benchmarking with latency assertions:

```yaml
steps:
  - name: Performance SLA Test
    method: GET
    url: /api/critical
    bench:
      requests: 1000
      concurrency: 50
    assert:
      latency: 200  # p95 must be under 200ms
      status: 200
```

### Success Rate Validation

Ensure high availability under load:

```yaml
steps:
  - name: Availability Test
    method: GET
    url: /api/health
    bench:
      requests: 5000
      concurrency: 100
    # Success rate is reported in output
```

## Best Practices

1. **Start small** - Begin with low concurrency and scale up
2. **Use warmup** - Allows JIT compilation and connection pooling
3. **Rate limit production** - Avoid overwhelming production systems
4. **Test in isolation** - Use dedicated test environments when possible
5. **Monitor server resources** - CPU, memory, database connections
6. **Consider think time** - Real users have delays between requests
7. **Benchmark at scale** - Test expected peak loads
8. **Multiple runs** - Run benchmarks multiple times for consistency

## Common Patterns

### Baseline vs. After Change

```yaml
name: Performance Regression Test

steps:
  # Run same benchmark before and after code changes
  - name: Baseline Benchmark
    method: GET
    url: /api/v1/products
    bench:
      requests: 1000
      concurrency: 50

  - name: New Version Benchmark
    method: GET
    url: /api/v2/products
    bench:
      requests: 1000
      concurrency: 50
```

### Scaling Test

```yaml
name: Scaling Test

steps:
  - name: Light Load
    method: GET
    url: /api/data
    bench:
      requests: 100
      concurrency: 10

  - name: Medium Load
    method: GET
    url: /api/data
    bench:
      requests: 500
      concurrency: 50

  - name: Heavy Load
    method: GET
    url: /api/data
    bench:
      requests: 1000
      concurrency: 100
```

## Limitations

- Benchmarks run from a single machine (not distributed)
- Connection pooling may affect results
- Network latency impacts measurements
- Server-side caching can skew results
- Memory usage scales with concurrency

---

See also:
- [workflow.md](workflow.md) - Main workflow reference
- [README.md](../README.md) - CLI benchmarking options
