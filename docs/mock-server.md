# Built-in Mock Server Reference

Start a local HTTP mock server for testing and development.

## Overview

QuicPulse includes a built-in mock server that can simulate API endpoints locally. This is useful for:

- Testing API clients without external dependencies
- Developing against APIs that don't exist yet
- Creating reproducible test environments
- Simulating error conditions and edge cases

## Quick Start

```bash
# Start a simple mock server on port 8000
quicpulse --mock

# Start with custom port
quicpulse --mock --mock-port=3000

# Start with route definitions
quicpulse --mock --mock-route="GET /api/users -> 200 [{\"id\":1,\"name\":\"John\"}]"

# Start with configuration file
quicpulse --mock --mock-config=mock-config.yaml
```

## CLI Options

| Option | Description | Default |
|--------|-------------|---------|
| `--mock` | Enable mock server mode | - |
| `--mock-port <PORT>` | Port to listen on | `8000` |
| `--mock-config <FILE>` | Load routes from config file | - |
| `--mock-route <ROUTE>` | Define inline route (can be repeated) | - |
| `--mock-cors` | Enable CORS headers | `false` |
| `--mock-latency <MS>` | Add artificial latency to responses | `0` |

## Route Definition Syntax

### Inline Routes

Define routes directly on the command line:

```bash
# Basic route
quicpulse --mock --mock-route="GET /hello -> 200 Hello, World!"

# JSON response
quicpulse --mock --mock-route='POST /api/users -> 201 {"id":1,"created":true}'

# Multiple routes
quicpulse --mock \
  --mock-route="GET /api/users -> 200 []" \
  --mock-route="POST /api/users -> 201 {}" \
  --mock-route="GET /api/health -> 200 OK"
```

### Route Syntax Format

```
METHOD PATH -> STATUS [BODY]
```

| Component | Description | Example |
|-----------|-------------|---------|
| METHOD | HTTP method (GET, POST, etc.) | `GET`, `POST`, `*` |
| PATH | URL path with optional parameters | `/api/users/:id` |
| STATUS | HTTP status code | `200`, `404`, `500` |
| BODY | Optional response body | `{"key":"value"}` |

### Path Parameters

Use `:param` syntax for dynamic path segments:

```bash
# Path parameter
quicpulse --mock --mock-route="GET /users/:id -> 200 {\"id\":\":id\"}"

# Multiple parameters
quicpulse --mock --mock-route="GET /users/:userId/posts/:postId -> 200 {}"
```

### Wildcards

| Pattern | Description | Example |
|---------|-------------|---------|
| `:param` | Named parameter | `/users/:id` matches `/users/123` |
| `*` | Single segment wildcard | `/api/*` matches `/api/anything` |
| `**` | Multi-segment wildcard | `/static/**` matches `/static/a/b/c` |

## Configuration File

### YAML Format

```yaml
# mock-config.yaml
port: 8000
cors: true
latency: 50  # ms

routes:
  - method: GET
    path: /api/users
    status: 200
    body:
      - id: 1
        name: Alice
      - id: 2
        name: Bob

  - method: GET
    path: /api/users/:id
    status: 200
    body:
      id: ":id"
      name: "User :id"

  - method: POST
    path: /api/users
    status: 201
    headers:
      Location: /api/users/123
    body:
      id: 123
      created: true

  - method: DELETE
    path: /api/users/:id
    status: 204

  - method: "*"
    path: /api/**
    status: 404
    body:
      error: Not Found
```

### JSON Format

```json
{
  "port": 8000,
  "cors": true,
  "routes": [
    {
      "method": "GET",
      "path": "/api/health",
      "status": 200,
      "body": {"status": "ok"}
    }
  ]
}
```

### TOML Format

```toml
port = 8000
cors = true
latency = 100

[[routes]]
method = "GET"
path = "/api/users"
status = 200
body = '[{"id": 1}]'
```

## Response Configuration

### Status Codes

```yaml
routes:
  - method: GET
    path: /success
    status: 200

  - method: GET
    path: /created
    status: 201

  - method: GET
    path: /not-found
    status: 404

  - method: GET
    path: /error
    status: 500
```

### Custom Headers

```yaml
routes:
  - method: GET
    path: /api/data
    status: 200
    headers:
      Content-Type: application/json
      X-Request-Id: "{{uuid}}"
      Cache-Control: no-cache
    body:
      data: value
```

### Response Body Types

```yaml
routes:
  # JSON object
  - method: GET
    path: /json
    body:
      key: value
      nested:
        field: data

  # JSON array
  - method: GET
    path: /array
    body:
      - item1
      - item2

  # Plain text
  - method: GET
    path: /text
    body: "Plain text response"

  # From file
  - method: GET
    path: /file
    body_file: ./responses/data.json
```

