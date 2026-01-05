# HTTP Client

QuicPulse supports HTTP/1.1, HTTP/2, and HTTP/3 (QUIC) protocols with comprehensive TLS/SSL configuration.

## Protocol Selection

### HTTP/1.1 (Default)

```bash
# Default behavior
quicpulse example.com/api

# Explicit HTTP/1.1
quicpulse --http-version 1.1 example.com/api

# Force HTTP/1.0
quicpulse --http-version 1.0 example.com/api
```

### HTTP/2

```bash
# Enable HTTP/2
quicpulse --http-version 2 example.com/api
```

HTTP/2 features:
- Multiplexed streams over single connection
- Header compression (HPACK)
- Server push support
- Binary framing

### HTTP/3 (QUIC)

```bash
# Enable HTTP/3
quicpulse --http3 example.com/api

# Or via version flag
quicpulse --http-version 3 example.com/api
```

HTTP/3 features:
- Based on QUIC transport protocol
- Reduced connection latency (0-RTT)
- Connection migration
- Independent streams (no head-of-line blocking)
- Built-in encryption

**Requirements:**
- HTTPS only (HTTP/3 requires TLS)
- Server must support QUIC/HTTP/3

---

## TLS/SSL Configuration

### Certificate Verification

```bash
# Default - verify server certificates
quicpulse https://example.com

# Disable verification (insecure)
quicpulse --verify no https://example.com

# Custom CA bundle
quicpulse --verify /path/to/ca-bundle.pem https://example.com
```

### TLS Version

```bash
# Minimum TLS 1.2 (recommended)
quicpulse --ssl tls1.2 https://example.com

# TLS 1.3 only
quicpulse --ssl tls1.3 https://example.com
```

Supported TLS versions:
- `tls1.2` - TLS 1.2 (minimum recommended)
- `tls1.3` - TLS 1.3 (latest)
- `auto` - Let system decide

**Note:** TLS 1.0 and 1.1 are deprecated and not supported.

### Client Certificates

For mutual TLS (mTLS) authentication:

```bash
# PEM certificate with separate key
quicpulse --cert /path/to/cert.pem --cert-key /path/to/key.pem https://example.com

# Encrypted private key
quicpulse --cert /path/to/cert.pem --cert-key /path/to/key.pem --cert-key-pass "password" https://example.com

# Combined certificate and key in single PEM
quicpulse --cert /path/to/combined.pem https://example.com
```

### Cipher Suites

Default cipher suites (via rustls):

**TLS 1.3:**
- `TLS_AES_256_GCM_SHA384`
- `TLS_AES_128_GCM_SHA256`
- `TLS_CHACHA20_POLY1305_SHA256`

**TLS 1.2:**
- `TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384`
- `TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384`
- `TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256`
- `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256`
- `TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256`
- `TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256`

---

## Proxy Configuration

### HTTP Proxy

```bash
# HTTP proxy
quicpulse --proxy http://proxy.example.com:8080 example.com/api

# HTTPS proxy
quicpulse --proxy https://proxy.example.com:8443 example.com/api

# Proxy with authentication
quicpulse --proxy http://user:pass@proxy.example.com:8080 example.com/api
```

### SOCKS Proxy

```bash
# SOCKS5 proxy
quicpulse --socks socks5://localhost:1080 example.com/api

# SOCKS5 with authentication
quicpulse --socks socks5://user:pass@localhost:1080 example.com/api

# SOCKS4 proxy
quicpulse --socks socks4://localhost:1080 example.com/api

# SOCKS5h (hostname resolution by proxy)
quicpulse --socks socks5h://localhost:1080 example.com/api
```

### Environment Variables

```bash
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080
export NO_PROXY=localhost,127.0.0.1,.internal.com

quicpulse example.com/api
```

---

## Network Options

### Timeouts

```bash
# Connection timeout in seconds
quicpulse --timeout 30 example.com/api

# Default: 30 seconds
```

### Redirects

```bash
# Follow redirects
quicpulse -F example.com/redirect

# Limit redirect count
quicpulse -F --max-redirects 5 example.com/redirect

# Show all intermediate responses
quicpulse -F --all example.com/redirect
```

### DNS Resolution

```bash
# Custom DNS resolution
quicpulse --resolve example.com:443:127.0.0.1 https://example.com/api

# Multiple resolutions
quicpulse --resolve example.com:443:127.0.0.1 --resolve api.example.com:443:127.0.0.2 https://example.com
```

### Interface Binding

```bash
# Bind to specific interface
quicpulse --interface eth0 example.com/api

# Bind to specific IP
quicpulse --local-address 192.168.1.100 example.com/api

# Specify local port range
quicpulse --local-port 40000-50000 example.com/api
```

