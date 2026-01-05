# QuicPulse CLI Reference

Complete reference for all command-line flags and options.

## Usage

```bash
quicpulse [METHOD] URL [REQUEST_ITEMS...] [OPTIONS]
```

## Positional Arguments

| Argument | Description |
|----------|-------------|
| `METHOD` | HTTP method (GET, POST, PUT, DELETE, etc.). Defaults to GET, or POST if data is present |
| `URL` | The URL to request. `http://` prefix is optional. Supports localhost shorthand: `:3000/foo` |
| `REQUEST_ITEM` | Request items: headers (`:`), query params (`==`), data (`=`), JSON (`:=`), files (`@`) |

---

## Content Type Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--json` | `-j` | (default) Serialize data as JSON object |
| `--form` | `-f` | Serialize data as form fields (`application/x-www-form-urlencoded`) |
| `--multipart` | | Force `multipart/form-data` encoding |
| `--boundary BOUNDARY` | | Custom boundary for multipart requests |
| `--raw RAW` | | Raw request body from string |

---

## Content Processing

| Flag | Short | Description |
|------|-------|-------------|
| `--compress` | `-x` | Compress request body with deflate. Use `-xx` to force compression |
| `--chunked` | | Use chunked transfer encoding |

---

## Output Processing

| Flag | Short | Description |
|------|-------|-------------|
| `--pretty STYLE` | | Output formatting: `all`, `colors`, `format`, `none` |
| `--style STYLE` | `-s` | Color scheme: `auto`, `solarized-dark`, `solarized-light`, `monokai`, `onedark`, `dracula`, `nord`, `gruvbox-dark`, `gruvbox-light` |
| `--pager` | | Pipe output through a pager (uses `$PAGER` or `less -R`) |
| `--pager-cmd COMMAND` | | Custom pager command (overrides `$PAGER`) |
| `--no-pager` | | Disable automatic paging |
| `--unsorted` | | Disable sorting of headers and JSON keys |
| `--sorted` | | Enable sorting of headers and JSON keys |
| `--response-charset ENCODING` | | Override response character encoding |
| `--response-mime MIME` | | Override response MIME type |
| `--format-options OPTIONS` | | Format options (e.g., `json.indent:4`) |

---

## Output Options

| Flag | Short | Description |
|------|-------|-------------|
| `--print WHAT` | `-p` | What to print: `H`(eaders), `B`(ody), `h`(response headers), `b`(response body), `m`(eta) |
| `--headers` | `-h` | Print only response headers (shortcut for `-p h`) |
| `--meta` | `-m` | Print only response metadata (shortcut for `-p m`) |
| `--body` | `-b` | Print only response body (shortcut for `-p b`) |
| `--verbose` | `-v` | Verbose output. Use `-vv` for even more verbose |
| `--all` | | Show intermediary requests/responses (redirects) |
| `--stream` | `-S` | Stream response body line by line |
| `--output FILE` | `-o` | Output file |
| `--download` | `-d` | Download mode: save response body to file |
| `--continue` | `-c` | Resume partial download |
| `--quiet` | `-q` | Suppress output. Use `-qq` for even more quiet |

---

## Sessions

| Flag | Description |
|------|-------------|
| `--session NAME` | Named session to create or update |
| `--session-read-only NAME` | Named session to read (without updating) |

---

## Authentication

### Basic Auth Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--auth CREDENTIALS` | `-a` | Authentication credentials (`user:password` or token) |
| `--auth-type TYPE` | `-A` | Authentication type (see below) |
| `--ignore-netrc` | | Skip credentials from `.netrc` |

### Auth Types (`-A TYPE`)

