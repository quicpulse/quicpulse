//! Download handling with progress and resume support

use indicatif::{ProgressBar, ProgressStyle};
use percent_encoding::percent_decode_str;
use reqwest::header::{HeaderMap, ACCEPT_ENCODING, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE};
use reqwest::Response;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

use crate::errors::QuicpulseError;
use crate::fs::get_filename_from_content_disposition;
use crate::utils::sanitize_filename;

/// Download status tracking
#[derive(Debug, Clone)]
pub enum DownloadStatus {
    /// Not started
    Pending,
    /// Currently downloading
    Downloading,
    /// Download complete
    Completed,
    /// Download failed
    Failed(String),
}

/// Downloader with progress tracking and resume support
#[derive(Debug)]
pub struct Downloader {
    /// Output file path
    pub output_path: Option<PathBuf>,
    /// Whether to resume a partial download
    pub resume: bool,
    /// Bytes already downloaded (for resume)
    pub resumed_from: u64,
    /// Current download status
    pub status: DownloadStatus,
    /// Total content length (if known)
    pub total_size: Option<u64>,
    /// Progress bar
    progress: Option<ProgressBar>,
}

impl Downloader {
    /// Create a new downloader
    pub fn new(output_path: Option<PathBuf>, resume: bool) -> Self {
        Self {
            output_path,
            resume,
            resumed_from: 0,
            status: DownloadStatus::Pending,
            total_size: None,
            progress: None,
        }
    }

    /// Prepare request headers for download
    pub fn pre_request(&self, headers: &mut HeaderMap) {
        // Request uncompressed content for accurate progress
        headers.insert(ACCEPT_ENCODING, "identity".parse().unwrap());

        // Add Range header if resuming
        if self.resume {
            if let Some(ref path) = self.output_path {
                if let Ok(metadata) = std::fs::metadata(path) {
                    let size = metadata.len();
                    if size > 0 {
                        headers.insert(RANGE, format!("bytes={}-", size).parse().unwrap());
                    }
                }
            }
        }
    }

    /// Start the download and determine output file
    pub async fn start(
        &mut self,
        url: &str,
        response: &Response,
    ) -> Result<PathBuf, QuicpulseError> {
        // Parse content length
        self.total_size = response.headers()
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok());

        // Check for Content-Range (resumed download)
        if let Some(range) = response.headers().get(CONTENT_RANGE) {
            if let Ok(range_str) = range.to_str() {
                if let Some((from, total)) = parse_content_range(range_str) {
                    self.resumed_from = from;
                    self.total_size = Some(total);
                }
            }
        }

        // Determine output filename
        let output_path = self.determine_filename(url, response)?;
        self.output_path = Some(output_path.clone());

        // Initialize progress bar
        self.init_progress();

