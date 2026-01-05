//! Download status tracking

/// Download status
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    /// Not started
    Pending,
    /// Currently in progress
    InProgress { bytes_downloaded: u64, total_bytes: Option<u64> },
    /// Completed successfully
    Completed { total_bytes: u64 },
    /// Failed with error
    Failed(String),
}

impl DownloadStatus {
    /// Check if download is complete
    pub fn is_complete(&self) -> bool {
        matches!(self, DownloadStatus::Completed { .. })
    }

    /// Check if download failed
    pub fn is_failed(&self) -> bool {
        matches!(self, DownloadStatus::Failed(_))
    }

    /// Get progress as percentage (0-100)
    pub fn progress_percent(&self) -> Option<f64> {
        match self {
            DownloadStatus::InProgress { bytes_downloaded, total_bytes: Some(total) } => {
                if *total > 0 {
                    Some((*bytes_downloaded as f64 / *total as f64) * 100.0)
                } else {
                    None
                }
            }
            DownloadStatus::Completed { .. } => Some(100.0),
            _ => None,
        }
    }
}
