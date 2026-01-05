# QuicPulse Documentation

Welcome to the QuicPulse documentation. QuicPulse is a powerful, feature-rich HTTP client for the command line, written in Rust.

## Quick Links

- [README](../README.md) - Quick start guide and overview
- [CLI Reference](cli-reference.md) - Complete command-line reference
- [Configuration](configuration.md) - Configuration file reference

---

## Getting Started

| Document | Description |
|----------|-------------|
| [README](../README.md) | Installation, quick start, and feature overview |
| [CLI Reference](cli-reference.md) | All 170+ command-line flags |
| [Configuration](configuration.md) | Config file options |

---

## Core Features

### HTTP Client
| Document | Description |
|----------|-------------|
| [HTTP Client](http-client.md) | HTTP/1.1, HTTP/2, HTTP/3 (QUIC), TLS, proxies |
| [Authentication](authentication.md) | Basic, Digest, Bearer, AWS, OAuth2, GCP, Azure |
| [Downloads & Uploads](downloads-uploads.md) | File transfers, resumable downloads, multipart |
| [Data Filtering](filtering.md) | JQ expressions, table/CSV output |

### Protocols
| Document | Description |
|----------|-------------|
| [GraphQL](workflow-graphql.md) | GraphQL queries and introspection |
| [gRPC](workflow-grpc.md) | gRPC calls with proto files |
| [WebSocket](workflow-websocket.md) | WebSocket connections and streaming |

---

## Workflows & Automation

| Document | Description |
|----------|-------------|
| [Workflow Guide](workflow.md) | Multi-step API automation |
| [Scripting](script.md) | Rune and JavaScript scripting |
| [Sessions](workflow-sessions.md) | Cookie and state persistence |
| [Output Control](workflow-output.md) | Response formatting and filtering |

### Workflow Features
| Document | Description |
|----------|-------------|
| [Benchmarking](workflow-benchmarking.md) | Load testing in workflows |
| [Fuzzing](workflow-fuzzing.md) | Security testing in workflows |
| [Downloads](workflow-downloads.md) | File downloads in workflows |
| [Uploads](workflow-uploads.md) | File uploads in workflows |
| [HAR Replay](workflow-har.md) | Browser recording replay |
| [OpenAPI Import](workflow-openapi.md) | Generate workflows from specs |
| [Plugins](workflow-plugins.md) | Plugin usage in workflows |

---

## Testing & Security

| Document | Description |
|----------|-------------|
| [Assertions](assertions.md) | Status, header, body, and time assertions |
| [Fuzzing](workflow-fuzzing.md) | Security vulnerability scanning |
| [Benchmarking](workflow-benchmarking.md) | Performance and load testing |

---

## Integrations

| Document | Description |
|----------|-------------|
| [Kubernetes](kubernetes.md) | Native k8s:// URL support |
| [OpenAPI Import](workflow-openapi.md) | Import from OpenAPI/Swagger specs |
| [HAR Replay](workflow-har.md) | Replay browser DevTools recordings |
| [SOCKS Proxy](socks-proxy.md) | SOCKS4/5 proxy configuration |

---

## Developer Tools

| Document | Description |
|----------|-------------|
| [Mock Server](mock-server.md) | Built-in HTTP mock server |
| [Plugins](plugins.md) | Plugin ecosystem and development |
| [Scripting](script.md) | Embedded Rune/JavaScript scripting |
| [Pager](pager.md) | Response paging configuration |

---

## Reference

| Document | Description |
|----------|-------------|
| [Architecture](architecture.md) | Codebase structure and modules |
| [CLI Reference](cli-reference.md) | Complete flag reference |
| [Configuration](configuration.md) | Config file reference |

---

## Feature Matrix

| Feature | CLI | Workflow | Notes |
|---------|-----|----------|-------|
| HTTP/1.1 | Yes | Yes | Default |
| HTTP/2 | Yes | Yes | `--http-version=2` |
| HTTP/3 (QUIC) | Yes | Yes | `--http3` |
| GraphQL | Yes | Yes | `-G` flag |
| gRPC | Yes | Yes | `--grpc` flag |
| WebSocket | Yes | Yes | `--ws` or ws:// URL |
| Basic Auth | Yes | Yes | `-a user:pass` |
| OAuth 2.0 | Yes | Yes | Multiple flows |
| AWS SigV4 | Yes | Yes | `-A aws-sigv4` |
| Sessions | Yes | Yes | `--session` |
| Assertions | Yes | Yes | `--assert-*` |
| Fuzzing | Yes | Yes | `--fuzz` |
| Benchmarking | Yes | Yes | `--bench` |
| Mock Server | Yes | - | `--mock` |
| Plugins | Yes | Yes | `--plugin` |
| Scripting | - | Yes | Rune/JavaScript |

---

## Version

This documentation is for QuicPulse v0.0.1.
