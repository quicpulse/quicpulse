# gRPC Workflow Documentation

Complete reference for gRPC steps in workflows - unary calls, streaming, and proto file handling.

## Table of Contents

- [Overview](#overview)
- [Basic Configuration](#basic-configuration)
- [Connection Types](#connection-types)
  - [Unencrypted (grpc://)](#unencrypted-grpc)
  - [TLS Encrypted (grpcs://)](#tls-encrypted-grpcs)
- [Call Types](#call-types)
  - [Unary Calls](#unary-calls)
  - [Server Streaming](#server-streaming)
  - [Client Streaming](#client-streaming)
  - [Bidirectional Streaming](#bidirectional-streaming)
- [Proto Files](#proto-files)
  - [Loading Proto Files](#loading-proto-files)
  - [Auto-Detection](#auto-detection)
- [Metadata and Headers](#metadata-and-headers)
- [Message Handling](#message-handling)
  - [Request Messages](#request-messages)
  - [Response Handling](#response-handling)
  - [Extraction](#extraction)
  - [Assertions](#assertions)
- [Advanced Examples](#advanced-examples)
  - [CRUD Operations](#crud-operations)
  - [Streaming Pipeline](#streaming-pipeline)
  - [Authentication](#authentication)
- [Configuration Reference](#configuration-reference)

---

## Overview

gRPC workflow steps allow you to:

- **Call** gRPC services using unary and streaming patterns
- **Load** proto files for proper message encoding
- **Send** JSON messages that are converted to protobuf
- **Extract** data from responses for chaining
- **Assert** on response content

gRPC steps are defined using the `grpc` field in a workflow step, with the URL pointing to your gRPC server.

---

## Basic Configuration

A minimal gRPC step:

```yaml
steps:
  - name: Simple gRPC Call
    url: grpc://localhost:50051
    grpc:
      service: mypackage.UserService
      method: GetUser
      message:
        user_id: 123
```

---

## Connection Types

### Unencrypted (grpc://)

For local development:

```yaml
steps:
  - name: Local gRPC Call
    url: grpc://localhost:50051
    grpc:
      service: example.GreeterService
      method: SayHello
      message:
        name: "World"
```

### TLS Encrypted (grpcs://)

For production:

```yaml
steps:
  - name: Secure gRPC Call
    url: grpcs://api.example.com:443
    grpc:
      service: example.GreeterService
      method: SayHello
      tls: true
      message:
        name: "World"
```

You can also use HTTPS URLs:

```yaml
steps:
  - name: HTTPS gRPC Call
    url: https://api.example.com:443
    grpc:
      service: example.GreeterService
      method: SayHello
      message:
        name: "World"
```

### Skip TLS Verification

For self-signed certificates:

```yaml
steps:
  - name: Insecure TLS
    url: grpcs://localhost:50051
    insecure: true
    grpc:
      service: example.Service
      method: Method
      message: {}
```

---

## Call Types

### Unary Calls

Single request, single response (default):

```yaml
steps:
  - name: Unary Call
    url: grpc://localhost:50051
    grpc:
      service: users.UserService
      method: GetUser
      proto_file: protos/user.proto
      message:
        user_id: "{{ user_id }}"
    extract:
      user_name: ".name"
      user_email: ".email"
    assert:
      body:
        - path: id
          equals: "{{ user_id }}"
```

### Server Streaming

Single request, stream of responses:

```yaml
steps:
  - name: Server Streaming
    url: grpc://localhost:50051
    grpc:
      service: events.EventService
      method: Subscribe
      proto_file: protos/events.proto
      streaming: server
      message:
        topic: "orders"
        limit: 10
    extract:
      # Body is a JSON array of all responses
      events: body
      first_event: ".[0]"
      event_count: ". | length"
    assert:
      body:
        - path: "[0].type"
          exists: true
```

### Client Streaming

Stream of requests, single response:

```yaml
steps:
  - name: Client Streaming
    url: grpc://localhost:50051
    grpc:
      service: analytics.AggregateService
      method: Aggregate
      proto_file: protos/analytics.proto
      streaming: client
      messages:
        - { value: 10, timestamp: 1234567890 }
        - { value: 20, timestamp: 1234567891 }
        - { value: 30, timestamp: 1234567892 }
    extract:
      total: ".total"
      average: ".average"
    assert:
      body:
        - path: total
          equals: 60
```

### Bidirectional Streaming

Stream of requests, stream of responses:

```yaml
steps:
  - name: Bidirectional Streaming
    url: grpc://localhost:50051
    grpc:
      service: chat.ChatService
      method: Chat
      proto_file: protos/chat.proto
      streaming: bidi
      messages:
        - { text: "Hello", user: "alice" }
        - { text: "Hi there!", user: "bob" }
        - { text: "How are you?", user: "alice" }
    extract:
      responses: body
```

---

## Proto Files

### Loading Proto Files

For correct message encoding, provide a proto file:

```yaml
steps:
  - name: With Proto File
    url: grpc://localhost:50051
    grpc:
      service: users.UserService
      method: CreateUser
      proto_file: ./protos/user.proto
      message:
        name: "John Doe"
        email: "john@example.com"
```

Relative paths are resolved from the workflow file location.

### Auto-Detection

Without a proto file, streaming modes must be specified explicitly:

```yaml
steps:
  # With proto file - streaming mode auto-detected
  - name: Auto-Detected Streaming
    url: grpc://localhost:50051
    grpc:
      service: events.EventService
      method: Subscribe
      proto_file: protos/events.proto
      message:
        topic: "orders"

  # Without proto file - must specify streaming mode
  - name: Manual Streaming Mode
    url: grpc://localhost:50051
    grpc:
      service: events.EventService
      method: Subscribe
      streaming: server
      message:
        topic: "orders"
```

---

## Metadata and Headers

### Using Step Headers

```yaml
steps:
  - name: With Headers
    url: grpc://localhost:50051
    headers:
      authorization: "Bearer {{ token }}"
      x-request-id: "{uuid}"
      x-client-version: "1.0.0"
    grpc:
      service: users.UserService
      method: GetUser
      message:
        user_id: 123
```

### Using gRPC Metadata

Alternative syntax within gRPC config:

```yaml
steps:
  - name: With Metadata
    url: grpc://localhost:50051
    grpc:
      service: users.UserService
      method: GetUser
      metadata:
        authorization: "Bearer {{ token }}"
        x-request-id: "{uuid}"
      message:
        user_id: 123
```

Both `headers` and `metadata` can be used together - they are merged.

---

## Message Handling

### Request Messages

Messages are specified as JSON and converted to protobuf:

```yaml
# Simple message
message:
  name: "John"
  age: 30

# Nested message
message:
  user:
    name: "John"
    email: "john@example.com"
  options:
    notify: true
    priority: "HIGH"

# With arrays
message:
  items:
    - { product_id: "prod_1", quantity: 2 }
    - { product_id: "prod_2", quantity: 1 }

# With workflow variables
message:
  user_id: "{{ user_id }}"
  name: "{{ user_name }}"
  timestamp: "{timestamp}"
```

### Response Handling

Responses are converted to JSON:

```yaml
steps:
  - name: Handle Response
    url: grpc://localhost:50051
    grpc:
      service: orders.OrderService
      method: GetOrder
      message:
        order_id: "{{ order_id }}"
    extract:
      status: ".status"
      items: ".items"
      total: ".total"
      created_at: ".created_at"
```

### Extraction

Extract values from gRPC responses:

```yaml
steps:
  - name: Extract Data
    url: grpc://localhost:50051
    grpc:
      service: products.ProductService
      method: ListProducts
      message:
        category: "electronics"
    extract:
      # Single values
      product_count: ". | length"
      first_product_id: ".[0].id"
      first_product_name: ".[0].name"
      
      # Arrays
      all_ids: ".[].id"
      all_prices: ".[].price"
```

### Assertions

Validate gRPC responses:

```yaml
steps:
  - name: With Assertions
    url: grpc://localhost:50051
    grpc:
      service: users.UserService
      method: GetUser
      message:
        user_id: 123
    assert:
      body:
        - path: id
          equals: 123
        
        - path: email
          matches: "^[a-z]+@example\\.com$"
        
        - path: status
          equals: "ACTIVE"
        
        - path: created_at
          exists: true
```

---

## Advanced Examples

### CRUD Operations

Complete gRPC CRUD workflow:

```yaml
name: gRPC CRUD Workflow
base_url: grpc://localhost:50051

variables:
  user_name: "Test User"
  user_email: "test@example.com"

steps:
  # Create
  - name: Create User
    url: ""
    grpc:
      service: users.UserService
      method: CreateUser
      proto_file: protos/user.proto
      message:
        name: "{{ user_name }}"
        email: "{{ user_email }}"
    extract:
      user_id: ".id"
    assert:
      body:
        - path: name
          equals: "{{ user_name }}"

  # Read
  - name: Get User
    url: ""
    grpc:
      service: users.UserService
      method: GetUser
      proto_file: protos/user.proto
      message:
        user_id: "{{ user_id }}"
    assert:
      body:
        - path: id
          equals: "{{ user_id }}"
        - path: email
          equals: "{{ user_email }}"

  # Update
  - name: Update User
    url: ""
    grpc:
      service: users.UserService
      method: UpdateUser
      proto_file: protos/user.proto
      message:
        user_id: "{{ user_id }}"
        name: "Updated Name"
    assert:
      body:
        - path: name
          equals: "Updated Name"

  # Delete
  - name: Delete User
    url: ""
    grpc:
      service: users.UserService
      method: DeleteUser
      proto_file: protos/user.proto
      message:
        user_id: "{{ user_id }}"
    assert:
      body:
        - path: success
          equals: true
```

### Streaming Pipeline

Process streaming data:

```yaml
name: Event Processing Pipeline

steps:
  # Subscribe to events
  - name: Get Recent Events
    url: grpc://localhost:50051
    grpc:
      service: events.EventService
      method: GetRecentEvents
      proto_file: protos/events.proto
      streaming: server
      message:
        limit: 100
        since_timestamp: "{timestamp}"
    extract:
      events: body
      event_count: ". | length"

  # Process each event (using client streaming)
  - name: Acknowledge Events
    url: grpc://localhost:50051
    skip_if: "{{ event_count }} == 0"
    grpc:
      service: events.EventService
      method: AcknowledgeEvents
      proto_file: protos/events.proto
      streaming: client
      messages: "{{ events | map(attribute='id') | list }}"
```

### Authentication

JWT authentication flow:

```yaml
name: Authenticated gRPC Workflow

steps:
  # Get token via HTTP
  - name: Get Auth Token
    method: POST
    url: https://auth.example.com/token
    body:
      client_id: "{{ client_id }}"
      client_secret: "{{ client_secret }}"
    extract:
      access_token: body.access_token

  # Use token for gRPC calls
  - name: Authenticated gRPC Call
    url: grpcs://api.example.com:443
    headers:
      authorization: "Bearer {{ access_token }}"
    grpc:
      service: protected.ProtectedService
      method: GetSecureData
      proto_file: protos/protected.proto
      message:
        resource_id: "{{ resource_id }}"
    assert:
      body:
        - path: data
          exists: true
```

### Mixed Protocol Workflow

Combine gRPC with HTTP and WebSocket:

```yaml
name: Multi-Protocol Workflow

steps:
  # REST API call
  - name: Get Configuration
    method: GET
    url: https://api.example.com/config
    extract:
      grpc_endpoint: body.grpc_endpoint
      ws_endpoint: body.ws_endpoint

  # gRPC call
  - name: Create Order via gRPC
    url: "{{ grpc_endpoint }}"
    grpc:
      service: orders.OrderService
      method: CreateOrder
      proto_file: protos/order.proto
      message:
        product_id: "{{ product_id }}"
        quantity: 1
    extract:
      order_id: ".id"

  # WebSocket subscription
  - name: Subscribe to Order Updates
    url: "{{ ws_endpoint }}"
    websocket:
      message: '{"action": "subscribe", "order_id": "{{ order_id }}"}'
      mode: listen
      max_messages: 1
      wait_response: 30000
    assert:
      body:
        - path: status
          equals: "CONFIRMED"
```

---

## Configuration Reference

### gRPC Configuration Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `service` | string | Yes | Full service name (package.Service) |
| `method` | string | Yes | Method name to call |
| `message` | object | No | Request message as JSON (for unary/server streaming) |
| `messages` | array | No | Multiple request messages (for client/bidi streaming) |
| `proto_file` | string | No | Path to .proto file for schema |
| `import_paths` | array | No | Additional proto import paths (future) |
| `tls` | boolean | No | Force TLS connection |
| `streaming` | string | No | Override streaming mode: "unary", "server", "client", "bidi" |
| `metadata` | object | No | gRPC metadata (alternative to step headers) |

### Step-Level Options

These standard workflow step options work with gRPC steps:

| Field | Type | Description |
|-------|------|-------------|
| `headers` | object | gRPC metadata headers |
| `timeout` | string | Request timeout (e.g., "30s") |
| `insecure` | boolean | Skip TLS verification |
| `extract` | object | Extract values from response |
| `assert` | object | Response assertions |
| `skip_if` | string | Conditional execution |
| `retries` | number | Retry on failure |

### URL Formats

Supported URL formats:

```yaml
# Standard gRPC URLs
url: grpc://localhost:50051
url: grpcs://api.example.com:443

# HTTP/HTTPS (auto-converted)
url: http://localhost:50051   # → grpc://
url: https://api.example.com  # → grpcs://

# With workflow base_url
base_url: grpc://localhost:50051
# Then steps can use:
url: ""  # Uses base_url
```

### Streaming Modes

| Mode | Request | Response | Use Case |
|------|---------|----------|----------|
| `unary` | Single | Single | Standard request/response |
| `server` | Single | Stream | Subscriptions, large data fetch |
| `client` | Stream | Single | Batch uploads, aggregations |
| `bidi` | Stream | Stream | Chat, real-time sync |

---

## See Also

- [Workflow Documentation](workflow.md) - Complete workflow reference
- [Scripting Reference](script.md) - Rune scripting for workflows
- [GraphQL Workflow Reference](workflow-graphql.md) - GraphQL steps
- [WebSocket Workflow Reference](workflow-websocket.md) - WebSocket steps
