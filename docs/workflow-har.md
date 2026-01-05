# Workflow HAR Replay Reference

HTTP Archive (HAR) replay integration for workflow steps.

## Overview

HAR replay allows you to incorporate recorded browser/proxy sessions into workflows. This is useful for:

- Replaying captured API interactions
- Reproducing reported bugs
- Testing with real-world request patterns
- Converting HAR recordings to automated tests

## Quick Start

```yaml
name: HAR Replay Test
base_url: https://api.example.com

steps:
  - name: Replay Captured Session
    url: https://api.example.com
    har:
      file: ./recordings/session.har
      entry_index: 0
```

## Configuration Reference

### HarConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file` | string | required | Path to HAR file |
| `entry_index` | integer | none | Specific entry to replay (0-based) |
| `skip_entries` | integer[] | none | Entry indices to skip |

## HAR File Format

HAR (HTTP Archive) is a JSON format for recording HTTP transactions:

```json
{
  "log": {
    "version": "1.2",
    "entries": [
      {
        "request": {
          "method": "GET",
          "url": "https://api.example.com/users",
          "headers": [...],
          "queryString": [...],
          "postData": {...}
        },
        "response": {
          "status": 200,
          "headers": [...],
          "content": {...}
        }
      }
    ]
  }
}
```

## Examples

### Replay Single Entry

Replay a specific request from HAR:

```yaml
steps:
  - name: Replay Login Request
    url: https://api.example.com
    har:
      file: ./recordings/auth-flow.har
      entry_index: 0  # First entry (login)
```

### Replay All Entries

Replay entire HAR file:

```yaml
steps:
  - name: Replay Full Session
    url: https://api.example.com
    har:
      file: ./recordings/user-journey.har
      # No entry_index = replay all
```

### Skip Specific Entries

Skip certain requests:

```yaml
steps:
  - name: Replay Without Static Assets
    url: https://api.example.com
    har:
      file: ./recordings/page-load.har
      skip_entries:
        - 2  # Skip image request
        - 5  # Skip CSS request
        - 6  # Skip JS request
```

### HAR with Assertions

Validate replayed responses:

```yaml
steps:
  - name: Replay and Verify
    url: https://api.example.com
    har:
      file: ./recordings/api-call.har
      entry_index: 0
    assert:
      status: 200
      body:
        - path: success
          equals: true
```

### HAR with Variable Extraction

Extract values from replayed responses:

```yaml
steps:
  - name: Replay Login
    url: https://api.example.com
    har:
      file: ./recordings/login.har
      entry_index: 0
    extract:
      user_id: body.user.id
      session_token: header.X-Session-Token

  - name: Use Extracted Values
    method: GET
    url: /users/{{ user_id }}
    headers:
      X-Session-Token: "{{ session_token }}"
```

### Multiple HAR Files

Combine multiple recordings:

```yaml
steps:
  - name: Setup from HAR
    url: https://api.example.com
    har:
      file: ./recordings/setup.har

  - name: Main Flow
    url: https://api.example.com
    har:
      file: ./recordings/main-flow.har

  - name: Cleanup
    url: https://api.example.com
    har:
      file: ./recordings/cleanup.har
```

### HAR with Session

Use session cookies with HAR:

```yaml
session: har-session

steps:
  - name: Replay Auth
    url: https://api.example.com
    har:
      file: ./recordings/auth.har
    # Cookies from HAR response saved to session

  - name: Use Session
    method: GET
    url: /protected
    # Session cookies applied
```

## Creating HAR Files

### Browser DevTools

1. Open Chrome/Firefox DevTools
2. Go to Network tab
3. Perform actions to record
4. Right-click → "Save all as HAR"

### Proxy Tools

- **Charles Proxy**: File → Export Session → HTTP Archive
- **Fiddler**: File → Export Sessions → HTTPArchive
- **mitmproxy**: `mitmdump -w flow.har`

### CLI Recording

```bash
# Record with quicpulse (if supported)
quicpulse --record-har=session.har GET https://api.example.com/users
```

## HAR Entry Selection

### By Index

```yaml
har:
  file: session.har
  entry_index: 3  # Fourth request (0-indexed)
```

### By Skipping

```yaml
har:
  file: session.har
  skip_entries: [0, 1, 5]  # Skip first two and sixth
```

### All Entries

```yaml
har:
  file: session.har
  # Replays all entries in sequence
```

## Integration with Workflows

### HAR + Custom Headers

Override HAR headers:

```yaml
steps:
  - name: Replay with Custom Auth
    url: https://api.example.com
    headers:
      Authorization: "Bearer {{ fresh_token }}"
    har:
      file: ./recordings/api-call.har
      entry_index: 0
```

### HAR + Pre-Script

Modify request before replay:

```yaml
steps:
  - name: Dynamic HAR Replay
    url: https://api.example.com
    pre_script:
      code: |
        vars["timestamp"] = std::time::now();
    har:
      file: ./recordings/timestamped-request.har
      entry_index: 0
```

### HAR + Post-Script

Process HAR response:

```yaml
steps:
  - name: Replay and Process
    url: https://api.example.com
    har:
      file: ./recordings/data-fetch.har
    post_script:
      code: |
        let body = json::parse(response["body"]);
        vars["total_items"] = json::len(body["items"]);
        println!("Fetched {} items", vars["total_items"]);
```

## Best Practices

1. **Sanitize sensitive data** - Remove passwords/tokens from HARs
2. **Use relative paths** - Make workflows portable
3. **Document entry purposes** - Add comments for each entry_index
4. **Combine with assertions** - Verify expected behavior
5. **Version control HARs** - Track changes over time
6. **Update regularly** - Refresh HARs when APIs change

## Common Use Cases

### Bug Reproduction

```yaml
name: Reproduce Bug #1234
description: Replays exact request sequence from bug report

steps:
  - name: Reproduce Issue
    url: https://api.example.com
    har:
      file: ./bugs/issue-1234.har
    assert:
      # Verify bug is fixed
      status: 200  # Was 500
```

### Regression Testing

```yaml
name: API Regression Test
description: Verify API responses match recorded baseline

steps:
  - name: Check User Endpoint
    url: https://api.example.com
    har:
      file: ./baseline/users.har
      entry_index: 0
    assert:
      status: 200
      body:
        - path: data
          exists: true
```

### Load Testing Setup

```yaml
name: Load Test Preparation

steps:
  - name: Capture Baseline
    url: https://api.example.com
    har:
      file: ./recordings/baseline.har
    # Use responses to inform load test parameters
```

## Limitations

- HAR timing information is not replicated
- Cookies may need session management
- Some headers may need updating (dates, tokens)
- Binary data in HAR may need special handling

## Troubleshooting

### Request Fails

1. Check URL matches current environment
2. Verify authentication is still valid
3. Update expired tokens/cookies
4. Check if API has changed

### Missing Headers

1. Some headers are stripped by browsers
2. Add required headers manually
3. Check HAR for all expected headers

### Entry Not Found

1. Verify `entry_index` is valid
2. Check HAR file format
3. Ensure entries array is populated

---

See also:
- [workflow.md](workflow.md) - Main workflow reference
- [README.md](../README.md) - CLI HAR options
