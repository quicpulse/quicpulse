# QuicPulse Architecture

This document describes the internal architecture and module structure of QuicPulse.

## Overview

QuicPulse is organized into 48 modules, each handling a specific aspect of HTTP client functionality. The codebase follows a modular design that separates concerns and allows for easy extension.

## Directory Structure

```
src/
├── main.rs                 # Entry point, signal handling
├── lib.rs                  # Library exports
├── core.rs                 # Main execution engine (80KB)
│
├── cli/                    # Command-line interface
│   ├── args.rs             # Argument definitions (clap)
│   ├── parser.rs           # Argument parsing
│   └── process.rs          # Post-processing logic
│
├── client/                 # HTTP client implementations
│   ├── mod.rs              # Client module exports
│   ├── http.rs             # HTTP/1.1 and HTTP/2 client
│   ├── http3.rs            # HTTP/3 (QUIC) client
│   ├── adapters.rs         # Connection adapters
│   ├── ssl.rs              # TLS/SSL configuration
│   └── unix_socket.rs      # Unix domain socket support
│
├── auth/                   # Authentication
│   ├── mod.rs              # Auth module exports
│   ├── aws.rs              # AWS SigV4 signing
│   ├── aws_config.rs       # AWS configuration
│   ├── aws_sso.rs          # AWS SSO integration
│   ├── aws_sts.rs          # AWS STS credentials
│   ├── azure.rs            # Azure CLI auth
│   ├── gcp.rs              # Google Cloud auth
│   ├── oauth2.rs           # OAuth 2.0 implementation
│   ├── oauth2_flows.rs     # OAuth flow implementations
│   └── netrc.rs            # .netrc file support
│
├── middleware/             # Request/response middleware
│   └── auth/               # Auth middleware
│       ├── mod.rs          # Middleware exports
│       ├── basic.rs        # Basic authentication
│       ├── bearer.rs       # Bearer token auth
│       ├── digest.rs       # Digest authentication
│       ├── apikey.rs       # API key auth
│       └── ntlm.rs         # NTLM/Negotiate/Kerberos
│
├── websocket/              # WebSocket protocol
│   ├── mod.rs              # WebSocket exports
│   ├── client.rs           # WebSocket client
│   ├── types.rs            # WebSocket types
│   ├── codec.rs            # Message encoding
│   ├── stream.rs           # Stream handling
│   └── interactive.rs      # Interactive REPL
│
├── grpc/                   # gRPC protocol
│   ├── mod.rs              # gRPC exports
│   ├── client.rs           # gRPC client
│   ├── reflection.rs       # Service reflection
│   └── codec.rs            # Protobuf codec
│
├── graphql/                # GraphQL protocol
│   ├── mod.rs              # GraphQL exports
│   ├── query.rs            # Query building
│   └── introspection.rs    # Schema introspection
│
├── pipeline/               # Workflow engine
│   ├── mod.rs              # Pipeline exports
│   ├── workflow.rs         # Workflow definitions
│   ├── runner.rs           # Workflow execution (131KB)
│   ├── assertions.rs       # Assertion handling
│   ├── dependency.rs       # Step dependency resolution
│   └── sharing.rs          # Workflow sharing
│
├── scripting/              # Scripting engine
│   ├── mod.rs              # Scripting exports
│   ├── runtime.rs          # Script runtime
│   └── modules/            # Built-in modules (23)
│       ├── mod.rs          # Module registry
│       ├── crypto.rs       # Crypto functions
│       ├── encoding.rs     # Encoding utilities
│       ├── json.rs         # JSON manipulation
│       ├── xml.rs          # XML parsing
│       ├── regex.rs        # Regular expressions
│       ├── url.rs          # URL manipulation
│       ├── date.rs         # Date/time functions
│       ├── cookie.rs       # Cookie handling
│       ├── jwt.rs          # JWT decoding
│       ├── schema.rs       # JSON Schema validation
│       ├── http.rs         # HTTP constants
│       ├── assert.rs       # Assertions
│       ├── env.rs          # Environment access
│       ├── faker.rs        # Fake data generation
│       ├── prompt.rs       # User input
│       ├── fs.rs           # File system (sandboxed)
│       ├── store.rs        # Key-value store
│       ├── console.rs      # Logging
│       ├── system.rs       # System utilities
│       └── request.rs      # HTTP requests
│
├── sessions/               # Session management
│   ├── mod.rs              # Session exports
│   ├── session.rs          # Session handling
│   └── cookies.rs          # Cookie management
│
├── config/                 # Configuration
│   ├── mod.rs              # Config exports
│   └── config.rs           # Config file handling
│
├── bench/                  # Benchmarking
│   ├── mod.rs              # Bench exports
│   ├── runner.rs           # Benchmark runner
│   └── stats.rs            # Statistics (HDR histogram)
│
├── fuzz/                   # Security fuzzing
│   ├── mod.rs              # Fuzz exports
│   ├── runner.rs           # Fuzz runner
│   └── payloads.rs         # Payload generation
│
├── mock/                   # Mock server
│   ├── mod.rs              # Mock exports
│   └── server.rs           # HTTP mock server
│
├── plugins/                # Plugin system
│   ├── mod.rs              # Plugin exports
│   ├── loader.rs           # Plugin loading
│   └── registry.rs         # Plugin registry
│
├── output/                 # Output formatting
│   ├── mod.rs              # Output exports
│   ├── formatters/         # Format implementations
│   │   ├── colors.rs       # Syntax highlighting
│   │   └── json.rs         # JSON formatting
│   ├── lexers/             # Syntax lexers
│   ├── streams.rs          # Output streams
│   ├── codec.rs            # Output codecs
│   ├── terminal.rs         # Terminal colors
│   ├── pager.rs            # Pager support
│   └── writer.rs           # Output writer
│
├── har/                    # HAR support
│   ├── mod.rs              # HAR exports
│   └── replay.rs           # HAR replay
│
├── openapi/                # OpenAPI import
│   ├── mod.rs              # OpenAPI exports
│   └── generator.rs        # Workflow generation
│
├── k8s/                    # Kubernetes support
│   ├── mod.rs              # K8s exports
│   ├── parser.rs           # URL parsing
│   └── portforward.rs      # Port-forward manager
│
├── downloads/              # Download handling
│   ├── mod.rs              # Download exports
│   └── downloader.rs       # Download manager
│
├── uploads/                # Upload handling
│   ├── mod.rs              # Upload exports
│   └── chunked.rs          # Chunked uploads
│
├── input/                  # Input parsing
│   ├── mod.rs              # Input exports
│   └── parser.rs           # Request item parsing
│
├── filter/                 # Data filtering
│   ├── mod.rs              # Filter exports
│   └── jq.rs               # JQ implementation
│
├── request/                # Request building
│   └── mod.rs              # Request construction
│
├── models/                 # Data models
│   └── types.rs            # Core type definitions
│
├── context/                # Execution context
│   └── mod.rs              # Environment handling
│
├── devexp/                 # Developer experience
│   └── mod.rs              # Curl import, .env support
│
├── table/                  # Table formatting
│   └── mod.rs              # ASCII table output
│
├── http/                   # HTTP utilities
│   └── mod.rs              # Headers, methods, status
│
├── errors.rs               # Error types
├── status.rs               # Exit status codes
├── signals.rs              # Interrupt handling
├── encoding.rs             # Content encoding
├── mime.rs                 # MIME type detection
├── magic.rs                # File type detection
├── json.rs                 # JSON utilities
├── cookies.rs              # Cookie utilities
├── strings.rs              # String utilities
└── utils.rs                # General utilities
```

