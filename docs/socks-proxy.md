# SOCKS Proxy Reference

Route HTTP requests through SOCKS4/SOCKS5 proxy servers.

## Overview

QuicPulse supports routing requests through SOCKS proxies, enabling:

- Bypassing network restrictions
- Routing through Tor network
- SSH tunneling via SOCKS
- Testing from different network locations
- Enhanced privacy and anonymity

## Quick Start

```bash
# SOCKS5 proxy (most common)
quicpulse --socks socks5://localhost:1080 httpbin.org/ip

# SOCKS4 proxy
quicpulse --socks socks4://localhost:1080 httpbin.org/ip

# Simple host:port (defaults to SOCKS5)
quicpulse --socks localhost:1080 httpbin.org/ip

# Via --proxy flag
quicpulse --proxy socks5://localhost:1080 httpbin.org/ip
```

## CLI Options

| Option | Description |
|--------|-------------|
| `--socks <URL>` | SOCKS proxy URL |
| `--socks-proxy <URL>` | Alias for `--socks` |
| `--proxy <URL>` | Generic proxy (supports SOCKS URLs) |

## Proxy URL Formats

### Supported Protocols

| Protocol | Description | Port |
|----------|-------------|------|
| `socks4://` | SOCKS4 protocol | 1080 |
| `socks4a://` | SOCKS4a with remote DNS | 1080 |
| `socks5://` | SOCKS5 protocol | 1080 |
| `socks5h://` | SOCKS5 with remote DNS | 1080 |

### URL Syntax

```
socks5://[user:password@]host[:port]
```

| Component | Description | Default |
|-----------|-------------|---------|
| protocol | SOCKS version | socks5 |
| user:password | Authentication | - |
| host | Proxy hostname | - |
| port | Proxy port | 1080 |

### Examples

```bash
# Standard SOCKS5
quicpulse --socks socks5://127.0.0.1:1080 example.com

# SOCKS5 with authentication
quicpulse --socks socks5://user:pass@proxy.example.com:1080 example.com

# SOCKS5 with remote DNS resolution
quicpulse --socks socks5h://localhost:1080 example.com

# IPv6 proxy address
quicpulse --socks "socks5://[::1]:1080" example.com

# Simple format (defaults to SOCKS5)
quicpulse --socks localhost:1080 example.com
```

## SOCKS Protocol Differences

### SOCKS4 vs SOCKS5

| Feature | SOCKS4 | SOCKS5 |
|---------|--------|--------|
| Authentication | None | Username/Password |
| IPv6 Support | No | Yes |
| UDP Support | No | Yes |
| Remote DNS | SOCKS4a only | socks5h |

### DNS Resolution

| Protocol | DNS Resolution |
|----------|---------------|
| `socks4://` | Local (client-side) |
| `socks4a://` | Remote (proxy-side) |
| `socks5://` | Local (client-side) |
| `socks5h://` | Remote (proxy-side) |

**Recommendation:** Use `socks5h://` for privacy to prevent DNS leaks.

## Authentication

### Username/Password

```bash
# Include credentials in URL
quicpulse --socks socks5://myuser:mypass@proxy.example.com:1080 example.com

# Note: Credentials are redacted in logs for security
```

### Security Note

Credentials in command line may be visible in process lists. For production use, consider:

1. Environment variables
2. Configuration files with restricted permissions
3. SSH tunnels without embedded credentials

## Common Use Cases

### SSH Tunnel

Create a SOCKS proxy through SSH:

```bash
# Terminal 1: Create SSH tunnel
ssh -D 1080 user@remote-server

# Terminal 2: Use QuicPulse through tunnel
quicpulse --socks localhost:1080 httpbin.org/ip
```

### Tor Network

Route requests through Tor:

```bash
# Start Tor (listens on port 9050)
tor

# Use Tor as SOCKS proxy
quicpulse --socks socks5h://localhost:9050 check.torproject.org
```

**Important:** Use `socks5h://` with Tor to prevent DNS leaks.

### Corporate Proxy

Route through corporate SOCKS proxy:

```bash
quicpulse --socks socks5://proxy.corp.example.com:1080 api.example.com/endpoint
```

### Testing from Different Locations

Use proxy servers in different regions:

```bash
# US proxy
quicpulse --socks socks5://us-proxy.example.com:1080 httpbin.org/ip

# EU proxy
quicpulse --socks socks5://eu-proxy.example.com:1080 httpbin.org/ip
```

## Combining with Other Options

### With HTTP Proxy

Use both SOCKS and HTTP proxies:

```bash
quicpulse \
  --socks socks5://localhost:1080 \
  --proxy http:http://http-proxy:8080 \
  example.com
```

### With TLS Options

```bash
quicpulse \
  --socks socks5://localhost:1080 \
  --verify=no \
  https://self-signed.example.com
```

### With Authentication

```bash
quicpulse \
  --socks socks5://localhost:1080 \
  -a user:password \
  api.example.com/secure
```

### In Workflows

```yaml
# workflow.yaml
name: Proxied Tests
proxy:
  socks: socks5://localhost:1080

steps:
  - name: Check IP
    method: GET
    url: https://httpbin.org/ip
```

## Configuration File

Add default SOCKS proxy to config:

```toml
# ~/.config/quicpulse/config.toml
[defaults]
socks_proxy = "socks5://localhost:1080"
```

## Environment Variables

Set proxy via environment:

```bash
# Set SOCKS proxy
export SOCKS_PROXY=socks5://localhost:1080
export ALL_PROXY=socks5://localhost:1080

# Use with QuicPulse
quicpulse httpbin.org/ip
```

## Troubleshooting

### Connection Refused

```
Error: Failed to connect to SOCKS proxy
```

1. Verify proxy server is running
2. Check hostname and port
3. Verify firewall allows connection

### Authentication Failed

```
Error: SOCKS authentication failed
```

1. Verify username and password
2. Check proxy supports authentication method
3. Ensure URL encoding for special characters

### DNS Resolution Issues

```
Error: Could not resolve hostname
```

1. Use `socks5h://` for remote DNS resolution
2. Check local DNS configuration
3. Verify proxy can reach target host

### Connection Timeout

```
Error: SOCKS proxy connection timed out
```

1. Increase timeout with `--timeout`
2. Check network connectivity to proxy
3. Verify proxy server is not overloaded

### TLS Errors Through Proxy

```
Error: SSL/TLS handshake failed
```

1. Verify target supports TLS
2. Check certificate validity
3. Try `--verify=no` for testing

## Security Considerations

1. **Credential Security**
   - Avoid embedding passwords in scripts
   - Use environment variables or secure config files
   - Consider SSH key-based tunnels

2. **DNS Privacy**
   - Use `socks5h://` to prevent DNS leaks
   - Important for Tor and privacy-focused use

3. **Trust**
   - Only use trusted proxy servers
   - Proxies can inspect unencrypted traffic
   - Use HTTPS for sensitive data

4. **Logging**
   - QuicPulse redacts credentials in output
   - Proxy servers may log connections

## API Reference

### Proxy URL Parsing

```rust
// Supported proxy URL formats
"socks4://host:port"
"socks4a://host:port"
"socks5://host:port"
"socks5h://host:port"
"socks5://user:pass@host:port"
"host:port"  // Defaults to socks5://
```

### CLI Argument

```rust
#[arg(long = "socks", alias = "socks-proxy", value_name = "URL")]
pub socks_proxy: Option<SensitiveUrl>
```

---

See also:
- [README.md](../README.md) - CLI reference
- [workflow.md](workflow.md) - Workflow reference