| Type | Aliases | Description |
|------|---------|-------------|
| `basic` | | HTTP Basic authentication |
| `digest` | | HTTP Digest authentication |
| `bearer` | | Bearer token authentication |
| `aws-sigv4` | `aws` | AWS Signature Version 4 |
| `gcp` | `google` | Google Cloud Platform (`gcloud auth print-access-token`) |
| `azure` | `az` | Azure CLI (`az account get-access-token`) |
| `oauth2` | `oauth` | OAuth 2.0 Client Credentials flow |
| `oauth2-auth-code` | `oauth-code` | OAuth 2.0 Authorization Code flow |
| `oauth2-device` | `oauth-device` | OAuth 2.0 Device Authorization flow |
| `ntlm` | | NTLM authentication (Windows Integrated Auth) |
| `negotiate` | | Negotiate authentication (Kerberos/NTLM auto-select) |
| `kerberos` | | Kerberos authentication |

### AWS SigV4 Options

| Flag | Description |
|------|-------------|
| `--aws-region REGION` | AWS region for SigV4 signing (e.g., `us-east-1`) |
| `--aws-service SERVICE` | AWS service name (e.g., `execute-api`, `s3`) |
| `--aws-profile PROFILE` | AWS profile from `~/.aws/credentials` or `~/.aws/config` |

### OAuth 2.0 Options

| Flag | Description |
|------|-------------|
| `--oauth-token-url URL` | OAuth 2.0 token endpoint URL |
| `--oauth-auth-url URL` | Authorization endpoint URL (for authorization code flow) |
| `--oauth-device-url URL` | Device authorization endpoint URL (for device flow) |
| `--oauth-redirect-port PORT` | Redirect port for authorization code flow (default: 8080) |
| `--oauth-pkce` | Use PKCE with authorization code flow |
| `--oauth-scope SCOPE` | OAuth 2.0 scope (can be used multiple times) |

---

## Protocol Options

| Flag | Description |
|------|-------------|
| `--http3` | Use HTTP/3 (QUIC) protocol |
| `--http-version VERSION` | Preferred HTTP version: `1.0`, `1.1`, `2`, `3` |

---

## GraphQL

| Flag | Short | Description |
|------|-------|-------------|
| `--graphql` | `-G` | GraphQL mode: wrap request as GraphQL query |
| `--graphql-query QUERY` | | GraphQL query string (alternative to `query=` request item) |
| `--graphql-operation NAME` | | GraphQL operation name |
| `--graphql-schema` | | Fetch GraphQL schema via introspection |

---

## gRPC

| Flag | Description |
|------|-------------|
| `--grpc` | gRPC mode: send gRPC request |
| `--proto FILE` | Path to `.proto` file |
| `--grpc-list` | List available gRPC services (via reflection) |
| `--grpc-describe SERVICE` | Describe a gRPC service or method |
| `--grpc-interactive` | Interactive gRPC REPL mode |
| `--grpc-plaintext` | Use plaintext HTTP/2 (h2c) without TLS |

---

## WebSocket

| Flag | Description |
|------|-------------|
| `--ws` | WebSocket mode (auto-detected for `ws://` URLs) |
| `--ws-subprotocol PROTOCOL` | WebSocket subprotocol to request |
| `--ws-send MESSAGE` | Send message and disconnect |
| `--ws-interactive` | Interactive WebSocket REPL mode |
| `--ws-listen` | Listen mode - receive messages only |
| `--ws-binary MODE` | Binary mode: `hex` or `base64` |
| `--ws-compress` | Enable permessage-deflate compression |
| `--ws-max-messages NUM` | Maximum messages to receive (0 = unlimited) |
| `--ws-ping-interval SECONDS` | Ping interval in seconds |

---

## Network

| Flag | Short | Description |
|------|-------|-------------|
| `--offline` | | Build request but don't send it |
| `--unix-socket PATH` | | Connect via Unix domain socket |
| `--proxy PROXY` | | Proxy URL (`protocol:url` format) |
| `--socks URL` | | SOCKS proxy URL (`socks4://`, `socks5://`) |
| `--follow` | `-F` | Follow redirects |
| `--max-redirects NUM` | | Maximum number of redirects (default: 30) |
| `--max-headers NUM` | | Maximum number of headers to accept |
| `--timeout SECONDS` | | Connection timeout in seconds |
| `--check-status` | | Exit with error on HTTP error status codes (4xx, 5xx) |
| `--path-as-is` | | Don't normalize URL path (keep `..`, etc.) |

