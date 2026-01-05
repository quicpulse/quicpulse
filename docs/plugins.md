# Plugin Ecosystem Reference

Extend QuicPulse functionality with plugins.

## Overview

QuicPulse supports a plugin ecosystem for extending functionality through:

- **Script plugins**: Rune or shell scripts
- **Binary plugins**: Compiled executables
- **Hook system**: Intercept request/response lifecycle
- **Registry**: Discover and install community plugins

## Quick Start

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
```

## CLI Options

| Option | Description |
|--------|-------------|
| `--plugin-list` | List installed plugins |
| `--plugin-install <NAME>` | Install plugin from registry |
| `--plugin-uninstall <NAME>` | Remove installed plugin |
| `--plugin-search <QUERY>` | Search plugin registry |
| `--plugin-update` | Update all installed plugins |
| `--plugin-dir <PATH>` | Custom plugin directory |
| `--plugins <LIST>` | Enable specific plugins for request |

## Plugin Locations

Plugins are loaded from these directories (in order):

1. `~/.config/quicpulse/plugins/` - User plugins
2. `/usr/local/share/quicpulse/plugins/` - System plugins
3. `./plugins/` - Project-local plugins
4. Custom directory via `--plugin-dir`

## Plugin Structure

### Directory Layout

```
plugins/
└── my-plugin/
    ├── manifest.toml       # Plugin metadata (required)
    ├── main.rune           # Script entry point
    └── README.md           # Documentation
```

### Manifest File

```toml
# manifest.toml
name = "my-plugin"
version = "1.0.0"
description = "My custom plugin"
author = "Your Name"
license = "MIT"

# Entry point (script or binary)
entry = "main.rune"
type = "script"  # or "binary"

# Supported hooks
hooks = ["pre_request", "post_response"]

# Dependencies (optional)
dependencies = ["crypto", "json"]

# Configuration schema (optional)
[config]
api_key = { type = "string", required = true }
timeout = { type = "integer", default = 30 }
```

### Plugin Types

| Type | Entry Point | Description |
|------|-------------|-------------|
| `script` | `.rune` file | Rune script plugin |
| `binary` | Executable | Compiled binary plugin |
| `shell` | `.sh` file | Shell script plugin |

## Hook System

### Available Hooks

| Hook | Trigger | Purpose |
|------|---------|---------|
| `pre_request` | Before sending request | Modify request |
| `post_response` | After receiving response | Process response |
| `on_error` | On request error | Handle errors |
| `auth` | Before authentication | Custom auth |
| `format` | Before output formatting | Custom formatting |
| `validate` | After response | Custom validation |

### Hook Context

Hooks receive a context object with request/response data:

```rune
// pre_request hook
pub fn pre_request(ctx) {
    // Access request data
    let method = ctx.request.method;
    let url = ctx.request.url;
    let headers = ctx.request.headers;
    let body = ctx.request.body;

    // Modify request
    ctx.request.headers["X-Custom"] = "value";

    // Return modified context
    ctx
}
```

```rune
// post_response hook
pub fn post_response(ctx) {
    // Access response data
    let status = ctx.response.status;
    let headers = ctx.response.headers;
    let body = ctx.response.body;

    // Log or process
    if status >= 400 {
        console::warn(`Request failed with ${status}`);
    }

    ctx
}
```

### Hook Results

Hooks can return different results:

```rune
// Continue normally
return HookResult::Ok(ctx);

// Stop processing
return HookResult::Stop("Reason to stop");

// Return error
return HookResult::Error("Error message");

// Modify body
return HookResult::WithBody(ctx, new_body);

// Modify URL
return HookResult::WithUrl(ctx, new_url);

// Modify headers
return HookResult::WithHeaders(ctx, new_headers);
```

## Writing Plugins

### Script Plugin (Rune)

```rune
// main.rune - JWT Authentication Plugin

// Hook: Called before each request
pub fn pre_request(ctx) {
    // Get config values
    let secret = ctx.config.get("secret")?;
    let algorithm = ctx.config.get("algorithm").unwrap_or("HS256");

    // Generate JWT token
    let payload = #{
        "sub": ctx.config.get("user_id")?,
        "iat": date::timestamp(),
        "exp": date::timestamp() + 3600,
    };

    let token = jwt::encode(payload, secret, algorithm);

    // Add Authorization header
    ctx.request.headers["Authorization"] = `Bearer ${token}`;

    HookResult::Ok(ctx)
}

