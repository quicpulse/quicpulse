# Workflow Output Reference

Output configuration, filtering, and response saving for workflow steps.

## Overview

Output configuration controls how workflow step results are displayed and saved:

- **OutputConfig**: Control what's displayed and how
- **FilterConfig**: Filter headers and body content
- **SaveConfig**: Save responses to files
- **Curl**: Generate curl commands for debugging

## Quick Start

```yaml
name: Output Control Demo
base_url: https://api.example.com

# Global output settings
output:
  verbose: true
  pretty: true

steps:
  - name: API Call
    method: GET
    url: /users
    output:
      format: json
      headers_only: false
    filter:
      exclude_headers:
        - X-Internal-*
    save:
      path: ./responses/users.json
      format: json
    curl: true
```

## Configuration Reference

### OutputConfig

Control display settings:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `verbose` | boolean | false | Show request + response |
| `headers_only` | boolean | false | Show only headers |
| `body_only` | boolean | false | Show only body |
| `format` | string | auto | Output format: "json", "xml", "raw", "table" |
| `colors` | boolean | true | Enable/disable colors |
| `print` | string | "hb" | Print options |
| `pretty` | boolean | true | Pretty print output |

### FilterConfig

Filter response content:

| Field | Type | Description |
|-------|------|-------------|
| `include_headers` | string[] | Headers to include (glob patterns) |
| `exclude_headers` | string[] | Headers to exclude (glob patterns) |
| `include_body` | string[] | Body fields to include (JSONPath) |
| `exclude_body` | string[] | Body fields to exclude (JSONPath) |

### SaveConfig

Save responses to files:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | string | required | Output file path |
| `what` | string | "response" | What to save: "response", "headers", "body", "all" |
| `format` | string | "raw" | Output format: "raw", "json", "har" |
| `append` | boolean | false | Append to file instead of overwrite |

### Curl Generation

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `curl` | boolean | false | Generate curl command |

## Output Examples

### Verbose Output

Show full request and response:

```yaml
steps:
  - name: Debug API Call
    method: POST
    url: /api/data
    body: '{"key": "value"}'
    output:
      verbose: true
```

### Headers Only

Show only response headers:

```yaml
steps:
  - name: Check Headers
    method: HEAD
    url: /api/resource
    output:
      headers_only: true
```

### Body Only

Show only response body:

```yaml
steps:
  - name: Get Data Only
    method: GET
    url: /api/data
    output:
      body_only: true
```

### JSON Format

Force JSON output:

```yaml
steps:
  - name: JSON Response
    method: GET
    url: /api/users
    output:
      format: json
      pretty: true
```

### Print Options

```yaml
steps:
  - name: Custom Print
    method: GET
    url: /api/data
    output:
      print: "hHbB"  # h=response headers, H=request headers,
                     # b=response body, B=request body
```

| Option | Description |
|--------|-------------|
| `h` | Response headers |
| `H` | Request headers |
| `b` | Response body |
| `B` | Request body |

### Disable Colors

For CI/CD or logging:

```yaml
output:
  colors: false

steps:
  - name: No Colors
    method: GET
    url: /api/data
```

## Filter Examples

### Exclude Internal Headers

Hide internal/sensitive headers:

```yaml
steps:
  - name: Public Response
    method: GET
    url: /api/data
    filter:
      exclude_headers:
        - X-Internal-*
        - X-Debug-*
        - X-Request-ID
```

### Include Specific Headers

Show only specific headers:

```yaml
steps:
  - name: Relevant Headers Only
    method: GET
    url: /api/data
    filter:
      include_headers:
        - Content-Type
        - Content-Length
        - Cache-Control
```

### Filter Body Fields

Exclude sensitive body data:

```yaml
steps:
  - name: Filtered Response
    method: GET
    url: /api/users/me
    filter:
      exclude_body:
        - password
        - ssn
        - credit_card
```

### Include Body Fields

Show only specific fields:

```yaml
steps:
  - name: Minimal Response
    method: GET
    url: /api/users
    filter:
      include_body:
        - id
        - name
        - email
```

### Combined Filters

Multiple filter criteria:

```yaml
steps:
  - name: Fully Filtered
    method: GET
    url: /api/data
    filter:
      exclude_headers:
        - X-*
        - Server
      include_body:
        - data
        - meta
      exclude_body:
        - data.internal
```

## Save Examples

### Save Full Response

Save complete response:

```yaml
steps:
  - name: Save Response
    method: GET
    url: /api/data
    save:
      path: ./responses/data.txt
      what: response
```

### Save Headers Only

Save just headers:

