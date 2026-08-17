//! Multipart form data handling
//!
//! Bug #5 fix: Provides async versions that use spawn_blocking to avoid
//! blocking tokio worker threads during synchronous file I/O operations.

use reqwest::multipart::{Form, Part};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::errors::QuicpulseError;

/// File upload details for multipart form data
#[derive(Debug, Clone)]
pub struct FileUpload {
    pub name: String,
    pub path: PathBuf,
    pub filename: Option<String>,
    pub content_type: Option<String>,
}

/// Maximum file size to load into memory (10MB)
/// Files larger than this will be streamed
const MAX_MEMORY_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Build a multipart form from file uploads and data
pub fn build_multipart_form(
    files: &[FileUpload],
    data: &[(String, String)],
) -> Result<Form, QuicpulseError> {
    let mut form = Form::new();

    // Add data fields
    for (name, value) in data {
        form = form.text(name.clone(), value.clone());
    }

    // Add file uploads
    for file in files {
        let part = create_file_part(file)?;
        form = form.part(file.name.clone(), part);
    }

    Ok(form)
}

/// Create a multipart Part from a FileUpload
/// Uses streaming for large files to prevent memory exhaustion
fn create_file_part(upload: &FileUpload) -> Result<Part, QuicpulseError> {
    // Get file metadata to check size
    let metadata = std::fs::metadata(&upload.path).map_err(QuicpulseError::Io)?;
    let file_size = metadata.len();

    // Determine filename
    let filename = upload
        .filename
        .clone()
        .or_else(|| {
            upload
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "file".to_string());

    // Determine MIME type
    let mime_type = if let Some(ref content_type) = upload.content_type {
        content_type.clone()
    } else if let Some(ext) = upload.path.extension().and_then(|e| e.to_str()) {
        guess_mime_type(ext).to_string()
    } else {
        "application/octet-stream".to_string()
    };

    // For small files, load into memory (faster for small uploads)
    // For large files, use streaming to prevent OOM
    let part = if file_size <= MAX_MEMORY_FILE_SIZE {
        // Small file: load into memory
        let mut file = File::open(&upload.path).map_err(QuicpulseError::Io)?;
        let mut contents = Vec::with_capacity(file_size as usize);
        file.read_to_end(&mut contents)
            .map_err(QuicpulseError::Io)?;

        Part::bytes(contents).file_name(filename)
    } else {
        // Large file: stream from disk
        let file = std::fs::File::open(&upload.path).map_err(QuicpulseError::Io)?;
        let async_file = tokio::fs::File::from_std(file);
        let stream = FramedRead::new(async_file, BytesCodec::new());
        let body = reqwest::Body::wrap_stream(stream);

        Part::stream_with_length(body, file_size).file_name(filename)
    };

    // Set content type
    let part = part
        .mime_str(&mime_type)
        .map_err(|e| QuicpulseError::Parse(format!("Invalid MIME type: {}", e)))?;

    Ok(part)
}

/// Bug #5 fix: Async version of build_multipart_form that uses spawn_blocking
/// to avoid blocking tokio worker threads during file I/O operations.
pub async fn build_multipart_form_async(
    files: Vec<FileUpload>,
    data: Vec<(String, String)>,
) -> Result<Form, QuicpulseError> {
    tokio::task::spawn_blocking(move || build_multipart_form(&files, &data))
        .await
        .map_err(|e| QuicpulseError::Parse(format!("Multipart form task panicked: {}", e)))?
}

/// Guess MIME type from file extension
fn guess_mime_type(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "gz" | "gzip" => "application/gzip",
        "tar" => "application/x-tar",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    /// Serialize a Form to its real multipart wire bytes so tests can assert
    /// on the actual headers and payload rather than just "it constructed".
    async fn form_body(form: Form) -> String {
        let mut stream = Box::pin(form.into_stream());
        let mut out = Vec::new();
        while let Some(chunk) = stream.next().await {
            out.extend_from_slice(&chunk.expect("form stream chunk"));
        }
        String::from_utf8_lossy(&out).to_string()
    }

    fn upload(name: &str, path: PathBuf) -> FileUpload {
        FileUpload {
            name: name.to_string(),
            path,
            filename: None,
            content_type: None,
        }
    }

    // ---- guess_mime_type ----

    #[test]
    fn test_guess_mime_type_known_extensions() {
        let cases = [
            ("txt", "text/plain"),
            ("html", "text/html"),
            ("htm", "text/html"),
            ("css", "text/css"),
            ("js", "application/javascript"),
            ("json", "application/json"),
            ("xml", "application/xml"),
            ("pdf", "application/pdf"),
            ("zip", "application/zip"),
            ("gz", "application/gzip"),
            ("gzip", "application/gzip"),
            ("tar", "application/x-tar"),
            ("png", "image/png"),
            ("jpg", "image/jpeg"),
            ("jpeg", "image/jpeg"),
            ("gif", "image/gif"),
            ("svg", "image/svg+xml"),
            ("webp", "image/webp"),
            ("ico", "image/x-icon"),
            ("mp3", "audio/mpeg"),
            ("wav", "audio/wav"),
            ("mp4", "video/mp4"),
            ("webm", "video/webm"),
        ];
        for (ext, expected) in cases {
            assert_eq!(guess_mime_type(ext), expected, "ext {ext}");
        }
    }

    #[test]
    fn test_guess_mime_type_is_case_insensitive() {
        assert_eq!(guess_mime_type("PNG"), "image/png");
        assert_eq!(guess_mime_type("JsOn"), "application/json");
    }

    #[test]
    fn test_guess_mime_type_unknown_falls_back_to_octet_stream() {
        assert_eq!(guess_mime_type("xyz"), "application/octet-stream");
        assert_eq!(guess_mime_type(""), "application/octet-stream");
    }

    // ---- create_file_part / build_multipart_form ----

    #[tokio::test]
    async fn test_form_contains_text_fields() {
        let form = build_multipart_form(
            &[],
            &[
                ("alpha".to_string(), "one".to_string()),
                ("beta".to_string(), "two".to_string()),
            ],
        )
        .unwrap();

        let body = form_body(form).await;
        assert!(body.contains(r#"name="alpha""#), "body: {body}");
        assert!(body.contains("one"));
        assert!(body.contains(r#"name="beta""#));
        assert!(body.contains("two"));
    }

    #[tokio::test]
    async fn test_file_part_uses_path_basename_and_sniffed_mime() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.json");
        std::fs::write(&path, br#"{"ok":true}"#).unwrap();

        let form = build_multipart_form(&[upload("doc", path)], &[]).unwrap();
        let body = form_body(form).await;

        assert!(body.contains(r#"name="doc""#), "body: {body}");
        assert!(body.contains(r#"filename="report.json""#), "body: {body}");
        assert!(body.contains("application/json"), "body: {body}");
        assert!(body.contains(r#"{"ok":true}"#), "file contents missing");
    }

    #[tokio::test]
    async fn test_explicit_filename_and_content_type_override_detection() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("actual.txt");
        std::fs::write(&path, b"data").unwrap();

        let form = build_multipart_form(
            &[FileUpload {
                name: "f".to_string(),
                path,
                filename: Some("renamed.bin".to_string()),
                content_type: Some("application/x-custom".to_string()),
            }],
            &[],
        )
        .unwrap();

        let body = form_body(form).await;
        assert!(body.contains(r#"filename="renamed.bin""#), "body: {body}");
        assert!(body.contains("application/x-custom"), "body: {body}");
        assert!(
            !body.contains("text/plain"),
            "sniffed mime should be overridden"
        );
    }

    #[tokio::test]
    async fn test_extensionless_file_falls_back_to_octet_stream() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("README");
        std::fs::write(&path, b"x").unwrap();

        let body = form_body(build_multipart_form(&[upload("f", path)], &[]).unwrap()).await;
        assert!(body.contains("application/octet-stream"), "body: {body}");
        assert!(body.contains(r#"filename="README""#), "body: {body}");
    }

    #[tokio::test]
    async fn test_empty_file_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        std::fs::write(&path, b"").unwrap();

        let body = form_body(build_multipart_form(&[upload("f", path)], &[]).unwrap()).await;
        assert!(body.contains(r#"filename="empty.txt""#), "body: {body}");
        assert!(body.contains("text/plain"));
    }

    #[tokio::test]
    async fn test_binary_file_contents_survive_intact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blob.bin");
        let bytes: Vec<u8> = (0u8..=255).collect();
        std::fs::write(&path, &bytes).unwrap();

        let form = build_multipart_form(&[upload("f", path)], &[]).unwrap();
        let mut stream = Box::pin(form.into_stream());
        let mut out = Vec::new();
        while let Some(chunk) = stream.next().await {
            out.extend_from_slice(&chunk.unwrap());
        }
        // The raw byte sequence must appear verbatim in the encoded form.
        assert!(
            out.windows(bytes.len()).any(|w| w == bytes.as_slice()),
            "binary payload was altered"
        );
    }

    #[tokio::test]
    async fn test_files_and_fields_combined() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"filedata").unwrap();

        let form = build_multipart_form(
            &[upload("upload", path)],
            &[("note".to_string(), "hello".to_string())],
        )
        .unwrap();

        let body = form_body(form).await;
        assert!(body.contains(r#"name="note""#), "body: {body}");
        assert!(body.contains("hello"));
        assert!(body.contains(r#"name="upload""#));
        assert!(body.contains("filedata"));
    }

    #[test]
    fn test_missing_file_returns_io_error() {
        let err = build_multipart_form(&[upload("f", PathBuf::from("/nonexistent/nope.txt"))], &[])
            .unwrap_err();
        assert!(
            matches!(err, QuicpulseError::Io(_)),
            "expected Io error, got {err:?}"
        );
    }

    #[test]
    fn test_invalid_content_type_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"x").unwrap();

        let err = build_multipart_form(
            &[FileUpload {
                name: "f".to_string(),
                path,
                filename: None,
                content_type: Some("not a mime type at all".to_string()),
            }],
            &[],
        )
        .unwrap_err();

        assert!(
            matches!(err, QuicpulseError::Parse(ref m) if m.contains("Invalid MIME type")),
            "expected Parse error, got {err:?}"
        );
    }

    #[test]
    fn test_empty_inputs_produce_a_valid_empty_form() {
        let form = build_multipart_form(&[], &[]).unwrap();
        assert!(!form.boundary().is_empty());
    }

    #[tokio::test]
    async fn test_large_file_takes_the_streaming_path() {
        // Files over MAX_MEMORY_FILE_SIZE are streamed from disk rather than
        // buffered, so exercise that branch explicitly.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.bin");
        let size = (MAX_MEMORY_FILE_SIZE + 1) as usize;
        std::fs::write(&path, vec![b'z'; size]).unwrap();

        let form = build_multipart_form(&[upload("big", path)], &[]).unwrap();
        let mut stream = Box::pin(form.into_stream());
        let mut total = 0usize;
        while let Some(chunk) = stream.next().await {
            total += chunk.unwrap().len();
        }
        // Payload plus boundaries/headers must exceed the file itself.
        assert!(
            total > size,
            "streamed {total} bytes for a {size}-byte file"
        );
    }

    // ---- async wrapper ----

    #[tokio::test]
    async fn test_build_multipart_form_async_matches_sync() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"async-data").unwrap();

        let form = build_multipart_form_async(
            vec![upload("f", path)],
            vec![("k".to_string(), "v".to_string())],
        )
        .await
        .unwrap();

        let body = form_body(form).await;
        assert!(body.contains("async-data"), "body: {body}");
        assert!(body.contains(r#"name="k""#));
    }

    #[tokio::test]
    async fn test_build_multipart_form_async_propagates_errors() {
        let err = build_multipart_form_async(
            vec![upload("f", PathBuf::from("/nonexistent/nope.txt"))],
            vec![],
        )
        .await
        .unwrap_err();
        assert!(matches!(err, QuicpulseError::Io(_)), "got {err:?}");
    }
}
