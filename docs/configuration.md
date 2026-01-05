# Configuration

QuicPulse can be configured via a TOML configuration file and environment variables.

## Configuration File

The configuration file is located at:

| Platform | Location |
|----------|----------|
| Linux | `~/.config/quicpulse/config.toml` |
| macOS | `~/Library/Application Support/quicpulse/config.toml` |
| Windows | `%APPDATA%\quicpulse\config.toml` |

### Full Configuration Example

```toml
# ~/.config/quicpulse/config.toml

[defaults]
# Default CLI options applied to all requests
options = [
    "--style", "monokai",
    "--sorted",
    "--follow",
    "--timeout", "30"
]

# Disable update check notifications
disable_update_warnings = false

[hooks]
# Run script before each request
pre_request = "hooks/pre_request.rune"

# Run script after each response
post_request = "hooks/post_request.rune"

# Run script on request failure
on_error = "hooks/on_error.rune"

# Workflow-level hooks
on_workflow_start = "hooks/workflow_start.rune"
on_workflow_end = "hooks/workflow_end.rune"

# Inline hooks (alternative to file-based hooks)
[hooks.pre_request_inline]
code = '''
pub fn main(ctx) {
    ctx.headers["X-Request-ID"] = uuid::v4();
}
'''
```

---

## Default Options

Set default CLI options that apply to all requests:

```toml
[defaults]
options = [
    "--style", "monokai",     # Color scheme
    "--sorted",                # Sort headers/JSON
    "--follow",                # Follow redirects
    "--timeout", "30",         # 30 second timeout
    "--verify", "yes"          # Verify SSL certs
]
```

These options are prepended to the command line, so explicit flags override them.

---

## Hooks

Hooks allow you to run scripts at specific points in the request lifecycle.

### Hook Types

| Hook | When | Use Case |
|------|------|----------|
| `pre_request` | Before each HTTP request | Add headers, modify request, logging |
| `post_request` | After each HTTP response | Metrics, response validation, logging |
| `on_error` | On request failure | Error handling, alerting |
| `on_workflow_start` | Before workflow execution | Setup, initialization |
| `on_workflow_end` | After workflow completion | Cleanup, reporting |

### File-Based Hooks

```toml
[hooks]
# Relative paths are resolved from config directory
pre_request = "hooks/pre_request.rune"

# Absolute paths also supported
post_request = "/home/user/scripts/log_response.rune"
```

### Inline Hooks

```toml
[hooks.pre_request_inline]
code = '''
pub fn main(ctx) {
    // Add timestamp to all requests
    ctx.headers["X-Timestamp"] = system::timestamp();
}
'''

[hooks.post_request_inline]
code = '''
pub fn main(ctx, response) {
    console::log(`${response.status} ${ctx.url}`);
}
'''
```

### Hook Context

Hooks receive context objects with request/response data:

**Pre-request hook:**
```rust
pub fn main(ctx) {
    // ctx.method - HTTP method
    // ctx.url - Request URL
    // ctx.headers - Request headers (modifiable)
    // ctx.body - Request body (modifiable)
}
```

**Post-request hook:**
```rust
pub fn main(ctx, response) {
    // ctx - Same as pre-request
    // response.status - Status code
    // response.headers - Response headers
    // response.body - Response body
    // response.time - Response time in ms
}
```

---

## Directory Structure

QuicPulse creates the following directory structure:

```
~/.config/quicpulse/
├── config.toml              # Main configuration
├── sessions/                # Named sessions
│   ├── myapi.json          # Session data
│   └── production.json
├── plugins/                 # Installed plugins
│   └── my-plugin/
├── oauth_tokens/           # Cached OAuth tokens
│   └── api.example.com.json
├── hooks/                   # Hook scripts (optional)
│   ├── pre_request.rune
│   └── post_request.rune
└── version_info.json       # Version check cache
```

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `QUICPULSE_CONFIG_DIR` | Override config directory location |
| `NO_COLOR` | Disable colors (set to any value) |
| `PAGER` | Pager command for `--pager` flag |
| `HTTP_PROXY` | HTTP proxy URL |
| `HTTPS_PROXY` | HTTPS proxy URL |
| `ALL_PROXY` | Proxy for all protocols |
| `NO_PROXY` | Hosts to bypass proxy |
| `AWS_ACCESS_KEY_ID` | AWS credentials |
| `AWS_SECRET_ACCESS_KEY` | AWS credentials |
| `AWS_SESSION_TOKEN` | AWS temporary credentials |
| `AWS_PROFILE` | AWS profile name |
| `AWS_REGION` | AWS region |
| `GOOGLE_APPLICATION_CREDENTIALS` | GCP service account |

