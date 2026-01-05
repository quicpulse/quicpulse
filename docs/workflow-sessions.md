# Workflow Sessions Reference

Session management for cookie and header persistence across workflow steps.

## Overview

Sessions allow you to persist cookies and authentication state across multiple requests in a workflow. This is useful for:

- Maintaining login state across requests
- Testing authentication flows
- Simulating real user sessions
- Sharing state between workflow runs

## Quick Start

```yaml
name: User Session Test
base_url: https://api.example.com
session: user-session

steps:
  - name: Login
    method: POST
    url: /auth/login
    body: '{"username": "user", "password": "pass"}'
    # Cookies from response are automatically saved

  - name: Access Protected Resource
    method: GET
    url: /api/profile
    # Session cookies automatically sent
```

## Configuration Reference

### Workflow-Level Session Settings

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `session` | string | none | Session name for persistence |
| `session_read_only` | boolean | false | Don't save session changes |

### Session Storage

Sessions are stored in:
- Linux/macOS: `~/.local/share/quicpulse/sessions/`
- Windows: `%APPDATA%\quicpulse\sessions\`

## Examples

### Basic Session Persistence

Cookies persist across steps:

```yaml
name: Cookie Session Test
base_url: https://api.example.com
session: my-session

steps:
  - name: Login
    method: POST
    url: /login
    form:
      username: testuser
      password: secret123

  - name: Get Dashboard
    method: GET
    url: /dashboard
    # Uses session cookies from login

  - name: Update Settings
    method: POST
    url: /settings
    body: '{"theme": "dark"}'
    # Session still active
```

### Named Sessions

Use different sessions for different users:

```yaml
name: Multi-User Test
base_url: https://api.example.com

steps:
  # Admin session
  - name: Admin Login
    method: POST
    url: /login
    session: admin-session
    body: '{"username": "admin", "password": "adminpass"}'

  - name: Admin Action
    method: POST
    url: /admin/users
    session: admin-session
    body: '{"action": "list"}'

  # User session
  - name: User Login
    method: POST
    url: /login
    session: user-session
    body: '{"username": "user", "password": "userpass"}'

  - name: User Action
    method: GET
    url: /profile
    session: user-session
```

### Read-Only Session

Use existing session without saving changes:

```yaml
name: Read-Only Session Test
base_url: https://api.example.com
session: existing-session
session_read_only: true

steps:
  - name: Use Existing Session
    method: GET
    url: /api/data
    # Uses cookies from existing session
    # New cookies won't be saved
```

### Session with Cookie Extraction

Extract and verify session cookies:

```yaml
name: Session Cookie Test
base_url: https://api.example.com
session: test-session

steps:
  - name: Login
    method: POST
    url: /auth/login
    body: '{"username": "test", "password": "test"}'
    extract:
      session_cookie: header.Set-Cookie
    assert:
      status: 200
      headers:
        - name: Set-Cookie
          contains: session=

  - name: Verify Session
    method: GET
    url: /api/me
    assert:
      status: 200
      body:
        - path: username
          equals: test
```

### OAuth2 with Session

Store OAuth tokens in session:

```yaml
name: OAuth2 Session Flow
base_url: https://api.example.com
session: oauth-session

variables:
  client_id: my-app
  client_secret: secret

steps:
  - name: Get Access Token
    method: POST
    url: /oauth/token
    form:
      grant_type: client_credentials
      client_id: "{{ client_id }}"
      client_secret: "{{ client_secret }}"
    extract:
      access_token: body.access_token

  - name: Use API
    method: GET
    url: /api/resource
    headers:
      Authorization: "Bearer {{ access_token }}"
```

### Cross-Workflow Session Sharing

First workflow - create session:

```yaml
# setup.yaml
name: Setup Session
base_url: https://api.example.com
session: shared-session

steps:
  - name: Login
    method: POST
    url: /auth/login
    body: '{"username": "test", "password": "test"}'
```

Second workflow - use session:

```yaml
# tests.yaml
name: Use Shared Session
base_url: https://api.example.com
session: shared-session
session_read_only: true

steps:
  - name: Run Tests
    method: GET
    url: /api/data
    assert:
      status: 200
```

Run in sequence:
```bash
quicpulse --run setup.yaml
quicpulse --run tests.yaml
```

## Session Data

### What's Persisted

- Cookies (including httpOnly)
- Cookie attributes (domain, path, expiry, secure)
- Session-specific headers (if configured)

### What's NOT Persisted

- Request/response bodies
- Extracted variables (use workflow variables instead)
- Temporary headers

## Managing Sessions

### List Sessions

```bash
ls ~/.local/share/quicpulse/sessions/
```

### Delete a Session

```bash
rm ~/.local/share/quicpulse/sessions/my-session.json
```

### Session File Format

Sessions are stored as JSON:

```json
{
  "name": "my-session",
  "cookies": [
    {
      "name": "session",
      "value": "abc123",
      "domain": "api.example.com",
      "path": "/",
      "expires": "2024-12-31T23:59:59Z",
      "secure": true,
      "http_only": true
    }
  ],
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:35:00Z"
}
```

## Best Practices

1. **Use descriptive session names** - `user-alice-session` vs `session1`
2. **Clean up old sessions** - Delete unused session files
3. **Use read-only for tests** - Prevent test pollution
4. **Separate environments** - Different sessions for dev/staging/prod
5. **Don't share sessions** - Each environment should have its own
6. **Handle session expiry** - Re-authenticate when needed

## Common Patterns

### Session Setup and Teardown

```yaml
name: Complete Session Lifecycle
base_url: https://api.example.com
session: test-session

steps:
  - name: Login
    method: POST
    url: /auth/login
    body: '{"username": "test", "password": "test"}'
    assert:
      status: 200

  - name: Do Work
    method: GET
    url: /api/data

  - name: Logout
    method: POST
    url: /auth/logout
    assert:
      status: 200
```

### Conditional Session Refresh

```yaml
name: Session Refresh Flow
base_url: https://api.example.com
session: persistent-session

steps:
  - name: Check Session
    method: GET
    url: /api/me
    extract:
      session_valid: status

  - name: Refresh If Needed
    method: POST
    url: /auth/refresh
    skip_if: "{{ session_valid }} == 200"
    body: '{"refresh_token": "{{ refresh_token }}"}'
```

## Troubleshooting

### Session Not Persisting

1. Check session name is set
2. Verify `session_read_only` is not true
3. Check write permissions for session directory
4. Look for cookie domain mismatches

### Cookies Not Sent

1. Verify cookie domain matches request URL
2. Check cookie path matches request path
3. Secure cookies require HTTPS
4. Check cookie expiry

### Session Conflicts

1. Use unique session names
2. Clear session before tests: `rm ~/.local/share/quicpulse/sessions/session-name.json`
3. Use read-only mode for parallel tests

---

See also:
- [workflow.md](workflow.md) - Main workflow reference
- [README.md](../README.md) - CLI session options
