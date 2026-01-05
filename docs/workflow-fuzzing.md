# Workflow Fuzzing Reference

Security fuzzing integration for workflow steps - automatically test APIs for common vulnerabilities.

## Overview

Fuzzing allows you to automatically test API endpoints for security vulnerabilities by injecting malicious payloads into request fields. When enabled on a workflow step, the fuzzer will:

1. Parse the request body to identify injectable fields
2. Generate payloads based on selected categories
3. Send multiple requests with injected payloads
4. Report anomalies (5xx errors, timeouts, unexpected behaviors)

## Quick Start

```yaml
name: Security Scan
base_url: https://api.example.com

steps:
  - name: Fuzz Login Endpoint
    method: POST
    url: /auth/login
    body: |
      {
        "username": "testuser",
        "password": "testpass"
      }
    fuzz:
      categories:
        - sql
        - xss
      concurrency: 5
```

## Configuration Reference

### FuzzConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fields` | string[] | all | Specific fields to fuzz |
| `categories` | string[] | all | Vulnerability categories to test |
| `risk_level` | integer | 1 | Minimum risk level (1-5) |
| `concurrency` | integer | 10 | Concurrent requests |
| `anomalies_only` | boolean | false | Only report anomalies |
| `stop_on_anomaly` | boolean | false | Stop on first anomaly |

### Fuzz Categories

| Category | Description | Example Payloads |
|----------|-------------|------------------|
| `sql` | SQL injection | `' OR '1'='1`, `; DROP TABLE users--` |
| `xss` | Cross-site scripting | `<script>alert(1)</script>`, `javascript:alert(1)` |
| `cmd` | Command injection | `; ls -la`, `| cat /etc/passwd` |
| `path` | Path traversal | `../../../etc/passwd`, `....//....//` |
| `boundary` | Boundary testing | Empty strings, very long strings, nulls |
| `type` | Type confusion | Arrays instead of strings, objects, null |
| `format` | Format string attacks | `%s%s%s%n`, `%x%x%x%x` |
| `int` | Integer overflow | `2147483647`, `-2147483648`, `99999999999` |
| `unicode` | Unicode edge cases | Zero-width chars, RTL overrides, homoglyphs |
| `nosql` | NoSQL injection | `{"$gt": ""}`, `{"$where": "1==1"}` |

### Risk Levels

Risk levels control payload aggressiveness:

| Level | Description | Use Case |
|-------|-------------|----------|
| 1 | Safe probing | Production systems |
| 2 | Light testing | Staging environments |
| 3 | Standard fuzzing | QA environments |
| 4 | Aggressive testing | Dev environments |
| 5 | Maximum coverage | Isolated test systems |

## Examples

### Fuzz Specific Fields

Only test specific form fields:

```yaml
steps:
  - name: Fuzz User Input Fields
    method: POST
    url: /api/users
    body: |
      {
        "username": "test",
        "email": "test@example.com",
        "bio": "User biography",
        "role": "user"
      }
    fuzz:
      fields:
        - username
        - bio
      categories:
        - xss
        - sql
```

### SQL Injection Testing

Focus on SQL injection vulnerabilities:

```yaml
steps:
  - name: SQL Injection Scan
    method: GET
    url: /api/search
    query:
      q: "test"
      category: "electronics"
      sort: "price"
    fuzz:
      categories:
        - sql
      risk_level: 3
      anomalies_only: true
```

### Comprehensive Security Scan

Full security audit with all categories:

```yaml
steps:
  - name: Full Security Audit
    method: POST
    url: /api/process
    body: |
      {
        "input": "user data",
        "config": {
          "option": "value"
        }
      }
    fuzz:
      categories:
        - sql
        - xss
        - cmd
        - path
        - nosql
      risk_level: 4
      concurrency: 20
      stop_on_anomaly: true
```

### API Boundary Testing

Test API input validation:

```yaml
steps:
  - name: Boundary Testing
    method: POST
    url: /api/create
    body: |
      {
        "name": "Product",
        "price": 99.99,
        "quantity": 10
      }
    fuzz:
      categories:
        - boundary
        - type
        - int
```

### Stop on First Vulnerability

For quick scans:

```yaml
steps:
  - name: Quick Vulnerability Check
    method: POST
    url: /api/vulnerable
    body: '{"data": "test"}'
    fuzz:
      stop_on_anomaly: true
      concurrency: 50
```

## Interpreting Results

### Anomaly Types

| Anomaly | Indication | Action |
|---------|------------|--------|
| 5xx Error | Server-side issue | Investigate error handling |
| Timeout | Resource exhaustion | Check for DoS vulnerability |
| Different Response | Injection worked | High-priority security issue |
| Stack Trace | Error disclosure | Information leakage |

### Sample Output

```
Fuzzing step "Fuzz Login Endpoint"...
  Testing field: username
    [ANOMALY] Payload: ' OR '1'='1
      Status: 500 (expected 401)
      Response: "SQL syntax error..."
    [OK] 15 other payloads passed
  Testing field: password
    [OK] 16 payloads passed

Summary:
  Total payloads: 32
  Anomalies: 1
  Duration: 2.3s
```

## Best Practices

1. **Start with low risk levels** - Use level 1-2 for production systems
2. **Use `anomalies_only`** - Reduces noise in output
3. **Target specific fields** - More efficient than fuzzing everything
4. **Run in isolated environments** - Fuzzing can trigger security alerts
5. **Review false positives** - Some anomalies may be expected behavior
6. **Combine with assertions** - Validate expected behavior after fuzzing

## Combining with Other Features

### Fuzzing with Authentication

```yaml
steps:
  - name: Get Auth Token
    method: POST
    url: /auth/login
    body: '{"username": "admin", "password": "secret"}'
    extract:
      token: body.access_token

  - name: Fuzz Authenticated Endpoint
    method: POST
    url: /api/admin/users
    headers:
      Authorization: "Bearer {{ token }}"
    body: '{"action": "create", "data": "test"}'
    fuzz:
      categories:
        - sql
        - cmd
```

### Fuzzing with Pre-Script

```yaml
steps:
  - name: Fuzz with Dynamic Data
    method: POST
    url: /api/process
    pre_script:
      code: |
        vars["timestamp"] = std::time::now();
        vars["nonce"] = crypto::random_string(16);
    body: |
      {
        "timestamp": "{{ timestamp }}",
        "nonce": "{{ nonce }}",
        "input": "test"
      }
    fuzz:
      fields:
        - input
```

## Security Considerations

- Fuzzing may trigger security monitoring alerts
- Some payloads could cause data corruption in non-isolated systems
- Always get proper authorization before fuzzing
- Log all fuzzing activities for audit purposes
- Consider rate limiting with `concurrency` setting

---

See also:
- [workflow.md](workflow.md) - Main workflow reference
- [README.md](../README.md) - CLI fuzzing options
