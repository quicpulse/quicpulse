# Authentication

QuicPulse supports a comprehensive set of authentication methods for different use cases.

## Quick Reference

| Auth Type | Flag | Example |
|-----------|------|---------|
| Basic | `-A basic` | `quicpulse -a user:pass example.com` |
| Digest | `-A digest` | `quicpulse -A digest -a user:pass example.com` |
| Bearer | `-A bearer` | `quicpulse -A bearer -a TOKEN example.com` |
| AWS SigV4 | `-A aws-sigv4` | `quicpulse -A aws-sigv4 example.com` |
| OAuth 2.0 | `-A oauth2` | `quicpulse -A oauth2 -a client:secret --oauth-token-url URL` |
| GCP | `-A gcp` | `quicpulse -A gcp example.com` |
| Azure | `-A azure` | `quicpulse -A azure example.com` |
| NTLM | `-A ntlm` | `quicpulse -A ntlm -a user:pass example.com` |

---

## Basic Authentication

HTTP Basic Authentication (RFC 7617) sends credentials as base64-encoded `user:password`.

```bash
# Explicit basic auth
quicpulse -a user:password example.com/api

# With auth type flag (same result)
quicpulse -A basic -a user:password example.com/api

# Prompt for password interactively
quicpulse -a user example.com/api
```

### How It Works

Basic auth creates an `Authorization` header with the format:
```
Authorization: Basic base64(username:password)
```

### Security Note

Basic auth transmits credentials in easily reversible base64 encoding. Always use HTTPS when using basic authentication.

---

## Digest Authentication

HTTP Digest Authentication (RFC 7616) uses a challenge-response mechanism that doesn't send the password in plain text.

```bash
quicpulse -A digest -a user:password example.com/protected
```

### How It Works

1. Initial request returns `401 Unauthorized` with `WWW-Authenticate: Digest ...` challenge
2. QuicPulse extracts the nonce and other parameters
3. Second request includes computed response hash

### Supported Algorithms

- MD5 (default)
- MD5-sess
- SHA-256
- SHA-256-sess

---

## Bearer Token

Bearer token authentication (RFC 6750) sends a token in the Authorization header.

```bash
# From command line
quicpulse -A bearer -a "eyJhbGciOiJIUzI1NiIs..." example.com/api

# From environment variable
export TOKEN="your-jwt-token"
quicpulse -A bearer -a "$TOKEN" example.com/api
```

### Authorization Header Format

```
Authorization: Bearer your-token-here
```

---

## AWS Signature Version 4

Sign requests using AWS SigV4 for AWS services and compatible APIs.

```bash
# Auto-detect credentials from environment/files
quicpulse -A aws-sigv4 s3.amazonaws.com/bucket

# With explicit region and service
quicpulse -A aws-sigv4 --aws-region us-east-1 --aws-service s3 s3.amazonaws.com/bucket

# Using specific AWS profile
quicpulse -A aws-sigv4 --aws-profile production api.example.com
```

### Credential Sources (in order)

1. **Environment Variables**
   - `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`
   - Optional: `AWS_SESSION_TOKEN` for temporary credentials

2. **AWS Credentials File** (`~/.aws/credentials`)
   ```ini
   [default]
   aws_access_key_id = AKIA...
   aws_secret_access_key = secret...

   [production]
   aws_access_key_id = AKIA...
   aws_secret_access_key = secret...
   ```

3. **AWS Config File** (`~/.aws/config`)
   ```ini
   [default]
   region = us-east-1

   [profile production]
   region = eu-west-1
   ```

4. **AWS SSO**
   - Reads SSO cache for profiles configured with `sso_start_url`
   - Use `aws sso login --profile PROFILE` first

5. **Instance Metadata Service (IMDS)**
   - On EC2, ECS, Lambda
   - Automatic with IAM roles

### AWS Options

| Flag | Description |
|------|-------------|
| `--aws-region REGION` | AWS region (e.g., `us-east-1`) |
| `--aws-service SERVICE` | AWS service name (e.g., `s3`, `execute-api`, `es`) |
| `--aws-profile PROFILE` | AWS profile from credentials/config |

### STS Temporary Credentials

QuicPulse automatically handles `assume_role` configurations:

```ini
# ~/.aws/config
[profile cross-account]
role_arn = arn:aws:iam::123456789012:role/CrossAccountRole
source_profile = default
```

```bash
quicpulse -A aws-sigv4 --aws-profile cross-account api.example.com
```

---

## OAuth 2.0

QuicPulse supports multiple OAuth 2.0 flows.

### Client Credentials Flow

For machine-to-machine authentication:

```bash
quicpulse -A oauth2 \
  -a "client_id:client_secret" \
  --oauth-token-url "https://auth.example.com/token" \
  --oauth-scope "read write" \
  api.example.com/data
```

### Authorization Code Flow

For user authentication with browser redirect:

```bash
quicpulse -A oauth2-auth-code \
  -a "client_id:client_secret" \
  --oauth-token-url "https://auth.example.com/token" \
  --oauth-auth-url "https://auth.example.com/authorize" \
  --oauth-redirect-port 8080 \
  api.example.com/user
```

With PKCE (recommended for public clients):