### Low-Level Network Controls

| Flag | Description |
|------|-------------|
| `--resolve HOST:PORT:ADDRESS` | Custom DNS resolution |
| `--interface INTERFACE` | Bind to network interface (e.g., `eth0`, `192.168.1.100`) |
| `--local-port PORT[-PORT]` | Local port range for outgoing connections |
| `--tcp-fastopen` | Enable TCP Fast Open (TFO) |
| `--local-address ADDRESS` | Set local address to bind to |

---

## SSL/TLS

| Flag | Description |
|------|-------------|
| `--verify VERIFY` | SSL certificate verification: `yes`/`no`/path-to-CA-bundle (default: `yes`) |
| `--ssl VERSION` | Minimum TLS version: `tls1`, `tls1.1`, `tls1.2`, `tls1.3` |
| `--ciphers CIPHERS` | Cipher suite specification |
| `--cert FILE` | Client certificate file |
| `--cert-key FILE` | Client private key file |
| `--cert-key-pass PASS` | Passphrase for encrypted client key |

---

## CI/CD & Automation

| Flag | Description |
|------|-------------|
| `--no-color` | Force disable colors in output |
| `--strict-tty` | Fail if stdout is not a TTY (for safety in CI) |
| `--log-format FORMAT` | Output format: `json` (JSON Lines) or `text` (default) |

---

## Troubleshooting

| Flag | Short | Description |
|------|-------|-------------|
| `--ignore-stdin` | `-I` | Don't read stdin (useful for scripting) |
| `--default-scheme SCHEME` | | Default URL scheme when not specified (default: `http`) |
| `--traceback` | | Show traceback on error |
| `--debug` | | Debug mode (implies `--traceback`) |

---

## Self-Update

| Flag | Description |
|------|-------------|
| `--update` | Update quicpulse to the latest version |

---

## Benchmarking

| Flag | Description |
|------|-------------|
| `--bench` | Enable benchmarking mode |
| `--requests NUM` | Number of requests to send (default: 100) |
| `--concurrency NUM` | Number of concurrent requests (default: 10) |

---

## Data Filtering & Formatting

| Flag | Short | Description |
|------|-------|-------------|
| `--filter EXPR` | `-J` | JQ filter expression to apply to JSON response |
| `--table` | | Output JSON array as ASCII table |
| `--csv` | | Output JSON array as CSV |

---

## Assertions

| Flag | Description |
|------|-------------|
| `--assert-status CODE` | Assert response status code (e.g., `200`, `2xx`, `200-299`) |
| `--assert-time DURATION` | Assert response time (e.g., `<500ms`, `<2s`) |
| `--assert-body PATTERN` | Assert response body contains pattern (JQ or literal) |
| `--assert-header HEADER[:VALUE]` | Assert response header exists and optionally matches value |

---

## Workflow Pipelines

### Execution

| Flag | Description |
|------|-------------|
| `--run FILE` | Run a workflow file (YAML/TOML) |
| `--env NAME` | Environment/profile for workflow variables |
| `--var NAME=VALUE` | Set workflow variable (can be used multiple times) |
| `--dry-run` | Show steps without executing |
| `--continue-on-failure` | Continue even if a step fails |
| `--workflow-retries NUM` | Number of retries for failed steps (default: 0) |
| `--workflow-verbose` | Show verbose progress |
| `--validate` | Validate workflow file without executing |

### Step Filtering

| Flag | Description |
|------|-------------|
| `--tags TAGS` | Run only steps with these tags (comma-separated) |
| `--include STEPS` | Include only these steps by name (comma-separated) |
| `--exclude PATTERNS` | Exclude steps matching patterns (comma-separated, supports regex) |