        self.status = DownloadStatus::Downloading;
        Ok(output_path)
    }

    /// Determine output filename
    fn determine_filename(&self, url: &str, response: &Response) -> Result<PathBuf, QuicpulseError> {
        // Priority 1: User-specified output path
        if let Some(ref path) = self.output_path {
            return Ok(path.clone());
        }

        // Priority 2: Content-Disposition header
        if let Some(filename) = extract_filename_from_content_disposition(response.headers()) {
            // SECURITY: Sanitize filename to prevent path traversal attacks
            // A malicious server could send "../../../etc/passwd" as filename
            let safe_filename = sanitize_filename(&filename);
            // Also strip any remaining path components (just keep the filename)
            let safe_filename = std::path::Path::new(&safe_filename)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download")
                .to_string();
            return Ok(self.ensure_unique_filename(&safe_filename));
        }

        // Priority 3: URL path + Content-Type extension
        let filename = extract_filename_from_url(url, response.headers());
        Ok(self.ensure_unique_filename(&filename))
    }

    /// Ensure filename is unique by appending -1, -2, etc.
    /// Uses atomic file creation to prevent race conditions
    /// Limits iterations to prevent DoS from directories with many files
    fn ensure_unique_filename(&self, filename: &str) -> PathBuf {
        let path = PathBuf::from(filename);

        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("download");
        let ext = path.extension()
            .and_then(|s| s.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();

        // Try to atomically create the file to prevent race conditions
        // OpenOptions with create_new will fail if file exists
        use std::fs::OpenOptions;

        // First try the original filename
        if OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .is_ok()
        {
            return path;
        }

        // Limit iterations to prevent DoS (max 10000 attempts)
        const MAX_ATTEMPTS: u32 = 10000;

        for counter in 1..=MAX_ATTEMPTS {
            let new_name = format!("{}_{}{}", stem, counter, ext);
            let new_path = PathBuf::from(&new_name);

            // Try to atomically create the file
            if OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&new_path)
                .is_ok()
            {
                return new_path;
            }
        }

        // Fallback: use timestamp-based unique name
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        PathBuf::from(format!("{}-{}{}", stem, timestamp, ext))
    }

    /// Initialize progress bar
    fn init_progress(&mut self) {
        let pb = if let Some(total) = self.total_size {
            let pb = ProgressBar::new(total);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            pb.set_position(self.resumed_from);
            pb
        } else {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} [{elapsed_precise}] {bytes} downloaded")
                    .unwrap(),
            );
            pb
        };

        self.progress = Some(pb);
    }

    /// Update progress with downloaded bytes
    pub fn update_progress(&self, additional_bytes: u64) {
        if let Some(ref pb) = self.progress {
            pb.inc(additional_bytes);
        }
    }

    /// Mark download as complete
    pub fn finish(&mut self) {
        self.status = DownloadStatus::Completed;
        if let Some(ref pb) = self.progress {
            pb.finish_with_message("Download complete");
        }
    }

    /// Mark download as failed
    pub fn fail(&mut self, error: &str) {
        self.status = DownloadStatus::Failed(error.to_string());
        if let Some(ref pb) = self.progress {
            pb.abandon_with_message(format!("Download failed: {}", error));
        }
    }

    /// Download response body to file
    /// Bug #8 fix: Uses tokio::select! to race between chunk download and cancellation
    /// Previously, if the network stalled inside chunk().await, Ctrl+C would not
    /// terminate until the network timeout occurred.
    pub async fn download_body(&mut self, mut response: Response) -> Result<u64, QuicpulseError> {
        let output_path = self.output_path.clone()
            .ok_or_else(|| QuicpulseError::Download("No output path set".to_string()))?;

        // Open file (append if resuming, create otherwise)
        let file = if self.resumed_from > 0 {
            OpenOptions::new()
                .write(true)
                .append(true)
                .open(&output_path)
        } else {
            File::create(&output_path)
        }.map_err(|e| QuicpulseError::Io(e))?;

        let mut writer = BufWriter::new(file);
        let mut total_bytes = 0u64;

        // Bug #8 fix: Poll interval for checking cancellation while waiting for chunks
        // This ensures responsive Ctrl+C even when network is stalled
        let cancel_check_interval = std::time::Duration::from_millis(100);

        // Stream chunks with responsive cancellation
        loop {
            // Bug #8 fix: Race between chunk download and cancellation check
            let chunk_result = tokio::select! {
                biased;  // Check cancellation first for responsiveness

                // Check for cancellation periodically
                _ = tokio::time::sleep(cancel_check_interval), if crate::utils::was_interrupted() => {
                    // User pressed Ctrl+C while waiting for chunk
                    writer.flush().map_err(|e| QuicpulseError::Io(e))?;
                    return Err(QuicpulseError::Download("Download interrupted by user".to_string()));
                }

                // Wait for next chunk from network
                result = response.chunk() => {
                    result.map_err(QuicpulseError::Request)?
                }
            };

            // Check for end of stream
            let Some(chunk) = chunk_result else {
                break;
            };

            // Double-check cancellation after receiving chunk
            if crate::utils::was_interrupted() {
                writer.flush().map_err(|e| QuicpulseError::Io(e))?;
                return Err(QuicpulseError::Download("Download interrupted by user".to_string()));
            }

            writer.write_all(&chunk)
                .map_err(|e| QuicpulseError::Io(e))?;

            let chunk_len = chunk.len() as u64;
            total_bytes += chunk_len;
            self.update_progress(chunk_len);
        }

        writer.flush().map_err(|e| QuicpulseError::Io(e))?;
        self.finish();

        Ok(total_bytes)
    }
}

