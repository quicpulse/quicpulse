# WebSocket Workflow Documentation

Complete reference for WebSocket steps in workflows - real-time API testing, streaming, and bidirectional communication.

## Table of Contents

- [Overview](#overview)
- [Basic Configuration](#basic-configuration)
- [Operation Modes](#operation-modes)
  - [Send Mode](#send-mode)
  - [Listen Mode](#listen-mode)
  - [Stream Mode](#stream-mode)
- [Message Types](#message-types)
  - [Text Messages](#text-messages)
  - [Binary Messages](#binary-messages)
  - [JSON Messages](#json-messages)
- [Connection Options](#connection-options)
  - [Headers](#headers)
  - [Subprotocols](#subprotocols)
  - [Compression](#compression)
  - [TLS/SSL](#tlsssl)
  - [Timeouts](#timeouts)
  - [Keep-Alive Pings](#keep-alive-pings)
- [Response Handling](#response-handling)
  - [Waiting for Responses](#waiting-for-responses)
  - [Multiple Messages](#multiple-messages)
  - [Extraction](#extraction)
  - [Assertions](#assertions)
- [Advanced Examples](#advanced-examples)
  - [Authentication Flow](#authentication-flow)
  - [Subscription Pattern](#subscription-pattern)
  - [Request-Response Pattern](#request-response-pattern)
  - [Chained WebSocket Steps](#chained-websocket-steps)
- [Configuration Reference](#configuration-reference)

---

## Overview

WebSocket workflow steps allow you to:

- **Connect** to WebSocket servers (ws:// and wss://)
- **Send** text or binary messages
- **Receive** responses and extract data
- **Assert** on message content
- **Chain** WebSocket interactions with HTTP/gRPC steps

WebSocket steps are defined using the `websocket` field in a workflow step, with the URL pointing to a WebSocket endpoint.

---

## Basic Configuration

A minimal WebSocket step requires a URL and the `websocket` configuration:

```yaml
steps:
  - name: Simple WebSocket Echo
    url: wss://echo.websocket.org
    websocket:
      message: "Hello, WebSocket!"
      wait_response: 5000
```

The URL can use:
- `ws://` - Unencrypted WebSocket
- `wss://` - TLS-encrypted WebSocket
- `http://` or `https://` - Auto-converted to ws/wss

---

## Operation Modes

### Send Mode

**Default mode.** Send a message and optionally wait for a response:

```yaml
steps:
  - name: Send Single Message
    url: wss://api.example.com/ws
    websocket:
      mode: send
      message: '{"action": "ping"}'
      wait_response: 5000
    extract:
      response: body
    assert:
      body:
        action: pong
```

Send mode is ideal for:
- Request/response patterns
- One-shot notifications
- Simple handshakes

### Listen Mode

Receive messages from the server:

```yaml
steps:
  - name: Listen for Events
    url: wss://stream.example.com/events
    websocket:
      mode: listen
      max_messages: 10
      wait_response: 30000
    extract:
      first_event: body
```

Listen mode options:
- `max_messages` - Stop after receiving N messages (default: 1)
- `wait_response` - Maximum time to wait in milliseconds

```yaml
steps:
  - name: Listen Until Timeout
    url: wss://stream.example.com/events
    websocket:
      mode: listen
      max_messages: 0  # Unlimited
      wait_response: 60000  # Stop after 60 seconds
```

### Stream Mode

Send multiple messages in sequence:

```yaml
steps:
  - name: Stream Multiple Messages
    url: wss://api.example.com/ws
    websocket:
      mode: stream
      messages:
        - '{"type": "subscribe", "channel": "orders"}'
        - '{"type": "subscribe", "channel": "trades"}'
        - '{"type": "subscribe", "channel": "ticker"}'
      wait_response: 5000
      max_messages: 3
```

Stream mode with variable substitution:

```yaml
steps:
  - name: Subscribe to User Channels
    url: wss://api.example.com/ws
    websocket:
      mode: stream
      messages:
        - '{"type": "subscribe", "channel": "user.{{ user_id }}.orders"}'
        - '{"type": "subscribe", "channel": "user.{{ user_id }}.notifications"}'
      wait_response: 10000
```

---

## Message Types

### Text Messages

Plain text or JSON strings:

```yaml
steps:
  - name: Send Text Message
    url: wss://echo.websocket.org
    websocket:
      message: "Hello, World!"
      wait_response: 5000
```

### Binary Messages

Send binary data encoded as hex or base64:

```yaml
steps:
  # Hex-encoded binary
  - name: Send Binary (Hex)
    url: wss://binary.example.com/ws
    websocket:
      binary: "48656c6c6f"  # "Hello" in hex
      binary_mode: hex
      wait_response: 5000

  # Base64-encoded binary
  - name: Send Binary (Base64)
    url: wss://binary.example.com/ws
    websocket:
      binary: "SGVsbG8gV29ybGQ="  # "Hello World" in base64
      binary_mode: base64
      wait_response: 5000
```

### JSON Messages

JSON is typically sent as text messages:

```yaml
steps:
  - name: Send JSON Message
    url: wss://api.example.com/ws
    websocket:
      message: |
        {
          "type": "request",
          "id": "{uuid}",
          "payload": {
            "action": "get_user",
            "user_id": "{{ user_id }}"
          }
        }
      wait_response: 5000
```

Using magic values in JSON:

```yaml
steps:
  - name: JSON with Magic Values
    url: wss://api.example.com/ws
    websocket:
      message: |
        {
          "request_id": "{uuid}",
          "timestamp": "{timestamp}",
          "client": "{random_string:8}"
        }
      wait_response: 5000
```

---

## Connection Options

### Headers

Add custom headers to the WebSocket handshake:

```yaml
steps:
  - name: WebSocket with Headers
    url: wss://api.example.com/ws
    headers:
      Authorization: "Bearer {{ access_token }}"
      X-API-Key: "{{ api_key }}"
      X-Request-ID: "{uuid}"
      User-Agent: "MyApp/1.0"
    websocket:
      message: '{"action": "connect"}'
      wait_response: 5000
```

### Subprotocols

Request specific WebSocket subprotocols:

```yaml
steps:
  # GraphQL over WebSocket
  - name: GraphQL Subscription
    url: wss://api.example.com/graphql
    websocket:
      subprotocol: graphql-ws
      message: |
        {
          "type": "connection_init",
          "payload": {"authorization": "{{ token }}"}
        }
      wait_response: 5000

  # MQTT over WebSocket
  - name: MQTT Connection
    url: wss://broker.example.com/mqtt
    websocket:
      subprotocol: mqtt
      message: "CONNECT"
      wait_response: 5000
```

### Compression

Enable permessage-deflate compression:

```yaml
steps:
  - name: Compressed WebSocket
    url: wss://api.example.com/ws
    websocket:
      message: '{"data": "large payload..."}'
      compress: true
      wait_response: 5000
```

### TLS/SSL

Skip TLS certificate verification for self-signed or test servers:

```yaml
steps:
  - name: Insecure WebSocket
    url: wss://localhost:8443/ws
    insecure: true
    websocket:
      message: "Hello"
      wait_response: 5000
```

### Timeouts

Set step timeout (connection + operation):

```yaml
steps:
  - name: WebSocket with Timeout
    url: wss://api.example.com/ws
    timeout: "30s"
    websocket:
      message: '{"action": "long_operation"}'
      wait_response: 25000
```

### Keep-Alive Pings

Send periodic ping frames to maintain connection:

```yaml
steps:
  - name: Long-Running WebSocket
    url: wss://stream.example.com/events
    websocket:
      mode: listen
      ping_interval: 30  # Send ping every 30 seconds
      max_messages: 100
      wait_response: 300000  # 5 minutes
```

---

## Response Handling

### Waiting for Responses

The `wait_response` field specifies how long (in milliseconds) to wait for a response:

```yaml
steps:
  - name: Quick Response Expected
    url: wss://api.example.com/ws
    websocket:
      message: '{"action": "ping"}'
      wait_response: 1000  # 1 second

  - name: Slow Response Expected
    url: wss://api.example.com/ws
    websocket:
      message: '{"action": "process_batch"}'
      wait_response: 60000  # 60 seconds
```

### Multiple Messages

When receiving multiple messages, they're collected into a JSON array:

```yaml
steps:
  - name: Collect Multiple Responses
    url: wss://stream.example.com/events
    websocket:
      mode: listen
      max_messages: 5
      wait_response: 10000
    extract:
      # When max_messages > 1, body is a JSON array
      all_events: body
      # Use JQ expressions for specific items
      first_event: ".[0]"
      last_event: ".[-1]"
```

### Extraction

Extract values from WebSocket responses:

```yaml
steps:
  - name: Extract from Response
    url: wss://api.example.com/ws
    websocket:
      message: '{"action": "get_session"}'
      wait_response: 5000
    extract:
      # Full response body
      response: body
      
      # Nested JSON fields (use JQ syntax)
      session_id: ".session_id"
      user_name: ".user.name"
      permissions: ".user.permissions"
```

Use extracted values in subsequent steps:

```yaml
steps:
  - name: Get Session
    url: wss://api.example.com/ws
    websocket:
      message: '{"action": "authenticate", "token": "{{ token }}"}'
      wait_response: 5000
    extract:
      session_id: ".session_id"

  - name: Use Session
    url: wss://api.example.com/ws
    websocket:
      message: '{"action": "subscribe", "session": "{{ session_id }}"}'
      wait_response: 5000
```

### Assertions

Validate WebSocket responses:

```yaml
steps:
  - name: Assert WebSocket Response
    url: wss://api.example.com/ws
    websocket:
      message: '{"action": "health"}'
      wait_response: 5000
    assert:
      body:
        status: healthy
        version: "1.0"
```

Complex assertions:

```yaml
steps:
  - name: Complex Assertions
    url: wss://api.example.com/ws
    websocket:
      message: '{"action": "get_stats"}'
      wait_response: 5000
    assert:
      body:
        # Check specific values
        - path: status
          equals: "ok"
        
        # Check existence
        - path: data.count
          exists: true
        
        # Check type
        - path: data.items
          type: array
        
        # Numeric comparison
        - path: data.count
          greater_than: 0
```

---

## Advanced Examples

### Authentication Flow

Complete authentication workflow:

```yaml
name: WebSocket Authentication Flow
base_url: wss://api.example.com

variables:
  username: testuser
  password: secret123

steps:
  - name: Connect and Authenticate
    url: "{{ base_url }}/ws"
    websocket:
      message: |
        {
          "type": "auth",
          "username": "{{ username }}",
          "password": "{{ password }}"
        }
      wait_response: 5000
    extract:
      auth_token: ".token"
      session_id: ".session_id"
    assert:
      body:
        type: auth_success

  - name: Subscribe with Token
    url: "{{ base_url }}/ws"
    headers:
      Authorization: "Bearer {{ auth_token }}"
    websocket:
      message: |
        {
          "type": "subscribe",
          "channels": ["orders", "trades"]
        }
      wait_response: 5000
    assert:
      body:
        type: subscribed
```

### Subscription Pattern

Subscribe and receive events:

```yaml
name: Event Subscription Workflow

steps:
  - name: Subscribe to Orders
    url: wss://stream.example.com/ws
    websocket:
      message: |
        {
          "action": "subscribe",
          "channel": "orders",
          "filter": {"symbol": "BTC/USD"}
        }
      mode: send
      wait_response: 5000
    assert:
      body:
        status: subscribed

  - name: Receive Order Events
    url: wss://stream.example.com/ws
    websocket:
      mode: listen
      max_messages: 10
      wait_response: 30000
      ping_interval: 10
    extract:
      orders: body
```

### Request-Response Pattern

Multiple request-response interactions:

```yaml
name: WebSocket API Workflow

steps:
  - name: Create Resource
    url: wss://api.example.com/ws
    websocket:
      message: |
        {
          "id": "{uuid}",
          "method": "create",
          "params": {
            "name": "Test Resource",
            "type": "example"
          }
        }
      wait_response: 5000
    extract:
      resource_id: ".result.id"
    assert:
      body:
        - path: result.id
          exists: true

  - name: Read Resource
    url: wss://api.example.com/ws
    websocket:
      message: |
        {
          "id": "{uuid}",
          "method": "read",
          "params": {"id": "{{ resource_id }}"}
        }
      wait_response: 5000
    assert:
      body:
        - path: result.name
          equals: "Test Resource"

  - name: Delete Resource
    url: wss://api.example.com/ws
    websocket:
      message: |
        {
          "id": "{uuid}",
          "method": "delete",
          "params": {"id": "{{ resource_id }}"}
        }
      wait_response: 5000
    assert:
      body:
        - path: result.deleted
          equals: true
```

### Chained WebSocket Steps

Combine HTTP and WebSocket steps:

```yaml
name: Mixed Protocol Workflow

steps:
  # HTTP: Get API token
  - name: Get Access Token
    method: POST
    url: https://api.example.com/auth/token
    body:
      client_id: "{{ client_id }}"
      client_secret: "{{ client_secret }}"
    extract:
      access_token: body.access_token

  # WebSocket: Connect with token
  - name: WebSocket Connect
    url: wss://api.example.com/ws
    headers:
      Authorization: "Bearer {{ access_token }}"
    websocket:
      message: '{"type": "connect"}'
      wait_response: 5000
    extract:
      ws_session: ".session_id"

  # WebSocket: Subscribe to events
  - name: Subscribe
    url: wss://api.example.com/ws
    headers:
      Authorization: "Bearer {{ access_token }}"
      X-Session: "{{ ws_session }}"
    websocket:
      message: '{"type": "subscribe", "channel": "updates"}'
      wait_response: 5000

  # HTTP: Trigger an event
  - name: Create Order (triggers WebSocket event)
    method: POST
    url: https://api.example.com/orders
    headers:
      Authorization: "Bearer {{ access_token }}"
    body:
      product_id: "prod_123"
      quantity: 1
    extract:
      order_id: body.id

  # WebSocket: Listen for the order event
  - name: Receive Order Event
    url: wss://api.example.com/ws
    headers:
      Authorization: "Bearer {{ access_token }}"
      X-Session: "{{ ws_session }}"
    websocket:
      mode: listen
      max_messages: 1
      wait_response: 10000
    assert:
      body:
        - path: type
          equals: order_created
        - path: data.id
          equals: "{{ order_id }}"
```

### GraphQL Subscriptions

GraphQL over WebSocket (graphql-ws protocol):

```yaml
name: GraphQL Subscription Workflow

variables:
  graphql_endpoint: wss://api.example.com/graphql

steps:
  - name: Initialize Connection
    url: "{{ graphql_endpoint }}"
    websocket:
      subprotocol: graphql-ws
      message: '{"type": "connection_init", "payload": {}}'
      wait_response: 5000
    assert:
      body:
        type: connection_ack

  - name: Subscribe to Events
    url: "{{ graphql_endpoint }}"
    websocket:
      subprotocol: graphql-ws
      message: |
        {
          "id": "1",
          "type": "subscribe",
          "payload": {
            "query": "subscription { orderCreated { id status } }"
          }
        }
      mode: listen
      max_messages: 5
      wait_response: 60000
    extract:
      events: body
```

---

## Configuration Reference

### WebSocket Configuration Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `message` | string | - | Single text message to send |
| `messages` | array | - | Multiple messages to send (stream mode) |
| `binary` | string | - | Binary data (hex or base64 encoded) |
| `binary_mode` | string | "hex" | Binary encoding: "hex" or "base64" |
| `subprotocol` | string | - | WebSocket subprotocol to request |
| `mode` | string | "send" | Operation mode: "send", "listen", "stream" |
| `max_messages` | number | 1 | Maximum messages to receive (0 = unlimited) |
| `ping_interval` | number | - | Keep-alive ping interval in seconds |
| `wait_response` | number | - | Wait for response (milliseconds) |
| `compress` | boolean | false | Enable permessage-deflate compression |

### Step-Level Options

These standard workflow step options work with WebSocket steps:

| Field | Type | Description |
|-------|------|-------------|
| `headers` | object | HTTP headers for WebSocket handshake |
| `timeout` | string | Step timeout (e.g., "30s") |
| `insecure` | boolean | Skip TLS certificate verification |
| `extract` | object | Extract values from response |
| `assert` | object | Response assertions |
| `delay` | number | Delay before step (milliseconds) |
| `skip_if` | string | Conditional execution |
| `retries` | number | Retry on failure |

### URL Formats

All these URL formats are supported:

```yaml
# WebSocket URLs
url: ws://localhost:8080/ws
url: wss://api.example.com/ws

# HTTP URLs (auto-converted)
url: http://localhost:8080/ws   # → ws://
url: https://api.example.com/ws # → wss://

# With port
url: wss://api.example.com:443/ws

# With path
url: wss://api.example.com/api/v2/websocket
```

---

## See Also

- [Workflow Documentation](workflow.md) - Complete workflow reference
- [Scripting Reference](script.md) - Rune scripting for workflows
