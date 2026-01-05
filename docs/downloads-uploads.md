# Downloads & Uploads

QuicPulse provides comprehensive support for file downloads and uploads with progress tracking, resume capability, and multipart uploads.

## Downloads

### Basic Download

Download a file to the current directory:

```bash
# Download mode - saves to file based on URL
quicpulse -d https://example.com/file.tar.gz

# Saves as: file.tar.gz
```

### Custom Output Path

```bash
# Specify output filename
quicpulse -o myfile.tar.gz https://example.com/file.tar.gz

# Download to specific directory
quicpulse -o /tmp/downloads/file.tar.gz https://example.com/file.tar.gz
```

### Resume Interrupted Downloads

Resume a partially downloaded file:

```bash
# Resume download (-c or --continue)
quicpulse -d -c https://example.com/large-file.iso

# Combine with output path
quicpulse -d -c -o large-file.iso https://example.com/large-file.iso
```

Resume works by:
1. Checking existing file size
2. Sending `Range: bytes=N-` header
3. Appending to existing file

**Note:** Server must support HTTP Range requests (return 206 Partial Content).

### Progress Display

Downloads show progress by default:

```
file.tar.gz [████████████████░░░░░░░░] 67% 134MB/200MB 5.2MB/s eta 12s
```

Progress includes:
- Filename
- Progress bar
- Percentage
- Downloaded/Total size
- Transfer speed
- Estimated time remaining

### Filename Detection

QuicPulse automatically detects the filename from:

1. **Content-Disposition header** (preferred)
   ```
   Content-Disposition: attachment; filename="report.pdf"
   ```

2. **URL path** (fallback)
   ```
   https://example.com/path/to/file.tar.gz → file.tar.gz
   ```

3. **Default** (if no name found)
   ```
   download → download
   ```

### Download Options

| Flag | Short | Description |
|------|-------|-------------|
| `--download` | `-d` | Enable download mode |
| `--output FILE` | `-o` | Custom output path |
| `--continue` | `-c` | Resume partial download |

---

## Uploads

### JSON Data Upload

```bash
# POST JSON data (default)
quicpulse POST example.com/api name=John age:=30

# Explicit JSON mode
quicpulse -j POST example.com/api name=John
```

### Form Data Upload

```bash
# Form-encoded data
quicpulse -f POST example.com/api name=John email=john@example.com
```

### File Upload

#### Single File

```bash
# Upload file as form field
quicpulse -f POST example.com/upload file@/path/to/document.pdf

# With custom MIME type
quicpulse -f POST example.com/upload "file@/path/to/data.bin;type=application/octet-stream"
```

#### Multiple Files

```bash
# Upload multiple files
quicpulse -f POST example.com/upload \
  avatar@photo.jpg \
  resume@document.pdf \
  cover_letter@letter.docx
```

### Multipart Upload

Force multipart/form-data encoding:

```bash
# Multipart with files and fields
quicpulse --multipart POST example.com/upload \
  name=John \
  avatar@photo.jpg

# Custom boundary
quicpulse --multipart --boundary "----MyBoundary" POST example.com/upload file@data.bin
```

### Raw Body Upload

Upload raw file contents as request body:

```bash
# Upload file as raw body
quicpulse POST example.com/api @data.json

# Upload with explicit content type
quicpulse POST example.com/api @data.xml Content-Type:application/xml
```

### Chunked Transfer

Enable chunked transfer encoding for large uploads:

```bash
# Chunked upload
quicpulse --chunked POST example.com/api @large-file.json

# Useful when content length is unknown
```

Chunk size: 100KB by default

### Request Compression

Compress request body before sending:

```bash
# Compress with deflate
quicpulse -x POST example.com/api @large-data.json

# Force compression even for small payloads
quicpulse -xx POST example.com/api data=value
```

### Upload from Stdin

```bash
# Pipe data to QuicPulse
echo '{"name": "John"}' | quicpulse POST example.com/api

# From command output
cat data.json | quicpulse POST example.com/api

# From here-doc
quicpulse POST example.com/api <<EOF
{
  "name": "John",
  "age": 30
}
EOF
```

---

## Workflows

### Download in Workflows

```yaml
name: Download Files

steps:
  - name: Download report
    request:
      url: https://api.example.com/reports/latest
    download:
      path: /tmp/reports/
      filename: "report_{{ timestamp }}.pdf"
      resume: true

  - name: Download with conditions
    request:
      url: https://api.example.com/data/export
    download:
      path: ./exports/
    when: "{{ env.EXPORT_ENABLED }}"
```

