# Workflow Plugins Reference

Plugin integration for workflow steps.

## Overview

Plugins extend workflow functionality with custom processing logic. Plugins can:

- Modify requests before sending
- Process responses after receiving
- Add custom headers or authentication
- Implement rate limiting
- Log requests/responses
- Transform data

## Quick Start

```yaml
name: Workflow with Plugins
base_url: https://api.example.com

# Global plugins applied to all steps
plugins:
  - request-logger

steps:
  - name: API Call
    method: GET
    url: /users
    plugins:
      - name: rate-limiter
        config:
          requests_per_second: 10
```

## Configuration Reference

### Workflow-Level Plugins

| Field | Type | Description |
|-------|------|-------------|
| `plugins` | string[] | List of plugin names to apply globally |

### Step-Level Plugins

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Plugin name |
| `config` | object | Plugin-specific configuration |

## Examples

### Global Plugin

Apply plugin to all steps:

```yaml
name: Logged Workflow
plugins:
  - request-logger

steps:
  - name: Step 1
    method: GET
    url: /api/data
    # request-logger applied

  - name: Step 2
    method: POST
    url: /api/data
    # request-logger applied
```

### Per-Step Plugin

Apply plugin to specific step:

```yaml
steps:
  - name: Rate Limited Call
    method: GET
    url: /api/expensive
    plugins:
      - name: rate-limiter
        config:
          requests_per_second: 5
          burst: 10
```

### Multiple Plugins

Chain multiple plugins:

```yaml
steps:
  - name: Full Processing
    method: POST
    url: /api/data
    plugins:
      - name: request-logger
      - name: retry-handler
        config:
          max_retries: 3
          retry_delay_ms: 1000
      - name: response-validator
        config:
          schema: ./schemas/response.json
```

### Plugin Configuration

Pass configuration to plugins:

```yaml
steps:
  - name: Custom Auth Plugin
    method: GET
    url: /api/secure
    plugins:
      - name: custom-auth
        config:
          auth_type: hmac
          secret_key: "{{ secret }}"
          header_name: X-Auth-Signature
```

## Built-in Plugins

### request-logger

Logs request and response details:

```yaml
plugins:
  - name: request-logger
    config:
      log_headers: true
      log_body: true
      log_timing: true
      output: ./logs/requests.log
```

### rate-limiter

Rate limit requests:

```yaml
plugins:
  - name: rate-limiter
    config:
      requests_per_second: 10
      burst: 20
      wait_on_limit: true
```

### retry-handler

Automatic retry on failure:

```yaml
plugins:
  - name: retry-handler
    config:
      max_retries: 3
      retry_delay_ms: 1000
      retry_on_status: [500, 502, 503, 504]
      exponential_backoff: true
```

### response-cache

Cache responses:

```yaml
plugins:
  - name: response-cache
    config:
      ttl_seconds: 300
      cache_key: "{{ method }}-{{ url }}"
```

## Custom Plugins

### Plugin Structure

Plugins are loaded from:
- `~/.config/quicpulse/plugins/`
- `./plugins/` (project local)

### Plugin Interface

```rust
pub trait Plugin {
    fn name(&self) -> &str;
    fn on_request(&self, request: &mut Request, config: &Value) -> Result<()>;
    fn on_response(&self, response: &Response, config: &Value) -> Result<()>;
}
```

### Example Custom Plugin

```rust
// plugins/my-plugin.rs
pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        "my-plugin"
    }

    fn on_request(&self, request: &mut Request, config: &Value) -> Result<()> {
        // Add custom header
        if let Some(header_value) = config.get("header_value") {
            request.headers.insert("X-Custom", header_value.as_str()?);
        }
        Ok(())
    }

    fn on_response(&self, response: &Response, config: &Value) -> Result<()> {
        // Log response status
        println!("Response: {}", response.status);
        Ok(())
    }
}
```

## Plugin Execution Order

1. Global plugins execute first (in order defined)
2. Step plugins execute second (in order defined)
3. Request phase: plugins modify request
4. HTTP request is sent
5. Response phase: plugins process response

```yaml
plugins:
  - global-plugin-1  # Executes 1st
  - global-plugin-2  # Executes 2nd

steps:
  - name: Step
    plugins:
      - name: step-plugin-1  # Executes 3rd
      - name: step-plugin-2  # Executes 4th
```

## Integration Patterns

### Authentication Plugin

```yaml
steps:
  - name: Authenticated Request
    method: GET
    url: /api/protected
    plugins:
      - name: aws-sigv4
        config:
          access_key: "{{ aws_access_key }}"
          secret_key: "{{ aws_secret_key }}"
          region: us-east-1
          service: execute-api
```

### Metrics Collection

```yaml
plugins:
  - metrics-collector

steps:
  - name: Monitored Request
    method: GET
    url: /api/data
    # Metrics automatically collected

# Metrics available after workflow:
# - request_count
# - response_times
# - error_rates
```

### Request Modification

```yaml
steps:
  - name: Modified Request
    method: POST
    url: /api/data
    body: '{"data": "value"}'
    plugins:
      - name: request-transformer
        config:
          add_headers:
            X-Request-ID: "{uuid}"
            X-Timestamp: "{timestamp}"
          sign_body: true
```

## Best Practices

1. **Use global plugins for cross-cutting concerns** - Logging, metrics
2. **Use step plugins for specific needs** - Auth, rate limiting
3. **Order plugins carefully** - Earlier plugins can affect later ones
4. **Configure via variables** - Make plugins configurable
5. **Test plugins independently** - Verify plugin behavior
6. **Document plugin requirements** - Note dependencies

## Common Patterns

### CI/CD Integration

```yaml
plugins:
  - name: junit-reporter
    config:
      output: ./test-results.xml

steps:
  - name: API Test
    method: GET
    url: /api/health
    assert:
      status: 200
```

### Security Scanning

```yaml
plugins:
  - name: security-scanner
    config:
      check_headers: true
      check_ssl: true
      report_file: ./security-report.json

steps:
  - name: Secure Endpoint
    method: GET
    url: /api/secure
```

### Performance Monitoring

```yaml
plugins:
  - name: performance-monitor
    config:
      alert_threshold_ms: 500
      notify_webhook: "{{ slack_webhook }}"

steps:
  - name: Performance Critical
    method: GET
    url: /api/fast
```

## Troubleshooting

### Plugin Not Found

1. Check plugin is installed in plugin directory
2. Verify plugin name matches exactly
3. Check file permissions

### Plugin Configuration Error

1. Validate config structure
2. Check required fields
3. Verify value types

### Plugin Execution Failure

1. Check plugin logs
2. Verify dependencies
3. Test plugin independently

---

See also:
- [workflow.md](workflow.md) - Main workflow reference
- [README.md](../README.md) - CLI plugin options
