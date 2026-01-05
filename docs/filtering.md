# Data Filtering

QuicPulse provides powerful data filtering capabilities for JSON responses using JQ expressions, and can output data as ASCII tables or CSV.

## JQ Filtering

Use JQ expressions to filter and transform JSON responses.

### Basic Usage

```bash
# Extract a field
quicpulse -J '.name' example.com/api/user

# Extract nested field
quicpulse -J '.data.user.email' example.com/api/user

# Get array element
quicpulse -J '.[0]' example.com/api/users

# Get all names from array
quicpulse -J '.[].name' example.com/api/users
```

### Field Access

| Expression | Description | Example Output |
|------------|-------------|----------------|
| `.field` | Get field value | `"John"` |
| `.nested.field` | Nested field | `"value"` |
| `.[0]` | Array index | First element |
| `.[-1]` | Last element | Last element |
| `.[]` | Iterate array | One result per item |

### Common Expressions

```bash
# Get all keys
quicpulse -J 'keys' example.com/api

# Get array length
quicpulse -J 'length' example.com/api/users

# Select specific fields
quicpulse -J '.[] | {name, email}' example.com/api/users

# Filter array elements
quicpulse -J '.[] | select(.active)' example.com/api/users

# Map values
quicpulse -J '[.[] | .name]' example.com/api/users
```

### Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `\|` | Pipe (chain filters) | `.data \| .users` |
| `,` | Multiple outputs | `.name, .email` |
| `+` | Addition/concatenation | `.first + " " + .last` |
| `-` | Subtraction | `.total - .discount` |
| `*` | Multiplication | `.price * .quantity` |
| `/` | Division | `.total / .count` |
| `//` | Alternative (if null) | `.value // "default"` |
| `==` | Equality | `. == "active"` |
| `!=` | Not equal | `. != null` |
| `<` `>` | Comparison | `. > 100` |
| `and` `or` `not` | Logical | `.a and .b` |

### Select and Filter

```bash
# Filter by condition
quicpulse -J '.[] | select(.age > 21)' example.com/api/users

# Multiple conditions
quicpulse -J '.[] | select(.active and .verified)' example.com/api/users

# Select by type
quicpulse -J '.[] | select(type == "object")' example.com/api/data

# Filter null values
quicpulse -J '.[] | select(.email != null)' example.com/api/users
```

### Transformations

```bash
# Create new object
quicpulse -J '{name: .name, total: (.price * .qty)}' example.com/api/product

# Array from object
quicpulse -J 'to_entries' example.com/api/config

# Object from array
quicpulse -J 'from_entries' example.com/api/pairs

# Sort array
quicpulse -J 'sort_by(.name)' example.com/api/users

# Reverse array
quicpulse -J 'reverse' example.com/api/items

# Unique values
quicpulse -J '[.[] | .category] | unique' example.com/api/products
```

### String Functions

| Function | Description | Example |
|----------|-------------|---------|
| `ascii_downcase` | Lowercase | `"ABC" \| ascii_downcase` |
| `ascii_upcase` | Uppercase | `"abc" \| ascii_upcase` |
| `split("x")` | Split string | `"a,b" \| split(",")` |
| `join("x")` | Join array | `["a","b"] \| join(",")` |
| `ltrimstr("x")` | Remove prefix | `"hello" \| ltrimstr("hel")` |
| `rtrimstr("x")` | Remove suffix | `"hello" \| rtrimstr("lo")` |
| `startswith("x")` | Check prefix | `"hello" \| startswith("hel")` |
| `endswith("x")` | Check suffix | `"hello" \| endswith("lo")` |
| `contains("x")` | Contains | `"hello" \| contains("ell")` |
| `test("regex")` | Regex match | `"hello" \| test("^h")` |

### Array Functions

| Function | Description | Example |
|----------|-------------|---------|
| `length` | Array length | `[1,2,3] \| length` → `3` |
| `first` | First element | `[1,2,3] \| first` → `1` |
| `last` | Last element | `[1,2,3] \| last` → `3` |
| `nth(n)` | Nth element | `[1,2,3] \| nth(1)` → `2` |
| `flatten` | Flatten nested | `[[1],[2]] \| flatten` |
| `group_by(.x)` | Group by field | Group by category |
| `sort_by(.x)` | Sort by field | Sort by name |
| `min_by(.x)` | Min by field | Minimum price |
| `max_by(.x)` | Max by field | Maximum price |
| `add` | Sum array | `[1,2,3] \| add` → `6` |

### Object Functions

| Function | Description | Example |
|----------|-------------|---------|
| `keys` | Get keys | `{a:1} \| keys` → `["a"]` |
| `values` | Get values | `{a:1} \| values` → `[1]` |
| `has("key")` | Has key | `{a:1} \| has("a")` → `true` |
| `in(obj)` | Key in object | `"a" \| in({a:1})` |
| `to_entries` | To key-value pairs | `{a:1}` → `[{key,value}]` |
| `from_entries` | From pairs | Reverse of above |
| `with_entries(f)` | Transform entries | Modify keys/values |

