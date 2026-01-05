# Workflow Downloads Reference

File download integration for workflow steps - save response bodies to files.

## Overview

The download feature allows you to save response content directly to files during workflow execution. This is useful for:

- Downloading binary files (images, PDFs, archives)
- Saving API responses for later analysis
- Capturing large response bodies
- Automating file retrieval workflows

## Quick Start

```yaml
name: Download Files
base_url: https://example.com

steps:
  - name: Download Report
    method: GET
    url: /reports/monthly.pdf
    download:
      path: ./downloads/report.pdf
```

## Configuration Reference

### DownloadConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | string | required | Output file path (supports variables) |
| `resume` | boolean | false | Resume partial downloads if supported |
| `overwrite` | boolean | false | Overwrite existing files |

## Examples

### Basic Download

Download a file to a specific path:

```yaml
steps:
  - name: Download Image
    method: GET
    url: /images/logo.png
    download:
      path: ./assets/logo.png
```

### Dynamic File Names

Use variables in file paths:

```yaml
variables:
  report_date: "2024-01-15"

steps:
  - name: Download Daily Report
    method: GET
    url: /reports/{{ report_date }}
    download:
      path: ./reports/report-{{ report_date }}.pdf
```

### Download with Magic Values

Generate unique file names:

```yaml
steps:
  - name: Download with Timestamp
    method: GET
    url: /exports/data.csv
    download:
      path: ./exports/data-{timestamp}.csv

  - name: Download with UUID
    method: GET
    url: /backups/latest
    download:
      path: ./backups/backup-{uuid}.zip
```

### Overwrite Existing Files

Replace existing files:

```yaml
steps:
  - name: Download Latest Version
    method: GET
    url: /releases/latest.zip
    download:
      path: ./releases/latest.zip
      overwrite: true
```

### Resume Partial Downloads

Resume interrupted downloads:

```yaml
steps:
  - name: Download Large File
    method: GET
    url: /large-files/dataset.zip
    download:
      path: ./data/dataset.zip
      resume: true
```

### Authenticated Downloads

Download protected files:

```yaml
steps:
  - name: Login
    method: POST
    url: /auth/login
    body: '{"username": "user", "password": "pass"}'
    extract:
      token: body.access_token

  - name: Download Protected File
    method: GET
    url: /files/confidential.pdf
    headers:
      Authorization: "Bearer {{ token }}"
    download:
      path: ./confidential/report.pdf
```

### Extract Info Then Download

Get file info before downloading:

```yaml
steps:
  - name: Get File Metadata
    method: GET
    url: /files/123/info
    extract:
      filename: body.filename
      size: body.size

  - name: Download File
    method: GET
    url: /files/123/content
    download:
      path: ./downloads/{{ filename }}
```

### Multiple Downloads

Download multiple files in sequence:

```yaml
variables:
  file_ids:
    - "123"
    - "456"
    - "789"

steps:
  - name: Download File 1
    method: GET
    url: /files/123
    download:
      path: ./downloads/file-123.bin

  - name: Download File 2
    method: GET
    url: /files/456
    download:
      path: ./downloads/file-456.bin

  - name: Download File 3
    method: GET
    url: /files/789
    download:
      path: ./downloads/file-789.bin
```

### Download with Assertions

Verify download success:

```yaml
steps:
  - name: Download and Verify
    method: GET
    url: /files/document.pdf
    download:
      path: ./docs/document.pdf
    assert:
      status: 200
      headers:
        - name: Content-Type
          contains: application/pdf
```

## Download Behavior

### Path Resolution

- Relative paths are resolved from the current working directory
- Parent directories are created automatically
- Use `./` prefix for relative paths

### File Handling

- Default: Skip if file exists
- `overwrite: true`: Replace existing file
- `resume: true`: Append to partial file if server supports Range

### Error Handling

Downloads fail if:
- Directory cannot be created
- File exists and `overwrite` is false
- Server returns non-2xx status
- Disk is full

## Integration with Other Features

### Download with Pre-Script

Dynamic path generation:

```yaml
steps:
  - name: Generate Download Path
    method: GET
    url: /files/latest
    pre_script:
      code: |
        let date = std::time::now();
        vars["download_path"] = format!("./downloads/{}-backup.zip", date);
    download:
      path: "{{ download_path }}"
```

### Download with Post-Script

Process after download:

```yaml
steps:
  - name: Download and Process
    method: GET
    url: /data/export.json
    download:
      path: ./temp/data.json
    post_script:
      code: |
        println("Downloaded file to ./temp/data.json");
        // Could trigger further processing
```

### Download with Session

Maintain session across downloads:

```yaml
session: download-session

steps:
  - name: Login
    method: POST
    url: /auth/login
    body: '{"username": "user", "password": "pass"}'

  - name: Download User Files
    method: GET
    url: /user/files/export.zip
    download:
      path: ./user-data/export.zip
```

## Best Practices

1. **Use descriptive paths** - Include context in filenames
2. **Handle large files** - Use `resume` for large downloads
3. **Verify downloads** - Add assertions for Content-Type/status
4. **Clean up temp files** - Remove downloaded files after processing
5. **Use variables** - Make paths dynamic and reusable
6. **Check disk space** - Ensure sufficient space for downloads

## Common Patterns

### Download Archive and Extract

```yaml
steps:
  - name: Download Archive
    method: GET
    url: /releases/v1.0.0.zip
    download:
      path: ./temp/release.zip

  # External extraction would happen outside workflow
```

### Backup Download

```yaml
name: Backup Download
base_url: https://api.example.com

steps:
  - name: Request Backup
    method: POST
    url: /backups
    extract:
      backup_id: body.id

  - name: Wait for Backup
    method: GET
    url: /backups/{{ backup_id }}
    delay: 5000
    assert:
      body:
        - path: status
          equals: ready

  - name: Download Backup
    method: GET
    url: /backups/{{ backup_id }}/download
    download:
      path: ./backups/backup-{date}.zip
```

### Conditional Download

```yaml
steps:
  - name: Check for Update
    method: GET
    url: /updates/check
    extract:
      has_update: body.available
      download_url: body.url

  - name: Download Update
    method: GET
    url: "{{ download_url }}"
    skip_if: "{{ has_update }} == false"
    download:
      path: ./updates/latest.zip
      overwrite: true
```

## Troubleshooting

### File Not Created

1. Check directory permissions
2. Verify path is valid
3. Ensure disk has space
4. Check response status code

### Partial Download

1. Enable `resume: true`
2. Check for network stability
3. Verify server supports Range headers

### Overwrite Issues

1. Set `overwrite: true` explicitly
2. Delete file before workflow
3. Use unique filenames with magic values

---

See also:
- [workflow.md](workflow.md) - Main workflow reference
- [README.md](../README.md) - CLI download options