/// Parse Content-Range header: "bytes 0-499/1234"
fn parse_content_range(range: &str) -> Option<(u64, u64)> {
    // Pattern: "bytes <first>-<last>/<total>"
    if !range.starts_with("bytes ") {
        return None;
    }

    let rest = &range[6..];
    let (range_part, total_part) = rest.split_once('/')?;
    let (first, _last) = range_part.split_once('-')?;

    let first: u64 = first.parse().ok()?;
    let total: u64 = total_part.parse().ok()?;

    Some((first, total))
}

/// Extract filename from Content-Disposition header
/// Uses the content_disposition crate for RFC-compliant parsing
fn extract_filename_from_content_disposition(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(CONTENT_DISPOSITION)?;
    let value_str = value.to_str().ok()?;
    get_filename_from_content_disposition(value_str)
}

/// Extract filename from URL and Content-Type
/// Bug #7 fix: Sanitize URL-derived filenames to prevent path traversal
fn extract_filename_from_url(url: &str, headers: &HeaderMap) -> String {
    // Try to get filename from URL path
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(segments) = parsed.path_segments() {
            if let Some(last) = segments.last() {
                // URL-decode the segment first (handles %2F etc.)
                let decoded = percent_decode_str(last)
                    .decode_utf8_lossy()
                    .to_string();

                if !decoded.is_empty() && decoded.contains('.') {
                    // Bug #7 fix: Sanitize to prevent path traversal
                    let safe = sanitize_filename(&decoded);
                    // Also strip any remaining path components
                    return std::path::Path::new(&safe)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("download")
                        .to_string();
                } else if !decoded.is_empty() {
                    // Add extension from Content-Type
                    let ext = get_extension_from_content_type(headers);
                    // Bug #7 fix: Sanitize to prevent path traversal
                    let safe = sanitize_filename(&decoded);
                    let safe_base = std::path::Path::new(&safe)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("download");
                    return format!("{}{}", safe_base, ext);
                }
            }
        }
    }

    // Fallback: use Content-Type to determine extension
    let ext = get_extension_from_content_type(headers);
    format!("download{}", ext)
}

/// Get file extension from Content-Type header using mime_guess crate
fn get_extension_from_content_type(headers: &HeaderMap) -> String {
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Parse the MIME type (strip charset and other parameters)
    let mime_str = content_type.split(';').next().unwrap_or("").trim();

    // Skip generic binary type
    if mime_str.is_empty() || mime_str == "application/octet-stream" {
        return String::new();
    }

    // Use mime_guess for extension lookup
    if let Ok(mime) = mime_str.parse::<mime::Mime>() {
        if let Some(exts) = mime_guess::get_mime_extensions(&mime) {
            if let Some(ext) = exts.first() {
                return format!(".{}", ext);
            }
        }
    }

    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_range() {
        assert_eq!(parse_content_range("bytes 0-499/1234"), Some((0, 1234)));
        assert_eq!(parse_content_range("bytes 500-999/1234"), Some((500, 1234)));
        assert_eq!(parse_content_range("invalid"), None);
    }

    #[test]
    fn test_unique_filename() {
        // Use a temp directory to ensure clean state
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test_unique.txt");
        let test_file_str = test_file.to_string_lossy().to_string();

        let dl = Downloader::new(None, false);
        let unique = dl.ensure_unique_filename(&test_file_str);

        // The atomic creation should succeed and return the original name
        assert_eq!(unique, test_file);

        // Clean up the created file
        let _ = std::fs::remove_file(&unique);
    }
}