// Hook: Called after each response
pub fn post_response(ctx) {
    // Verify response JWT if present
    if let Some(token) = ctx.response.headers.get("X-JWT-Response") {
        let secret = ctx.config.get("secret")?;
        match jwt::decode(token, secret) {
            Ok(claims) => {
                ctx.vars["jwt_claims"] = claims;
            }
            Err(e) => {
                console::warn(`Invalid JWT in response: ${e}`);
            }
        }
    }

    HookResult::Ok(ctx)
}
```

### Binary Plugin

Binary plugins receive JSON via stdin and output JSON to stdout:

```json
// Input (stdin)
{
    "hook": "pre_request",
    "context": {
        "request": {
            "method": "GET",
            "url": "https://api.example.com/users",
            "headers": {},
            "body": null
        },
        "config": {
            "api_key": "secret123"
        }
    }
}
```

```json
// Output (stdout)
{
    "result": "ok",
    "context": {
        "request": {
            "method": "GET",
            "url": "https://api.example.com/users",
            "headers": {
                "X-API-Key": "secret123"
            },
            "body": null
        }
    }
}
```

### Shell Plugin

```bash
#!/bin/bash
# main.sh - Simple logging plugin

# Read JSON input
INPUT=$(cat)

# Extract hook type
HOOK=$(echo "$INPUT" | jq -r '.hook')

case "$HOOK" in
    pre_request)
        URL=$(echo "$INPUT" | jq -r '.context.request.url')
        METHOD=$(echo "$INPUT" | jq -r '.context.request.method')
        echo "[$(date)] $METHOD $URL" >> request.log
        ;;
    post_response)
        STATUS=$(echo "$INPUT" | jq -r '.context.response.status')
        echo "[$(date)] Response: $STATUS" >> request.log
        ;;
esac

# Return unchanged context
echo "$INPUT" | jq '{result: "ok", context: .context}'
```

## Plugin Configuration

### Per-Request Configuration

```bash
# Enable plugins with configuration
quicpulse --plugins "jwt-auth:secret=mysecret" api.example.com/endpoint
```

### Configuration File

```toml
# ~/.config/quicpulse/config.toml
[plugins]
enabled = ["jwt-auth", "request-logger"]

[plugins.jwt-auth]
secret = "my-secret-key"
algorithm = "HS256"

[plugins.request-logger]
output = "~/logs/requests.log"
format = "json"
```

### Workflow Configuration

```yaml
# workflow.yaml
plugins:
  jwt-auth:
    enabled: true
    config:
      secret: "{{ jwt_secret }}"
      user_id: "{{ user_id }}"

steps:
  - name: Authenticated Request
    method: GET
    url: /api/secure
    # jwt-auth plugin adds Authorization header
```

## Plugin Registry

### Searching Plugins

```bash
# Search by name
quicpulse --plugin-search jwt

# Search by category
quicpulse --plugin-search "category:auth"

# List all available plugins
quicpulse --plugin-search "*"
```

### Installing Plugins

```bash
# Install from registry
quicpulse --plugin-install jwt-auth

# Install specific version
quicpulse --plugin-install jwt-auth@1.2.0

# Install from URL
quicpulse --plugin-install https://github.com/user/plugin/releases/latest

# Install from local path
quicpulse --plugin-install ./my-plugin/
```

### Publishing Plugins

```bash
# Validate plugin structure
quicpulse --plugin-validate ./my-plugin/

# Package for distribution
quicpulse --plugin-package ./my-plugin/