### TCP Fast Open

```bash
# Enable TCP Fast Open (TFO)
quicpulse --tcp-fastopen example.com/api
```

---

## Unix Domain Sockets

Connect to services via Unix sockets:

```bash
# Docker API
quicpulse --unix-socket /var/run/docker.sock http://localhost/containers/json

# Nginx status
quicpulse --unix-socket /var/run/nginx.sock http://localhost/status
```

---

## Request Building

### Methods

```bash
# Explicit method
quicpulse GET example.com/resource
quicpulse POST example.com/resource
quicpulse PUT example.com/resource
quicpulse DELETE example.com/resource
quicpulse PATCH example.com/resource
quicpulse HEAD example.com/resource
quicpulse OPTIONS example.com/resource

# Method inferred from data
quicpulse example.com/api name=value    # POST (data present)
quicpulse example.com/api               # GET (no data)
```

### Headers

```bash
# Custom headers
quicpulse example.com Accept:application/json X-Custom:value

# Remove default header
quicpulse example.com User-Agent;

# Empty header value
quicpulse example.com X-Empty:
```

### URL Shorthand

```bash
# Localhost shorthand
quicpulse :3000/api              # http://localhost:3000/api
quicpulse :/api                  # http://localhost/api
quicpulse :8080                  # http://localhost:8080/

# Scheme defaulting
quicpulse example.com            # http://example.com
quicpulse --default-scheme https example.com  # https://example.com
```

### Path Handling

```bash
# Normalize path (default)
quicpulse example.com/../etc/passwd    # Normalized to /etc/passwd

# Keep path as-is
quicpulse --path-as-is example.com/../etc/passwd
```

---

## Content Encoding

### Request Compression

```bash
# Compress request body
quicpulse -x POST example.com/api data=large_content

# Force compression
quicpulse -xx POST example.com/api data=content
```

### Chunked Transfer

```bash
# Enable chunked transfer encoding
quicpulse --chunked POST example.com/api @large_file.json
```

---

## Response Handling

### Status Checking

```bash
# Exit with error on 4xx/5xx
quicpulse --check-status example.com/api
echo $?  # 0 for 2xx/3xx, non-zero for 4xx/5xx
```

### Streaming

```bash
# Stream response line by line
quicpulse -S example.com/stream

# Useful for Server-Sent Events, logs, etc.
```

### Maximum Headers

```bash
# Limit number of response headers
quicpulse --max-headers 100 example.com/api
```

---

## Offline Mode

Build and inspect requests without sending:

```bash
# Show what would be sent
quicpulse --offline POST example.com/api name=value

# Export as curl command
quicpulse --curl POST example.com/api name=value
```

---

## Connection Pooling

QuicPulse maintains connection pools for efficient HTTP communication:

- HTTP/1.1: Keep-alive connections reused
- HTTP/2: Multiplexed streams on single connection
- Connections automatically cleaned up after idle timeout

---

## User Agent

Default User-Agent string:
```
quicpulse/0.0.1
```

Override:
```bash
quicpulse example.com User-Agent:my-client/1.0
```

---

## Debug and Troubleshooting

### Verbose Output

```bash
# Show request details
quicpulse -v example.com/api

# Even more verbose (includes TLS details)
quicpulse -vv example.com/api

# Debug mode with full tracebacks
quicpulse --debug example.com/api
```

### Show Request

```bash
# Show request headers
quicpulse -p H example.com/api

# Show request headers and body
quicpulse -p HB POST example.com/api name=value

# Show everything
quicpulse -p HBhbm example.com/api
```

---

## Examples

### Complete Request with All Options

```bash
quicpulse -vv \
  --http-version 2 \
  --ssl tls1.3 \
  --cert /path/to/cert.pem \
  --cert-key /path/to/key.pem \
  --timeout 60 \
  -F --max-redirects 10 \
  --proxy http://proxy:8080 \
  POST https://api.example.com/resource \
  Authorization:"Bearer token" \
  Content-Type:application/json \
  name=value
```

### API Testing

```bash
# Quick API test
quicpulse --check-status api.example.com/health

# With assertions
quicpulse --assert-status 200 --assert-time "<500ms" api.example.com/health
```

### Download Large File

```bash
# Download with progress
quicpulse -d https://releases.example.com/file.tar.gz

# Resume interrupted download
quicpulse -d -c https://releases.example.com/file.tar.gz
```

---

## See Also

- [CLI Reference](cli-reference.md) - All command-line flags
- [Authentication](authentication.md) - Auth methods
- [SOCKS Proxy](socks-proxy.md) - SOCKS proxy details
- [Downloads & Uploads](downloads-uploads.md) - File transfers
