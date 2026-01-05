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
    let metadata = std::fs::metadata(&upload.path)
        .map_err(|e| QuicpulseError::Io(e))?;
    let file_size = metadata.len();

    // Determine filename
    let filename = upload.filename.clone()
        .or_else(|| {
            upload.path.file_name()
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
        let mut file = File::open(&upload.path)
            .map_err(|e| QuicpulseError::Io(e))?;
        let mut contents = Vec::with_capacity(file_size as usize);
        file.read_to_end(&mut contents)
            .map_err(|e| QuicpulseError::Io(e))?;

        Part::bytes(contents).file_name(filename)
    } else {
        // Large file: stream from disk
        let file = std::fs::File::open(&upload.path)
            .map_err(|e| QuicpulseError::Io(e))?;
        let async_file = tokio::fs::File::from_std(file);
        let stream = FramedRead::new(async_file, BytesCodec::new());
        let body = reqwest::Body::wrap_stream(stream);

        Part::stream_with_length(body, file_size).file_name(filename)
    };

    // Set content type
    let part = part.mime_str(&mime_type)
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
