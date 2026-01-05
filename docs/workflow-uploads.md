# Workflow Uploads Reference

Advanced file upload integration for workflow steps - chunked transfers and compression.

## Overview

The upload feature provides advanced file upload capabilities:

- Chunked transfer encoding for large files
- Request body compression (gzip, deflate, brotli)
- Configurable chunk sizes
- Content type management

## Quick Start

```yaml
name: File Upload Workflow
base_url: https://api.example.com

steps:
  - name: Upload Large File
    method: POST
    url: /uploads
    upload:
      file: ./data/large-file.zip
      chunked: true
      compress: gzip
```

## Configuration Reference

### UploadConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file` | string | required | Path to file to upload |
| `chunked` | boolean | false | Use chunked transfer encoding |
| `chunk_size` | string | "1MB" | Size of each chunk (e.g., "512KB", "2MB") |
| `compress` | string | none | Compression: "gzip", "deflate", "br" |
| `field_name` | string | "file" | Field name for multipart uploads |
| `content_type` | string | auto | Override content type |

## Examples

### Basic Upload

Simple file upload:

```yaml
steps:
  - name: Upload File
    method: POST
    url: /api/files
    upload:
      file: ./documents/report.pdf
```

### Chunked Upload

Upload large files in chunks:

```yaml
steps:
  - name: Upload Large Dataset
    method: POST
    url: /api/data/import
    upload:
      file: ./data/dataset.csv
      chunked: true
      chunk_size: 5MB
```

### Compressed Upload

Compress request body:

```yaml
steps:
  - name: Upload Compressed
    method: POST
    url: /api/logs
    upload:
      file: ./logs/application.log
      compress: gzip
```

### Chunked + Compressed

Combine chunking and compression:

```yaml
steps:
  - name: Upload Large Compressed
    method: POST
    url: /api/backup
    upload:
      file: ./backup/data.tar
      chunked: true
      chunk_size: 10MB
      compress: gzip
```

### Custom Content Type

Override auto-detected content type:

```yaml
steps:
  - name: Upload JSON Data
    method: POST
    url: /api/import
    upload:
      file: ./data/custom.dat
      content_type: application/json
```

### Multipart Field Name

Specify field name for multipart:

```yaml
steps:
  - name: Upload Document
    method: POST
    url: /api/documents
    upload:
      file: ./docs/contract.pdf
      field_name: document
      content_type: application/pdf
```

## Compression Options

### Gzip Compression

Most widely supported:

```yaml
upload:
  file: ./data/large.json
  compress: gzip
```

Adds header: `Content-Encoding: gzip`

### Deflate Compression

Alternative compression:

```yaml
upload:
  file: ./data/large.json
  compress: deflate
```

Adds header: `Content-Encoding: deflate`

### Brotli Compression

Best compression ratio:

```yaml
upload:
  file: ./data/large.json
  compress: br
```

Adds header: `Content-Encoding: br`

## Chunk Size Examples

| Size | Use Case |
|------|----------|
| `256KB` | Small files, high latency networks |
| `1MB` | Default, balanced performance |
| `5MB` | Large files, stable connections |
| `10MB` | Very large files, fast networks |
| `50MB` | Massive files, low overhead priority |

```yaml
# Small chunks for reliability
upload:
  file: ./data/file.bin
  chunked: true
  chunk_size: 256KB

# Large chunks for speed
upload:
  file: ./data/huge.bin
  chunked: true
  chunk_size: 50MB
```

## Integration Patterns

### Upload with Authentication

```yaml
steps:
  - name: Login
    method: POST
    url: /auth/login
    body: '{"username": "user", "password": "pass"}'
    extract:
      token: body.access_token

  - name: Upload Authenticated
    method: POST
    url: /api/files
    headers:
      Authorization: "Bearer {{ token }}"
    upload:
      file: ./data/private.zip
      compress: gzip
```

### Upload with Assertions

```yaml
steps:
  - name: Upload and Verify
    method: POST
    url: /api/upload
    upload:
      file: ./data/file.bin
      chunked: true
    assert:
      status: 201
      body:
        - path: uploaded
          equals: true
        - path: size
          greater_than: 0
    extract:
      file_id: body.id
```

### Upload with Progress Tracking

```yaml
steps:
  - name: Upload Large File
    method: POST
    url: /api/upload
    upload:
      file: ./data/huge.zip
      chunked: true
      chunk_size: 5MB
    post_script:
      code: |
        println!("Upload complete!");
        println!("File ID: {}", response["body"]["id"]);
```

### Multiple File Uploads

Sequential file uploads:

```yaml
steps:
  - name: Upload File 1
    method: POST
    url: /api/files
    upload:
      file: ./data/file1.pdf
    extract:
      file1_id: body.id

  - name: Upload File 2
    method: POST
    url: /api/files
    upload:
      file: ./data/file2.pdf
    extract:
      file2_id: body.id

  - name: Link Files
    method: POST
    url: /api/documents
    body: |
      {
        "files": ["{{ file1_id }}", "{{ file2_id }}"]
      }
```

## Upload vs Multipart

### Using `upload`

For single file uploads with advanced features:

```yaml
upload:
  file: ./data/file.bin
  chunked: true
  compress: gzip
```

### Using `multipart`

For form-based uploads with multiple fields:

```yaml
multipart:
  - name: file
    path: ./data/file.bin
  - name: description
    value: "My file description"
  - name: tags
    value: '["tag1", "tag2"]'
```

## Best Practices

1. **Use chunking for large files** - Prevents timeouts
2. **Match chunk size to network** - Smaller for unstable connections
3. **Compress text-based files** - JSON, logs, CSV
4. **Don't compress already compressed** - ZIP, PNG, JPEG
5. **Set correct content type** - Some APIs require specific types
6. **Handle upload failures** - Use retries for reliability

## Common Patterns

### Retry on Failure

```yaml
steps:
  - name: Reliable Upload
    method: POST
    url: /api/upload
    retries: 3
    timeout: 120000  # 2 minutes
    upload:
      file: ./data/important.zip
      chunked: true
      chunk_size: 5MB
```

### Upload with Metadata

```yaml
steps:
  - name: Upload with Info
    method: POST
    url: /api/files
    headers:
      X-File-Name: "report.pdf"
      X-File-Type: "report"
      X-Upload-Date: "{now}"
    upload:
      file: ./reports/monthly.pdf
      compress: gzip
```

### Environment-Specific Upload

```yaml
environments:
  development:
    chunk_size: "256KB"  # Small for dev
  production:
    chunk_size: "10MB"   # Large for prod

steps:
  - name: Upload Data
    method: POST
    url: /api/upload
    upload:
      file: ./data/file.bin
      chunked: true
      chunk_size: "{{ chunk_size }}"
```

## Troubleshooting

### Upload Timeout

1. Enable chunking
2. Reduce chunk size
3. Increase timeout

### Compression Not Working

1. Verify server supports Content-Encoding
2. Check Accept-Encoding in response
3. Try different compression method

### Wrong Content Type

1. Set `content_type` explicitly
2. Check file extension
3. Verify server expectations

### Chunk Upload Failure

1. Reduce chunk size
2. Check network stability
3. Verify server supports chunked encoding

---

See also:
- [workflow.md](workflow.md) - Main workflow reference
- [README.md](../README.md) - CLI upload options
