# Assertions

QuicPulse provides built-in assertions for testing HTTP responses in CI/CD pipelines and automated testing.

## Quick Reference

| Assertion | Flag | Example |
|-----------|------|---------|
| Status Code | `--assert-status` | `--assert-status 200` |
| Response Time | `--assert-time` | `--assert-time "<500ms"` |
| Body Content | `--assert-body` | `--assert-body ".success"` |
| Header | `--assert-header` | `--assert-header "Content-Type:json"` |

---

## Status Code Assertions

Assert that the response has a specific status code or falls within a range.

### Exact Status Code

```bash
# Assert exactly 200
quicpulse --assert-status 200 example.com/api

# Assert 201 Created
quicpulse --assert-status 201 POST example.com/users name=John
```

### Status Code Range

```bash
# Any 2xx status
quicpulse --assert-status 200-299 example.com/api

# Success range (200-204)
quicpulse --assert-status 200-204 example.com/api
```

### Status Code Class

```bash
# Any 2xx (success)
quicpulse --assert-status 2xx example.com/api

# Any 4xx (client error)
quicpulse --assert-status 4xx example.com/missing

# Any 5xx (server error)
quicpulse --assert-status 5xx example.com/error
```

### Exit Code

When assertions fail, QuicPulse exits with code 4:

```bash
quicpulse --assert-status 200 example.com/not-found
echo $?  # 4 (assertion failed)
```

---

## Response Time Assertions

Assert that the response is received within a time limit.

```bash
# Response must be faster than 500ms
quicpulse --assert-time "<500ms" example.com/api

# Response must be faster than 2 seconds
quicpulse --assert-time "<2s" example.com/slow

# Response must be faster than 1.5 seconds
quicpulse --assert-time "<1500ms" example.com/api
```

### Supported Time Units

| Unit | Example | Duration |
|------|---------|----------|
| `ms` | `500ms` | 500 milliseconds |
| `s` | `2s` | 2 seconds |
| `m` | `1m` | 1 minute |

---

## Body Content Assertions

Assert that the response body contains specific content.

### Literal Substring

```bash
# Body contains "success"
quicpulse --assert-body "success" example.com/api

# Body contains "error" (should fail for healthy API)
quicpulse --assert-body "error" example.com/api
```

### Key:Value Pattern

For JSON responses, assert specific field values:

```bash
# Assert success field is true
quicpulse --assert-body "success:true" example.com/api

# Assert status field equals "ok"
quicpulse --assert-body "status:ok" example.com/health
```

### JQ Expressions

Use JQ expressions for complex assertions:

```bash
# Assert .success exists and is truthy
quicpulse --assert-body ".success" example.com/api

# Assert array has items
quicpulse --assert-body ".users | length > 0" example.com/users

# Assert nested field
quicpulse --assert-body ".data.items[0].id" example.com/api

# Assert specific value
quicpulse --assert-body '.status == "active"' example.com/status
```

### JQ Expression Truthiness

JQ expressions are evaluated for truthiness:

| Value | Truthy |
|-------|--------|
| `true` | Yes |
| `false` | No |
| `null` | No |
| Non-zero number | Yes |
| Zero (`0`) | No |
| Non-empty string | Yes |
| Empty string (`""`) | No |
| Non-empty array | Yes |
| Empty array (`[]`) | No |
| Non-empty object | Yes |
| Empty object (`{}`) | No |

---

## Header Assertions

Assert that response headers exist and optionally match values.

### Header Exists

```bash
# Assert Content-Type header exists
quicpulse --assert-header "Content-Type" example.com/api

# Assert custom header exists
quicpulse --assert-header "X-Request-Id" example.com/api
```

### Header Value Contains

```bash
# Assert Content-Type contains "json"
quicpulse --assert-header "Content-Type:json" example.com/api

# Assert Content-Type is application/json
quicpulse --assert-header "Content-Type:application/json" example.com/api

# Assert Cache-Control contains "no-cache"
quicpulse --assert-header "Cache-Control:no-cache" example.com/api
```

### Multiple Headers

```bash
# Assert multiple headers
quicpulse \
  --assert-header "Content-Type:json" \
  --assert-header "X-Request-Id" \
  example.com/api
```

---

## Combining Assertions

Combine multiple assertions for comprehensive testing:

