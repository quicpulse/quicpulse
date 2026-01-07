# QuicPulse

A powerful, API testing framework for the command line, written in Rust. Inspired by [HTTPie](https://httpie.io/) with extended features for modern API development.

## Table of Contents

- [Features Overview](#features-overview)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Request Syntax](#request-syntax)
- [Authentication](#authentication)
- [Sessions](#sessions)
- [Output Control](#output-control)
- [Data Filtering](#data-filtering)
- [Downloads](#downloads)
- [Assertions](#assertions)
- [GraphQL](#graphql)
- [gRPC](#grpc)
- [WebSocket](#websocket)
- [Workflows](#workflows)
- [Kubernetes](#kubernetes)
- [OpenAPI Import](#openapi-import)
- [HAR Replay](#har-replay)
- [Fuzzing](#fuzzing)
- [Benchmarking](#benchmarking)
- [Mock Server](#mock-server)
- [Plugins](#plugins)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)
- [Documentation](#documentation)
- [Contributing](#contributing)

## Features Overview

| Category | Features |
|----------|----------|
| **HTTP Methods** | GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS, and custom methods |
| **Request Data** | Headers (`:`), Query params (`==`), Form data (`=`), JSON fields (`:=`), File uploads (`@`) |
| **Content Types** | JSON (default), Form (`-f`), Multipart (`--multipart`), Raw body (`--raw`) |
| **Authentication** | Basic, Digest, Bearer, AWS SigV4, OAuth 2.0, GCP, Azure |
| **Sessions** | Persistent cookies and headers (`--session`) |
| **Protocols** | HTTP/1.1, HTTP/2, HTTP/3 (QUIC), gRPC, GraphQL, WebSocket |
| **Kubernetes** | Native `k8s://` URLs with automatic port-forwarding |
| **Workflows** | Multi-step API automation with YAML/TOML files, step dependencies, tag filtering |
| **Testing** | Assertions, Fuzzing, Benchmarking |
| **Import/Export** | OpenAPI, HAR files, cURL commands |
| **Output** | Syntax highlighting, JSON formatting, Table/CSV output, Pager support |
| **CI/CD** | JUnit/JSON/TAP reports, JSON Lines logging, response persistence |
| **Mock Server** | Built-in mock server for testing |
| **Plugins** | Extensible plugin ecosystem with hooks |
| **Proxy** | HTTP, HTTPS, SOCKS4, SOCKS5 proxy support |

## Installation

### Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/quicpulse/quicpulse/releases/latest):

### Self-Update

QuicPulse can update itself to the latest version:

```bash
quicpulse --update
```

### Build from Source

```bash
cargo build --release

# Binary will be at ./target/release/quicpulse
```

### Android (Termux)

QuicPulse provides static musl binaries that work on Android via [Termux](https://termux.dev/).

📖 **[Complete Termux Guide](docs/termux.md)** | 🐳 **[Docker Testing](Dockerfile.termux)**

1. **Download the appropriate musl binary for your Android device:**
   - **ARM64 (most modern Android devices):** `quicpulse-linux-arm64-musl.tar.gz`
   - **x86_64 (Android emulators):** `quicpulse-linux-x86_64-musl.tar.gz`

2. **Install in Termux:**
   ```bash
   # Install required tools
   pkg install wget tar

   # Download (replace with appropriate architecture)
   wget https://github.com/quicpulse/quicpulse/releases/latest/download/quicpulse-linux-arm64-musl.tar.gz

   # Extract
   tar -xzf quicpulse-linux-arm64-musl.tar.gz

   # Make executable and move to PATH
   chmod +x quicpulse
   mv quicpulse $PREFIX/bin/
   ```

3. **Verify installation:**
   ```bash
   quicpulse --version
   ```

**Note:** Use the `-musl` suffixed binaries for Android. The standard Linux binaries (with `-gnu` suffix) require glibc and will not work on Android.

#### Troubleshooting HTTPS on Android/Termux

If you encounter SSL/TLS certificate errors when making HTTPS requests, install CA certificates:

```bash
pkg update
pkg install ca-certificates
```

If you still have issues:
1. **Check system time:** Incorrect date/time causes certificate validation failures
2. **Verify certificates installed:** `ls -la $PREFIX/etc/tls/certs/`
3. **Test with HTTP first:** `quicpulse http://httpbin.org/get` to isolate TLS issues

**For complete troubleshooting and advanced setup, see [docs/termux.md](docs/termux.md).**

Additional resources:
- [Termux CA Certificates Issue #1546](https://github.com/termux/termux-packages/issues/1546)
- [Termux TLS Verification Issue #4893](https://github.com/termux/termux-app/issues/4893)

## Quick Start

```bash
# Simple GET request
quicpulse httpbin.org/get

# POST with JSON data
quicpulse POST httpbin.org/post name=John age:=30

# POST with form data
quicpulse -f POST httpbin.org/post name=John email=john@example.com

# Custom headers
quicpulse httpbin.org/headers User-Agent:MyApp/1.0 Accept:application/json

# Query parameters
quicpulse httpbin.org/get search==query page==1
```

## Request Syntax

| Syntax | Description | Example |
|--------|-------------|---------|
| `Header:Value` | HTTP header | `Content-Type:application/json` |
| `Header:` | Empty header value | `Accept-Encoding:` |
| `Header;` | Header from file | `Token@:./token.txt` |
| `param==value` | Query parameter | `search==hello` |
| `field=value` | String data field | `name=John` |
| `field:=value` | JSON data field | `age:=30` `active:=true` |
| `field=@file` | Data from file | `data=@./input.json` |
| `field:=@file` | JSON from file | `config:=@./config.json` |
| `file@path` | File upload | `image@./photo.jpg` |

## Authentication

```bash
# Basic auth
quicpulse -a user:password httpbin.org/basic-auth/user/password

# Bearer token
quicpulse -A bearer -a "token123" httpbin.org/bearer

# Digest auth
quicpulse -A digest -a user:password httpbin.org/digest-auth/auth/user/password

# AWS SigV4
quicpulse -A aws-sigv4 -a "ACCESS_KEY:SECRET_KEY" \
  --aws-region us-east-1 --aws-service s3 \
  https://s3.amazonaws.com/bucket/object

# OAuth 2.0 Client Credentials
quicpulse -A oauth2 -a "client_id:client_secret" \
  --oauth-token-url https://auth.example.com/token \
  https://api.example.com/resource

# GCP Authentication (uses gcloud CLI)
quicpulse -A gcp https://cloud-run-service.run.app/api

# Azure Authentication (uses az CLI)
quicpulse -A azure https://my-app.azurewebsites.net/api
```

## Sessions

```bash
# Create/use a named session
quicpulse --session=mysession POST api.example.com/login username=admin password=secret

# Subsequent requests use the session (cookies persist)
quicpulse --session=mysession api.example.com/profile

# Read-only session (don't save changes)
quicpulse --session-read-only=mysession api.example.com/data
```

## Output Control

```bash
# Headers only
quicpulse -h httpbin.org/get

# Body only
quicpulse -b httpbin.org/get

# Verbose (show request and response)
quicpulse -v httpbin.org/get

# Quiet mode
quicpulse -q httpbin.org/get

# Control what to print: H=request headers, B=request body, h=response headers, b=response body
quicpulse --print=Hh httpbin.org/get

# Pretty printing options
quicpulse --pretty=all httpbin.org/json     # Colors + formatting (forced)
quicpulse --pretty=colors httpbin.org/json  # Colors only (compact JSON)
quicpulse --pretty=format httpbin.org/json  # Formatting only (no colors)
quicpulse --pretty=none httpbin.org/json    # Raw output

# Force disable colors (overrides --pretty)
quicpulse --no-color httpbin.org/json
```

### Pretty Mode Reference

| Mode | Formatting | Colors | Use Case |
|------|------------|--------|----------|
| `--pretty=all` | Indented | Forced on | Pretty output even when piping |
| `--pretty=colors` | Compact | Forced on | Colored compact JSON for CI logs |
| `--pretty=format` | Indented | Off | Readable JSON for saving to files |
| `--pretty=none` | Compact | Off | Machine-parseable raw output |
| *(default)* | Indented | TTY only | Normal interactive use |

**Note:** By default, colors are only applied when output goes to a terminal (TTY). Use `--pretty=all` or `--pretty=colors` to force colors when piping to files or other commands.

## Data Filtering

```bash
# Filter JSON response with jq expression
quicpulse httpbin.org/json --filter='.slideshow.title'

# Nested jq queries
quicpulse api.example.com/users --filter='.[0].address.city'

# Table output for arrays
quicpulse api.example.com/users --table

# CSV export
quicpulse api.example.com/users --csv > users.csv

# Select specific fields for table/CSV
quicpulse api.example.com/users --table --fields=id,name,email
quicpulse api.example.com/users --csv --fields=id,email > emails.csv
```

## Downloads

```bash
# Download mode
quicpulse -d https://example.com/file.zip

# Specify output file
quicpulse -d -o myfile.zip https://example.com/file.zip

# Resume partial download
quicpulse -d -c https://example.com/largefile.zip
```

## Assertions

```bash
# Assert status code
quicpulse --assert-status=200 httpbin.org/get

# Assert response time
quicpulse --assert-time="<500ms" httpbin.org/get

# Assert body content (jq expression)
quicpulse --assert-body='.success == true' api.example.com/health

# Assert header
quicpulse --assert-header="Content-Type:application/json" httpbin.org/json

# Multiple assertions
quicpulse --assert-status=200 --assert-time="<1s" --assert-body='.status == "ok"' api.example.com/health
```

## Proxy & SSL

```bash
# HTTP proxy
quicpulse --proxy=http://proxy:8080 httpbin.org/get

# SOCKS5 proxy
quicpulse --proxy=socks5://localhost:1080 httpbin.org/get

# Dedicated SOCKS proxy option
quicpulse --socks=socks5://localhost:1080 httpbin.org/get

# SOCKS proxy with authentication
quicpulse --socks=socks5://user:pass@localhost:1080 httpbin.org/get

# SOCKS5h for remote DNS resolution (privacy)
quicpulse --socks=socks5h://localhost:9050 httpbin.org/get

# Skip SSL verification
quicpulse --verify=no https://self-signed.example.com

# Custom CA bundle
quicpulse --verify=/path/to/ca-bundle.crt https://internal.example.com

# Client certificate (mutual TLS)
quicpulse --cert=client.pem --cert-key=client-key.pem https://mutual-tls.example.com

# Encrypted client key
quicpulse --cert=client.pem --cert-key=encrypted.key --cert-key-pass=secret https://example.com

# Minimum TLS version
quicpulse --ssl=tls1.2 https://example.com
quicpulse --ssl=tls1.3 https://modern.example.com

# Custom cipher suite
quicpulse --ciphers=ECDHE-RSA-AES256-GCM-SHA384 https://example.com
```

## Timeouts & Retries

```bash
# Connection timeout (seconds)
quicpulse --timeout=30 httpbin.org/delay/5

# Follow redirects (default: enabled)
quicpulse -F httpbin.org/redirect/3

# Limit redirects
quicpulse --max-redirects=5 httpbin.org/redirect/10

# Exit with error on 4xx/5xx status
quicpulse --check-status httpbin.org/status/404
```

---

## Content Processing

```bash
# Compress request body with deflate
quicpulse -x POST httpbin.org/post data=large-content

# Use chunked transfer encoding
quicpulse --chunked POST httpbin.org/post @large-file.json

# Stream response body line by line
quicpulse -S https://stream.example.com/events

# Show all intermediary requests (redirects)
quicpulse --all httpbin.org/redirect/3
```

---

## Advanced Output Options

```bash
# Print only metadata
quicpulse -m httpbin.org/get

# Override response charset
quicpulse --response-charset=utf-8 httpbin.org/encoding/utf8

# Override response MIME type
quicpulse --response-mime=application/json httpbin.org/html

# Custom JSON formatting
quicpulse --format-options="json.indent:4" httpbin.org/json

# Disable key sorting
quicpulse --unsorted httpbin.org/json
```

---

## Protocol Options

```bash
# Force HTTP/3 (QUIC)
quicpulse --http3 https://http3.example.com

# Specify HTTP version
quicpulse --http-version=2 https://example.com

# Keep path as-is (don't normalize)
quicpulse --path-as-is "httpbin.org/anything/../get"

# Set default scheme
quicpulse --default-scheme=https example.com
```

---

## Kubernetes Support

Access services running in Kubernetes clusters using automatic port-forwarding:

```bash
# k8s:// URL format: k8s://service.namespace[:port][/path]
quicpulse k8s://api-server.default:8080/health

# Access monitoring services
quicpulse k8s://grafana.monitoring:3000/api/health

# With query parameters
quicpulse k8s://api.production:8080/users?limit=10

# POST to Kubernetes service
quicpulse POST k8s://backend.staging:8080/api/users name=John
```

QuicPulse automatically:
- Parses the k8s:// URL
- Finds an available local port
- Starts `kubectl port-forward` in the background
- Rewrites the URL to localhost
- Cleans up port-forwards on exit

Requirements:
- `kubectl` must be installed and in PATH
- Valid kubeconfig with cluster access

---

## CI/CD Integration

### CI-Friendly Output Modes

```bash
# JSON Lines log format for log aggregation
quicpulse --run=workflow.yaml --log-format=json

# Disable colors for CI environments
quicpulse --no-color --run=workflow.yaml

# Strict TTY mode (fail if not a terminal)
quicpulse --strict-tty --run=workflow.yaml
```

### JSON Lines Output

When using `--log-format=json`, output is formatted as newline-delimited JSON:

```json
{"timestamp":"2024-01-15T10:30:00Z","step":"login","event":"request_start","method":"POST","url":"https://api.example.com/auth"}
{"timestamp":"2024-01-15T10:30:01Z","step":"login","event":"request_complete","status":200,"duration_ms":234}
```

---

## GraphQL

```bash
# GraphQL query
quicpulse -G POST api.example.com/graphql \
  query='{ users { id name } }'

# GraphQL with variables
quicpulse -G POST api.example.com/graphql \
  query='query GetUser($id: ID!) { user(id: $id) { name } }' \
  variables:='{"id": "123"}'

# GraphQL schema introspection
quicpulse -G --graphql-schema api.example.com/graphql
```

---

## gRPC

QuicPulse provides native gRPC support with dynamic message encoding.

### Basic gRPC Call

```bash
# gRPC call (auto-assigns field numbers alphabetically)
quicpulse --grpc grpc://localhost:50051/mypackage.UserService/GetUser \
  user_id:=123

# With headers (metadata)
quicpulse --grpc grpc://localhost:50051/mypackage.UserService/GetUser \
  user_id:=123 Authorization:"Bearer token"
```

### Using Proto Files (Recommended)

For correct field encoding, provide a `.proto` file:

```bash
# gRPC call with proto schema
quicpulse --grpc --proto=user.proto \
  grpc://localhost:50051/mypackage.UserService/GetUser \
  user_id:=123

# List services from proto file
quicpulse --grpc --proto=user.proto --grpc-list grpc://localhost:50051

# Describe a service
quicpulse --grpc --proto=user.proto --grpc-describe=UserService grpc://localhost:50051
```

### gRPC with TLS

```bash
# Secure gRPC (grpcs://)
quicpulse --grpc grpcs://api.example.com:443/package.Service/Method message:='{"id": 1}'
```

### gRPC Streaming

QuicPulse supports all gRPC streaming modes. Streaming types are auto-detected from the proto file.

#### Server Streaming

Server sends multiple responses for a single request:

```bash
# Server streaming call - outputs NDJSON (one JSON object per line)
quicpulse --grpc --proto=service.proto \
  grpc://localhost:50051/mypackage.EventService/Subscribe \
  topic="news"
```

Output (NDJSON format):
```
{"event":"article_published","id":1}
{"event":"article_updated","id":2}
{"event":"article_deleted","id":3}
```

#### Client Streaming

Client sends multiple requests, server responds once. Provide input as NDJSON via stdin:

```bash
# Client streaming - reads NDJSON from stdin
echo '{"value":1}
{"value":2}
{"value":3}' | quicpulse --grpc --proto=service.proto \
  grpc://localhost:50051/mypackage.AggregateService/Sum
```

#### Bidirectional Streaming

Both client and server stream messages:

```bash
# Bidirectional streaming
echo '{"message":"hello"}
{"message":"world"}' | quicpulse --grpc --proto=service.proto \
  grpc://localhost:50051/mypackage.ChatService/Chat
```

---

## WebSocket

QuicPulse provides native WebSocket support with multiple operation modes.

### Basic WebSocket Connection

```bash
# Connect to WebSocket server (auto-detected from ws:// URL)
quicpulse ws://echo.websocket.org

# Secure WebSocket (wss://)
quicpulse wss://api.example.com/ws

# Explicit WebSocket mode for http:// URLs
quicpulse --ws http://localhost:8080/socket
```

### One-Shot Mode

Send a single message and receive one response:

```bash
# Send text message
quicpulse ws://echo.websocket.org --ws-send "Hello, WebSocket!"

# Send JSON message using request items
quicpulse ws://api.example.com/ws type=subscribe channel=orders

# Send with custom headers
quicpulse ws://api.example.com/ws Authorization:"Bearer token" --ws-send "ping"
```

### Listen Mode

Receive messages continuously:

```bash
# Listen for messages
quicpulse ws://stream.example.com --ws-listen

# Limit number of messages
quicpulse ws://stream.example.com --ws-listen --ws-max-messages 100

# Keep connection alive with pings
quicpulse ws://stream.example.com --ws-listen --ws-ping-interval 30
```

### Interactive Mode

Interactive REPL for bidirectional communication:

```bash
# Start interactive session
quicpulse wss://api.example.com/ws --ws-interactive
```

In interactive mode, type messages to send. Special commands:
- `/quit` or `/q` - Close connection and exit
- `/ping [message]` - Send ping frame
- `/binary <hex|base64> <data>` - Send binary message
- `/close` - Send close frame
- `/help` - Show available commands

### Binary Messages

```bash
# Send hex-encoded binary data
quicpulse ws://example.com/ws --ws-binary hex --ws-send "48656c6c6f"

# Send base64-encoded binary data
quicpulse ws://example.com/ws --ws-binary base64 --ws-send "SGVsbG8="
```

### Subprotocols

```bash
# Request specific subprotocol
quicpulse wss://api.example.com/ws --ws-subprotocol graphql-ws

# Multiple subprotocols (server chooses one)
quicpulse wss://api.example.com/ws --ws-subprotocol "graphql-ws, graphql-transport-ws"
```

### Stdin Input (NDJSON)

Send multiple messages from stdin:

```bash
# Send messages from stdin (newline-delimited JSON)
echo '{"type":"subscribe","channel":"orders"}
{"type":"subscribe","channel":"trades"}' | quicpulse wss://api.example.com/ws
```

### WebSocket Flags Reference

| Flag | Description |
|------|-------------|
| `--ws` | Enable WebSocket mode (auto-detected for ws:// URLs) |
| `--ws-send <MSG>` | Send message and disconnect |
| `--ws-listen` | Listen mode - receive messages only |
| `--ws-interactive` | Interactive REPL mode |
| `--ws-subprotocol <PROTO>` | Request specific subprotocol |
| `--ws-binary <MODE>` | Binary mode: 'hex' or 'base64' |
| `--ws-compress` | Enable permessage-deflate compression |
| `--ws-max-messages <N>` | Maximum messages to receive (0 = unlimited) |
| `--ws-ping-interval <SEC>` | Ping interval in seconds |

---

## Scripting

QuicPulse includes powerful embedded scripting support with **two languages**:

- **[Rune](https://rune-rs.github.io/)** - Default scripting language (Rust-like syntax)
- **JavaScript** - Full JavaScript support via QuickJS (bundled plugin)

**[Complete Scripting Reference →](docs/script.md)**

### Script Language Selection

Scripts are detected automatically by file extension, or you can specify explicitly:

```yaml
# Auto-detected from file extension
pre_script:
  file: ./scripts/setup.js    # JavaScript (.js)

post_script:
  file: ./scripts/validate.rn  # Rune (.rn)

# Explicit type field
script_assert:
  type: javascript
  code: |
    response.status === 200 && json.is_valid(response.body)
```

### Available Modules

Both Rune and JavaScript have access to the same modules:

| Module | Purpose |
|--------|---------|
| `crypto` | SHA256/512, HMAC, UUID, random generation |
| `encoding` | Base64, URL, hex encoding/decoding |
| `json` | JSON parsing, JSONPath queries, manipulation |
| `xml` | XML parsing and conversion |
| `regex` | Pattern matching and replacement |
| `url` | URL parsing and manipulation |
| `date` | Date/time parsing and arithmetic |
| `cookie` | Cookie parsing and building |
| `jwt` | JWT token decoding and inspection |
| `schema` | JSON Schema validation |
| `http` | HTTP status constants and helpers |
| `assert` | Test assertions (eq, gt, status checks) |
| `env` | Environment variables and system info |
| `faker` | 50+ fake data generators (names, emails, etc.) |
| `prompt` | Interactive user input (text, password, confirm) |
| `fs` | Sandboxed file system access |
| `store` | Global workflow state storage |
| `console` | Structured logging and output |
| `system` | System utilities, timing, platform info |

### Rune Example

```rune
// Pre-request: Generate signed request
let timestamp = crypto::timestamp();
let signature = crypto::hmac_sha256(vars["secret"], `${timestamp}:${vars["body"]}`);
vars["signature"] = signature;
vars["timestamp"] = timestamp;
```

### JavaScript Example

```javascript
// Pre-request: Generate signed request
const timestamp = crypto.timestamp();
const signature = crypto.hmac_sha256(store.get("secret"), `${timestamp}:${request.body}`);
request.headers["X-Signature"] = signature;
request.headers["X-Timestamp"] = timestamp;
```

```javascript
// Post-response: Extract and validate
const body = JSON.parse(response.body);
assert.eq(response.status, 200);
assert.is_true(body.user !== undefined);
store.set("user_id", body.user.id);
```

---

## Workflows

Automate multi-step API interactions with YAML or TOML workflow files.

**[Complete Workflow Reference →](docs/workflow.md)**

### Features

- **Variables & Templating** - `{{variable}}` syntax with Tera filters
- **Magic Values** - Dynamic values: `{uuid}`, `{email}`, `{random_string:10}`
- **Environments** - Environment-specific configuration (dev, staging, prod)
- **Extraction & Chaining** - Extract values from responses for subsequent requests
- **Assertions** - Validate status, headers, latency, body content
- **Scripting** - Pre-request, post-response, and assertion scripts (Rune)
- **Protocols** - HTTP/1.1, HTTP/2, HTTP/3, GraphQL, gRPC, WebSocket
- **Sessions** - Cookie and state persistence across steps
- **Security Testing** - Fuzzing and vulnerability scanning
- **Performance** - Benchmarking and load testing
- **File Operations** - Downloads, chunked uploads, compression
- **Integrations** - HAR replay, OpenAPI import, plugins
- **Output Control** - Filtering, saving responses, curl generation
- **CI/CD Reports** - JUnit XML, JSON, TAP output formats

### Quick Example

```yaml
name: User API Test
base_url: https://api.example.com

steps:
  - name: Create User
    method: POST
    url: /users
    body:
      email: "{email}"
      name: "{full_name}"
    extract:
      user_id: body.id
    assert:
      status: 201

  - name: Get User
    method: GET
    url: /users/{{ user_id }}
    assert:
      status: 200
```

### Running Workflows

```bash
quicpulse --run=workflow.yaml                    # Basic run
quicpulse --run=workflow.yaml --env=staging      # With environment
quicpulse --run=workflow.yaml --var=key=value    # Override variable
quicpulse --run=workflow.yaml --report-junit=results.xml  # CI/CD report
```

### Step Filtering

```bash
# Run only steps with specific tags
quicpulse --run=workflow.yaml --tags=smoke,auth

# Include/exclude specific steps
quicpulse --run=workflow.yaml --include=login,logout
quicpulse --run=workflow.yaml --exclude=cleanup

# Save responses to directory
quicpulse --run=workflow.yaml --save-responses=./responses
```

---

## Dynamic Magic Values

Generate dynamic data in requests using magic value placeholders:

```bash
# UUID generation
quicpulse POST api.example.com/items id:={uuid}

# Timestamps
quicpulse POST api.example.com/events timestamp:={timestamp} created_at="{now}"

# Random data
quicpulse POST api.example.com/users \
  id:={uuid} \
  username="{random_string:8}" \
  age:={random_int:18:65} \
  score:={random_float}
```

### Available Magic Values

| Magic Value | Description | Example Output |
|------------|-------------|----------------|
| `{uuid}` or `{uuid4}` | UUID v4 | `550e8400-e29b-41d4-a716-446655440000` |
| `{uuid7}` | UUID v7 (time-ordered) | `018f3b3c-...` |
| `{timestamp}` | Unix timestamp (seconds) | `1699876543` |
| `{timestamp_ms}` | Unix timestamp (milliseconds) | `1699876543123` |
| `{now}` | ISO 8601 datetime | `2024-01-15T10:30:00Z` |
| `{now:FORMAT}` | Custom formatted datetime | `{now:%Y-%m-%d}` → `2024-01-15` |
| `{now_local}` | Local time ISO 8601 | `2024-01-15T10:30:00-08:00` |
| `{date}` | Today's date (UTC) | `2024-01-15` |
| `{time}` | Current time (UTC) | `10:30:00` |
| `{random_int}` or `{rand}` | Random integer | `42` |
| `{random_int:max}` | Random integer 0 to max | `{random_int:100}` → `42` |
| `{random_int:min:max}` | Random integer in range | `{random_int:1:10}` → `7` |
| `{random_float}` or `{randf}` | Random float 0.0-1.0 | `0.723456` |
| `{random_float:max}` | Random float 0 to max | `{random_float:100}` |
| `{random_string:N}` or `{rands:N}` | Random alphanumeric | `{random_string:8}` → `a8Bf2kLm` |
| `{random_hex:N}` or `{hex:N}` | Random hex string | `{random_hex:16}` → `4f8a2bc9d1e3f7a2` |
| `{random_bytes:N}` or `{bytes:N}` | Random base64 bytes | `{random_bytes:16}` → `SGVsbG8gV29ybGQ=` |
| `{random_bool}` or `{bool}` | Random true/false | `true` |
| `{email}` | Random email | `abc12345@example.com` |
| `{email:domain}` | Email with custom domain | `{email:test.org}` → `user@test.org` |
| `{first_name}` | Random first name | `Emma` |
| `{last_name}` | Random last name | `Johnson` |
| `{full_name}` | Random full name | `Emma Johnson` |
| `{lorem:N}` | Lorem ipsum words | `{lorem:5}` → `lorem ipsum dolor sit amet` |
| `{pick:a,b,c}` | Random choice from list | `{pick:red,green,blue}` → `green` |
| `{seq}` | Sequential counter | `1`, `2`, `3`... |
| `{seq:start}` | Counter starting at N | `{seq:100}` → `100`, `101`... |
| `{seq_reset}` | Reset sequence to 0 | `0` |
| `{env:VAR}` | Environment variable | `{env:API_KEY}` → value of $API_KEY |

---

## OpenAPI Import

Generate test workflows from OpenAPI/Swagger specifications:

```bash
# Generate workflow from OpenAPI spec
quicpulse --import-openapi=api-spec.yaml --generate-workflow=tests.yaml

# Generate with custom base URL
quicpulse --import-openapi=api-spec.yaml --generate-workflow=tests.yaml \
  --openapi-base-url=https://staging.api.example.com

# Filter by tags
quicpulse --import-openapi=api-spec.yaml --generate-workflow=tests.yaml \
  --openapi-tags=users,orders

# Include deprecated endpoints
quicpulse --import-openapi=api-spec.yaml --generate-workflow=tests.yaml \
  --openapi-include-deprecated

# List available endpoints
quicpulse --import-openapi=api-spec.yaml --openapi-list
```

### Generated Workflow Features

- CRUD operation ordering (POST → GET → PUT → PATCH → DELETE)
- Automatic ID extraction and chaining
- Schema-based magic value generation
- Latency and status assertions
- Environment variable support

---

## HAR Replay

Import and replay browser HAR (HTTP Archive) files:

```bash
# List requests in HAR file
quicpulse --import-har=recording.har --har-list

# Replay all requests
quicpulse --import-har=recording.har

# Replay specific requests by index (1-based)
quicpulse --import-har=recording.har --har-index=1 --har-index=3 --har-index=5

# Filter by URL pattern (regex)
quicpulse --import-har=recording.har --har-filter='api\.example\.com/users'

# Interactive selection
quicpulse --import-har=recording.har --har-interactive

# Add delay between requests
quicpulse --import-har=recording.har --har-delay=500ms

# Combine filters
quicpulse --import-har=recording.har --har-filter='api' --har-delay=100ms
```

---

## Security Fuzzing

Test APIs for common vulnerabilities:

```bash
# Fuzz all data fields
quicpulse --fuzz POST api.example.com/login username=test password=test

# Fuzz specific fields
quicpulse --fuzz --fuzz-field=username POST api.example.com/login username=test password=test

# Specific vulnerability categories
quicpulse --fuzz --fuzz-category=sql --fuzz-category=xss POST api.example.com/search query=test

# Higher risk payloads (1-5)
quicpulse --fuzz --fuzz-risk=3 POST api.example.com/login username=test

# Only show anomalies (5xx errors, timeouts)
quicpulse --fuzz --fuzz-anomalies-only POST api.example.com/api

# Stop on first anomaly
quicpulse --fuzz --fuzz-stop-on-anomaly POST api.example.com/api
```

### Fuzz Categories

| Category | Description |
|----------|-------------|
| `sql` | SQL injection payloads |
| `xss` | Cross-site scripting payloads |
| `cmd` | Command injection payloads |
| `path` | Path traversal payloads |
| `boundary` | Boundary testing (empty, long strings) |
| `type` | Type confusion (arrays, objects, null) |
| `format` | Format string attacks |
| `int` | Integer overflow/underflow |
| `unicode` | Unicode edge cases |
| `nosql` | NoSQL injection payloads |

---

## Benchmarking

Load test APIs with concurrent requests:

```bash
# Simple benchmark (100 requests, 10 concurrent)
quicpulse --bench --requests=100 --concurrency=10 httpbin.org/get

# Short form
quicpulse --bench -n 100 -c 10 httpbin.org/get

# Benchmark POST with data
quicpulse --bench -n 500 -c 20 POST httpbin.org/post name=test
```

### Benchmark Output

QuicPulse displays detailed statistics including:
- Total requests and success rate
- Response time: min, max, mean, median
- Percentiles: p50, p90, p95, p99
- Requests per second (throughput)
- Transfer rate

---

## Response Pager

Automatically page long responses through your preferred pager:

```bash
# Enable pager for long output
quicpulse --pager httpbin.org/json

# Force pager even for short output
quicpulse --pager --pager-force httpbin.org/get

# Use a specific pager command
quicpulse --pager --pager-cmd="less -R" httpbin.org/json
```

Environment variables:
- `PAGER` - Default pager program (default: `less -FRX`)
- `QUICPULSE_PAGER` - Override for QuicPulse

**[Complete Pager Reference ->](docs/pager.md)**

---

## Built-in Mock Server

Start a local HTTP mock server for testing:

```bash
# Start mock server on default port 8000
quicpulse --mock

# Custom port
quicpulse --mock --mock-port=3000

# Define routes inline
quicpulse --mock --mock-route="GET /api/users -> 200 [{\"id\":1}]"

# Load routes from config file
quicpulse --mock --mock-config=mock-config.yaml

# Enable CORS
quicpulse --mock --mock-cors

# Add artificial latency
quicpulse --mock --mock-latency=100
```

### Route Configuration (YAML)

```yaml
# mock-config.yaml
port: 8000
cors: true

routes:
  - method: GET
    path: /api/users/:id
    status: 200
    body:
      id: ":id"
      name: "User :id"

  - method: POST
    path: /api/users
    status: 201
    body:
      created: true
```

**[Complete Mock Server Reference ->](docs/mock-server.md)**

---

## Plugin Ecosystem

Extend QuicPulse with plugins:

```bash
# List installed plugins
quicpulse --plugin-list

# Search for plugins
quicpulse --plugin-search auth

# Install a plugin
quicpulse --plugin-install jwt-auth

# Uninstall a plugin
quicpulse --plugin-uninstall jwt-auth

# Update all plugins
quicpulse --plugin-update

# Use specific plugins for a request
quicpulse --plugins "jwt-auth,request-logger" api.example.com/endpoint
```

### Plugin Hooks

Plugins can intercept the request/response lifecycle:

| Hook | Description |
|------|-------------|
| `pre_request` | Modify request before sending |
| `post_response` | Process response after receiving |
| `on_error` | Handle request errors |
| `auth` | Custom authentication |
| `format` | Custom output formatting |
| `validate` | Custom response validation |

**[Complete Plugin Reference ->](docs/plugins.md)**

---

## Configuration

QuicPulse reads configuration from `~/.config/quicpulse/config.toml`:

```toml
# Default options applied to all requests
[defaults]
timeout = 30
verify = true
pretty = "all"
style = "monokai"

# Default headers
[defaults.headers]
User-Agent = "QuicPulse/1.0"

# Environment-specific settings
[environments.production]
base_url = "https://api.example.com"

[environments.staging]
base_url = "https://staging.api.example.com"
```

---

## Developer Tools

```bash
# Generate curl command
quicpulse --curl POST api.example.com/users name=John

# Offline mode (build request without sending)
quicpulse --offline POST api.example.com/users name=John

# Generate shell completions
quicpulse --generate-completions=bash > quicpulse.bash
quicpulse --generate-completions=zsh > _quicpulse
quicpulse --generate-completions=fish > quicpulse.fish
quicpulse --generate-completions=powershell > quicpulse.ps1

# Load environment variables from file
quicpulse --env-file=.env api.example.com/users

# Disable auto-loading of .env
quicpulse --no-env api.example.com/users

# Don't read from stdin (useful in scripts)
quicpulse -I api.example.com/users
```

## Troubleshooting

```bash
# Show traceback on error
quicpulse --traceback api.example.com/error

# Debug mode (verbose + traceback)
quicpulse --debug api.example.com/users

# Skip .netrc credentials
quicpulse --ignore-netrc api.example.com/users
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Request error |
| 3 | Server error (5xx) |
| 4 | Client error (4xx) |
| 5 | Assertion failed |
| 130 | Interrupted (Ctrl+C) |

---

## Documentation

For detailed documentation on all features, see the [docs/](docs/) folder:

### Reference

| Document | Description |
|----------|-------------|
| [CLI Reference](docs/cli-reference.md) | Complete reference for all 170+ CLI flags |
| [Configuration](docs/configuration.md) | Config file and environment variables |
| [Architecture](docs/architecture.md) | Codebase structure and module overview |

### Core Features

| Document | Description |
|----------|-------------|
| [HTTP Client](docs/http-client.md) | HTTP/1.1, HTTP/2, HTTP/3, TLS, proxies |
| [Authentication](docs/authentication.md) | All authentication methods detailed |
| [Assertions](docs/assertions.md) | Response testing and CI/CD integration |
| [Downloads & Uploads](docs/downloads-uploads.md) | File transfers, resume, multipart |
| [Data Filtering](docs/filtering.md) | JQ expressions, table, CSV output |

### Protocols

| Document | Description |
|----------|-------------|
| [GraphQL](docs/workflow-graphql.md) | GraphQL queries and introspection |
| [gRPC](docs/workflow-grpc.md) | gRPC calls with reflection |
| [WebSocket](docs/workflow-websocket.md) | WebSocket connections |

### Workflows & Automation

| Document | Description |
|----------|-------------|
| [Workflow Guide](docs/workflow.md) | Multi-step API automation |
| [Scripting](docs/script.md) | Rune and JavaScript scripting |
| [Sessions](docs/workflow-sessions.md) | Cookie and state persistence |

### Integrations

| Document | Description |
|----------|-------------|
| [Kubernetes](docs/kubernetes.md) | Native k8s:// URL support |
| [OpenAPI](docs/workflow-openapi.md) | Generate workflows from specs |
| [HAR Replay](docs/workflow-har.md) | Replay browser recordings |
| [Termux (Android)](docs/termux.md) | Running on Android with Termux |

### Developer Tools

| Document | Description |
|----------|-------------|
| [Mock Server](docs/mock-server.md) | Built-in HTTP mock server |
| [Plugins](docs/plugins.md) | Plugin ecosystem |
| [Pager](docs/pager.md) | Output paging |

---

## Contributing

We welcome contributions to QuicPulse! To contribute, you must certify that you have the right to submit your contribution and agree to license it under the project's dual MIT/Apache-2.0 license.

### Developer Certificate of Origin (DCO)

QuicPulse uses the [Developer Certificate of Origin (DCO)](https://developercertificate.org/) process. This is a lightweight way for contributors to certify that they wrote or otherwise have the right to submit code or documentation to an open source project.

#### Inbound = Outbound License

All contributions to QuicPulse are made under the same dual MIT/Apache-2.0 license as the project itself. By signing off on your commits, you agree that your contributions will be licensed under these same terms, with no additional restrictions.

#### How to Sign Off Commits

Contributors sign-off that they adhere to these requirements by adding a Signed-off-by line to commit messages.

```
This is my commit message

Signed-off-by: Random J Developer <random@developer.example.org>
```

Git even has a `-s` command line option to append this automatically to your commit message:

```bash
$ git commit -s -m 'This is my commit message'
```

#### Signing Off Previous Commits

If you forgot to sign off your commits:

**For a single commit:**
```bash
git commit --amend --signoff
git push --force-with-lease
```

**For multiple commits (rebase last N commits):**
```bash
git rebase --signoff HEAD~N
git push --force-with-lease
```

#### What the Sign-Off Means

By signing off, you certify that:
1. You wrote the contribution, or have the right to submit it under an open source license
2. You agree to license your contribution under the project's MIT OR Apache-2.0 dual license
3. You understand that your contribution is public and may be redistributed

For the full text of the Developer Certificate of Origin, see https://developercertificate.org/

#### Configure Git Identity

Make sure your git identity is configured correctly before committing:

```bash
git config --global user.name "Your Name"
git config --global user.email "your.email@example.com"
```

### Contribution Guidelines

- Ensure your code follows the project's coding standards
- Include tests for new features when applicable
- Update documentation as needed
- All commits must include DCO sign-off
- Pull requests without properly signed commits cannot be merged

---

## License

QuicPulse is dual-licensed under either:

- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
