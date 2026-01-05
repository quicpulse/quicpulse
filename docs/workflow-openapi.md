# Workflow OpenAPI Reference

OpenAPI specification integration for workflow steps.

## Overview

OpenAPI integration allows you to generate workflow steps directly from OpenAPI/Swagger specifications. This is useful for:

- Automatic API testing from specs
- Contract-first testing
- Generating test workflows from documentation
- Validating API implementation matches spec

## Quick Start

```yaml
name: OpenAPI-Driven Tests
base_url: https://api.example.com

steps:
  - name: Test Create User
    url: https://api.example.com
    openapi:
      spec: ./specs/api.yaml
      operation_id: createUser
    body:
      name: "Test User"
      email: "test@example.com"
```

## Configuration Reference

### OpenApiConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `spec` | string | required | Path to OpenAPI spec file (YAML/JSON) |
| `operation_id` | string | none | Operation ID to execute |
| `path` | string | none | API path (alternative to operation_id) |
| `method` | string | none | HTTP method (used with path) |

## Examples

### By Operation ID

Execute specific operation by ID:

```yaml
steps:
  - name: Create Pet
    url: https://petstore.example.com
    openapi:
      spec: ./specs/petstore.yaml
      operation_id: createPet
    body:
      name: "Fluffy"
      category: "cat"
```

### By Path and Method

Specify path and method directly:

```yaml
steps:
  - name: Get User by ID
    url: https://api.example.com
    openapi:
      spec: ./specs/api.yaml
      path: /users/{userId}
      method: GET
```

### Full CRUD Workflow

Complete API testing from spec:

```yaml
name: User API Test Suite
base_url: https://api.example.com

variables:
  test_email: "test-{uuid}@example.com"

steps:
  - name: Create User
    url: https://api.example.com
    openapi:
      spec: ./specs/api.yaml
      operation_id: createUser
    body:
      name: "Test User"
      email: "{{ test_email }}"
    extract:
      user_id: body.id
    assert:
      status: 201

  - name: Get User
    url: https://api.example.com
    openapi:
      spec: ./specs/api.yaml
      operation_id: getUser
    assert:
      status: 200

  - name: Update User
    url: https://api.example.com
    openapi:
      spec: ./specs/api.yaml
      operation_id: updateUser
    body:
      name: "Updated Name"
    assert:
      status: 200

  - name: Delete User
    url: https://api.example.com
    openapi:
      spec: ./specs/api.yaml
      operation_id: deleteUser
    assert:
      status: 204
```

### With Variable Substitution

Use workflow variables in OpenAPI requests:

```yaml
variables:
  user_id: "123"

steps:
  - name: Get Specific User
    url: https://api.example.com/users/{{ user_id }}
    openapi:
      spec: ./specs/api.yaml
      path: /users/{userId}
      method: GET
```

### OpenAPI with Assertions

Validate against spec expectations:

```yaml
steps:
  - name: Verify API Contract
    url: https://api.example.com
    openapi:
      spec: ./specs/api.yaml
      operation_id: listUsers
    assert:
      status: 200
      headers:
        - name: Content-Type
          contains: application/json
      body:
        - path: data
          type: array
        - path: meta.total
          type: number
```

### Multiple Specs

Use different specs for different services:

```yaml
steps:
  - name: Auth Service
    url: https://auth.example.com
    openapi:
      spec: ./specs/auth-api.yaml
      operation_id: login
    body:
      username: "user"
      password: "pass"
    extract:
      token: body.access_token

  - name: User Service
    url: https://users.example.com
    openapi:
      spec: ./specs/users-api.yaml
      operation_id: getProfile
    headers:
      Authorization: "Bearer {{ token }}"
```

## OpenAPI Spec Requirements

### Supported Versions

- OpenAPI 3.0.x
- OpenAPI 3.1.x
- Swagger 2.0 (limited)

### Spec Structure

```yaml
openapi: 3.0.3
info:
  title: Example API
  version: 1.0.0

paths:
  /users:
    get:
      operationId: listUsers      # Used by operation_id
      summary: List all users
      responses:
        '200':
          description: Success
    post:
      operationId: createUser
      summary: Create a user
      requestBody:
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/User'

  /users/{userId}:
    get:
      operationId: getUser
      parameters:
        - name: userId
          in: path
          required: true
```

