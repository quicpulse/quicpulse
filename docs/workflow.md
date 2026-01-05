# Workflow Documentation

Complete reference for HTTP workflow files - multi-step API testing, automation, and orchestration.

## Table of Contents

- [Overview](#overview)
- [File Format](#file-format)
- [Workflow Structure](#workflow-structure)
- [Step Configuration](#step-configuration)
- [Variables and Templating](#variables-and-templating)
- [Environments](#environments)
- [Authentication](#authentication)
- [Extraction and Chaining](#extraction-and-chaining)
- [Assertions](#assertions)
- [Scripting](#scripting)
- [GraphQL Support](#graphql-support)
- [gRPC Support](#grpc-support)
- [WebSocket Support](#websocket-support)
- [Execution Control](#execution-control)
- [CLI Reference](#cli-reference)
- [Report Formats](#report-formats)
- [Complete Examples](#complete-examples)

---

## Overview

Workflows allow you to define multi-step HTTP request sequences with:

- **Variable extraction** - Extract values from responses for use in subsequent requests
- **Assertions** - Validate response status, body, headers, and latency
- **Scripting** - Custom logic using Rune scripts (pre-request, post-response, assertions)
- **Environments** - Environment-specific configuration (dev, staging, production)
- **Conditional execution** - Skip steps based on conditions
- **Retry logic** - Automatic retries with configurable backoff
- **Multiple protocols** - HTTP/1.1, HTTP/2, HTTP/3, GraphQL, gRPC

---

## File Format

Workflows support both YAML (`.yaml`, `.yml`) and TOML (`.toml`) formats.

### YAML Format

```yaml
name: My Workflow
description: Description of the workflow
base_url: https://api.example.com

variables:
  api_key: default_key
  timeout: 30

steps:
  - name: First Step
    method: GET
    url: /endpoint
```

### TOML Format

```toml
name = "My Workflow"
description = "Description of the workflow"
base_url = "https://api.example.com"

[variables]
api_key = "default_key"
timeout = 30

[[steps]]
name = "First Step"
method = "GET"
url = "/endpoint"
```

---

## Workflow Structure

### Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Workflow name (displayed in output and reports) |
| `description` | string | No | Human-readable description |
| `base_url` | string | No | Base URL prepended to relative step URLs |
| `variables` | object | No | Default variables available to all steps |
| `environments` | object | No | Environment-specific variable overrides |
| `headers` | object | No | Default headers applied to all steps |
| `steps` | array | Yes | List of workflow steps to execute |

### Complete Example

```yaml
name: E-Commerce API Test Suite
description: Full test coverage for order lifecycle

base_url: https://api.shop.example.com/v2

variables:
  api_version: v2
  default_currency: USD
  page_size: 20

environments:
  dev:
    base_url: https://dev-api.shop.example.com/v2
    api_key: dev_key_123
  staging:
    base_url: https://staging-api.shop.example.com/v2
    api_key: staging_key_456
  production:
    base_url: https://api.shop.example.com/v2
    api_key: prod_key_789

headers:
  Accept: application/json
  X-API-Version: "{{ api_version }}"
  X-Request-ID: "{uuid}"

steps:
  - name: Health Check
    method: GET
    url: /health
    assert:
      status: 200
```

---

## Step Configuration

Each step in a workflow represents a single HTTP request.

### All Step Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | - | Step name (required, used in output/reports) |
| `method` | string | GET | HTTP method (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS) |
| `url` | string | - | Request URL (required, can be relative if base_url set) |
| `tags` | array | - | Tags for filtering (e.g., `["smoke", "auth"]`) |
| `depends_on` | array | - | Step names this step depends on |
| `query` | object | - | Query string parameters |
| `headers` | object | - | Request headers (merged with workflow headers) |
| `body` | string | - | Request body content |
| `raw` | bool | false | Send body without processing |
| `form` | object | - | URL-encoded form data |
| `multipart` | array | - | Multipart form data with file uploads |
| `auth` | object | - | Authentication configuration |
| `extract` | object | - | Extract values from response |
| `assert` | object | - | Response assertions |
| `skip_if` | string | - | Condition to skip this step |
| `delay` | integer | 0 | Delay before request (milliseconds) |
| `timeout` | integer | 30000 | Request timeout (milliseconds) |
| `retries` | integer | 0 | Number of retry attempts on failure |
| `retry_delay` | integer | 1000 | Delay between retries (milliseconds) |
| `proxy` | string | - | Proxy URL for this step |
| `verify` | bool | true | Verify SSL certificates |
| `cert` | string | - | Path to client certificate |
| `key` | string | - | Path to client certificate key |
| `http_version` | string | - | Force HTTP version (1.0, 1.1, 2, 3) |
| `graphql` | object | - | GraphQL configuration |
| `grpc` | object | - | gRPC configuration |
| `pre_script` | object | - | Script to run before request |
| `post_script` | object | - | Script to run after response |
| `script_assert` | object | - | Script-based assertions |

### Method and URL

```yaml
steps:
  # Relative URL (uses base_url)
  - name: Get Users
    method: GET
    url: /users

  # Absolute URL (ignores base_url)
  - name: External API
    method: POST
    url: https://external-api.com/webhook

  # URL with path parameters (using variables)
  - name: Get User By ID
    method: GET
    url: /users/{{ user_id }}
```

### Tags and Filtering

Tag steps for selective execution:

```yaml
steps:
  - name: Quick Health Check
    url: /health
    tags:
      - smoke
      - quick

  - name: Full Authentication Test
    method: POST
    url: /auth/login
    tags:
      - auth
      - full
    body: '{"user": "test", "pass": "secret"}'

  - name: Cleanup
    url: /cleanup
    tags:
      - cleanup
```

Run tagged steps:

```bash
# Run only smoke tests
quicpulse --run workflow.yaml --tags=smoke

# Run multiple tag groups
quicpulse --run workflow.yaml --tags=smoke,auth

# Exclude cleanup steps
quicpulse --run workflow.yaml --exclude=cleanup
```

### Step Dependencies

Define explicit dependencies between steps with `depends_on`:

```yaml
steps:
  - name: Create User
    method: POST
    url: /users
    extract:
      user_id: body.id

  - name: Create Profile
    method: POST
    url: /profiles
    depends_on:
      - Create User
    body: '{"user_id": "{{ user_id }}"}'

  - name: Create Settings
    method: POST
    url: /settings
    depends_on:
      - Create User
    body: '{"user_id": "{{ user_id }}"}'

  - name: Send Welcome Email
    method: POST
    url: /emails/welcome
    depends_on:
      - Create Profile
      - Create Settings
    body: '{"user_id": "{{ user_id }}"}'
```

When `depends_on` is specified:
- Steps are reordered using topological sort
- Steps with no dependencies can run in parallel (future)
- Circular dependencies are detected and reported as errors
- If a dependency fails, dependent steps are skipped

### Query Parameters

```yaml
steps:
  - name: Search with Filters
    method: GET
    url: /search
    query:
      q: "search term"
      page: 1
      limit: "{{ page_size }}"
      sort: "created_at:desc"
      filter: active
```

Generates: `/search?q=search+term&page=1&limit=20&sort=created_at%3Adesc&filter=active`

### Headers

```yaml
steps:
  - name: Authenticated Request
    method: GET
    url: /protected
    headers:
      Authorization: "Bearer {{ access_token }}"
      Content-Type: application/json
      X-Custom-Header: custom-value
      X-Request-ID: "{uuid}"  # Magic value
      X-Timestamp: "{timestamp}"  # Magic value
```

### Request Body

#### JSON Body

```yaml
steps:
  - name: Create User
    method: POST
    url: /users
    headers:
      Content-Type: application/json
    body: |
      {
        "name": "{{ user_name }}",
        "email": "{{ user_email }}",
        "role": "user",
        "created_at": "{now}"
      }
```

#### Raw Body

```yaml
steps:
  - name: Send Raw XML
    method: POST
    url: /xml-endpoint
    raw: true
    headers:
      Content-Type: application/xml
    body: |
      <?xml version="1.0"?>
      <request>
        <user>{{ username }}</user>
      </request>
```

### Form Data

#### URL-Encoded Form

```yaml
steps:
  - name: Login Form
    method: POST
    url: /login
    form:
      username: "{{ username }}"
      password: "{{ password }}"
      remember_me: "true"
```

#### Multipart Form (File Uploads)

```yaml
steps:
  - name: Upload Document
    method: POST
    url: /upload
    multipart:
      - name: file
        filename: document.pdf
        path: /path/to/document.pdf
        content_type: application/pdf

      - name: metadata
        value: '{"type": "invoice", "date": "{date}"}'
        content_type: application/json

      - name: description
        value: "Uploaded at {now}"
```

Multipart Field Options:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Form field name (required) |
| `value` | string | Text value (for non-file fields) |
| `path` | string | Path to file (for file uploads) |
| `filename` | string | Override filename in upload |
| `content_type` | string | MIME type of the field |

---

## Variables and Templating

### Variable Sources

Variables can come from multiple sources (in order of precedence):

1. **Extracted values** - From previous step responses
2. **CLI variables** - `--var key=value`
3. **Environment variables** - Selected with `--env`
4. **Workflow defaults** - `variables` section
5. **Magic values** - Dynamic runtime values

### Template Syntax

Uses [Tera](https://keats.github.io/tera/) templating (Jinja2-like):

```yaml
variables:
  base_path: /api/v2
  default_limit: 25

steps:
  - name: Templated Request
    url: "{{ base_path }}/users"
    query:
      limit: "{{ default_limit }}"
      search: "{{ search_term | default(value='') }}"
    headers:
      Authorization: "Bearer {{ token }}"
    body: |
      {
        "id": {{ user_id }},
        "name": "{{ name | upper }}",
        "email": "{{ email | lower }}",
        "tags": {{ tags | json_encode() }}
      }
```

### Tera Filters

| Filter | Description | Example |
|--------|-------------|---------|
| `upper` | Uppercase | `{{ name \| upper }}` |
| `lower` | Lowercase | `{{ email \| lower }}` |
| `trim` | Trim whitespace | `{{ input \| trim }}` |
| `default` | Default value | `{{ var \| default(value="none") }}` |
| `json_encode` | JSON encode | `{{ obj \| json_encode() }}` |
| `urlencode` | URL encode | `{{ path \| urlencode }}` |
| `length` | Get length | `{{ items \| length }}` |
| `first` | First element | `{{ items \| first }}` |
| `last` | Last element | `{{ items \| last }}` |
| `join` | Join array | `{{ items \| join(sep=",") }}` |
| `replace` | Replace string | `{{ s \| replace(from="a", to="b") }}` |
| `truncate` | Truncate string | `{{ s \| truncate(length=20) }}` |

### Magic Values

Dynamic values expanded at runtime:

| Value | Description | Example Output |
|-------|-------------|----------------|
| `{uuid}` | UUID v4 | `550e8400-e29b-41d4-a716-446655440000` |
| `{uuid7}` | UUID v7 (time-ordered) | `01890a5d-ac96-7...` |
| `{now}` | ISO 8601 timestamp | `2024-01-15T10:30:00Z` |
| `{now:FORMAT}` | Formatted timestamp | `{now:%Y-%m-%d}` → `2024-01-15` |
| `{now_local}` | Local timestamp | `2024-01-15T10:30:00-05:00` |
| `{timestamp}` | Unix timestamp (seconds) | `1705315800` |
| `{timestamp_ms}` | Unix timestamp (milliseconds) | `1705315800000` |
| `{date}` | Current date | `2024-01-15` |
| `{time}` | Current time | `10:30:00` |
| `{random_int}` | Random integer (0-100) | `42` |
| `{random_int:MAX}` | Random integer (0-MAX) | `{random_int:1000}` → `573` |
| `{random_int:MIN:MAX}` | Random integer (MIN-MAX) | `{random_int:10:20}` → `15` |
| `{random_float}` | Random float (0.0-1.0) | `0.7342` |
| `{random_string:N}` | Random alphanumeric | `{random_string:8}` → `aB3xK9mZ` |
| `{random_hex:N}` | Random hex | `{random_hex:16}` → `a3f2b1c9...` |
| `{random_bytes:N}` | Random bytes (base64) | `{random_bytes:32}` |
| `{random_bool}` | Random boolean | `true` or `false` |
| `{env:VAR}` | Environment variable | `{env:API_KEY}` |
| `{pick:a,b,c}` | Random choice | `{pick:red,green,blue}` → `green` |
| `{seq}` | Sequential counter | `1`, `2`, `3`... |
| `{seq:START}` | Sequential from START | `{seq:100}` → `100`, `101`... |
| `{email}` | Random email | `user_abc123@example.com` |
| `{email:DOMAIN}` | Random email at domain | `{email:test.com}` → `user_x@test.com` |
| `{first_name}` | Random first name | `John` |
| `{last_name}` | Random last name | `Smith` |
| `{full_name}` | Random full name | `John Smith` |
| `{lorem:N}` | Lorem ipsum words | `{lorem:5}` → `Lorem ipsum dolor sit amet` |

Magic values can be used in:
- URLs: `/users/{uuid}`
- Headers: `X-Request-ID: {uuid}`
- Query parameters: `timestamp={timestamp}`
- Body content: `"id": "{uuid}"`

---

## Environments

Define environment-specific configurations:

```yaml
name: API Tests
base_url: https://api.example.com

variables:
  timeout: 30
  retries: 3

environments:
  development:
    base_url: http://localhost:3000
    api_key: dev_key
    timeout: 60
    debug: true

  staging:
    base_url: https://staging-api.example.com
    api_key: staging_key
    timeout: 30

  production:
    base_url: https://api.example.com
    api_key: prod_key
    timeout: 10
    retries: 5

steps:
  - name: Health Check
    url: /health
    headers:
      X-API-Key: "{{ api_key }}"
    timeout: "{{ timeout }}000"  # Convert to milliseconds
```

Run with specific environment:

```bash
# Uses development settings
quicpulse --run workflow.yaml --env development

# Uses production settings
quicpulse --run workflow.yaml --env production
```

---

## Authentication

### Basic Authentication

```yaml
steps:
  - name: Basic Auth Request
    url: /protected
    auth:
      type: basic
      username: myuser
      password: mypassword
```

### Bearer Token

```yaml
steps:
  - name: Bearer Auth Request
    url: /api/resource
    auth:
      type: bearer
      token: "{{ access_token }}"
```

### Digest Authentication

```yaml
steps:
  - name: Digest Auth Request
    url: /secure
    auth:
      type: digest
      username: myuser
      password: mypassword
```

### OAuth2 Flow Example

```yaml
name: OAuth2 Workflow

variables:
  client_id: my_client
  client_secret: secret123
  token_url: https://auth.example.com/oauth/token

steps:
  - name: Get Access Token
    method: POST
    url: "{{ token_url }}"
    form:
      grant_type: client_credentials
      client_id: "{{ client_id }}"
      client_secret: "{{ client_secret }}"
    extract:
      access_token: body.access_token
      expires_in: body.expires_in
    assert:
      status: 200

  - name: Use Token
    url: /api/protected
    auth:
      type: bearer
      token: "{{ access_token }}"
    assert:
      status: 200
```

---

## Extraction and Chaining

Extract values from responses for use in subsequent steps.

### Extraction Syntax

```yaml
extract:
  variable_name: source.path
```

### Extraction Sources

| Source | Description | Example |
|--------|-------------|---------|
| `body` | Response body (JSON) | `body.user.id` |
| `body_raw` | Raw response body | `body_raw` |
| `header` | Response header | `header.X-Request-ID` |
| `headers` | All headers (JSON) | `headers` |
| `status` | HTTP status code | `status` |
| `latency` | Response time (ms) | `latency` |

### JSON Path Extraction

```yaml
steps:
  - name: Create User
    method: POST
    url: /users
    body: '{"name": "John", "email": "john@example.com"}'
    extract:
      # Simple path
      user_id: body.id

      # Nested path
      user_name: body.data.user.name

      # Array access
      first_item: body.items[0]
      last_item: body.items[-1]

      # Array length (via body access)
      item_count: body.total

      # Headers
      request_id: header.X-Request-ID
      content_type: header.Content-Type

      # Status
      response_status: status

      # Full body
      full_response: body_raw

  - name: Get Created User
    method: GET
    url: /users/{{ user_id }}
    headers:
      X-Correlation-ID: "{{ request_id }}"
    assert:
      status: 200
      body:
        - path: name
          equals: John
```

### Chaining Example

```yaml
name: Order Flow

steps:
  - name: Create Customer
    method: POST
    url: /customers
    body: '{"name": "Alice"}'
    extract:
      customer_id: body.id

  - name: Create Product
    method: POST
    url: /products
    body: '{"name": "Widget", "price": 29.99}'
    extract:
      product_id: body.id
      product_price: body.price

  - name: Create Order
    method: POST
    url: /orders
    body: |
      {
        "customer_id": "{{ customer_id }}",
        "items": [
          {"product_id": "{{ product_id }}", "quantity": 2}
        ],
        "total": {{ product_price * 2 }}
      }
    extract:
      order_id: body.id
      order_status: body.status

  - name: Confirm Order
    method: POST
    url: /orders/{{ order_id }}/confirm
    assert:
      status: 200
      body:
        - path: status
          equals: confirmed
```

---

## Assertions

Validate responses with declarative assertions.

### Status Assertions

```yaml
assert:
  # Exact match
  status: 200

  # Range (200-299)
  status: 2xx

  # Multiple valid statuses (in YAML list form)
  # status: [200, 201, 204]
```

### Latency Assertions

```yaml
assert:
  # Maximum response time in milliseconds
  latency: 500
```

### Body Assertions

```yaml
assert:
  body:
    # Equals check
    - path: status
      equals: success

    # Contains check
    - path: message
      contains: "created successfully"

    # Regex match
    - path: email
      matches: "^[a-z]+@example\\.com$"

    # Type checking
    - path: count
      type: number

    # Existence check
    - path: data.user.id
      exists: true

    # Null check
    - path: deleted_at
      is_null: true

    # Not null
    - path: created_at
      is_null: false

    # Numeric comparisons
    - path: total
      greater_than: 0

    - path: items
      less_than: 100

    # Length check (for arrays/strings)
    - path: items
      length: 5

    # JSON value comparison
    - path: config
      equals_json: '{"enabled": true, "mode": "auto"}'
```

### Header Assertions

```yaml
assert:
  headers:
    - name: Content-Type
      contains: application/json

    - name: X-RateLimit-Remaining
      exists: true

    - name: Cache-Control
      equals: "no-cache"

    - name: X-Request-ID
      matches: "^[a-f0-9-]{36}$"
```

### Complete Assertion Example

```yaml
steps:
  - name: Create Order
    method: POST
    url: /orders
    body: '{"product": "widget", "quantity": 5}'
    assert:
      status: 201
      latency: 1000
      headers:
        - name: Content-Type
          contains: json
        - name: Location
          matches: "^/orders/[0-9]+$"
      body:
        - path: id
          exists: true
          type: number
        - path: status
          equals: pending
        - path: items
          length: 1
        - path: total
          greater_than: 0
        - path: created_at
          is_null: false
```

---

## Scripting

Execute custom Rune scripts for dynamic behavior.

### Pre-Request Scripts

Run before sending the request:

```yaml
steps:
  - name: Request with Pre-Script
    method: POST
    url: /api/resource
    pre_script:
      code: |
        // Access and modify variables
        let timestamp = crypto::timestamp();
        let signature = crypto::hmac_sha256(vars["secret"], `${timestamp}:${vars["payload"]}`);

        // Set variables for use in request
        vars["timestamp"] = timestamp;
        vars["signature"] = signature;
    headers:
      X-Timestamp: "{{ timestamp }}"
      X-Signature: "{{ signature }}"
```

### Post-Response Scripts

Run after receiving the response:

```yaml
steps:
  - name: Request with Post-Script
    method: GET
    url: /api/data
    post_script:
      code: |
        // Access response data
        let body = json::parse(response["body"]);
        let status = response["status"];

        // Process and store results
        if status == 200 {
          let items = body["items"];
          vars["item_count"] = json::len(items);
          vars["first_item_id"] = items[0]["id"];
        }

        // Log information
        println(`Received ${vars["item_count"]} items`);
```

### Script-Based Assertions

Complex validation logic:

```yaml
steps:
  - name: Validate Complex Response
    method: GET
    url: /api/report
    script_assert:
      code: |
        let body = json::parse(response["body"]);
        let status = response["status"];

        // Basic assertions
        assert::eq(status, 200, "Status should be 200");

        // Type checking
        assert::is_true(body["data"] != (), "Data should exist");

        // Business logic validation
        let total = body["summary"]["total"];
        let items = body["items"];

        let calculated_total = 0;
        for item in items {
          calculated_total = calculated_total + item["amount"];
        }

        assert::eq(total, calculated_total, "Total should match sum of items");

        // Check all items have required fields
        for item in items {
          assert::is_true(item["id"] != (), "Item must have id");
          assert::is_true(item["amount"] > 0, "Amount must be positive");
        }
```

### Script File Reference

Use external script files:

```yaml
steps:
  - name: Request with External Script
    method: POST
    url: /api/complex
    pre_script:
      file: scripts/prepare_request.rune
    post_script:
      file: scripts/process_response.rune
    script_assert:
      file: scripts/validate_response.rune
```

### Script Context

Scripts have access to:

| Variable | Type | Description |
|----------|------|-------------|
| `vars` | object | All workflow variables (read/write) |
| `response` | object | Response data (post_script/script_assert only) |
| `response["status"]` | integer | HTTP status code |
| `response["body"]` | string | Response body |
| `response["headers"]` | object | Response headers |
| `response["latency"]` | integer | Response time in ms |

See [script.md](script.md) for complete scripting module reference.

---

## GraphQL Support

Native GraphQL request support.

**[Complete GraphQL Workflow Reference →](workflow-graphql.md)**

### Basic Query

```yaml
steps:
  - name: GraphQL Query
    method: POST
    url: /graphql
    graphql:
      query: |
        query GetUser($id: ID!) {
          user(id: $id) {
            id
            name
            email
            posts {
              title
              published
            }
          }
        }
      variables:
        id: "{{ user_id }}"
    extract:
      user_name: body.data.user.name
      post_count: body.data.user.posts
```

### Mutation

```yaml
steps:
  - name: Create User Mutation
    method: POST
    url: /graphql
    graphql:
      query: |
        mutation CreateUser($input: CreateUserInput!) {
          createUser(input: $input) {
            id
            name
            email
          }
        }
      variables:
        input:
          name: "{{ name }}"
          email: "{{ email }}"
      operation_name: CreateUser
    extract:
      new_user_id: body.data.createUser.id
```

### GraphQL Configuration

| Field | Type | Description |
|-------|------|-------------|
| `query` | string | GraphQL query or mutation |
| `variables` | object | Query variables |
| `operation_name` | string | Operation name (for multi-operation documents) |
| `introspection` | boolean | Run standard introspection query |

---

## gRPC Support

Native gRPC request support.

**[Complete gRPC Workflow Reference →](workflow-grpc.md)**

### Unary Call

```yaml
steps:
  - name: gRPC Unary Call
    url: grpc://localhost:50051
    grpc:
      service: greeter.Greeter
      method: SayHello
      proto: protos/greeter.proto
      message:
        name: "{{ user_name }}"
    extract:
      greeting: body.message
    assert:
      status: 200
```

### With Metadata

```yaml
steps:
  - name: gRPC with Metadata
    url: grpc://api.example.com:443
    grpc:
      service: orders.OrderService
      method: CreateOrder
      proto: protos/orders.proto
      message:
        customer_id: "{{ customer_id }}"
        items:
          - product_id: "prod_123"
            quantity: 2
      metadata:
        authorization: "Bearer {{ token }}"
        x-request-id: "{uuid}"
```

### gRPC Configuration

| Field | Type | Description |
|-------|------|-------------|
| `service` | string | Full service name (package.Service) |
| `method` | string | Method name |
| `proto_file` | string | Path to .proto file |
| `message` | object | Request message (for unary/server streaming) |
| `messages` | array | Multiple messages (for client/bidi streaming) |
| `metadata` | object | gRPC metadata (headers) |
| `tls` | boolean | Force TLS connection |
| `streaming` | string | Streaming mode: "unary", "server", "client", "bidi" |

---

## WebSocket Support

Native WebSocket support for real-time API testing.

**[Complete WebSocket Workflow Reference →](workflow-websocket.md)**

### Send and Receive

Send a message and wait for a response:

```yaml
steps:
  - name: WebSocket Echo
    url: wss://echo.websocket.org
    websocket:
      message: '{"action": "ping"}'
      mode: send
      wait_response: 5000
    extract:
      response: body
    assert:
      body:
        action: pong
```

### Listen Mode

Listen for incoming messages:

```yaml
steps:
  - name: Subscribe to Events
    url: wss://api.example.com/events
    websocket:
      message: '{"action": "subscribe", "channel": "orders"}'
      mode: listen
      max_messages: 5
      wait_response: 10000
```

### Stream Mode

Send multiple messages:

```yaml
steps:
  - name: Stream Messages
    url: wss://api.example.com/ws
    websocket:
      mode: stream
      messages:
        - '{"type": "subscribe", "channel": "trades"}'
        - '{"type": "subscribe", "channel": "orders"}'
      wait_response: 5000
```

### Binary Messages

```yaml
steps:
  - name: Send Binary
    url: wss://api.example.com/binary
    websocket:
      binary: "48656c6c6f"
      binary_mode: hex
      wait_response: 1000
```

### WebSocket with Headers

```yaml
steps:
  - name: Authenticated WebSocket
    url: wss://api.example.com/ws
    headers:
      Authorization: "Bearer {{ token }}"
      X-Client-ID: "{{ client_id }}"
    websocket:
      message: '{"action": "authenticate"}'
      subprotocol: graphql-ws
      wait_response: 5000
```

### WebSocket Configuration

| Field | Type | Description |
|-------|------|-------------|
| `message` | string | Single message to send |
| `messages` | array | Multiple messages to send (stream mode) |
| `binary` | string | Binary data (hex or base64 encoded) |
| `binary_mode` | string | Binary encoding: "hex" or "base64" |
| `subprotocol` | string | WebSocket subprotocol to request |
| `mode` | string | Operation mode: "send", "listen", or "stream" |
| `max_messages` | number | Maximum messages to receive (0 = unlimited) |
| `ping_interval` | number | Keep-alive ping interval in seconds |
| `wait_response` | number | Wait for response (milliseconds) |
| `compress` | boolean | Enable permessage-deflate compression |

---

## Execution Control

### Conditional Execution

Skip steps based on conditions:

```yaml
steps:
  - name: Check Feature Flag
    url: /api/features
    extract:
      new_feature_enabled: body.new_checkout

  - name: New Checkout Flow
    url: /api/checkout/v2
    skip_if: "{{ new_feature_enabled }} == false"

  - name: Legacy Checkout Flow
    url: /api/checkout/v1
    skip_if: "{{ new_feature_enabled }} == true"
```

### Delays

```yaml
steps:
  - name: Submit Job
    method: POST
    url: /jobs
    extract:
      job_id: body.id

  - name: Wait and Check Status
    url: /jobs/{{ job_id }}
    delay: 5000  # Wait 5 seconds before request
    assert:
      body:
        - path: status
          equals: completed
```

### Timeouts

```yaml
steps:
  - name: Quick Request
    url: /fast-endpoint
    timeout: 5000  # 5 second timeout

  - name: Long Running Request
    url: /slow-endpoint
    timeout: 300000  # 5 minute timeout
```

### Retries

```yaml
steps:
  - name: Unreliable Endpoint
    url: /flaky-service
    retries: 3
    retry_delay: 2000  # Wait 2 seconds between retries
    assert:
      status: 200
```

---

## CLI Reference

### Running Workflows

```bash
# Basic execution
quicpulse --run workflow.yaml

# With environment
quicpulse --run workflow.yaml --env production

# With variable overrides
quicpulse --run workflow.yaml --var api_key=abc123 --var debug=true

# Continue on failure
quicpulse --run workflow.yaml --continue-on-failure

# Validate without executing
quicpulse --run workflow.yaml --validate
```

### Report Generation

```bash
# JUnit XML (for CI/CD)
quicpulse --run workflow.yaml --report-junit results.xml

# JSON report
quicpulse --run workflow.yaml --report-json results.json

# TAP format
quicpulse --run workflow.yaml --report-tap results.tap

# Multiple formats
quicpulse --run workflow.yaml --report-junit results.xml --report-json results.json
```

### All Workflow Options

| Flag | Description |
|------|-------------|
| `--run <file>` | Run workflow file |
| `--env <name>` | Select environment |
| `--var <key=value>` | Override variable (repeatable) |
| `--continue-on-failure` | Continue executing after failures |
| `--validate` | Validate workflow without executing |
| `--tags <tags>` | Run only steps with specified tags (comma-separated) |
| `--include <steps>` | Include only specified steps by name (comma-separated) |
| `--exclude <steps>` | Exclude specified steps by name (comma-separated) |
| `--save-responses <dir>` | Save all responses to directory |
| `--log-format <format>` | Output format: `text` (default) or `json` |
| `--no-color` | Disable colored output |
| `--report-junit <file>` | Generate JUnit XML report |
| `--report-json <file>` | Generate JSON report |
| `--report-tap <file>` | Generate TAP report |

### Response Persistence

Save all responses to a directory for debugging or auditing:

```bash
quicpulse --run workflow.yaml --save-responses=./responses
```

Response files are saved as JSON with templated names: `{step_name}_{status}_{timestamp}.json`

Each response file contains:
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "step_name": "create_user",
  "method": "POST",
  "url": "https://api.example.com/users",
  "request_headers": { "Content-Type": "application/json" },
  "request_body": "{\"name\": \"John\"}",
  "status_code": 201,
  "response_headers": { "content-type": "application/json" },
  "response_body": "{\"id\": 123, \"name\": \"John\"}",
  "duration_ms": 234,
  "assertions": [
    { "assertion": "status == 201", "passed": true }
  ]
}
```

---

## Report Formats

### JUnit XML

Standard CI/CD integration format:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="My Workflow" tests="5" failures="1" time="2.345">
  <testsuite name="My Workflow" tests="5" failures="1" time="2.345">
    <testcase name="Login" time="0.234"/>
    <testcase name="Get Profile" time="0.123"/>
    <testcase name="Update Settings" time="0.456">
      <failure message="Assertion failed: status">
        Expected 200, got 500
      </failure>
    </testcase>
  </testsuite>
</testsuites>
```

### GitLab CI Configuration

GitLab parses this XML to display a "Tests" tab in the Pipeline view.

Usage in `.gitlab-ci.yml`:

```yaml
api_testing:
  stage: test
  image: ubuntu:latest
  script:
    # Run QuicPulse and generate the report
    - ./quicpulse run workflow.yml --report-junit=results.xml
  artifacts:
    when: always
    reports:
      junit: results.xml  # <--- THIS IS THE KEY
```

### Jenkins Configuration

Jenkins uses the standard "JUnit Plugin" (installed by default on almost every instance) to visualize trends and failures.

Usage in `Jenkinsfile`:

```groovy
pipeline {
    agent any
    stages {
        stage('API Tests') {
            steps {
                // Run QuicPulse
                sh './quicpulse run workflow.yml --report-junit=results.xml'
            }
            post {
                always {
                    // Ingest the report
                    junit 'results.xml'
                }
            }
        }
    }
}
```

### JSON Report

```json
{
  "name": "My Workflow",
  "success": false,
  "duration_ms": 2345,
  "total_steps": 5,
  "passed": 4,
  "failed": 1,
  "skipped": 0,
  "steps": [
    {
      "name": "Login",
      "success": true,
      "duration_ms": 234,
      "status": 200,
      "assertions": []
    }
  ]
}
```

### TAP Format

```
TAP version 14
1..5
ok 1 - Login (234ms)
ok 2 - Get Profile (123ms)
not ok 3 - Update Settings (456ms)
  ---
  message: "Assertion failed: Expected status 200, got 500"
  severity: fail
  ...
ok 4 - Delete Session (89ms)
ok 5 - Verify Logout (112ms)
```

---

## Complete Examples

### REST API Test Suite

```yaml
name: User Management API Tests
description: Complete CRUD test coverage
base_url: https://api.example.com/v1

variables:
  test_email: "test_{uuid}@example.com"
  test_name: "Test User"

environments:
  dev:
    base_url: http://localhost:3000/v1
  staging:
    base_url: https://staging-api.example.com/v1

headers:
  Content-Type: application/json
  Accept: application/json

steps:
  - name: Health Check
    method: GET
    url: /health
    assert:
      status: 200
      latency: 500
      body:
        - path: status
          equals: healthy

  - name: Create User
    method: POST
    url: /users
    body: |
      {
        "name": "{{ test_name }}",
        "email": "{{ test_email }}",
        "role": "user"
      }
    extract:
      user_id: body.id
      created_at: body.created_at
    assert:
      status: 201
      body:
        - path: id
          exists: true
        - path: email
          equals: "{{ test_email }}"

  - name: Get User
    method: GET
    url: /users/{{ user_id }}
    assert:
      status: 200
      body:
        - path: name
          equals: "{{ test_name }}"
        - path: email
          equals: "{{ test_email }}"

  - name: Update User
    method: PATCH
    url: /users/{{ user_id }}
    body: |
      {
        "name": "Updated Name"
      }
    assert:
      status: 200
      body:
        - path: name
          equals: "Updated Name"

  - name: List Users
    method: GET
    url: /users
    query:
      limit: 10
      offset: 0
    assert:
      status: 200
      body:
        - path: data
          exists: true

  - name: Delete User
    method: DELETE
    url: /users/{{ user_id }}
    assert:
      status: 204

  - name: Verify Deletion
    method: GET
    url: /users/{{ user_id }}
    assert:
      status: 404
```

### Authentication Flow

```yaml
name: OAuth2 Authentication Flow
base_url: https://auth.example.com

variables:
  client_id: my_app
  redirect_uri: https://myapp.com/callback

steps:
  - name: Get Authorization URL
    method: GET
    url: /oauth/authorize
    query:
      response_type: code
      client_id: "{{ client_id }}"
      redirect_uri: "{{ redirect_uri }}"
      scope: "read write"
      state: "{random_string:32}"
    assert:
      status: 302
    extract:
      auth_redirect: header.Location

  - name: Exchange Code for Token
    method: POST
    url: /oauth/token
    form:
      grant_type: authorization_code
      code: "{{ auth_code }}"
      client_id: "{{ client_id }}"
      client_secret: "{{ client_secret }}"
      redirect_uri: "{{ redirect_uri }}"
    extract:
      access_token: body.access_token
      refresh_token: body.refresh_token
      expires_in: body.expires_in
    assert:
      status: 200
      body:
        - path: token_type
          equals: Bearer

  - name: Use Access Token
    method: GET
    url: https://api.example.com/me
    auth:
      type: bearer
      token: "{{ access_token }}"
    assert:
      status: 200

  - name: Refresh Token
    method: POST
    url: /oauth/token
    form:
      grant_type: refresh_token
      refresh_token: "{{ refresh_token }}"
      client_id: "{{ client_id }}"
    extract:
      access_token: body.access_token
    assert:
      status: 200
```

### E-Commerce Order Flow with Scripts

```yaml
name: E-Commerce Order Flow
description: Complete order lifecycle with validation
base_url: https://api.shop.example.com

variables:
  currency: USD

steps:
  - name: Get Product Catalog
    method: GET
    url: /products
    query:
      category: electronics
      in_stock: true
    extract:
      products: body.items
    post_script:
      code: |
        let body = json::parse(response["body"]);
        let products = body["items"];

        // Find cheapest in-stock product
        let cheapest = ();
        let min_price = 999999;

        for p in products {
          if p["price"] < min_price && p["stock"] > 0 {
            min_price = p["price"];
            cheapest = p;
          }
        }

        vars["selected_product"] = cheapest["id"];
        vars["product_price"] = cheapest["price"];
        println(`Selected product: ${cheapest["name"]} at $${cheapest["price"]}`);

  - name: Add to Cart
    method: POST
    url: /cart
    body: |
      {
        "product_id": "{{ selected_product }}",
        "quantity": 2
      }
    extract:
      cart_id: body.cart_id
      cart_total: body.total
    script_assert:
      code: |
        let body = json::parse(response["body"]);
        let expected = vars["product_price"] * 2;

        assert::eq(body["total"], expected, "Cart total should be 2x product price");
        assert::eq(body["items_count"], 1, "Should have 1 line item");

  - name: Apply Discount Code
    method: POST
    url: /cart/{{ cart_id }}/discount
    body: '{"code": "SAVE10"}'
    extract:
      discount_amount: body.discount
      final_total: body.total
    assert:
      status: 200

  - name: Checkout
    method: POST
    url: /checkout
    pre_script:
      code: |
        // Generate order reference
        vars["order_ref"] = `ORD-${crypto::random_string(8)}`;
        vars["idempotency_key"] = crypto::uuid_v4();
    headers:
      Idempotency-Key: "{{ idempotency_key }}"
    body: |
      {
        "cart_id": "{{ cart_id }}",
        "reference": "{{ order_ref }}",
        "payment_method": "card",
        "shipping_address": {
          "street": "123 Main St",
          "city": "New York",
          "zip": "10001"
        }
      }
    extract:
      order_id: body.order_id
      payment_url: body.payment_url
    assert:
      status: 201
      latency: 3000

  - name: Verify Order Created
    method: GET
    url: /orders/{{ order_id }}
    delay: 1000
    assert:
      status: 200
      body:
        - path: status
          equals: pending_payment
        - path: reference
          equals: "{{ order_ref }}"
```

---

## Best Practices

1. **Use descriptive step names** - They appear in output and reports
2. **Extract only what you need** - Avoid extracting entire responses
3. **Set appropriate timeouts** - Different endpoints have different latency
4. **Use environments** - Separate dev/staging/production configurations
5. **Add meaningful assertions** - Validate business logic, not just status codes
6. **Use scripts for complex logic** - Keep YAML readable
7. **Handle failures gracefully** - Use `--continue-on-failure` in CI/CD
8. **Generate reports** - Use JUnit XML for CI/CD integration
9. **Version control workflows** - Treat them as code
10. **Parameterize with variables** - Make workflows reusable

---

## Additional Feature References

For detailed documentation on specific workflow features:

### Protocol Support
- [GraphQL Workflows](workflow-graphql.md) - GraphQL queries and mutations
- [gRPC Workflows](workflow-grpc.md) - gRPC unary and streaming calls
- [WebSocket Workflows](workflow-websocket.md) - WebSocket connections and messaging

### Testing & Security
- [Fuzzing](workflow-fuzzing.md) - Security vulnerability testing
- [Benchmarking](workflow-benchmarking.md) - Load testing and performance
- [Sessions](workflow-sessions.md) - Cookie and state persistence

### Data Management
- [Downloads](workflow-downloads.md) - File download handling
- [Uploads](workflow-uploads.md) - Chunked and compressed uploads
- [Output & Filtering](workflow-output.md) - Display control, filtering, and saving

### Integration
- [HAR Replay](workflow-har.md) - HTTP Archive replay
- [OpenAPI](workflow-openapi.md) - OpenAPI-driven testing
- [Plugins](workflow-plugins.md) - Plugin integration

---

See also:
- [script.md](script.md) - Complete scripting reference
- [README.md](../README.md) - CLI reference and features