# Publish to registry (requires authentication)
quicpulse --plugin-publish ./my-plugin/
```

## Built-in Plugins

### request-logger

Log all requests and responses:

```toml
[plugins.request-logger]
output = "./logs/requests.log"
format = "json"  # or "text"
log_headers = true
log_body = true
log_timing = true
```

### retry-handler

Automatic retry on failure:

```toml
[plugins.retry-handler]
max_retries = 3
retry_delay_ms = 1000
retry_on_status = [500, 502, 503, 504]
exponential_backoff = true
```

### rate-limiter

Rate limit requests:

```toml
[plugins.rate-limiter]
requests_per_second = 10
burst = 20
wait_on_limit = true
```

## Plugin Execution Order

1. Load plugins from all directories
2. Sort by priority (manifest `priority` field)
3. For each hook:
   - Execute global plugins first
   - Execute request-specific plugins
   - Chain results through each plugin

## Troubleshooting

### Plugin Not Found

```
Error: Plugin 'my-plugin' not found
```

1. Check plugin is in correct directory
2. Verify manifest.toml exists
3. Check plugin name matches manifest

### Hook Execution Failed

```
Error: Hook 'pre_request' failed in plugin 'my-plugin'
```

1. Check plugin logs
2. Verify hook function signature
3. Test plugin independently

### Permission Denied

```
Error: Cannot execute plugin binary
```

1. Check file permissions (`chmod +x`)
2. Verify binary is compatible with OS
3. Check security settings (Gatekeeper, SELinux)

### Configuration Error

```
Error: Invalid plugin configuration
```

1. Check config matches schema
2. Verify required fields are present
3. Check value types

## Best Practices

1. **Keep plugins focused** - One plugin, one purpose
2. **Handle errors gracefully** - Never crash the main process
3. **Document configuration** - Clear config schema in manifest
4. **Version plugins** - Semantic versioning for compatibility
5. **Test thoroughly** - Unit tests for all hooks
6. **Secure secrets** - Never log sensitive data

## Building Plugins

### Creating a Script Plugin

1. **Create plugin directory:**

```bash
mkdir -p ~/.config/quicpulse/plugins/my-auth-plugin
cd ~/.config/quicpulse/plugins/my-auth-plugin
```

2. **Create manifest.toml:**

```toml
name = "my-auth-plugin"
version = "1.0.0"
description = "Custom authentication plugin"
author = "Your Name"
license = "MIT"

entry = "main.rune"
type = "script"

hooks = ["pre_request"]

[config]
api_key = { type = "string", required = true }
```

3. **Create main.rune:**

```rune
pub fn pre_request(ctx) {
    let api_key = ctx.config.get("api_key")?;
    ctx.request.headers["X-API-Key"] = api_key;
    HookResult::Ok(ctx)
}
```

4. **Enable in config:**

```yaml
# ~/.config/quicpulse/plugins.yaml
plugins:
  my-auth-plugin:
    enabled: true
    config:
      api_key: "your-secret-key"
```

### Creating a Binary Plugin (Rust)

1. **Create new Rust project:**

```bash
cargo new my-binary-plugin
cd my-binary-plugin
```

2. **Add dependencies to Cargo.toml:**

```toml
[package]
name = "my-binary-plugin"
version = "1.0.0"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

3. **Implement the plugin (src/main.rs):**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};

#[derive(Deserialize)]
struct PluginInput {
    hook: String,
    context: HookContext,
}