---

## Table Output

Format JSON arrays as ASCII tables:

```bash
# Output as table
quicpulse --table example.com/api/users
```

### Example Output

```
┌────┬──────────┬─────────────────────┬────────┐
│ id │ name     │ email               │ active │
├────┼──────────┼─────────────────────┼────────┤
│ 1  │ John     │ john@example.com    │ true   │
│ 2  │ Jane     │ jane@example.com    │ true   │
│ 3  │ Bob      │ bob@example.com     │ false  │
└────┴──────────┴─────────────────────┴────────┘
```

### Combining with JQ

```bash
# Filter then display as table
quicpulse -J '.[] | select(.active)' --table example.com/api/users

# Select specific fields for table
quicpulse -J '[.[] | {name, email}]' --table example.com/api/users
```

### Table Features

- Automatic column width adjustment
- Bold headers
- Unicode box-drawing characters
- Empty cells for missing values
- Nested values flattened to string

---

## CSV Output

Export JSON arrays as CSV:

```bash
# Output as CSV
quicpulse --csv example.com/api/users
```

### Example Output

```csv
id,name,email,active
1,John,john@example.com,true
2,Jane,jane@example.com,true
3,Bob,bob@example.com,false
```

### CSV to File

```bash
# Save to file
quicpulse --csv example.com/api/users > users.csv

# Using output flag
quicpulse --csv -o users.csv example.com/api/users
```

### Combining with JQ

```bash
# Filter then export as CSV
quicpulse -J '[.[] | select(.active) | {name, email}]' --csv example.com/api/users
```

### CSV Features

- Automatic header from object keys
- Proper escaping of commas and quotes
- UTF-8 encoding
- RFC 4180 compliant

---

## Practical Examples

### API Response Processing

```bash
# Get user names
quicpulse -J '.[].name' example.com/api/users

# Get first 5 users
quicpulse -J '.[:5]' example.com/api/users

# Count results
quicpulse -J 'length' example.com/api/users

# Check if empty
quicpulse -J 'length == 0' example.com/api/users
```

### Data Analysis

```bash
# Sum all prices
quicpulse -J '[.[].price] | add' example.com/api/products

# Average price
quicpulse -J '[.[].price] | add / length' example.com/api/products

# Find max value
quicpulse -J 'max_by(.price)' example.com/api/products

# Group by category
quicpulse -J 'group_by(.category)' example.com/api/products
```

### Data Transformation

```bash
# Flatten nested structure
quicpulse -J '.data.items | flatten' example.com/api/nested

# Merge objects
quicpulse -J '.defaults + .overrides' example.com/api/config

# Create lookup map
quicpulse -J '[.[] | {(.id|tostring): .name}] | add' example.com/api/items
```

### Reporting

```bash
# Summary statistics
quicpulse -J '{
  total: length,
  active: [.[] | select(.active)] | length,
  inactive: [.[] | select(.active | not)] | length
}' example.com/api/users

# Export specific columns
quicpulse -J '[.[] | {name, email, created_at}]' --csv example.com/api/users > report.csv
```

---

## Workflows with Filtering

### Filter in Workflow Steps

```yaml
name: Data Processing

steps:
  - name: Get users
    request:
      url: https://api.example.com/users
    extract:
      active_users: ".[] | select(.active)"
      user_count: "length"

  - name: Process active users
    request:
      url: https://api.example.com/process
      json:
        users: "{{ active_users }}"
```

### Export in Workflows

```yaml
steps:
  - name: Export data
    request:
      url: https://api.example.com/data
    filter: "[.[] | {id, name, status}]"
    output:
      format: csv
      path: ./exports/data_{{ timestamp }}.csv
```

---

## Error Handling

### Invalid Filter

```bash
quicpulse -J '.invalid[' example.com/api
# Error: Failed to parse filter: ...
```

### Non-JSON Response

```bash
quicpulse -J '.field' example.com/page.html
# Error: Response is not JSON
```

### Missing Field

```bash
quicpulse -J '.nonexistent' example.com/api
# Returns: null
```

### Empty Results

```bash
quicpulse -J '.[] | select(.impossible)' example.com/api
# Returns: (no output)
```

---

## Tips and Tricks

### Default Values

```bash
# Use default if null
quicpulse -J '.value // "N/A"' example.com/api
```

### Conditional Output

```bash
# Output different values based on condition
quicpulse -J 'if .active then "Active" else "Inactive" end' example.com/api/user
```

### Debug Filter

```bash
# Show structure
quicpulse -J 'keys' example.com/api

# Show types
quicpulse -J 'type' example.com/api
```

### Preserve Raw Strings

```bash
# Raw output (no JSON quotes)
quicpulse -J -r '.name' example.com/api/user
# Output: John (not "John")
```

---

## See Also

- [CLI Reference](cli-reference.md) - All filtering flags
- [Assertions](assertions.md) - Using JQ in assertions
- [Workflows](workflow.md) - Filtering in automation