```bash
quicpulse -A oauth2-auth-code \
  -a "client_id" \
  --oauth-token-url "https://auth.example.com/token" \
  --oauth-auth-url "https://auth.example.com/authorize" \
  --oauth-pkce \
  api.example.com/user
```

### Device Authorization Flow

For devices without browsers (smart TVs, CLI tools):

```bash
quicpulse -A oauth2-device \
  -a "client_id" \
  --oauth-token-url "https://auth.example.com/token" \
  --oauth-device-url "https://auth.example.com/device" \
  api.example.com/data
```

This displays a URL and code for the user to authorize on another device.

### OAuth 2.0 Options

| Flag | Description |
|------|-------------|
| `--oauth-token-url URL` | Token endpoint |
| `--oauth-auth-url URL` | Authorization endpoint (auth code flow) |
| `--oauth-device-url URL` | Device authorization endpoint |
| `--oauth-redirect-port PORT` | Local redirect port (default: 8080) |
| `--oauth-pkce` | Enable PKCE |
| `--oauth-scope SCOPE` | OAuth scope (can be repeated) |

### Token Caching

OAuth tokens are cached in `~/.config/quicpulse/oauth_tokens/` and automatically refreshed when expired.

---

## Google Cloud Platform

Authenticate using gcloud CLI credentials:

```bash
# Uses `gcloud auth print-access-token` automatically
quicpulse -A gcp storage.googleapis.com/bucket/object
```

### Prerequisites

1. Install Google Cloud SDK
2. Run `gcloud auth login` or `gcloud auth application-default login`

### Scopes

GCP auth uses the access token from the currently authenticated gcloud account.

---

## Azure CLI

Authenticate using Azure CLI credentials:

```bash
# Uses `az account get-access-token` automatically
quicpulse -A azure management.azure.com/subscriptions
```

### Prerequisites

1. Install Azure CLI
2. Run `az login`

### Resource Scopes

For specific resources:

```bash
quicpulse -A azure \
  "https://management.azure.com/subscriptions?api-version=2020-01-01"
```

---

## NTLM / Windows Integrated Auth

For Windows domain authentication:

```bash
# NTLM authentication
quicpulse -A ntlm -a "DOMAIN\\user:password" intranet.company.com

# Negotiate (auto-select Kerberos or NTLM)
quicpulse -A negotiate -a "user@DOMAIN:password" intranet.company.com

# Kerberos
quicpulse -A kerberos -a "user@REALM:password" intranet.company.com
```

### Kerberos Prerequisites

For Kerberos authentication, obtain a ticket first:

```bash
kinit user@REALM.COM
quicpulse -A kerberos intranet.company.com
```

---

## .netrc File Support

QuicPulse reads credentials from `~/.netrc` automatically:

```
# ~/.netrc
machine api.example.com
  login username
  password secret

machine github.com
  login myuser
  password ghp_xxxxxxxxxxxx
```

### Disable .netrc

```bash
quicpulse --ignore-netrc api.example.com
```

---

## API Key Authentication

While not a built-in auth type, you can add API keys via headers:

```bash
# X-API-Key header
quicpulse api.example.com X-API-Key:your-api-key

# Authorization header with custom scheme
quicpulse api.example.com "Authorization:ApiKey your-key"

# Query parameter
quicpulse "api.example.com?api_key=your-key"
```

---

## Sessions with Authentication

Combine authentication with sessions for stateful requests:

```bash
# First request - authenticate and create session
quicpulse -a user:pass --session myapi api.example.com/login

# Subsequent requests - reuse cookies and tokens
quicpulse --session myapi api.example.com/data
```

---

## Workflows with Authentication

Authentication can be specified at the workflow level:

```yaml
# workflow.yaml
name: API Tests
auth:
  type: bearer
  token: "{{ env.API_TOKEN }}"

steps:
  - name: Get users
    request:
      url: https://api.example.com/users
```

Or per-step:

```yaml
steps:
  - name: Public endpoint
    request:
      url: https://api.example.com/public

  - name: Protected endpoint
    auth:
      type: basic
      username: admin
      password: "{{ env.ADMIN_PASS }}"
    request:
      url: https://api.example.com/admin
```

### AWS SigV4 in Workflows

```yaml
auth:
  type: aws-sigv4
  region: us-east-1
  service: execute-api
  profile: production  # optional

steps:
  - name: Call API Gateway
    request:
      url: https://abc123.execute-api.us-east-1.amazonaws.com/prod/resource
```

### OAuth 2.0 in Workflows

```yaml
auth:
  type: oauth2
  client_id: "{{ env.CLIENT_ID }}"
  client_secret: "{{ env.CLIENT_SECRET }}"
  token_url: https://auth.example.com/oauth/token
  scopes:
    - read
    - write

steps:
  - name: API Call
    request:
      url: https://api.example.com/data
```

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `AWS_ACCESS_KEY_ID` | AWS access key |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key |
| `AWS_SESSION_TOKEN` | AWS session token (temporary credentials) |
| `AWS_PROFILE` | AWS profile name |
| `AWS_REGION` | AWS region |
| `GOOGLE_APPLICATION_CREDENTIALS` | Path to GCP service account JSON |

---

## See Also

- [CLI Reference](cli-reference.md) - All authentication flags
- [Sessions](workflow-sessions.md) - Persisting auth state
- [Workflows](workflow.md) - Authentication in automation