---

## Per-Project Configuration

QuicPulse looks for configuration in the current directory:

```
project/
├── .quicpulse/
│   ├── config.toml         # Project-specific config
│   └── sessions/           # Project sessions
├── .env                    # Environment variables
└── quicpulse.toml          # Alternative config location
```

Project configuration is merged with global configuration, with project settings taking precedence.

---

## .env File Support

QuicPulse automatically loads `.env` files from the current directory:

```bash
# .env
API_BASE_URL=https://api.example.com
API_TOKEN=secret-token
DEBUG=true
```

Use in requests:

```bash
quicpulse $API_BASE_URL/users Authorization:Bearer\ $API_TOKEN
```

Or in workflows:

```yaml
steps:
  - name: Get users
    request:
      url: "{{ env.API_BASE_URL }}/users"
      headers:
        Authorization: "Bearer {{ env.API_TOKEN }}"
```

Disable automatic loading:

```bash
quicpulse --no-env example.com/api
```

Load specific file:

```bash
quicpulse --env-file .env.production example.com/api
```

---

## Sessions Directory

Named sessions are stored in the sessions directory:

```json
// ~/.config/quicpulse/sessions/myapi.json
{
  "cookies": [
    {
      "name": "session_id",
      "value": "abc123",
      "domain": "api.example.com",
      "path": "/",
      "http_only": true,
      "secure": true,
      "expires": 1735689600
    }
  ],
  "auth": {
    "type": "bearer",
    "token": "eyJhbGciOiJIUzI1NiIs..."
  },
  "headers": {
    "X-Custom-Header": "value"
  }
}
```

---

## Plugin Configuration

Configure plugins in the main config file:

```toml
[plugins]
# Enable specific plugins
enabled = ["auth-helper", "request-logger"]

# Plugin-specific settings
[plugins.auth-helper]
provider = "okta"
domain = "mycompany.okta.com"

[plugins.request-logger]
output = "/var/log/quicpulse.log"
format = "json"
```

---

## Output Configuration

Configure default output behavior:

```toml
[output]
# Default color style
style = "monokai"

# Sort headers and JSON keys
sorted = true

# JSON formatting
json_indent = 4
json_sort_keys = true

# XML formatting
xml_indent = 2

# Pager settings
pager = "less -R"
auto_pager = true
auto_pager_threshold = 100  # Lines before auto-paging
```

---

## Proxy Configuration

Configure proxies in the config file:

```toml
[proxy]
http = "http://proxy.example.com:8080"
https = "http://proxy.example.com:8080"

# Hosts to bypass proxy
no_proxy = ["localhost", "127.0.0.1", ".internal.com"]

# SOCKS proxy
socks = "socks5://localhost:1080"
```

---

## SSL/TLS Configuration

Configure default SSL settings:

```toml
[ssl]
# Minimum TLS version
min_version = "tls1.2"

# Custom CA bundle
ca_bundle = "/etc/ssl/certs/ca-certificates.crt"

# Client certificate
cert = "/path/to/client.pem"
cert_key = "/path/to/client.key"

# Verify server certificates (default: true)
verify = true
```

---

## Example Configurations

### Development Configuration

```toml
# ~/.config/quicpulse/config.toml

[defaults]
options = [
    "--style", "monokai",
    "--verify", "no",      # Disable cert verification for local dev
    "--follow",
    "--verbose"
]

[hooks.post_request_inline]
code = '''
pub fn main(ctx, response) {
    if response.status >= 400 {
        console::error(`ERROR: ${response.status} on ${ctx.url}`);
    }
}
'''
```

### Production Configuration

```toml
# ~/.config/quicpulse/config.toml

[defaults]
options = [
    "--verify", "yes",
    "--timeout", "30",
    "--check-status"
]
disable_update_warnings = true

[ssl]
min_version = "tls1.2"
ca_bundle = "/etc/ssl/certs/ca-certificates.crt"

[proxy]
https = "http://corporate-proxy.example.com:8080"
no_proxy = ["localhost", ".internal.example.com"]
```

### CI/CD Configuration

```toml
# Used in CI pipelines

[defaults]
options = [
    "--no-color",
    "--quiet",
    "--check-status",
    "--timeout", "60"
]

[output]
log_format = "json"
```

---

## See Also

- [CLI Reference](cli-reference.md) - All command-line flags
- [Scripting](script.md) - Writing hook scripts
- [Plugins](plugins.md) - Plugin system
- [Sessions](workflow-sessions.md) - Session management