#[derive(Deserialize, Serialize)]
struct HookContext {
    request: Option<RequestData>,
    response: Option<ResponseData>,
    config: HashMap<String, serde_json::Value>,
    vars: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
struct RequestData {
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
struct ResponseData {
    status: u16,
    headers: HashMap<String, String>,
    body: serde_json::Value,
}

#[derive(Serialize)]
struct PluginOutput {
    result: String,
    context: HookContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn main() {
    // Read JSON input from stdin
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let plugin_input: PluginInput = serde_json::from_str(&input).unwrap();

    // Process the hook
    let output = match plugin_input.hook.as_str() {
        "pre_request" => handle_pre_request(plugin_input.context),
        "post_response" => handle_post_response(plugin_input.context),
        _ => PluginOutput {
            result: "ok".to_string(),
            context: plugin_input.context,
            error: None,
        },
    };

    // Write JSON output to stdout
    let output_json = serde_json::to_string(&output).unwrap();
    io::stdout().write_all(output_json.as_bytes()).unwrap();
}

fn handle_pre_request(mut ctx: HookContext) -> PluginOutput {
    if let Some(ref mut request) = ctx.request {
        // Add custom header from config
        if let Some(api_key) = ctx.config.get("api_key") {
            if let Some(key) = api_key.as_str() {
                request.headers.insert("X-API-Key".to_string(), key.to_string());
            }
        }
    }

    PluginOutput {
        result: "ok".to_string(),
        context: ctx,
        error: None,
    }
}

fn handle_post_response(ctx: HookContext) -> PluginOutput {
    // Log response status
    if let Some(ref response) = ctx.response {
        eprintln!("[my-plugin] Response status: {}", response.status);
    }

    PluginOutput {
        result: "ok".to_string(),
        context: ctx,
        error: None,
    }
}
```

4. **Build the plugin:**

```bash
cargo build --release
```

5. **Create manifest and install:**

```bash
mkdir -p ~/.config/quicpulse/plugins/my-binary-plugin
cp target/release/my-binary-plugin ~/.config/quicpulse/plugins/my-binary-plugin/

cat > ~/.config/quicpulse/plugins/my-binary-plugin/manifest.toml << 'EOF'
name = "my-binary-plugin"
version = "1.0.0"
description = "Custom binary plugin"
entry = "my-binary-plugin"
type = "binary"
hooks = ["pre_request", "post_response"]

[config]
api_key = { type = "string", required = true }
EOF
```

### Creating a JavaScript Plugin

For plugins that need JavaScript execution, enable the `quicpulse-javascript` bundled plugin first.

1. **Create plugin directory:**

```bash
mkdir -p ~/.config/quicpulse/plugins/my-js-plugin
```

2. **Create manifest.toml:**

```toml
name = "my-js-plugin"
version = "1.0.0"
description = "JavaScript-based plugin"
entry = "main.js"
type = "script"
hooks = ["pre_request"]

[config]
prefix = { type = "string", default = "X-Custom" }
```

3. **Create main.js:**

```javascript
// Pre-request hook
function pre_request(ctx) {
    const prefix = ctx.config.prefix || "X-Custom";
    const timestamp = crypto.timestamp();

    ctx.request.headers[prefix + "-Timestamp"] = timestamp.toString();
    ctx.request.headers[prefix + "-Request-ID"] = crypto.uuid_v4();

    return { result: "ok", context: ctx };
}
```

### Testing Plugins

**Test script plugins:**

```bash
# Validate plugin structure
quicpulse --plugin-validate ./my-plugin/

# Test with a simple request
quicpulse --plugins my-plugin httpbin.org/get
```

**Test binary plugins manually:**

```bash
# Create test input
echo '{"hook":"pre_request","context":{"request":{"method":"GET","url":"http://test.com","headers":{},"body":null},"config":{"api_key":"test"},"vars":{}}}' | ./my-binary-plugin
```

**Integration testing:**

```yaml
# test-workflow.yaml
name: Plugin Test
base_url: https://httpbin.org

plugins:
  my-plugin:
    enabled: true
    config:
      api_key: "test-key"

steps:
  - name: Test Plugin Headers
    method: GET
    url: /headers
    assertions:
      - json.request.headers["X-Api-Key"] == "test-key"
```

```bash
quicpulse --run test-workflow.yaml
```

## API Reference

### PluginConfig

```rust
pub struct PluginConfig {
    pub enabled_plugins: Vec<String>,
    pub plugin_dirs: Vec<PathBuf>,
    pub plugin_configs: HashMap<String, toml::Value>,
}
```

### PluginHook

```rust
pub enum PluginHook {
    PreRequest,
    PostResponse,
    OnError,
    Auth,
    Format,
    Validate,
}
```

### HookContext

```rust
pub struct HookContext {
    pub request: Option<RequestData>,
    pub response: Option<ResponseData>,
    pub config: HashMap<String, Value>,
    pub vars: HashMap<String, Value>,
    pub error: Option<String>,
}
```

### HookResult

```rust
pub enum HookResult {
    Ok(HookContext),
    Stop(String),
    Error(String),
    WithBody(HookContext, String),
    WithUrl(HookContext, String),
    WithHeaders(HookContext, HashMap<String, String>),
}
```

---

See also:
- [workflow-plugins.md](workflow-plugins.md) - Workflow plugin integration
- [script.md](script.md) - Scripting reference (Rune and JavaScript)
- [README.md](../README.md) - CLI reference