### Output & Reporting

| Flag | Description |
|------|-------------|
| `--save-responses DIR` | Save response data from each step to directory |
| `--report-junit FILE` | Generate JUnit XML report (for CI/CD) |
| `--report-json FILE` | Generate JSON report |
| `--report-tap FILE` | Generate TAP (Test Anything Protocol) report |

### Workflow Sharing

| Flag | Description |
|------|-------------|
| `--workflow-list` | List available workflows (local and remote) |
| `--workflow-pull URL_OR_NAME` | Pull a workflow from remote registry |
| `--workflow-push FILE` | Push a workflow to remote registry |
| `--workflow-public` | Publish workflow publicly |
| `--workflow-tags TAGS` | Tags for published workflow |
| `--workflow-description DESC` | Description for published workflow |
| `--workflow-registry URL` | Remote registry URL (default: GitHub gists) |
| `--workflow-search QUERY` | Search remote workflows |

---

## Security Fuzzing

| Flag | Description |
|------|-------------|
| `--fuzz` | Enable fuzzing mode |
| `--fuzz-field FIELD` | Fields to fuzz (can be repeated, default: all data fields) |
| `--fuzz-category CATEGORY` | Fuzz categories: `sql`, `xss`, `cmd`, `path`, `boundary`, `type`, `format`, `int`, `unicode`, `nosql` |
| `--fuzz-concurrency NUM` | Concurrency for fuzz requests (default: 10) |
| `--fuzz-risk LEVEL` | Minimum risk level for payloads 1-5 (default: 1) |
| `--fuzz-anomalies-only` | Only show anomalies (5xx errors, timeouts) |
| `--fuzz-stop-on-anomaly` | Stop fuzzing on first anomaly found |
| `--fuzz-dict FILE` | Custom fuzzing dictionary file (one payload per line) |
| `--fuzz-payload PAYLOAD` | Custom fuzzing payload (can be repeated) |

---

## HAR Replay

| Flag | Description |
|------|-------------|
| `--import-har FILE` | Import and replay requests from HAR file |
| `--har-interactive` | Interactive mode: select which requests to replay |
| `--har-filter PATTERN` | Filter HAR requests by URL pattern (regex) |
| `--har-delay DURATION` | Delay between replayed requests (e.g., `100ms`, `1s`) |
| `--har-list` | Only show HAR entries without replaying |
| `--har-index INDEX` | Replay specific request by index (1-based, can be repeated) |

---

## OpenAPI Import

| Flag | Description |
|------|-------------|
| `--import-openapi FILE` | Import OpenAPI/Swagger specification |
| `--generate-workflow FILE` | Output file for generated workflow (default: stdout) |
| `--openapi-base-url URL` | Base URL override for generated workflow |
| `--openapi-include-deprecated` | Include deprecated endpoints |
| `--openapi-tag TAG` | Filter endpoints by tag (can be repeated) |
| `--openapi-exclude-tag TAG` | Exclude endpoints by tag (can be repeated) |
| `--openapi-fuzz` | Include fuzz test payloads in generated workflow |
| `--openapi-list` | List all endpoints without generating workflow |

---

## Developer Experience

| Flag | Description |
|------|-------------|
| `--curl` | Print equivalent curl command instead of sending request |
| `--import-curl COMMAND` | Import and execute a curl command |
| `--http-file FILE` | Import and execute requests from `.http`/`.rest` file |
| `--http-request NAME\|INDEX` | Run specific request from `.http` file by name or index |
| `--http-list` | List all requests in a `.http` file |
| `--generate LANGUAGE` | Generate code snippet: `python`, `node`, `go`, `java`, `php`, `rust`, `ruby` |
| `--env-file FILE` | Load environment variables from `.env` file |
| `--no-env` | Disable auto-loading of `.env` file |

---

## Mock Server