## Integration Patterns

### Generate Then Customize

Use OpenAPI as starting point, then customize:

```yaml
steps:
  - name: OpenAPI Base Request
    url: https://api.example.com
    openapi:
      spec: ./specs/api.yaml
      operation_id: createOrder
    # Override/add custom values
    headers:
      X-Custom-Header: "value"
    body:
      items:
        - product_id: "123"
          quantity: 2
```

### OpenAPI + Extraction

Chain OpenAPI operations:

```yaml
steps:
  - name: Create Resource
    url: https://api.example.com
    openapi:
      spec: ./specs/api.yaml
      operation_id: createResource
    body:
      name: "Test Resource"
    extract:
      resource_id: body.id

  - name: Use Resource
    url: https://api.example.com/resources/{{ resource_id }}
    openapi:
      spec: ./specs/api.yaml
      path: /resources/{resourceId}
      method: GET
```

### OpenAPI + Scripting

Dynamic operation selection:

```yaml
steps:
  - name: Dynamic API Call
    url: https://api.example.com
    pre_script:
      code: |
        // Select operation based on condition
        if vars["use_v2"] {
          vars["operation"] = "createUserV2";
        } else {
          vars["operation"] = "createUser";
        }
    openapi:
      spec: ./specs/api.yaml
      operation_id: "{{ operation }}"
```

## Generating Workflows from OpenAPI

### CLI Generation

```bash
# Generate workflow from entire spec
quicpulse --import-openapi=api.yaml --generate-workflow=tests.yaml

# Generate for specific tags
quicpulse --import-openapi=api.yaml --generate-workflow=tests.yaml \
  --openapi-tags=users,orders

# Custom base URL
quicpulse --import-openapi=api.yaml --generate-workflow=tests.yaml \
  --openapi-base-url=https://staging.api.example.com
```

### Generated Workflow Features

- Automatic CRUD ordering
- Magic values for schema types
- Extracted IDs for chaining
- Status assertions from spec

## Best Practices

1. **Keep specs up to date** - Sync specs with actual API
2. **Use operation IDs** - More stable than paths
3. **Validate schemas** - Ensure spec is valid before use
4. **Override when needed** - Customize generated requests
5. **Chain operations** - Use extraction for realistic flows
6. **Version control specs** - Track spec changes

## Common Patterns

### Contract Testing

```yaml
name: API Contract Test
description: Verify API matches OpenAPI specification

steps:
  - name: Test Each Endpoint
    url: https://api.example.com
    openapi:
      spec: ./specs/api.yaml
      operation_id: "{{ operation_id }}"
    assert:
      # Status from spec
      status: 200
```

### Environment-Specific Testing

```yaml
environments:
  staging:
    api_spec: ./specs/staging-api.yaml
  production:
    api_spec: ./specs/production-api.yaml

steps:
  - name: Test API
    url: "{{ base_url }}"
    openapi:
      spec: "{{ api_spec }}"
      operation_id: healthCheck
```

### Integration Test Suite

```yaml
name: Integration Tests from OpenAPI

steps:
  # Setup
  - name: Create Test Data
    openapi:
      spec: ./specs/api.yaml
      operation_id: createTestData
    extract:
      test_id: body.id

  # Test Operations
  - name: Read Data
    openapi:
      spec: ./specs/api.yaml
      operation_id: getData

  - name: Update Data
    openapi:
      spec: ./specs/api.yaml
      operation_id: updateData

  # Cleanup
  - name: Delete Test Data
    openapi:
      spec: ./specs/api.yaml
      operation_id: deleteTestData
```

## Troubleshooting

### Operation Not Found

1. Check `operationId` matches spec exactly
2. Verify spec file path is correct
3. Ensure spec is valid YAML/JSON

### Path Not Found

1. Verify path matches spec paths
2. Check for typos in path
3. Ensure method matches path

### Schema Mismatch

1. Compare request body with spec schema
2. Check required fields
3. Verify data types match

---

See also:
- [workflow.md](workflow.md) - Main workflow reference
- [README.md](../README.md) - CLI OpenAPI options