### Upload in Workflows

```yaml
name: Upload Files

steps:
  - name: Upload document
    request:
      method: POST
      url: https://api.example.com/upload
      multipart:
        file:
          path: /path/to/document.pdf
          content_type: application/pdf
        metadata:
          value: '{"type": "report"}'
          content_type: application/json

  - name: Upload with progress
    request:
      method: PUT
      url: https://storage.example.com/files/backup.tar.gz
      body:
        file: /path/to/backup.tar.gz
    options:
      chunked: true
      show_progress: true
```

---

## Request Item Syntax

### File Upload Syntax

| Syntax | Description | Example |
|--------|-------------|---------|
| `field@file` | Upload file as form field | `avatar@photo.jpg` |
| `field@file;type=mime` | With explicit MIME type | `doc@file.bin;type=application/octet-stream` |
| `field@file;filename=name` | With custom filename | `data@export.csv;filename=report.csv` |
| `@file` | Raw file as request body | `@data.json` |

### File Path Handling

```bash
# Absolute path
quicpulse -f POST example.com/upload file@/home/user/document.pdf

# Relative path
quicpulse -f POST example.com/upload file@./document.pdf

# Home directory expansion
quicpulse -f POST example.com/upload file@~/documents/report.pdf
```

---

## MIME Type Detection

QuicPulse automatically detects MIME types:

| Extension | MIME Type |
|-----------|-----------|
| `.json` | `application/json` |
| `.xml` | `application/xml` |
| `.html` | `text/html` |
| `.css` | `text/css` |
| `.js` | `application/javascript` |
| `.txt` | `text/plain` |
| `.pdf` | `application/pdf` |
| `.jpg`, `.jpeg` | `image/jpeg` |
| `.png` | `image/png` |
| `.gif` | `image/gif` |
| `.svg` | `image/svg+xml` |
| `.zip` | `application/zip` |
| `.tar.gz` | `application/gzip` |

Override with explicit type:
```bash
quicpulse -f POST example.com/upload "file@data.bin;type=application/custom"
```

---

## Progress Callbacks

### Download Progress

Progress bar shows:
- Filename
- Visual progress bar
- Percentage complete
- Bytes transferred / Total size
- Transfer speed
- ETA

### Upload Progress

For chunked uploads:
- Bytes sent
- Total size (if known)
- Transfer speed

---

## Error Handling

### Download Errors

| Error | Cause | Solution |
|-------|-------|----------|
| File exists | Output file already exists | Use `-c` to resume or choose different name |
| Permission denied | Cannot write to output path | Check directory permissions |
| Disk full | Not enough space | Free up disk space |
| Range not satisfiable | Server doesn't support resume | Download from beginning |

### Upload Errors

| Error | Cause | Solution |
|-------|-------|----------|
| File not found | Input file doesn't exist | Check file path |
| Permission denied | Cannot read input file | Check file permissions |
| Payload too large | Server rejects large files | Use chunked upload or compress |

---

## Examples

### Download Large File with Resume

```bash
# Start download
quicpulse -d -o ubuntu.iso https://releases.ubuntu.com/22.04/ubuntu.iso

# If interrupted, resume with:
quicpulse -d -c -o ubuntu.iso https://releases.ubuntu.com/22.04/ubuntu.iso
```

### Batch Download

```bash
# Download multiple files
for url in file1.tar.gz file2.tar.gz file3.tar.gz; do
  quicpulse -d "https://example.com/downloads/$url"
done
```

### Upload with Authentication

```bash
# Upload with bearer token
quicpulse -A bearer -a "$TOKEN" -f POST \
  https://api.example.com/upload \
  document@report.pdf

# Upload with basic auth
quicpulse -a admin:secret -f POST \
  https://api.example.com/upload \
  file@data.csv
```

### S3 Upload with AWS SigV4

```bash
# Upload to S3
quicpulse -A aws-sigv4 --aws-region us-east-1 --aws-service s3 \
  PUT "https://my-bucket.s3.amazonaws.com/path/file.txt" \
  @local-file.txt
```

---

## See Also

- [CLI Reference](cli-reference.md) - All download/upload flags
- [HTTP Client](http-client.md) - Request configuration
- [Workflows](workflow.md) - Automated file transfers