## Dynamic Responses

### Template Variables

Use `{{variable}}` syntax for dynamic values:

```yaml
routes:
  - method: GET
    path: /api/users/:id
    status: 200
    body:
      id: "{{params.id}}"
      timestamp: "{{timestamp}}"
      uuid: "{{uuid}}"
```

### Available Variables

| Variable | Description |
|----------|-------------|
| `{{params.<name>}}` | Path parameter value |
| `{{query.<name>}}` | Query string parameter |
| `{{header.<name>}}` | Request header value |
| `{{uuid}}` | Random UUID |
| `{{timestamp}}` | Unix timestamp |
| `{{now}}` | ISO 8601 datetime |

### Conditional Responses

```yaml
routes:
  - method: GET
    path: /api/users/:id
    conditions:
      - if: "params.id == '1'"
        status: 200
        body: {"id": 1, "name": "Admin"}
      - if: "params.id == '999'"
        status: 404
        body: {"error": "User not found"}
    default:
      status: 200
      body: {"id": "{{params.id}}", "name": "User"}
```

## CORS Configuration

Enable CORS for cross-origin requests:

```bash
# Enable CORS globally
quicpulse --mock --mock-cors
```

Or in config file:

```yaml
cors: true
cors_config:
  allow_origins:
    - "*"
  allow_methods:
    - GET
    - POST
    - PUT
    - DELETE
  allow_headers:
    - Content-Type
    - Authorization
  expose_headers:
    - X-Request-Id
  max_age: 3600
```

## Latency Simulation

Add artificial delays to simulate network latency:

```bash
# Add 100ms latency to all routes
quicpulse --mock --mock-latency=100
```

Per-route latency in config:

```yaml
routes:
  - method: GET
    path: /fast
    latency: 10
    body: fast

  - method: GET
    path: /slow
    latency: 2000
    body: slow
```

## TLS/HTTPS

Enable TLS for HTTPS mock server:

```yaml
tls:
  enabled: true
  cert: ./certs/server.crt
  key: ./certs/server.key
```

## Examples

### API Mock for Testing

```yaml
# api-mock.yaml
port: 8080
cors: true

routes:
  # Health check
  - method: GET
    path: /health
    status: 200
    body: {status: ok}

  # User CRUD
  - method: GET
    path: /users
    body: [{id: 1, name: Alice}, {id: 2, name: Bob}]

  - method: GET
    path: /users/:id
    body: {id: "{{params.id}}", name: "User {{params.id}}"}

  - method: POST
    path: /users
    status: 201
    body: {id: 3, created: true}

  - method: PUT
    path: /users/:id
    body: {id: "{{params.id}}", updated: true}

  - method: DELETE
    path: /users/:id
    status: 204

  # Error simulation
  - method: GET
    path: /error
    status: 500
    body: {error: Internal Server Error}
```

### Static File Server

```yaml
routes:
  - method: GET
    path: /static/**
    body_file: "./public/{{path}}"
    headers:
      Cache-Control: "max-age=3600"
```

### OAuth Mock

```yaml
routes:
  - method: POST
    path: /oauth/token
    status: 200
    body:
      access_token: "mock-token-{{uuid}}"
      token_type: Bearer
      expires_in: 3600

  - method: GET
    path: /oauth/userinfo
    status: 200
    body:
      sub: "user-123"
      name: "Test User"
      email: "test@example.com"
```

## Integration with QuicPulse

### Testing Workflows

```yaml
# workflow.yaml
name: API Tests
base_url: http://localhost:8080

steps:
  - name: Health Check
    method: GET
    url: /health
    assert:
      status: 200
```

Run mock server in background:

```bash
# Terminal 1: Start mock server
quicpulse --mock --mock-config=api-mock.yaml

# Terminal 2: Run tests
quicpulse --run=workflow.yaml
```

### Proxy Mode

Forward unmatched requests to a real server:

```yaml
proxy:
  enabled: true
  target: https://api.example.com
  forward_unmatched: true
```

## Troubleshooting

### Port Already in Use

```bash
# Use a different port
quicpulse --mock --mock-port=3001

# Or find and kill the process using the port
lsof -i :8000
```

### Route Not Matching

1. Check route order (more specific routes first)
2. Verify HTTP method matches
3. Check path parameters syntax
4. Use wildcards for catch-all routes

### CORS Issues

1. Enable `--mock-cors` flag
2. Check allowed origins in config
3. Verify preflight OPTIONS handling

---

See also:
- [README.md](../README.md) - CLI reference
- [workflow.md](workflow.md) - Workflow reference
