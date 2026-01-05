# Scripting Reference

Complete reference for scripting in workflows. QuicPulse supports **two scripting languages**:

- **Rune** - Default scripting language (Rust-like syntax)
- **JavaScript** - Full JavaScript support via QuickJS

Both languages have access to the same modules and functionality.

## Table of Contents

- [Language Selection](#language-selection)
- [Overview](#overview)
- [JavaScript Quick Reference](#javascript-quick-reference)
- [Script Context](#script-context)
- [Module Reference](#module-reference)
  - [crypto](#crypto-module) - Hashing, HMAC, random, UUID
  - [encoding](#encoding-module) - Base64, URL, hex encoding
  - [json](#json-module) - JSON parsing and manipulation
  - [xml](#xml-module) - XML parsing and conversion
  - [regex](#regex-module) - Pattern matching
  - [url](#url-module) - URL parsing and building
  - [date](#date-module) - Date/time operations
  - [cookie](#cookie-module) - Cookie handling
  - [jwt](#jwt-module) - JWT token inspection
  - [schema](#schema-module) - JSON Schema validation
  - [http](#http-module) - HTTP constants and helpers
  - [assert](#assert-module) - Test assertions
  - [env](#env-module) - Environment variables
  - [faker](#faker-module) - Test data generation
  - [prompt](#prompt-module) - Interactive input
  - [fs](#fs-module) - Sandboxed file access
  - [store](#store-module) - Workflow state storage
  - [console](#console-module) - Structured logging
  - [system](#system-module) - System utilities
  - [request](#request-module) - HTTP requests from scripts
- [Complete Examples](#complete-examples)
- [Best Practices](#best-practices)

---

## Language Selection

Scripts are detected automatically by file extension, or you can specify the language explicitly:

### By File Extension

| Extension | Language |
|-----------|----------|
| `.rn`, `.rune` | Rune |
| `.js`, `.mjs`, `.cjs` | JavaScript |

```yaml
steps:
  - name: Test
    pre_script:
      file: ./scripts/setup.js    # Detected as JavaScript
    post_script:
      file: ./scripts/validate.rn  # Detected as Rune
```

### Explicit Type Field

```yaml
steps:
  - name: Test
    pre_script:
      type: javascript   # or "js"
      code: |
        const token = crypto.uuid_v4();
        request.headers["X-Request-ID"] = token;

    script_assert:
      type: rune
      code: |
        response["status"] == 200
```

---

## Overview

Scripts use the [Rune](https://rune-rs.github.io/) programming language by default. Rune is a dynamic scripting language designed for embedding in Rust applications. Scripts can be used in three contexts within workflows:

| Context | Purpose | Available Data |
|---------|---------|----------------|
| `pre_script` | Execute before request | `vars` (read/write) |
| `post_script` | Execute after response | `vars`, `response` |
| `script_assert` | Validate response | `vars`, `response` |

### Basic Syntax

```rune
// Variables
let x = 42;
let name = "hello";
let list = [1, 2, 3];

// Conditionals
if x > 10 {
    println("big");
} else {
    println("small");
}

// Loops
for item in list {
    println(item);
}

// Functions
fn add(a, b) {
    a + b
}

// String interpolation
let msg = `Hello ${name}!`;
println(msg);
```

---

## JavaScript Quick Reference

JavaScript scripts have access to the same modules as Rune, but with JavaScript syntax.

### Basic Syntax

```javascript
// Variables
const x = 42;
let name = "hello";
const list = [1, 2, 3];

// Conditionals
if (x > 10) {
    console.log("big");
} else {
    console.log("small");
}

// Loops
for (const item of list) {
    console.log(item);
}

// Arrow functions
const add = (a, b) => a + b;

// Template literals
const msg = `Hello ${name}!`;
console.log(msg);
```

### Accessing Context

```javascript
// Request data (pre_script)
request.method       // HTTP method
request.url          // Request URL
request.headers      // Headers object
request.body         // Request body

// Response data (post_script, script_assert)
response.status      // Status code (number)
response.headers     // Response headers object
response.body        // Response body (string)

// Variables (via store module)
store.get("key")            // Read variable
store.set("key", "value")   // Write variable
```

### Module Access

All modules are available as global objects:

```javascript
// Crypto
const token = crypto.uuid_v4();
const hash = crypto.sha256_hex("data");

// JSON
const data = JSON.parse(response.body);  // Native JS
const value = json.query(jsonStr, "$.user.id");  // JSONPath

// Assert
assert.eq(response.status, 200);
assert.is_true(data.success);

// Console
console.log("Info message");
console.error("Error message");

// And all other modules: encoding, regex, url, date, cookie, jwt,
// schema, http, env, faker, prompt, fs, system, xml
```

### Example: Complete Pre-Script

```javascript
// Generate authentication headers
const timestamp = crypto.timestamp();
const nonce = crypto.uuid_v4();
const secret = store.get("api_secret");

const message = `${timestamp}:${nonce}:${request.body}`;
const signature = crypto.hmac_sha256(secret, message);

request.headers["X-Timestamp"] = String(timestamp);
request.headers["X-Nonce"] = nonce;
request.headers["X-Signature"] = signature;

console.log(`Request signed at ${timestamp}`);
```

### Example: Complete Post-Script

```javascript
// Parse and validate response
const body = JSON.parse(response.body);

if (response.status === 200) {
    // Extract data for next steps
    store.set("user_id", body.user.id);
    store.set("session_token", body.token);

    console.log(`Logged in as user ${body.user.id}`);
} else {
    console.error(`Login failed: ${body.error}`);
}
```

### Example: Script Assertion

```javascript
// Return boolean for assertion
const body = JSON.parse(response.body);

response.status === 200 &&
body.items !== undefined &&
body.items.length > 0 &&
body.items.every(item => item.id !== undefined)
```

---

## Script Context

### Variables (`vars`)

Access and modify workflow variables:

```rune
// Read a variable
let api_key = vars["api_key"];

// Write a variable
vars["token"] = "abc123";

// Use in calculations
vars["total"] = vars["quantity"] * vars["price"];

// Dynamic keys
let key = "user_id";
vars[key] = 12345;
```

### Response (`response`)

Available in `post_script` and `script_assert`:

```rune
// Response properties
let status = response["status"];        // HTTP status code (integer)
let body = response["body"];            // Response body (string)
let headers = response["headers"];      // Headers (object)
let latency = response["latency"];      // Response time in ms

// Access headers
let content_type = response["headers"]["content-type"];

// Parse JSON body
let data = json::parse(response["body"]);
let user_id = data["user"]["id"];
```

### Script Types

#### Pre-Script

Runs before the request is sent. Use to prepare data, generate signatures, etc.

```yaml
steps:
  - name: Signed Request
    method: POST
    url: /api/secure
    pre_script:
      code: |
        // Generate timestamp and signature
        let ts = crypto::timestamp();
        let payload = json::stringify(vars["request_data"]);
        let signature = crypto::hmac_sha256(vars["secret_key"], `${ts}:${payload}`);

        vars["timestamp"] = ts;
        vars["signature"] = signature;
    headers:
      X-Timestamp: "{{ timestamp }}"
      X-Signature: "{{ signature }}"
```

#### Post-Script

Runs after the response is received. Use to extract data, transform values, etc.

```yaml
steps:
  - name: Process Response
    method: GET
    url: /api/users
    post_script:
      code: |
        let body = json::parse(response["body"]);

        // Extract and transform data
        let users = body["data"];
        vars["user_count"] = json::len(users);

        // Store first user ID
        if json::len(users) > 0 {
            vars["first_user"] = json::get(users, "0.id");
        }

        // Log results
        println(`Found ${vars["user_count"]} users`);
```

#### Script Assert

Runs after post_script. Use for complex validation logic.

```yaml
steps:
  - name: Validate Response
    method: GET
    url: /api/orders
    script_assert:
      code: |
        let status = response["status"];
        let body = json::parse(response["body"]);

        // Status check
        assert::eq(status, 200);

        // Business logic validation
        let orders = body["orders"];
        assert::is_true(json::len(orders) > 0);

        // Validate each order
        for i in 0..json::len(orders) {
            let order = json::get(orders, `${i}`);
            let order_obj = json::parse(order);

            assert::is_true(order_obj["total"] > 0);
            assert::is_true(order_obj["status"] != "cancelled");
        }
```

### External Script Files

Reference external script files:

```yaml
steps:
  - name: Complex Request
    method: POST
    url: /api/data
    pre_script:
      file: scripts/prepare_request.rune
    post_script:
      file: scripts/process_response.rune
    script_assert:
      file: scripts/validate.rune
```

---

## Module Reference

### crypto Module

Cryptographic functions for hashing, HMAC, and random generation.

#### Hash Functions

| Function | Description | Returns |
|----------|-------------|---------|
| `sha256_hex(input)` | SHA-256 hash | 64-char hex string |
| `sha512_hex(input)` | SHA-512 hash | 128-char hex string |
| `md5_hex(input)` | MD5 hash | 32-char hex string |
| `sha1_hex(input)` | SHA-1 hash | 40-char hex string |

```rune
let hash = crypto::sha256_hex("hello world");
// "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"

let md5 = crypto::md5_hex("test");
// "098f6bcd4621d373cade4e832627b4f6"
```

#### HMAC Functions

| Function | Description | Returns |
|----------|-------------|---------|
| `hmac_sha256(key, message)` | HMAC-SHA256 | 64-char hex string |
| `hmac_sha512(key, message)` | HMAC-SHA512 | 128-char hex string |
| `hmac_sha256_base64(key, message)` | HMAC-SHA256 | Base64 string |

```rune
// Generate API signature
let timestamp = crypto::timestamp();
let message = `POST:/api/data:${timestamp}`;
let signature = crypto::hmac_sha256(vars["api_secret"], message);

vars["signature"] = signature;
vars["timestamp"] = timestamp;
```

#### Random Functions

| Function | Description | Returns |
|----------|-------------|---------|
| `random_hex(length)` | Random hex bytes | Hex string |
| `random_bytes_base64(length)` | Random bytes | Base64 string |
| `random_int(min, max)` | Random integer | Integer |
| `random_string(length)` | Random alphanumeric | String |
| `uuid_v4()` | UUID version 4 | UUID string |
| `uuid_v7()` | UUID version 7 (time-based) | UUID string |

```rune
let session_id = crypto::uuid_v4();
// "550e8400-e29b-41d4-a716-446655440000"

let token = crypto::random_hex(32);
// "a3f2b1c9e8d7f6a5b4c3d2e1f0a9b8c7..."

let otp = crypto::random_int(100000, 999999);
// 847291

let password = crypto::random_string(16);
// "aB3xK9mZ2pQ5wR8n"
```

#### Timestamp Functions

| Function | Description | Returns |
|----------|-------------|---------|
| `timestamp()` | Unix timestamp (seconds) | i64 |
| `timestamp_ms()` | Unix timestamp (milliseconds) | i64 |

```rune
let now = crypto::timestamp();
// 1705315800

let now_ms = crypto::timestamp_ms();
// 1705315800123
```

---

### encoding Module

String encoding and decoding utilities.

| Function | Description |
|----------|-------------|
| `base64_encode(input)` | Encode to Base64 |
| `base64_decode(input)` | Decode from Base64 |
| `url_encode(input)` | URL/percent encode |
| `url_decode(input)` | URL/percent decode |
| `hex_encode(input)` | Encode to hex |
| `hex_decode(input)` | Decode from hex |
| `html_escape(input)` | Escape HTML entities |

```rune
// Base64
let encoded = encoding::base64_encode("hello world");
// "aGVsbG8gd29ybGQ="

let decoded = encoding::base64_decode("aGVsbG8gd29ybGQ=");
// "hello world"

// URL encoding
let url_safe = encoding::url_encode("hello world & more");
// "hello%20world%20%26%20more"

let original = encoding::url_decode("hello%20world");
// "hello world"

// Hex encoding
let hex = encoding::hex_encode("ABC");
// "414243"

let text = encoding::hex_decode("414243");
// "ABC"

// HTML escaping
let safe = encoding::html_escape("<script>alert('xss')</script>");
// "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"
```

---

### json Module

JSON parsing, querying, and manipulation.

#### Parsing and Serialization

| Function | Description |
|----------|-------------|
| `parse(json_string)` | Parse JSON string |
| `stringify(json_string)` | Serialize to compact JSON |
| `stringify_pretty(json_string)` | Serialize with formatting |

```rune
// Parse response body
let data = json::parse(response["body"]);

// Access parsed values
let user_id = data["user"]["id"];
let items = data["items"];
```

#### Querying

| Function | Description |
|----------|-------------|
| `query(json, jsonpath)` | JSONPath query (returns array) |
| `query_first(json, jsonpath)` | JSONPath query (returns first match) |
| `get(json, path)` | Get value by dot notation |
| `keys(json)` | Get object keys as array |
| `values(json)` | Get object/array values |
| `len(json)` | Get length of array/object/string |
| `has(json, key)` | Check if key exists |

```rune
let json = response["body"];

// Dot notation access
let name = json::get(json, "user.profile.name");
let first_item = json::get(json, "items.0");

// JSONPath queries
let all_ids = json::query(json, "$.items[*].id");
let first_id = json::query_first(json, "$.items[0].id");

// Inspection
let count = json::len(json);
let obj_keys = json::keys(json);
let has_user = json::has(json, "user");
```

#### Type Checking

| Function | Description |
|----------|-------------|
| `is_object(json)` | Check if value is object |
| `is_array(json)` | Check if value is array |
| `is_string(json)` | Check if value is string |
| `is_number(json)` | Check if value is number |
| `is_bool(json)` | Check if value is boolean |
| `is_null(json)` | Check if value is null |
| `type_of(json)` | Get type name |

```rune
let json = response["body"];

if json::is_array(json) {
    println(`Array with ${json::len(json)} items`);
}

let value_type = json::type_of(json::get(json, "count"));
// "number"
```

#### Manipulation

| Function | Description |
|----------|-------------|
| `merge(base, overlay)` | Merge two objects (overlay wins) |
| `set(json, path, value)` | Set value at dot notation path |
| `remove(json, key)` | Remove key from object |

```rune
// Merge objects
let defaults = '{"timeout": 30, "retries": 3}';
let overrides = '{"timeout": 60}';
let config = json::merge(defaults, overrides);
// {"timeout": 60, "retries": 3}

// Set nested value
let obj = '{"user": {}}';
let updated = json::set(obj, "user.name", "\"John\"");
// {"user": {"name": "John"}}
```

#### Comparison

| Function | Description |
|----------|-------------|
| `equals(a, b)` | Check if two JSON values are equal |
| `diff(a, b)` | Get differences between objects |

```rune
let a = '{"x": 1, "y": 2}';
let b = '{"x": 1, "y": 3}';

let are_equal = json::equals(a, b);
// false

let changes = json::diff(a, b);
// {"added": {}, "removed": {}, "changed": {"y": {"from": 2, "to": 3}}}
```

---

### xml Module

XML parsing and querying.

| Function | Description |
|----------|-------------|
| `parse(xml)` / `to_json(xml)` | Convert XML to JSON |
| `get_text(xml, tag)` | Get text content of element |
| `get_attr(xml, tag, attr)` | Get attribute value |
| `count_elements(xml, tag)` | Count occurrences of tag |
| `has_element(xml, tag)` | Check if tag exists |
| `is_valid(xml)` | Check if XML is well-formed |

```rune
let xml = response["body"];

// Convert to JSON for easier access
let json = xml::to_json(xml);

// Direct queries
let title = xml::get_text(xml, "title");
let id = xml::get_attr(xml, "item", "id");
let item_count = xml::count_elements(xml, "item");

// Validation
if xml::is_valid(xml) {
    println("Valid XML");
}
```

#### XML to JSON Format

```xml
<root>
  <item id="123">Hello</item>
</root>
```

Converts to:

```json
{
  "root": {
    "#children": [
      {
        "item": {
          "@id": "123",
          "#text": "Hello"
        }
      }
    ]
  }
}
```

Attributes are prefixed with `@`, text content uses `#text`.

---

### regex Module

Regular expression operations.

#### Matching

| Function | Description |
|----------|-------------|
| `test(input, pattern)` | Check if pattern matches |
| `find(input, pattern)` | Find first match |
| `match_all(input, pattern)` | Find all matches (JSON array) |
| `count(input, pattern)` | Count matches |

```rune
let text = "Order #12345 and Order #67890";

// Test for match
let has_order = regex::test(text, r"Order #\d+");
// true

// Find first match
let first = regex::find(text, r"#(\d+)");
// "#12345"

// Find all matches
let all = regex::match_all(text, r"#\d+");
// ["#12345", "#67890"]

// Count matches
let count = regex::count(text, r"Order");
// 2
```

#### Capturing

| Function | Description |
|----------|-------------|
| `capture(input, pattern)` | Capture groups as array |
| `capture_named(input, pattern)` | Capture named groups as object |

```rune
let text = "John Smith, age 30";

// Positional capture groups
let groups = regex::capture(text, r"(\w+) (\w+), age (\d+)");
// ["John", "Smith", "30"]

// Named capture groups
let named = regex::capture_named(text, r"(?P<first>\w+) (?P<last>\w+), age (?P<age>\d+)");
// {"first": "John", "last": "Smith", "age": "30"}
```

#### Replacement

| Function | Description |
|----------|-------------|
| `replace(input, pattern, replacement)` | Replace first match |
| `replace_all(input, pattern, replacement)` | Replace all matches |

```rune
let text = "foo bar foo";

let result = regex::replace(text, "foo", "baz");
// "baz bar foo"

let result_all = regex::replace_all(text, "foo", "baz");
// "baz bar baz"
```

#### Utility

| Function | Description |
|----------|-------------|
| `split(input, pattern)` | Split by pattern (JSON array) |
| `escape(input)` | Escape regex special chars |
| `is_valid(pattern)` | Check if pattern is valid |

```rune
// Split
let parts = regex::split("a,b;c", r"[,;]");
// ["a", "b", "c"]

// Escape for literal matching
let escaped = regex::escape("hello.world");
// "hello\\.world"

// Validate pattern
let valid = regex::is_valid(r"[a-z]+");
// true
```

---

### url Module

URL parsing and manipulation.

#### Parsing

| Function | Description |
|----------|-------------|
| `parse(url)` | Parse URL to JSON object |
| `is_valid(url)` | Check if URL is valid |

```rune
let parsed = url::parse("https://user:pass@example.com:8080/path?key=value#section");
// {
//   "scheme": "https",
//   "host": "example.com",
//   "port": 8080,
//   "path": "/path",
//   "query": "key=value",
//   "fragment": "section",
//   "username": "user",
//   "password": "pass"
// }
```

#### Component Extraction

| Function | Description |
|----------|-------------|
| `scheme(url)` | Get scheme/protocol |
| `host(url)` | Get hostname |
| `port(url)` | Get port (-1 if not set) |
| `path(url)` | Get path |
| `query(url)` | Get query string |
| `fragment(url)` | Get fragment |
| `username(url)` | Get username |
| `password(url)` | Get password |

```rune
let u = "https://api.example.com:8080/v1/users?limit=10";

let scheme = url::scheme(u);    // "https"
let host = url::host(u);        // "api.example.com"
let port = url::port(u);        // 8080
let path = url::path(u);        // "/v1/users"
let query = url::query(u);      // "limit=10"
```

#### Query String Operations

| Function | Description |
|----------|-------------|
| `query_param(url, key)` | Get specific query param |
| `query_params(url)` | Get all params as JSON object |
| `set_query_param(url, key, value)` | Set/add query param |
| `remove_query_param(url, key)` | Remove query param |

```rune
let u = "https://api.example.com/search?q=test&page=1";

// Get param
let query = url::query_param(u, "q");
// "test"

// Get all params
let params = url::query_params(u);
// {"q": "test", "page": "1"}

// Modify params
let updated = url::set_query_param(u, "page", "2");
// "https://api.example.com/search?q=test&page=2"

let cleaned = url::remove_query_param(u, "page");
// "https://api.example.com/search?q=test"
```

#### Building/Modification

| Function | Description |
|----------|-------------|
| `join(base, relative)` | Join base URL with relative path |
| `set_path(url, path)` | Set the path component |
| `set_query(url, query)` | Set the query string |
| `set_fragment(url, fragment)` | Set the fragment |

```rune
// Join URLs
let full = url::join("https://api.example.com/v1/", "../v2/users");
// "https://api.example.com/v2/users"

// Modify components
let modified = url::set_path("https://example.com/old", "/new/path");
// "https://example.com/new/path"
```

#### Encoding

| Function | Description |
|----------|-------------|
| `encode(input)` | URL encode string |
| `decode(input)` | URL decode string |
| `encode_component(input)` | Strict component encoding |
| `decode_component(input)` | Decode component |

```rune
let encoded = url::encode("hello world");
// "hello%20world"

let decoded = url::decode("hello%20world");
// "hello world"
```

---

### date Module

Date/time parsing, formatting, and manipulation.

#### Current Time

| Function | Description |
|----------|-------------|
| `now()` | Current UTC time as ISO 8601 |
| `now_utc()` | Current UTC time (no offset) |
| `now_local()` | Current local time as ISO 8601 |
| `timestamp()` | Unix timestamp (seconds) |
| `timestamp_ms()` | Unix timestamp (milliseconds) |

```rune
let now = date::now();
// "2024-01-15T10:30:45+00:00"

let ts = date::timestamp();
// 1705315845
```

#### Parsing

| Function | Description |
|----------|-------------|
| `parse(date, format)` | Parse with custom format |
| `parse_iso(date)` | Parse ISO 8601 date |
| `parse_rfc2822(date)` | Parse RFC 2822 date |
| `from_timestamp(ts)` | Create from Unix timestamp (s) |
| `from_timestamp_ms(ts)` | Create from Unix timestamp (ms) |

```rune
// Parse ISO date
let dt = date::parse_iso("2024-01-15T10:30:00Z");

// Parse custom format
let dt2 = date::parse("2024/01/15 10:30:00", "%Y/%m/%d %H:%M:%S");

// From timestamp
let dt3 = date::from_timestamp(1705315800);
```

#### Formatting

| Function | Description |
|----------|-------------|
| `format(date, format)` | Format with custom pattern |
| `to_iso(date)` | Format as ISO 8601 |
| `to_rfc2822(date)` | Format as RFC 2822 |
| `to_timestamp(date)` | Convert to Unix timestamp |

```rune
let dt = date::now();

let formatted = date::format(dt, "%Y-%m-%d");
// "2024-01-15"

let rfc = date::to_rfc2822(dt);
// "Mon, 15 Jan 2024 10:30:45 +0000"
```

#### Date Components

| Function | Description |
|----------|-------------|
| `year(date)` | Get year |
| `month(date)` | Get month (1-12) |
| `day(date)` | Get day (1-31) |
| `hour(date)` | Get hour (0-23) |
| `minute(date)` | Get minute (0-59) |
| `second(date)` | Get second (0-59) |
| `weekday(date)` | Get weekday (0=Sun, 6=Sat) |
| `day_of_year(date)` | Get day of year (1-366) |

```rune
let dt = "2024-06-15T14:30:45+00:00";

let year = date::year(dt);      // 2024
let month = date::month(dt);    // 6
let day = date::day(dt);        // 15
let hour = date::hour(dt);      // 14
let weekday = date::weekday(dt); // 6 (Saturday)
```

#### Arithmetic

| Function | Description |
|----------|-------------|
| `add_days(date, n)` | Add n days |
| `add_hours(date, n)` | Add n hours |
| `add_minutes(date, n)` | Add n minutes |
| `add_seconds(date, n)` | Add n seconds |
| `subtract_days(date, n)` | Subtract n days |

```rune
let dt = "2024-01-15T10:00:00+00:00";

let tomorrow = date::add_days(dt, 1);
// "2024-01-16T10:00:00+00:00"

let next_hour = date::add_hours(dt, 1);
// "2024-01-15T11:00:00+00:00"

let yesterday = date::subtract_days(dt, 1);
// "2024-01-14T10:00:00+00:00"
```

#### Comparison

| Function | Description |
|----------|-------------|
| `diff_days(date1, date2)` | Difference in days |
| `diff_hours(date1, date2)` | Difference in hours |
| `diff_seconds(date1, date2)` | Difference in seconds |
| `is_before(date1, date2)` | Check if date1 < date2 |
| `is_after(date1, date2)` | Check if date1 > date2 |

```rune
let dt1 = "2024-01-01T00:00:00+00:00";
let dt2 = "2024-01-10T00:00:00+00:00";

let days = date::diff_days(dt1, dt2);
// 9

let before = date::is_before(dt1, dt2);
// true
```

#### Utility

| Function | Description |
|----------|-------------|
| `start_of_day(date)` | Get 00:00:00 of same day |
| `end_of_day(date)` | Get 23:59:59 of same day |
| `is_valid(date)` | Check if date is valid |

```rune
let dt = "2024-01-15T14:30:00+00:00";

let start = date::start_of_day(dt);
// "2024-01-15T00:00:00+00:00"

let end = date::end_of_day(dt);
// "2024-01-15T23:59:59+00:00"
```

---

### cookie Module

HTTP cookie parsing and manipulation.

#### Parsing

| Function | Description |
|----------|-------------|
| `parse(cookie_header)` | Parse Cookie header to JSON |
| `parse_set_cookie(header)` | Parse Set-Cookie header |
| `get(cookie_header, name)` | Get specific cookie value |

```rune
// Parse Cookie header
let header = "session=abc123; user=john; theme=dark";
let cookies = cookie::parse(header);
// {"session": "abc123", "user": "john", "theme": "dark"}

// Get specific cookie
let session = cookie::get(header, "session");
// "abc123"

// Parse Set-Cookie header
let set_cookie = "session=abc123; Path=/; HttpOnly; Secure; Max-Age=3600";
let details = cookie::parse_set_cookie(set_cookie);
// {
//   "name": "session",
//   "value": "abc123",
//   "path": "/",
//   "httpOnly": true,
//   "secure": true,
//   "maxAge": 3600
// }
```

#### Building

| Function | Description |
|----------|-------------|
| `build(name, value)` | Build simple cookie string |
| `build_set_cookie(json)` | Build Set-Cookie header |

```rune
// Simple cookie
let cookie = cookie::build("session", "abc123");
// "session=abc123"

// Full Set-Cookie
let set_cookie = cookie::build_set_cookie('{
    "name": "session",
    "value": "abc123",
    "path": "/",
    "secure": true,
    "httpOnly": true,
    "maxAge": 3600,
    "sameSite": "Strict"
}');
// "session=abc123; Path=/; Max-Age=3600; SameSite=Strict; Secure; HttpOnly"
```

#### Manipulation

| Function | Description |
|----------|-------------|
| `merge(cookies1, cookies2)` | Merge cookie strings |
| `remove(cookies, name)` | Remove cookie from string |
| `to_header(json)` | Convert JSON to Cookie header |

```rune
// Merge cookies
let merged = cookie::merge("a=1; b=2", "b=3; c=4");
// "a=1; b=3; c=4" (b is overwritten)

// Remove cookie
let filtered = cookie::remove("a=1; b=2; c=3", "b");
// "a=1; c=3"

// JSON to header
let header = cookie::to_header('{"session": "abc", "user": "john"}');
// "session=abc; user=john"
```

#### Validation

| Function | Description |
|----------|-------------|
| `is_expired(cookie_json)` | Check if cookie expired |
| `is_secure(cookie_json)` | Check Secure flag |
| `is_http_only(cookie_json)` | Check HttpOnly flag |

```rune
let cookie_json = cookie::parse_set_cookie("session=abc; HttpOnly; Secure");

let is_secure = cookie::is_secure(cookie_json);
// true

let is_http_only = cookie::is_http_only(cookie_json);
// true
```

---

### jwt Module

JWT token decoding and inspection (without cryptographic verification).

#### Decoding

| Function | Description |
|----------|-------------|
| `decode(token)` | Decode payload to JSON |
| `decode_header(token)` | Decode header to JSON |
| `decode_payload(token)` | Alias for decode() |

```rune
let token = vars["access_token"];

let payload = jwt::decode(token);
// {"sub": "1234567890", "name": "John Doe", "iat": 1516239022, "exp": 1705315800}

let header = jwt::decode_header(token);
// {"alg": "HS256", "typ": "JWT"}
```

#### Claim Accessors

| Function | Description |
|----------|-------------|
| `get_claim(token, name)` | Get specific claim |
| `get_exp(token)` | Get expiration timestamp |
| `get_iat(token)` | Get issued-at timestamp |
| `get_sub(token)` | Get subject |
| `get_iss(token)` | Get issuer |
| `get_aud(token)` | Get audience |

```rune
let token = vars["access_token"];

let subject = jwt::get_sub(token);
// "1234567890"

let issuer = jwt::get_iss(token);
// "auth.example.com"

let exp = jwt::get_exp(token);
// 1705315800
```

#### Validation Helpers

| Function | Description |
|----------|-------------|
| `is_expired(token)` | Check if token is expired |
| `expires_in(token)` | Seconds until expiration |
| `is_valid_format(token)` | Check JWT format |
| `parts_count(token)` | Count token parts |

```rune
let token = vars["access_token"];

if jwt::is_expired(token) {
    println("Token has expired!");
    // Trigger token refresh...
}

let remaining = jwt::expires_in(token);
println(`Token expires in ${remaining} seconds`);

// Format validation
if !jwt::is_valid_format(token) {
    println("Invalid JWT format");
}
```

---

### schema Module

JSON Schema validation.

| Function | Description |
|----------|-------------|
| `validate(json, schema)` | Validate and get detailed result |
| `is_valid(json, schema)` | Quick validation check |
| `errors(json, schema)` | Get validation errors only |

```rune
let data = response["body"];
let schema = '{
    "type": "object",
    "required": ["id", "name", "email"],
    "properties": {
        "id": {"type": "integer"},
        "name": {"type": "string"},
        "email": {"type": "string", "format": "email"}
    }
}';

// Quick check
if schema::is_valid(data, schema) {
    println("Data is valid");
} else {
    // Get detailed errors
    let errors = schema::errors(data, schema);
    println(`Validation errors: ${errors}`);
}

// Full validation result
let result = schema::validate(data, schema);
// {"valid": false, "errors": [{"path": "/email", "message": "..."}]}
```

#### Schema Helpers

| Function | Description |
|----------|-------------|
| `type_string()` | String type schema |
| `type_number()` | Number type schema |
| `type_integer()` | Integer type schema |
| `type_boolean()` | Boolean type schema |
| `type_array()` | Array type schema |
| `type_object()` | Object type schema |
| `email()` | Email format schema |
| `uuid()` | UUID format schema |
| `date()` | Date format schema |
| `url()` | URL format schema |

```rune
// These return schema fragments
let string_schema = schema::type_string();
// '{"type": "string"}'

let email_schema = schema::email();
// '{"type": "string", "format": "email"}'
```

---

### http Module

HTTP status code constants and helpers.

#### Status Code Constants

```rune
// Success
http::OK                    // 200
http::CREATED               // 201
http::ACCEPTED              // 202
http::NO_CONTENT            // 204

// Redirect
http::MOVED_PERMANENTLY     // 301
http::FOUND                 // 302
http::NOT_MODIFIED          // 304

// Client Error
http::BAD_REQUEST           // 400
http::UNAUTHORIZED          // 401
http::FORBIDDEN             // 403
http::NOT_FOUND             // 404
http::METHOD_NOT_ALLOWED    // 405
http::CONFLICT              // 409
http::GONE                  // 410
http::UNPROCESSABLE_ENTITY  // 422
http::TOO_MANY_REQUESTS     // 429

// Server Error
http::INTERNAL_SERVER_ERROR // 500
http::BAD_GATEWAY           // 502
http::SERVICE_UNAVAILABLE   // 503
http::GATEWAY_TIMEOUT       // 504
```

#### Status Code Helpers

| Function | Description |
|----------|-------------|
| `is_success(status)` | Check if 2xx |
| `is_redirect(status)` | Check if 3xx |
| `is_client_error(status)` | Check if 4xx |
| `is_server_error(status)` | Check if 5xx |
| `is_error(status)` | Check if 4xx or 5xx |

```rune
let status = response["status"];

if http::is_success(status) {
    println("Request succeeded");
} else if http::is_client_error(status) {
    println("Client error");
} else if http::is_server_error(status) {
    println("Server error");
}

// Using constants
assert::eq(response["status"], http::OK);
```

---

### assert Module

Assertion functions for testing and validation.

#### Basic Assertions

| Function | Description |
|----------|-------------|
| `eq(a, b)` | Assert a equals b |
| `ne(a, b)` | Assert a not equals b |
| `is_true(value)` | Assert value is true |
| `is_false(value)` | Assert value is false |

```rune
let status = response["status"];
let body = json::parse(response["body"]);

assert::eq(status, 200);
assert::ne(status, 500);
assert::is_true(json::has(body, "data"));
assert::is_false(json::is_null(body));
```

#### Comparison Assertions

| Function | Description |
|----------|-------------|
| `gt(a, b)` | Assert a > b |
| `gte(a, b)` | Assert a >= b |
| `lt(a, b)` | Assert a < b |
| `lte(a, b)` | Assert a <= b |

```rune
let count = json::get(body, "count");
let latency = response["latency"];

assert::gt(count, 0);
assert::lte(latency, 1000);
```

#### HTTP Status Assertions

| Function | Description |
|----------|-------------|
| `status_success(status)` | Assert 2xx status |
| `status_redirect(status)` | Assert 3xx status |
| `status_client_error(status)` | Assert 4xx status |
| `status_server_error(status)` | Assert 5xx status |

```rune
assert::status_success(response["status"]);
```

#### Soft Assertions (Non-Panicking)

Return boolean instead of panicking on failure:

| Function | Description |
|----------|-------------|
| `check_eq(a, b)` | Check equality |
| `check_ne(a, b)` | Check inequality |
| `check_gt(a, b)` | Check greater than |
| `check_gte(a, b)` | Check greater or equal |
| `check_lt(a, b)` | Check less than |
| `check_lte(a, b)` | Check less or equal |

```rune
// Collect multiple failures
let errors = [];

if !assert::check_eq(status, 200) {
    errors.push("Status should be 200");
}

if !assert::check_gt(count, 0) {
    errors.push("Count should be positive");
}

if errors.len() > 0 {
    panic(`Validation failed: ${errors}`);
}
```

---

### env Module

Environment variable access with security restrictions.

#### Allowed Variables

For security, only specific environment variables are accessible:
- System: `HOME`, `USER`, `LANG`, `LC_ALL`, `TZ`, `SHELL`, `TERM`, `PATH`, `PWD`
- Temp: `TMPDIR`, `TEMP`, `TMP`
- Proxy: `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`, `ALL_PROXY`
- Custom: Any variable starting with `QUICPULSE_` or `QP_`

| Function | Description |
|----------|-------------|
| `get(name)` | Get environment variable value |
| `get_or(name, default)` | Get with default fallback |
| `has(name)` | Check if variable exists |
| `os()` | Get operating system name |
| `arch()` | Get CPU architecture |
| `now()` | Unix timestamp (seconds) |
| `now_millis()` | Unix timestamp (milliseconds) |
| `now_iso()` | Current time as ISO 8601 |

```rune
// Environment variables
let user = env::get("USER");
let api_key = env::get_or("QP_API_KEY", "default_key");

if env::has("HTTP_PROXY") {
    println("Using proxy");
}

// System info
let os = env::os();       // "linux", "macos", "windows"
let arch = env::arch();   // "x86_64", "aarch64"

// Time
let ts = env::now();      // 1705315800
let iso = env::now_iso(); // "2024-01-15T10:30:00Z"
```

---

### faker Module

Generate realistic fake test data.

#### Name Generators

| Function | Description | Example |
|----------|-------------|---------|
| `name()` | Full name | "John Smith" |
| `first_name()` | First name | "Emma" |
| `last_name()` | Last name | "Johnson" |
| `name_with_title()` | Name with title | "Dr. Jane Doe" |
| `title()` | Title only | "Mr.", "Dr." |
| `suffix()` | Name suffix | "Jr.", "III" |

#### Internet Generators

| Function | Description | Example |
|----------|-------------|---------|
| `email()` | Random email | "user123@gmail.com" |
| `safe_email()` | Safe domain email | "john@example.org" |
| `free_email()` | Free provider email | "jane@yahoo.com" |
| `username()` | Username | "cooluser42" |
| `password()` | Random password (8-20 chars) | "xK9#mPq2" |
| `password_range(min, max)` | Password with length range | `password_range(12, 16)` |
| `domain()` | Domain suffix | "com", "org" |
| `ipv4()` | IPv4 address | "192.168.1.1" |
| `ipv6()` | IPv6 address | "2001:db8::1" |
| `mac_address()` | MAC address | "00:1B:44:11:3A:B7" |
| `user_agent()` | Browser user agent | "Mozilla/5.0..." |

#### Address Generators

| Function | Description | Example |
|----------|-------------|---------|
| `city()` | City name | "San Francisco" |
| `street_name()` | Street name | "Oak Avenue" |
| `street_address()` | Full street address | "123 Oak Avenue" |
| `zip_code()` | ZIP/postal code | "94102" |
| `state()` | State name | "California" |
| `state_abbr()` | State abbreviation | "CA" |
| `country()` | Country name | "United States" |
| `country_code()` | Country code | "US" |
| `latitude()` | Latitude (float) | 37.7749 |
| `longitude()` | Longitude (float) | -122.4194 |

#### Phone Generators

| Function | Description | Example |
|----------|-------------|---------|
| `phone_number()` | Phone number | "+1-555-123-4567" |
| `cell_number()` | Cell phone | "+1-555-987-6543" |

#### Company Generators

| Function | Description | Example |
|----------|-------------|---------|
| `company_name()` | Company name | "Acme Corporation" |
| `company_suffix()` | Company suffix | "Inc.", "LLC" |
| `industry()` | Industry name | "Technology" |
| `profession()` | Profession | "Software Engineer" |
| `buzzword()` | Business buzzword | "synergy" |
| `catch_phrase()` | Marketing phrase | "Innovative solutions" |

#### Lorem Ipsum Generators

| Function | Description |
|----------|-------------|
| `word()` | Single word |
| `words()` | 3-8 words |
| `sentence()` | 5-12 word sentence |
| `sentences()` | 2-5 sentences |
| `paragraph()` | 3-7 sentence paragraph |
| `paragraphs()` | 2-4 paragraphs |

#### Other Generators

| Function | Description |
|----------|-------------|
| `credit_card_number()` | Credit card number |
| `file_name()` | File name |
| `file_path()` | File path |
| `file_extension()` | File extension |
| `mime_type()` | MIME type |
| `bool()` | Random boolean (50% chance) |
| `bool_ratio(percent)` | Boolean with custom probability |
| `number()` | Random integer |
| `number_range(min, max)` | Integer in range |
| `float()` | Random float 0.0-1.0 |
| `float_range(min, max)` | Float in range |

```rune
// Generate test user
let user = {
    "name": faker::name(),
    "email": faker::safe_email(),
    "phone": faker::phone_number(),
    "address": {
        "street": faker::street_address(),
        "city": faker::city(),
        "state": faker::state_abbr(),
        "zip": faker::zip_code()
    }
};

// Random values
let age = faker::number_range(18, 65);
let is_active = faker::bool_ratio(80);  // 80% chance true
let score = faker::float_range(0.0, 100.0);
```

---

### prompt Module

Interactive user input during script execution.

| Function | Description |
|----------|-------------|
| `text(message)` | Prompt for text input |
| `text_default(message, default)` | Text input with default |
| `password(message)` | Hidden password input |
| `confirm(message)` | Yes/no confirmation |
| `confirm_default(message, default)` | Confirmation with default |
| `select(message, options)` | Select from options (returns index) |

```rune
// Text input
let username = prompt::text("Enter username: ");
let api_key = prompt::text_default("API Key: ", "default_key");

// Password (hidden)
let password = prompt::password("Enter password: ");

// Confirmation
if prompt::confirm("Proceed with request?") {
    // continue...
}

let should_retry = prompt::confirm_default("Retry on failure?", true);

// Selection (options comma-separated)
let choice = prompt::select("Choose environment:", "dev,staging,production");
// Returns 0 for dev, 1 for staging, 2 for production

if choice == 0 {
    vars["base_url"] = "http://localhost:3000";
} else if choice == 1 {
    vars["base_url"] = "https://staging.example.com";
} else {
    vars["base_url"] = "https://api.example.com";
}
```

---

### fs Module

Sandboxed file system access with security restrictions.

#### Security

- Only allows reading from current directory and temp directory
- Blocks sensitive paths: `.env`, `.git`, `.ssh`, `credentials`, `secrets`, `private`, `password`

#### Reading

| Function | Description |
|----------|-------------|
| `read(path)` | Read file to string |
| `read_lines(path)` | Read file (same as read) |
| `read_json(path)` | Read and parse JSON file |

#### File Info

| Function | Description |
|----------|-------------|
| `exists(path)` | Check if path exists |
| `is_file(path)` | Check if path is a file |
| `is_dir(path)` | Check if path is a directory |
| `size(path)` | Get file size in bytes |

#### Path Utilities

| Function | Description |
|----------|-------------|
| `join(base, path)` | Join path components |
| `basename(path)` | Get file name from path |
| `dirname(path)` | Get directory from path |
| `extension(path)` | Get file extension |
| `temp_dir()` | Get temp directory path |
| `cwd()` | Get current working directory |

```rune
// Read files
let config = fs::read("config.json");
let data = fs::read_json("data/payload.json");

// Check files
if fs::exists("template.json") {
    let size = fs::size("template.json");
    println(`Template is ${size} bytes`);
}

// Path utilities
let full_path = fs::join(fs::cwd(), "data/file.txt");
let name = fs::basename("/path/to/file.json");  // "file.json"
let dir = fs::dirname("/path/to/file.json");    // "/path/to"
let ext = fs::extension("file.json");           // "json"

// Temp directory
let temp = fs::temp_dir();
let temp_file = fs::join(temp, "output.json");
```

---

### store Module

Global key-value store that persists across workflow steps.

#### Basic Operations

| Function | Description |
|----------|-------------|
| `get(key)` | Get value as JSON string |
| `set(key, value)` | Set value from JSON string |
| `delete(key)` | Delete a key |
| `has(key)` | Check if key exists |

#### Typed Operations

| Function | Description |
|----------|-------------|
| `get_string(key)` | Get as string |
| `set_string(key, value)` | Set string value |
| `get_int(key)` | Get as integer |
| `set_int(key, value)` | Set integer value |
| `get_float(key)` | Get as float |
| `set_float(key, value)` | Set float value |
| `get_bool(key)` | Get as boolean |
| `set_bool(key, value)` | Set boolean value |
| `get_json(key)` | Get as JSON string |
| `set_json(key, value)` | Set from JSON string |

#### Utility Functions

| Function | Description |
|----------|-------------|
| `keys()` | Get all keys (comma-separated) |
| `clear()` | Clear all values |
| `count()` | Count number of keys |
| `incr(key)` | Increment integer value |
| `decr(key)` | Decrement integer value |

#### List Operations

| Function | Description |
|----------|-------------|
| `push(key, value)` | Push to list |
| `pop(key)` | Pop from list |
| `list_len(key)` | Get list length |

```rune
// Store values from login step
store::set_string("auth_token", response["headers"]["authorization"]);
store::set_int("user_id", json::get(body, "user.id"));

// Use in later step
let token = store::get_string("auth_token");
vars["token"] = token;

// Counting/tracking
let count = store::incr("request_count");
println(`Request #${count}`);

// List operations
store::push("processed_ids", vars["id"]);
store::push("processed_ids", vars["id2"]);
let total = store::list_len("processed_ids");

// Clean up
store::clear();
```

---

### console Module

Structured logging to stderr (doesn't interfere with JSON output).

#### Logging Levels

| Function | Description | Color |
|----------|-------------|-------|
| `log(msg)` / `info(msg)` | Info message | Cyan |
| `warn(msg)` | Warning message | Yellow |
| `error(msg)` | Error message | Red |
| `debug(msg)` | Debug (verbose mode only) | Magenta |
| `trace(msg)` | Trace (very verbose only) | Gray |
| `success(msg)` | Success with checkmark | Green |
| `fail(msg)` | Failure with X | Red |

#### Output

| Function | Description |
|----------|-------------|
| `print(msg)` | Print without newline |
| `println(msg)` | Print with newline |
| `newline()` | Print blank line |
| `hr()` | Print horizontal rule |
| `json(json_str)` | Pretty-print JSON |
| `table(json_array)` | Print array as table |

#### Timing

| Function | Description |
|----------|-------------|
| `time(label)` | Start a timer |
| `time_end(label)` | End timer and print elapsed |

#### Grouping

| Function | Description |
|----------|-------------|
| `group(label)` | Start indented group |
| `group_end()` | End group |

#### Progress

| Function | Description |
|----------|-------------|
| `progress(msg, percent)` | Show progress bar |

```rune
// Logging
console::info("Starting request...");
console::warn("Rate limit approaching");
console::error("Request failed!");
console::debug("This only shows with -v flag");

// Success/failure
if response["status"] == 200 {
    console::success("Request succeeded");
} else {
    console::fail("Request failed");
}

// Timing
console::time("api_call");
// ... make request ...
console::time_end("api_call");  // Prints: ⏱ api_call: 234.56ms

// Grouping
console::group("User Validation");
console::info("Checking email...");
console::info("Checking permissions...");
console::group_end();

// Pretty print JSON
console::json(response["body"]);

// Progress (for loops)
for i in 0..100 {
    console::progress("Processing", i);
}
```

---

### system Module

System utilities for timing, delays, and system information.

#### Sleep/Delay

| Function | Description |
|----------|-------------|
| `sleep(ms)` | Sleep for milliseconds (max 5 min) |
| `sleep_secs(secs)` | Sleep for seconds (max 5 min) |

#### Time Functions

| Function | Description |
|----------|-------------|
| `now()` | Current time in milliseconds |
| `now_secs()` | Current time in seconds |
| `timestamp()` | ISO 8601 timestamp |

#### System Info

| Function | Description | Example |
|----------|-------------|---------|
| `platform()` | Operating system | "linux", "macos", "windows" |
| `arch()` | CPU architecture | "x86_64", "aarch64" |
| `hostname()` | Machine hostname | "my-machine" |
| `username()` | Current user | "john" |
| `home_dir()` | User home directory | "/home/john" |

#### Process Info

| Function | Description |
|----------|-------------|
| `pid()` | Current process ID |
| `args()` | Command line arguments (comma-separated) |

```rune
// Delays (useful for rate limiting)
system::sleep(1000);     // Wait 1 second
system::sleep_secs(5);   // Wait 5 seconds

// Timing
let start = system::now();
// ... do work ...
let elapsed = system::now() - start;
println(`Took ${elapsed}ms`);

// Timestamp
let ts = system::timestamp();
// "2024-01-15T10:30:45.123Z"

// System info
let os = system::platform();
let arch = system::arch();
let host = system::hostname();
println(`Running on ${os}/${arch} (${host})`);

// User info
let user = system::username();
let home = system::home_dir();
```

---

### request Module

Make HTTP requests from within scripts. Useful for intra-workflow requests, chained authentication, or ad-hoc API calls.

#### HTTP Methods

| Function | Description |
|----------|-------------|
| `get(url)` | Perform GET request |
| `post(url, body)` | Perform POST request |
| `put(url, body)` | Perform PUT request |
| `patch(url, body)` | Perform PATCH request |
| `delete(url)` | Perform DELETE request |
| `send(method, url, body)` | Generic HTTP request |

All functions return a JSON string with the response data:

```json
{
  "status": 200,
  "ok": true,
  "body": "...",
  "json": { ... },
  "headers": { "content-type": "application/json" },
  "duration_ms": 123,
  "error": null
}
```

#### Basic Usage

```rune
// Simple GET request
let response = request::get("https://api.example.com/health");
let data = json::parse(response);

if data["ok"] {
    println(`Health check passed in ${data["duration_ms"]}ms`);
}

// POST with JSON body
let body = '{"username": "test", "password": "secret"}';
let response = request::post("https://api.example.com/login", body);
let data = json::parse(response);

if data["status"] == 200 {
    let json_body = data["json"];
    vars["token"] = json::get(json_body, "token");
}
```

#### Chained Authentication

```rune
// First, get an auth token
let auth_response = request::post(
    "https://auth.example.com/token",
    '{"client_id": "my_app", "grant_type": "client_credentials"}'
);
let auth_data = json::parse(auth_response);

if auth_data["ok"] {
    let token = json::get(auth_data["json"], "access_token");
    vars["access_token"] = token;
    println("Authentication successful");
} else {
    panic(`Auth failed: ${auth_data["error"]}`);
}
```

#### Generic Request

```rune
// Using the generic send function
let response = request::send("PATCH", "https://api.example.com/users/123", '{"name": "Updated"}');
let data = json::parse(response);

assert::eq(data["status"], 200);
```

#### Error Handling

```rune
let response = request::get("https://api.example.com/data");
let data = json::parse(response);

if data["error"] != () {
    console::error(`Request failed: ${data["error"]}`);
} else if !data["ok"] {
    console::warn(`Request returned status ${data["status"]}`);
    console::warn(`Body: ${data["body"]}`);
} else {
    // Process successful response
    let items = json::get(data["json"], "items");
    println(`Received ${json::len(items)} items`);
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `status` | integer | HTTP status code (0 if request failed) |
| `ok` | boolean | True if status is 2xx |
| `body` | string | Raw response body |
| `json` | object/null | Parsed JSON body (null if not JSON) |
| `headers` | object | Response headers |
| `duration_ms` | integer | Request duration in milliseconds |
| `error` | string/null | Error message (null if no error) |

---

## Complete Examples

### API Signature Generation

```yaml
steps:
  - name: Signed API Request
    method: POST
    url: /api/secure/resource
    pre_script:
      code: |
        // Build canonical request
        let method = "POST";
        let path = "/api/secure/resource";
        let timestamp = crypto::timestamp();
        let nonce = crypto::uuid_v4();
        let body = json::stringify(vars["request_body"]);

        // Create signature
        let string_to_sign = `${method}\n${path}\n${timestamp}\n${nonce}\n${body}`;
        let signature = crypto::hmac_sha256_base64(vars["api_secret"], string_to_sign);

        // Set headers
        vars["x_timestamp"] = timestamp;
        vars["x_nonce"] = nonce;
        vars["x_signature"] = signature;
    headers:
      X-Timestamp: "{{ x_timestamp }}"
      X-Nonce: "{{ x_nonce }}"
      X-Signature: "{{ x_signature }}"
      Content-Type: application/json
    body: "{{ request_body }}"
```

### OAuth Token Refresh

```yaml
steps:
  - name: Use API with Token Refresh
    method: GET
    url: /api/protected
    pre_script:
      code: |
        // Check if token needs refresh
        let token = vars["access_token"];

        if jwt::is_valid_format(token) {
            let expires_in = jwt::expires_in(token);

            if expires_in < 300 {  // Less than 5 minutes
                println("Token expiring soon, should refresh");
                vars["need_refresh"] = true;
            }
        }
    auth:
      type: bearer
      token: "{{ access_token }}"
```

### Complex Response Validation

```yaml
steps:
  - name: Validate Order Response
    method: GET
    url: /api/orders/{{ order_id }}
    script_assert:
      code: |
        let status = response["status"];
        let body = json::parse(response["body"]);

        // Status check
        assert::eq(status, http::OK);

        // Required fields
        assert::is_true(json::has(body, "id"));
        assert::is_true(json::has(body, "items"));
        assert::is_true(json::has(body, "total"));

        // Type checks
        assert::eq(json::type_of(json::get(body, "items")), "array");
        assert::eq(json::type_of(json::get(body, "total")), "number");

        // Business rules
        let items = json::get(body, "items");
        let item_count = json::len(items);
        assert::gt(item_count, 0);

        // Validate item structure
        for i in 0..item_count {
            let item = json::parse(json::get(items, `${i}`));

            assert::is_true(json::has(item, "product_id"));
            assert::is_true(json::has(item, "quantity"));
            assert::is_true(json::has(item, "price"));

            let qty = json::get(item, "quantity");
            let price = json::get(item, "price");

            // Parse numeric values
            let qty_num = json::parse(qty);
            let price_num = json::parse(price);

            assert::gt(qty_num, 0);
            assert::gt(price_num, 0);
        }

        // Check response time
        let latency = response["latency"];
        assert::lt(latency, 2000);  // Under 2 seconds

        println("All validations passed!");
```

### Cookie Session Management

```yaml
steps:
  - name: Login
    method: POST
    url: /auth/login
    body: '{"username": "test", "password": "secret"}'
    post_script:
      code: |
        let set_cookie = response["headers"]["set-cookie"];
        let session = cookie::parse_set_cookie(set_cookie);

        vars["session_cookie"] = cookie::get(set_cookie, "session");
        vars["session_details"] = session;

        println(`Got session: ${vars["session_cookie"]}`);

  - name: Access Protected Resource
    method: GET
    url: /api/protected
    headers:
      Cookie: "session={{ session_cookie }}"
    script_assert:
      code: |
        assert::eq(response["status"], 200);

        // Check session hasn't expired
        let session = json::parse(vars["session_details"]);
        assert::is_false(cookie::is_expired(vars["session_details"]));
```

### Date-Based API Testing

```yaml
steps:
  - name: Get Recent Events
    method: GET
    url: /api/events
    pre_script:
      code: |
        // Get events from last 7 days
        let now = date::now();
        let week_ago = date::subtract_days(now, 7);

        vars["start_date"] = date::format(week_ago, "%Y-%m-%d");
        vars["end_date"] = date::format(now, "%Y-%m-%d");
    query:
      start: "{{ start_date }}"
      end: "{{ end_date }}"
    script_assert:
      code: |
        let body = json::parse(response["body"]);
        let events = json::get(body, "events");

        // Verify all events are within date range
        let start = date::parse_iso(vars["start_date"] + "T00:00:00Z");
        let end = date::end_of_day(date::parse_iso(vars["end_date"] + "T00:00:00Z"));

        for i in 0..json::len(events) {
            let event = json::parse(json::get(events, `${i}`));
            let event_date = json::get(event, "date");
            let event_dt = date::parse_iso(event_date);

            assert::is_true(date::is_after(event_dt, start) || date::diff_seconds(start, event_dt) == 0);
            assert::is_true(date::is_before(event_dt, end) || date::diff_seconds(event_dt, end) == 0);
        }
```

---

## Best Practices

### 1. Parse Once, Use Many

```rune
// Good: Parse once
let body = json::parse(response["body"]);
let user = json::get(body, "user");
let name = json::get(user, "name");
let email = json::get(user, "email");

// Avoid: Parsing multiple times
let name = json::get(json::parse(response["body"]), "user.name");
let email = json::get(json::parse(response["body"]), "user.email");
```

### 2. Use Soft Assertions for Multiple Checks

```rune
// Collect all failures instead of stopping at first
let failures = [];

if !assert::check_eq(status, 200) {
    failures.push(`Expected status 200, got ${status}`);
}

if !assert::check_gt(count, 0) {
    failures.push("Count should be positive");
}

if !assert::check_lt(latency, 1000) {
    failures.push(`Latency ${latency}ms exceeds threshold`);
}

if failures.len() > 0 {
    let msg = "";
    for f in failures {
        msg = `${msg}\n- ${f}`;
    }
    panic(`Validation failures:${msg}`);
}
```

### 3. Validate Before Use

```rune
// Check format before decoding
let token = vars["token"];
if jwt::is_valid_format(token) {
    let exp = jwt::get_exp(token);
    // Use exp...
} else {
    println("Invalid token format");
}

// Check JSON validity
if json::is_object(response["body"]) {
    let data = json::parse(response["body"]);
    // Use data...
}
```

### 4. Use Constants for Magic Numbers

```rune
// Good: Use http constants
assert::eq(response["status"], http::OK);
assert::eq(response["status"], http::NOT_FOUND);

// Avoid: Magic numbers
assert::eq(response["status"], 200);
assert::eq(response["status"], 404);
```

### 5. Log for Debugging

```rune
// Add debugging output
println(`Request starting at ${date::now()}`);
println(`Using token that expires in ${jwt::expires_in(vars["token"])} seconds`);

let body = json::parse(response["body"]);
println(`Received ${json::len(json::get(body, "items"))} items`);
println(`Response time: ${response["latency"]}ms`);
```

### 6. Handle Missing Data

```rune
// Check existence before access
let body = json::parse(response["body"]);

if json::has(body, "error") {
    let error = json::get(body, "error.message");
    panic(`API error: ${error}`);
}

// Use type checking
let count = json::get(body, "count");
if json::type_of(count) == "number" {
    assert::gt(json::parse(count), 0);
}
```

---

See also:
- [workflow.md](workflow.md) - Workflow reference
- [README.md](../README.md) - CLI reference and features
