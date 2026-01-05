# GraphQL Workflow Documentation

Complete reference for GraphQL steps in workflows - queries, mutations, subscriptions, and schema introspection.

## Table of Contents

- [Overview](#overview)
- [Basic Configuration](#basic-configuration)
- [Queries](#queries)
  - [Simple Query](#simple-query)
  - [Query with Variables](#query-with-variables)
  - [Named Operations](#named-operations)
- [Mutations](#mutations)
- [Schema Introspection](#schema-introspection)
- [Variable Handling](#variable-handling)
  - [Static Variables](#static-variables)
  - [Dynamic Variables](#dynamic-variables)
  - [Chained Variables](#chained-variables)
- [Response Handling](#response-handling)
  - [Extraction](#extraction)
  - [Assertions](#assertions)
  - [Error Handling](#error-handling)
- [Authentication](#authentication)
- [Advanced Examples](#advanced-examples)
  - [CRUD Operations](#crud-operations)
  - [Pagination](#pagination)
  - [Nested Queries](#nested-queries)
- [Configuration Reference](#configuration-reference)

---

## Overview

GraphQL workflow steps allow you to:

- **Execute** queries and mutations against GraphQL APIs
- **Pass** variables to parameterized operations
- **Introspect** schemas to discover available types and operations
- **Extract** data from responses for chaining
- **Assert** on response structure and content

GraphQL steps are defined using the `graphql` field in a workflow step, with the URL pointing to your GraphQL endpoint.

---

## Basic Configuration

A minimal GraphQL step:

```yaml
steps:
  - name: Simple GraphQL Query
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        {
          users {
            id
            name
          }
        }
```

GraphQL requests are always sent as `POST` with `Content-Type: application/json`.

---

## Queries

### Simple Query

Query without variables:

```yaml
steps:
  - name: Get All Users
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        query {
          users {
            id
            name
            email
            createdAt
          }
        }
    extract:
      user_count: ".data.users | length"
      first_user_id: ".data.users[0].id"
```

### Query with Variables

Parameterized queries:

```yaml
steps:
  - name: Get User by ID
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        query GetUser($id: ID!) {
          user(id: $id) {
            id
            name
            email
            profile {
              avatar
              bio
            }
          }
        }
      variables:
        id: "{{ user_id }}"
    extract:
      user_name: ".data.user.name"
      user_email: ".data.user.email"
```

### Named Operations

Documents with multiple operations:

```yaml
steps:
  - name: Get User and Posts
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        query GetUser($id: ID!) {
          user(id: $id) { id name }
        }
        
        query GetUserPosts($userId: ID!) {
          posts(userId: $userId) { id title }
        }
      operation_name: GetUser
      variables:
        id: "{{ user_id }}"
```

---

## Mutations

Create, update, and delete data:

```yaml
steps:
  # Create
  - name: Create User
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        mutation CreateUser($input: CreateUserInput!) {
          createUser(input: $input) {
            id
            name
            email
          }
        }
      variables:
        input:
          name: "{{ user_name }}"
          email: "{email}"
          password: "{{ password }}"
    extract:
      new_user_id: ".data.createUser.id"
    assert:
      body:
        - path: data.createUser.id
          exists: true

  # Update
  - name: Update User
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        mutation UpdateUser($id: ID!, $input: UpdateUserInput!) {
          updateUser(id: $id, input: $input) {
            id
            name
            updatedAt
          }
        }
      variables:
        id: "{{ new_user_id }}"
        input:
          name: "Updated Name"
    assert:
      body:
        - path: data.updateUser.name
          equals: "Updated Name"

  # Delete
  - name: Delete User
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        mutation DeleteUser($id: ID!) {
          deleteUser(id: $id) {
            success
            message
          }
        }
      variables:
        id: "{{ new_user_id }}"
    assert:
      body:
        - path: data.deleteUser.success
          equals: true
```

---

## Schema Introspection

Query the GraphQL schema:

```yaml
steps:
  - name: Introspect Schema
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: ""  # Ignored when introspection is true
      introspection: true
    extract:
      query_type: ".data.__schema.queryType.name"
      mutation_type: ".data.__schema.mutationType.name"
      types: ".data.__schema.types"

  - name: Get Schema Types
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        {
          __schema {
            types {
              name
              kind
              description
            }
          }
        }
    extract:
      type_names: ".data.__schema.types[].name"
```

---

## Variable Handling

### Static Variables

Hard-coded values:

```yaml
graphql:
  query: |
    query GetPosts($limit: Int!, $status: PostStatus!) {
      posts(limit: $limit, status: $status) {
        id
        title
      }
    }
  variables:
    limit: 10
    status: "PUBLISHED"
```

### Dynamic Variables

Using workflow variables and magic values:

```yaml
variables:
  default_limit: 25

steps:
  - name: Search Posts
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        query SearchPosts($term: String!, $limit: Int!, $cursor: String) {
          searchPosts(term: $term, limit: $limit, after: $cursor) {
            edges {
              node { id title }
              cursor
            }
            pageInfo {
              hasNextPage
            }
          }
        }
      variables:
        term: "{{ search_term }}"
        limit: "{{ default_limit }}"
        cursor: null
```

### Chained Variables

Using extracted values from previous steps:

```yaml
steps:
  - name: Get Current User
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        query {
          me {
            id
            organizationId
          }
        }
    extract:
      user_id: ".data.me.id"
      org_id: ".data.me.organizationId"

  - name: Get Organization Members
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        query GetOrgMembers($orgId: ID!) {
          organization(id: $orgId) {
            members {
              id
              name
              role
            }
          }
        }
      variables:
        orgId: "{{ org_id }}"
```

---

## Response Handling

### Extraction

Extract data from GraphQL responses:

```yaml
steps:
  - name: Query with Extraction
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        query GetOrder($id: ID!) {
          order(id: $id) {
            id
            status
            items {
              productId
              quantity
              price
            }
            total
            customer {
              id
              name
            }
          }
        }
      variables:
        id: "{{ order_id }}"
    extract:
      # Direct path extraction
      order_status: ".data.order.status"
      order_total: ".data.order.total"
      customer_name: ".data.order.customer.name"
      
      # Array operations
      item_count: ".data.order.items | length"
      first_item: ".data.order.items[0]"
      all_product_ids: ".data.order.items[].productId"
```

### Assertions

Validate GraphQL responses:

```yaml
steps:
  - name: Query with Assertions
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        query GetProduct($id: ID!) {
          product(id: $id) {
            id
            name
            price
            inStock
            category { name }
          }
        }
      variables:
        id: "prod_123"
    assert:
      status: 200
      body:
        # Check for no errors
        - path: errors
          is_null: true
        
        # Check data structure
        - path: data.product.id
          equals: "prod_123"
        
        - path: data.product.price
          greater_than: 0
        
        - path: data.product.inStock
          type: boolean
        
        - path: data.product.category.name
          exists: true
```

### Error Handling

Handle GraphQL errors:

```yaml
steps:
  - name: Handle Errors
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        query GetProduct($id: ID!) {
          product(id: $id) {
            id
            name
          }
        }
      variables:
        id: "invalid_id"
    extract:
      has_errors: ".errors != null"
      error_message: ".errors[0].message"
      error_code: ".errors[0].extensions.code"
```

---

## Authentication

### Bearer Token

```yaml
steps:
  - name: Authenticated Query
    method: POST
    url: https://api.example.com/graphql
    headers:
      Authorization: "Bearer {{ access_token }}"
    graphql:
      query: |
        query {
          me {
            id
            name
            email
          }
        }
```

### API Key

```yaml
steps:
  - name: API Key Auth
    method: POST
    url: https://api.example.com/graphql
    headers:
      X-API-Key: "{{ api_key }}"
    graphql:
      query: |
        query {
          publicData {
            items { id }
          }
        }
```

### OAuth Flow

```yaml
steps:
  - name: Get OAuth Token
    method: POST
    url: https://auth.example.com/oauth/token
    form:
      grant_type: client_credentials
      client_id: "{{ client_id }}"
      client_secret: "{{ client_secret }}"
    extract:
      access_token: body.access_token

  - name: GraphQL with OAuth
    method: POST
    url: https://api.example.com/graphql
    headers:
      Authorization: "Bearer {{ access_token }}"
    graphql:
      query: |
        query {
          protectedResource {
            data
          }
        }
```

---

## Advanced Examples

### CRUD Operations

Complete CRUD workflow:

```yaml
name: Product CRUD Workflow
base_url: https://api.example.com/graphql

variables:
  product_name: "Test Product"
  product_price: 29.99

steps:
  # Create
  - name: Create Product
    method: POST
    url: ""
    graphql:
      query: |
        mutation CreateProduct($input: ProductInput!) {
          createProduct(input: $input) {
            id
            name
            price
          }
        }
      variables:
        input:
          name: "{{ product_name }}"
          price: "{{ product_price }}"
    extract:
      product_id: ".data.createProduct.id"
    assert:
      body:
        - path: data.createProduct.name
          equals: "{{ product_name }}"

  # Read
  - name: Get Product
    method: POST
    url: ""
    graphql:
      query: |
        query GetProduct($id: ID!) {
          product(id: $id) {
            id
            name
            price
          }
        }
      variables:
        id: "{{ product_id }}"
    assert:
      body:
        - path: data.product.id
          equals: "{{ product_id }}"

  # Update
  - name: Update Product
    method: POST
    url: ""
    graphql:
      query: |
        mutation UpdateProduct($id: ID!, $input: ProductInput!) {
          updateProduct(id: $id, input: $input) {
            id
            name
            price
          }
        }
      variables:
        id: "{{ product_id }}"
        input:
          price: 39.99
    assert:
      body:
        - path: data.updateProduct.price
          equals: 39.99

  # Delete
  - name: Delete Product
    method: POST
    url: ""
    graphql:
      query: |
        mutation DeleteProduct($id: ID!) {
          deleteProduct(id: $id) {
            success
          }
        }
      variables:
        id: "{{ product_id }}"
    assert:
      body:
        - path: data.deleteProduct.success
          equals: true

  # Verify deletion
  - name: Verify Deleted
    method: POST
    url: ""
    graphql:
      query: |
        query GetProduct($id: ID!) {
          product(id: $id) {
            id
          }
        }
      variables:
        id: "{{ product_id }}"
    assert:
      body:
        - path: data.product
          is_null: true
```

### Pagination

Cursor-based pagination:

```yaml
steps:
  - name: First Page
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        query GetUsers($first: Int!, $after: String) {
          users(first: $first, after: $after) {
            edges {
              node {
                id
                name
              }
              cursor
            }
            pageInfo {
              hasNextPage
              endCursor
            }
          }
        }
      variables:
        first: 10
    extract:
      has_next: ".data.users.pageInfo.hasNextPage"
      end_cursor: ".data.users.pageInfo.endCursor"
      users: ".data.users.edges[].node"

  - name: Second Page
    method: POST
    url: https://api.example.com/graphql
    skip_if: "{{ has_next }} == false"
    graphql:
      query: |
        query GetUsers($first: Int!, $after: String) {
          users(first: $first, after: $after) {
            edges {
              node { id name }
            }
            pageInfo {
              hasNextPage
              endCursor
            }
          }
        }
      variables:
        first: 10
        after: "{{ end_cursor }}"
```

### Nested Queries

Complex nested data:

```yaml
steps:
  - name: Nested Query
    method: POST
    url: https://api.example.com/graphql
    graphql:
      query: |
        query GetOrderDetails($orderId: ID!) {
          order(id: $orderId) {
            id
            status
            customer {
              id
              name
              addresses {
                type
                street
                city
              }
            }
            items {
              id
              quantity
              product {
                id
                name
                category {
                  name
                  parent {
                    name
                  }
                }
              }
            }
            payment {
              method
              status
              transactions {
                id
                amount
                timestamp
              }
            }
          }
        }
      variables:
        orderId: "{{ order_id }}"
    extract:
      customer_name: ".data.order.customer.name"
      shipping_address: ".data.order.customer.addresses[] | select(.type == \"SHIPPING\")"
      total_items: ".data.order.items | length"
      payment_status: ".data.order.payment.status"
```

---

## Configuration Reference

### GraphQL Configuration Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | Yes* | GraphQL query/mutation string |
| `variables` | object | No | Variables to pass to the operation |
| `operation_name` | string | No | Name of operation (for multi-operation documents) |
| `introspection` | boolean | No | Run standard introspection query instead |

*Required unless `introspection: true`

### Step-Level Options

These standard workflow step options work with GraphQL steps:

| Field | Type | Description |
|-------|------|-------------|
| `headers` | object | HTTP headers (Authorization, etc.) |
| `timeout` | string | Request timeout (e.g., "30s") |
| `extract` | object | Extract values from response |
| `assert` | object | Response assertions |
| `skip_if` | string | Conditional execution |
| `retries` | number | Retry on failure |

### Response Structure

GraphQL responses follow this structure:

```json
{
  "data": {
    "operationName": { ... }
  },
  "errors": [
    {
      "message": "Error message",
      "locations": [{"line": 1, "column": 2}],
      "path": ["field", "subfield"],
      "extensions": { "code": "ERROR_CODE" }
    }
  ]
}
```

Use JQ paths like `.data.operationName.field` for extraction.

---

## See Also

- [Workflow Documentation](workflow.md) - Complete workflow reference
- [Scripting Reference](script.md) - Rune scripting for workflows
- [WebSocket Workflow Reference](workflow-websocket.md) - WebSocket steps