```bash
quicpulse \
  --assert-status 200 \
  --assert-time "<500ms" \
  --assert-body ".success" \
  --assert-header "Content-Type:json" \
  example.com/api
```

All assertions must pass for the command to succeed.

---

## CI/CD Integration

### Basic Health Check

```bash
#!/bin/bash
# health_check.sh

if quicpulse --assert-status 200 --assert-time "<2s" https://api.example.com/health; then
    echo "API is healthy"
    exit 0
else
    echo "API health check failed"
    exit 1
fi
```

### GitHub Actions

```yaml
# .github/workflows/api-test.yml
name: API Tests

on:
  push:
    branches: [main]
  schedule:
    - cron: '*/5 * * * *'  # Every 5 minutes

jobs:
  health-check:
    runs-on: ubuntu-latest
    steps:
      - name: Install QuicPulse
        run: cargo install quicpulse

      - name: Run health check
        run: |
          quicpulse \
            --assert-status 200 \
            --assert-time "<1s" \
            --assert-body ".status:ok" \
            https://api.example.com/health
```

### Jenkins Pipeline

```groovy
pipeline {
    agent any
    stages {
        stage('API Health Check') {
            steps {
                sh '''
                    quicpulse \
                        --assert-status 2xx \
                        --assert-time "<500ms" \
                        --assert-header "Content-Type:json" \
                        https://api.example.com/health
                '''
            }
        }
    }
}
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (all assertions passed) |
| 1 | General error |
| 2 | Request error (connection, timeout) |
| 3 | HTTP error (with `--check-status`) |
| 4 | Assertion failed |

---

## Assertions in Workflows

Assertions can be used in workflow files:

```yaml
# api-tests.yaml
name: API Tests

steps:
  - name: Health check
    request:
      url: https://api.example.com/health
    assertions:
      status: 200
      time: "<500ms"
      body:
        - ".status == \"ok\""
        - ".database.connected"
      headers:
        - "Content-Type:application/json"

  - name: Get users
    request:
      url: https://api.example.com/users
    assertions:
      status: 2xx
      body:
        - ".users | length > 0"
        - ".users[0].id"

  - name: Create user
    request:
      method: POST
      url: https://api.example.com/users
      json:
        name: "Test User"
        email: "test@example.com"
    assertions:
      status: 201
      body:
        - ".id"
        - ".name == \"Test User\""
```

### Workflow Assertion Syntax

```yaml
assertions:
  # Status code
  status: 200           # Exact
  status: 2xx           # Class
  status: 200-299       # Range

  # Response time
  time: "<500ms"

  # Body assertions (array of patterns)
  body:
    - ".success"              # JQ expression
    - "success:true"          # Key:value pattern
    - "expected string"       # Literal substring

  # Header assertions
  headers:
    - "Content-Type"                    # Exists
    - "Content-Type:application/json"   # Contains value
```

---

## Verbose Output

Get detailed assertion results:

```bash
quicpulse -v \
  --assert-status 200 \
  --assert-time "<500ms" \
  --assert-body ".success" \
  example.com/api
```

Output includes:
- Each assertion result (pass/fail)
- Actual values vs expected
- Detailed error messages

---

## Examples

### API Endpoint Testing

```bash
# Test CRUD operations
quicpulse --assert-status 200 GET example.com/api/items
quicpulse --assert-status 201 POST example.com/api/items name=Test
quicpulse --assert-status 200 PUT example.com/api/items/1 name=Updated
quicpulse --assert-status 204 DELETE example.com/api/items/1
```

### Performance Testing

```bash
# Assert response times for different endpoints
quicpulse --assert-time "<100ms" example.com/api/fast
quicpulse --assert-time "<500ms" example.com/api/medium
quicpulse --assert-time "<2s" example.com/api/slow
```

### JSON API Validation

```bash
# Comprehensive JSON API test
quicpulse \
  --assert-status 200 \
  --assert-header "Content-Type:application/json" \
  --assert-body ".data | length > 0" \
  --assert-body ".pagination.total" \
  --assert-body ".meta.version" \
  example.com/api/data
```

---

## See Also

- [CLI Reference](cli-reference.md) - All assertion flags
- [Workflows](workflow.md) - Assertions in automated tests
- [Filtering](filtering.md) - JQ expressions