| Flag | Description |
|------|-------------|
| `--mock` | Start a mock HTTP server (alias: `--serve`) |
| `--mock-config FILE` | Mock server config file (YAML, JSON, or TOML) |
| `--mock-port PORT` | Mock server port (default: 8080) |
| `--mock-route ROUTE` | Mock route: `METHOD:PATH:BODY` (can be repeated) |
| `--mock-cors` | Enable CORS on mock server |
| `--mock-latency MS` | Simulate latency (min-max ms, e.g., `50-200`) |
| `--mock-host HOST` | Mock server bind host (default: `127.0.0.1`) |
| `--mock-log` | Log mock server requests to stderr |
| `--mock-record FILE` | Record requests to HAR file |
| `--mock-tls-cert FILE` | TLS certificate for HTTPS mock server |
| `--mock-tls-key FILE` | TLS private key for HTTPS mock server |
| `--mock-proxy URL` | Proxy unmatched requests to another server |

---

## Plugin Ecosystem

| Flag | Description |
|------|-------------|
| `--plugin-list` | List installed plugins (alias: `--plugins`) |
| `--plugin-install NAME_OR_URL` | Install a plugin from registry or git URL |
| `--plugin-uninstall NAME` | Uninstall a plugin |
| `--plugin-search QUERY` | Search for plugins in registry |
| `--plugin-update` | Update installed plugins |
| `--plugin-dir DIR` | Plugin directory |
| `--plugin NAME` | Enable specific plugin(s) for this request |

---

## Hidden/Internal Flags

| Flag | Description |
|------|-------------|
| `--generate-completions SHELL` | Generate shell completions: `bash`, `zsh`, `fish`, `powershell`, `elvish` |
| `--generate-manpage` | Generate man page to stdout |
| `--script-allow-dir DIR` | Allow scripting to access directory (can be repeated) |

---

## Request Item Syntax

QuicPulse uses a simple syntax for specifying request components:

| Syntax | Type | Example |
|--------|------|---------|
| `Header:Value` | HTTP Header | `Accept:application/json` |
| `Header:` | Empty Header | `Accept:` |
| `Header;` | Unset Header | `User-Agent;` |
| `name=value` | Data Field | `username=admin` |
| `name:=json` | JSON Value | `count:=42` `enabled:=true` |
| `name==value` | Query Param | `search==term` |
| `name@file` | File Upload | `avatar@photo.jpg` |
| `name@file;type=mime` | File with MIME | `doc@file.pdf;type=application/pdf` |
| `@file` | Raw File Body | `@data.json` |

### Examples

```bash
# JSON data with header
quicpulse POST example.com/api name=John age:=30 Authorization:Bearer\ token

# Form submission
quicpulse -f POST example.com/login username=admin password=secret

# Query parameters
quicpulse example.com/search q==rust page==1 limit==20

# File upload
quicpulse -f POST example.com/upload file@document.pdf

# Multiple headers
quicpulse example.com X-Custom:value Accept:application/json
```

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `QUICPULSE_CONFIG_DIR` | Configuration directory location |
| `PAGER` | Pager command for `--pager` flag |
| `NO_COLOR` | Disable colors (set to any value) |
| `HTTPS_PROXY` | HTTPS proxy URL |
| `HTTP_PROXY` | HTTP proxy URL |
| `ALL_PROXY` | Proxy for all protocols |
| `NO_PROXY` | Comma-separated list of hosts to bypass proxy |

---

## Exit Codes

| Code | Description |
|------|-------------|
| 0 | Success |
| 1 | General error |
| 2 | Request error (connection failed, timeout, etc.) |
| 3 | HTTP error status (with `--check-status`) |
| 4 | Assertion failed |
| 5 | Workflow error |

---

## See Also

- [README](../README.md) - Quick start guide
- [Configuration](configuration.md) - Config file reference
- [Authentication](authentication.md) - Detailed auth documentation
- [Workflows](workflow.md) - Multi-step automation