## Core Components

### 1. Entry Point (`main.rs`)

The entry point sets up:
- Ctrl+C signal handler for graceful shutdown
- Command-line argument collection
- Environment initialization
- Execution via `core::run()`

### 2. Core Engine (`core.rs`)

The largest module (80KB) orchestrating:
- Request building and execution
- Protocol detection (HTTP, WebSocket, gRPC, GraphQL)
- Response processing and output
- Session management
- Plugin execution

### 3. CLI (`cli/`)

Command-line interface using `clap`:
- **args.rs**: 170+ flag definitions organized by category
- **parser.rs**: Argument parsing and validation
- **process.rs**: URL normalization, method inference

### 4. HTTP Client (`client/`)

Multi-protocol HTTP client:
- **http.rs**: HTTP/1.1 and HTTP/2 via `reqwest`
- **http3.rs**: HTTP/3 over QUIC via `h3-quinn`
- **ssl.rs**: TLS configuration and certificates
- **unix_socket.rs**: Unix domain socket connections

### 5. Workflow Engine (`pipeline/`)

Multi-step automation:
- **workflow.rs**: YAML/TOML workflow definitions
- **runner.rs**: Step execution, variable extraction, assertions
- **dependency.rs**: Topological sorting for step dependencies
- **assertions.rs**: Status, header, body, latency checks

### 6. Scripting (`scripting/`)

Embedded scripting support:
- **runtime.rs**: Rune and JavaScript execution
- **modules/**: 23 built-in modules for crypto, encoding, HTTP, etc.

## Data Flow

```
CLI Arguments
     │
     ▼
┌─────────────────┐
│   core::run()   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│ Request Builder │────▶│ Authentication  │
└────────┬────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│   HTTP Client   │────▶│  TLS/Proxy      │
└────────┬────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐
│ Response Handler│
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│ Output Formatter│────▶│ Pager/File      │
└─────────────────┘     └─────────────────┘
```

## Extension Points

### 1. Plugins
- Pre-request hooks
- Post-response hooks
- Custom authentication
- Output formatting

### 2. Scripting Modules
- Add new modules in `scripting/modules/`
- Register in module registry

### 3. Authentication Methods
- Add providers in `auth/` or `middleware/auth/`
- Register in auth handling logic

### 4. Output Formatters
- Add formatters in `output/formatters/`
- Add lexers for syntax highlighting

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `reqwest` | HTTP/1.1 and HTTP/2 client |
| `h3-quinn` | HTTP/3 over QUIC |
| `tokio` | Async runtime |
| `clap` | CLI argument parsing |
| `serde` | Serialization |
| `rune` | Scripting language |
| `tonic` | gRPC client |
| `tokio-tungstenite` | WebSocket client |
| `indicatif` | Progress bars |
| `hdrhistogram` | Latency statistics |

## Testing

Tests are organized by module:
- Unit tests: `#[cfg(test)]` blocks in source files
- Integration tests: `tests/` directory
- Workflow tests: `tests/test_workflow.rs`

Run tests with:
```bash
cargo test
```

## Performance Considerations

1. **Connection Pooling**: HTTP client reuses connections
2. **Async I/O**: All network operations are async via Tokio
3. **Streaming**: Large responses can be streamed
4. **HDR Histogram**: Accurate percentile calculation for benchmarks
5. **Lazy Initialization**: Heavy resources initialized on demand