```yaml
steps:
  - name: Save Headers
    method: GET
    url: /api/data
    save:
      path: ./headers/response-headers.txt
      what: headers
```

### Save Body Only

Save just the body:

```yaml
steps:
  - name: Save Body
    method: GET
    url: /api/data
    save:
      path: ./data/response.json
      what: body
```

### Save as JSON

Format as JSON:

```yaml
steps:
  - name: Save as JSON
    method: GET
    url: /api/users
    save:
      path: ./data/users.json
      what: body
      format: json
```

### Save as HAR

Save in HAR format:

```yaml
steps:
  - name: Save as HAR
    method: GET
    url: /api/data
    save:
      path: ./recordings/request.har
      what: all
      format: har
```

### Append to File

Append multiple responses:

```yaml
steps:
  - name: Request 1
    method: GET
    url: /api/data/1
    save:
      path: ./logs/responses.log
      append: true

  - name: Request 2
    method: GET
    url: /api/data/2
    save:
      path: ./logs/responses.log
      append: true
```

### Dynamic Save Path

Use variables in path:

```yaml
steps:
  - name: Save with Timestamp
    method: GET
    url: /api/data
    save:
      path: ./responses/data-{timestamp}.json
      format: json

  - name: Save with UUID
    method: GET
    url: /api/backup
    save:
      path: ./backups/backup-{uuid}.json
```

## Curl Generation

### Generate Curl Command

Output curl equivalent:

```yaml
steps:
  - name: Debug with Curl
    method: POST
    url: /api/data
    headers:
      Content-Type: application/json
      Authorization: "Bearer token123"
    body: '{"key": "value"}'
    curl: true
```

Output:
```
curl -X POST 'https://api.example.com/api/data' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer token123' \
  -d '{"key": "value"}'
```

### All Steps with Curl

Generate curl for all steps:

```yaml
steps:
  - name: Step 1
    method: GET
    url: /api/users
    curl: true

  - name: Step 2
    method: POST
    url: /api/users
    body: '{"name": "Test"}'
    curl: true
```

## Global vs Step Settings

### Global Output

Apply to all steps:

```yaml
output:
  verbose: true
  pretty: true
  colors: true

steps:
  - name: Step 1
    url: /api/data
    # Uses global output settings

  - name: Step 2
    url: /api/users
    # Uses global output settings
```

### Override Global

Step settings override global:

```yaml
output:
  verbose: false

steps:
  - name: Normal Step
    url: /api/data
    # Not verbose

  - name: Debug Step
    url: /api/debug
    output:
      verbose: true  # Overrides global
```

## Integration Patterns

### CI/CD Friendly Output

```yaml
output:
  colors: false
  verbose: false
  pretty: false

steps:
  - name: CI Test
    method: GET
    url: /api/health
    save:
      path: ./test-results/health-check.json
      format: json
```

### Debug Mode

```yaml
steps:
  - name: Full Debug
    method: POST
    url: /api/complex
    body: '{"data": "test"}'
    output:
      verbose: true
      pretty: true
    filter:
      # Show everything for debugging
    save:
      path: ./debug/request.har
      what: all
      format: har
    curl: true
```

### Logging Pattern

```yaml
steps:
  - name: Logged Request
    method: GET
    url: /api/data
    output:
      body_only: true
    save:
      path: ./logs/responses.jsonl
      what: body
      format: json
      append: true
```

## Best Practices

1. **Use global output for consistency** - Set defaults at workflow level
2. **Filter sensitive data** - Exclude passwords, tokens in output
3. **Save responses for debugging** - Keep HAR files for troubleshooting
4. **Generate curl for sharing** - Easy to reproduce issues
5. **Disable colors in CI** - Clean log output
6. **Use pretty for development** - Better readability

## Common Patterns

### Clean Output for Reports

```yaml
output:
  colors: false
  pretty: true

steps:
  - name: Report Data
    url: /api/metrics
    output:
      body_only: true
      format: json
    filter:
      include_body:
        - metrics
        - summary
```

### Full Audit Trail

```yaml
steps:
  - name: Audited Request
    method: POST
    url: /api/transactions
    body: '{"amount": 100}'
    save:
      path: ./audit/transaction-{timestamp}.har
      what: all
      format: har
```

### Development Debugging

```yaml
steps:
  - name: Debug API
    method: POST
    url: /api/test
    output:
      verbose: true
    curl: true
    save:
      path: ./debug/last-request.json
      format: json
```

---

See also:
- [workflow.md](workflow.md) - Main workflow reference
- [README.md](../README.md) - CLI output options
