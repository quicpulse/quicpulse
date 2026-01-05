//! Sandboxed filesystem module for scripts
//!
//! Provides controlled file system access with security restrictions.
//! Only allows reading from specific directories and file types.

use dashmap::DashSet;
use once_cell::sync::Lazy;
use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use std::path::{Path, PathBuf};

/// Allowed directories for file access (can be configured)
/// Uses DashSet for lock-free concurrent access
static ALLOWED_DIRS: Lazy<DashSet<PathBuf>> = Lazy::new(DashSet::new);

/// Blocked file patterns
const BLOCKED_PATTERNS: &[&str] = &[
    ".env",
    ".git",
    "id_rsa",
    "id_ed25519",
    ".ssh",
    "credentials",
    "secrets",
    ".aws",
    "private",
    "password",
    ".gnupg",
];

/// Bug #7 fix: Maximum file size scripts can read (10MB)
/// Prevents OOM/DoS when scripts attempt to read very large files
const MAX_READ_SIZE: u64 = 10 * 1024 * 1024;

/// Create the fs module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("fs")?;

    // File reading
    module.function("read", read_file).build()?;
    module.function("read_lines", read_lines).build()?;
    module.function("read_json", read_json).build()?;

    // File info
    module.function("exists", file_exists).build()?;
    module.function("is_file", is_file).build()?;
    module.function("is_dir", is_dir).build()?;
    module.function("size", file_size).build()?;

    // Path utilities
    module.function("join", path_join).build()?;
    module.function("basename", basename).build()?;
    module.function("dirname", dirname).build()?;
    module.function("extension", extension).build()?;

    // Temp file utilities
    module.function("temp_dir", temp_dir).build()?;
    module.function("cwd", current_dir).build()?;

    Ok(module)
}

/// Add an allowed directory for file access
pub fn allow_directory(path: &Path) {
    if let Ok(canonical) = path.canonicalize() {
        ALLOWED_DIRS.insert(canonical);
    } else {
        ALLOWED_DIRS.insert(path.to_path_buf());
    }
}

/// Clear all allowed directories
pub fn clear_allowed_directories() {
    ALLOWED_DIRS.clear();
}

/// Check if a path is allowed for basic queries (exists, is_file, etc.)
fn is_path_allowed(path: &Path) -> bool {
    // Check for blocked patterns - check each path component individually
    // Skip the first few components which are system paths (e.g., /private on macOS)
    // We only want to block user-level directories named "private", not /private/var
    let components: Vec<_> = path.components().collect();
    let skip_count = if cfg!(target_os = "macos") {
        // On macOS, canonical paths start with /private/var/... - skip the root "private"
        if components.len() > 1 {
            if let std::path::Component::Normal(first) = components.get(1).unwrap_or(&std::path::Component::CurDir) {
                if first.to_string_lossy() == "private" { 2 } else { 1 }
            } else { 1 }
        } else { 1 }
    } else { 1 };

    for component in components.iter().skip(skip_count) {
        if let std::path::Component::Normal(name) = component {
            let name_lower = name.to_string_lossy().to_lowercase();
            for pattern in BLOCKED_PATTERNS {
                if name_lower == *pattern
                    || name_lower.starts_with(&format!("{}.", pattern))
                    || name_lower.starts_with(&format!("{}_", pattern))
                    || name_lower.starts_with(&format!("{}-", pattern))
                    || (name_lower.ends_with("~") && name_lower.starts_with(pattern))
                    || (name_lower.contains(pattern) && (pattern.starts_with('.') || pattern.contains("rsa") || pattern.contains("ed25519")))
                {
                    return false;
                }
            }
        }
    }

    // Bug #8 fix: When no allowed dirs configured, only allow temp directory
    // Previously allowed current directory, but this is a security risk if user
    // runs from / or home directory - grants access to entire filesystem
    if ALLOWED_DIRS.is_empty() {
        // Only allow temp directory for empty config (principle of least privilege)
        let temp = std::env::temp_dir();
        if path.starts_with(&temp) {
            return true;
        }
        if let Ok(canonical_temp) = temp.canonicalize() {
            if path.starts_with(&canonical_temp) {
                return true;
            }
            // Also check canonical form of input path
            if let Ok(canonical_path) = path.canonicalize() {
                if canonical_path.starts_with(&canonical_temp) {
                    return true;
                }
            }
        }
        return false;
    }

    // Check against allowed directories (non-blocking iteration with DashSet)
    for allowed in ALLOWED_DIRS.iter() {
        if path.starts_with(allowed.key()) {
            return true;
        }
    }

    false
}

/// Check if an already-canonicalized path is allowed
/// This prevents TOCTOU attacks by working directly with the canonical path
fn is_path_allowed_canonical(canonical: &Path) -> bool {
    // Check for blocked patterns - check each path component individually
    // Skip the first few components which are system paths (e.g., /private on macOS)
    // We only want to block user-level directories named "private", not /private/var
    let components: Vec<_> = canonical.components().collect();
    let skip_count = if cfg!(target_os = "macos") {
        // On macOS, canonical paths start with /private/var/... - skip the root "private"
        if components.len() > 1 {
            if let std::path::Component::Normal(first) = components.get(1).unwrap_or(&std::path::Component::CurDir) {
                if first.to_string_lossy() == "private" { 2 } else { 1 }
            } else { 1 }
        } else { 1 }
    } else { 1 };

    for component in components.iter().skip(skip_count) {
        if let std::path::Component::Normal(name) = component {
            let name_lower = name.to_string_lossy().to_lowercase();
            for pattern in BLOCKED_PATTERNS {
                if name_lower == *pattern
                    || name_lower.starts_with(&format!("{}.", pattern))
                    || name_lower.starts_with(&format!("{}_", pattern))
                    || name_lower.starts_with(&format!("{}-", pattern))
                    || (name_lower.ends_with("~") && name_lower.starts_with(pattern))
                    || (name_lower.contains(pattern) && (pattern.starts_with('.') || pattern.contains("rsa") || pattern.contains("ed25519")))
                {
                    return false;
                }
            }
        }
    }

    // Bug #8 fix: When no allowed dirs configured, only allow temp directory
    // Previously allowed current directory, but this is a security risk if user
    // runs from / or home directory - grants access to entire filesystem
    if ALLOWED_DIRS.is_empty() {
        // Only allow temp directory for empty config (principle of least privilege)
        let temp = std::env::temp_dir();
        if let Ok(canonical_temp) = temp.canonicalize() {
            if canonical.starts_with(&canonical_temp) {
                return true;
            }
        }
        return false;
    }

    // Check against allowed directories (non-blocking iteration with DashSet)
    for allowed in ALLOWED_DIRS.iter() {
        if canonical.starts_with(allowed.key()) {
            return true;
        }
    }

    false
}

/// Read a file to string
/// Returns JSON object with "ok", "content", and optional "error" fields
fn read_file(path: &str) -> RuneString {
    let path = Path::new(path);

    // Canonicalize first to prevent TOCTOU attacks
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            let result = serde_json::json!({
                "ok": false,
                "content": "",
                "error": format!("Cannot resolve path: {}", e)
            });
            return RuneString::try_from(result.to_string()).unwrap_or_default();
        }
    };

    if !is_path_allowed_canonical(&canonical) {
        let result = serde_json::json!({
            "ok": false,
            "content": "",
            "error": "Access denied: path not in allowed directories"
        });
        return RuneString::try_from(result.to_string()).unwrap_or_default();
    }

    // Bug #7 fix: Check file size before reading to prevent OOM
    // Scripts reading huge files (e.g., 500MB log) would previously crash with OOM
    match std::fs::metadata(&canonical) {
        Ok(meta) if meta.len() > MAX_READ_SIZE => {
            let result = serde_json::json!({
                "ok": false,
                "content": "",
                "error": format!("File too large: {} bytes exceeds 10MB limit", meta.len())
            });
            return RuneString::try_from(result.to_string()).unwrap_or_default();
        }
        Err(e) => {
            let result = serde_json::json!({
                "ok": false,
                "content": "",
                "error": format!("Cannot read file metadata: {}", e)
            });
            return RuneString::try_from(result.to_string()).unwrap_or_default();
        }
        _ => {} // File size is OK
    }

    // Use the canonicalized path for reading to prevent TOCTOU
    match std::fs::read_to_string(&canonical) {
        Ok(content) => {
            let result = serde_json::json!({
                "ok": true,
                "content": content
            });
            RuneString::try_from(result.to_string()).unwrap_or_default()
        }
        Err(e) => {
            let result = serde_json::json!({
                "ok": false,
                "content": "",
                "error": format!("Failed to read file: {}", e)
            });
            RuneString::try_from(result.to_string()).unwrap_or_default()
        }
    }
}

/// Read a file and return lines as newline-separated string
fn read_lines(path: &str) -> RuneString {
    read_file(path)
}

/// Read and parse a JSON file
fn read_json(path: &str) -> RuneString {
    let content = read_file(path);
    if content.is_empty() {
        return RuneString::try_from("null").unwrap_or_default();
    }

    // Validate it's valid JSON
    match serde_json::from_str::<serde_json::Value>(content.as_str()) {
        Ok(_) => content,
        Err(_) => RuneString::try_from("null").unwrap_or_default(),
    }
}

/// Check if a file exists
/// Bug #2 fix: Canonicalize path first to prevent traversal bypass
fn file_exists(path: &str) -> bool {
    let path = Path::new(path);
    // For exists check, we can't canonicalize non-existent paths
    // So check both the raw path and try to canonicalize
    if let Ok(canonical) = path.canonicalize() {
        is_path_allowed_canonical(&canonical) && canonical.exists()
    } else {
        // Path doesn't exist or can't be resolved - check raw path is in allowed area
        is_path_allowed(path) && path.exists()
    }
}

/// Check if path is a file
/// Bug #2 fix: Canonicalize path first to prevent traversal bypass
fn is_file(path: &str) -> bool {
    let path = Path::new(path);
    // Canonicalize first to resolve traversal attempts
    if let Ok(canonical) = path.canonicalize() {
        is_path_allowed_canonical(&canonical) && canonical.is_file()
    } else {
        false // Can't canonicalize means path doesn't exist
    }
}

/// Check if path is a directory
/// Bug #2 fix: Canonicalize path first to prevent traversal bypass
fn is_dir(path: &str) -> bool {
    let path = Path::new(path);
    // Canonicalize first to resolve traversal attempts
    if let Ok(canonical) = path.canonicalize() {
        is_path_allowed_canonical(&canonical) && canonical.is_dir()
    } else {
        false // Can't canonicalize means path doesn't exist
    }
}

/// Get file size in bytes
/// Bug #2 fix: Canonicalize path first to prevent traversal bypass
fn file_size(path: &str) -> i64 {
    let path = Path::new(path);
    // Canonicalize first to resolve traversal attempts
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return -1,
    };

    if !is_path_allowed_canonical(&canonical) {
        return -1;
    }

    match std::fs::metadata(&canonical) {
        Ok(meta) => meta.len() as i64,
        Err(_) => -1,
    }
}

/// Join path components
fn path_join(base: &str, path: &str) -> RuneString {
    let joined = Path::new(base).join(path);
    RuneString::try_from(joined.to_string_lossy().to_string()).unwrap_or_default()
}

/// Get the file name from a path
fn basename(path: &str) -> RuneString {
    let path = Path::new(path);
    match path.file_name() {
        Some(name) => RuneString::try_from(name.to_string_lossy().to_string()).unwrap_or_default(),
        None => RuneString::new(),
    }
}

/// Get the directory portion of a path
fn dirname(path: &str) -> RuneString {
    let path = Path::new(path);
    match path.parent() {
        Some(parent) => RuneString::try_from(parent.to_string_lossy().to_string()).unwrap_or_default(),
        None => RuneString::new(),
    }
}

/// Get the file extension
fn extension(path: &str) -> RuneString {
    let path = Path::new(path);
    match path.extension() {
        Some(ext) => RuneString::try_from(ext.to_string_lossy().to_string()).unwrap_or_default(),
        None => RuneString::new(),
    }
}

/// Get the temp directory path
fn temp_dir() -> RuneString {
    RuneString::try_from(std::env::temp_dir().to_string_lossy().to_string()).unwrap_or_default()
}

/// Get the current working directory
fn current_dir() -> RuneString {
    match std::env::current_dir() {
        Ok(path) => RuneString::try_from(path.to_string_lossy().to_string()).unwrap_or_default(),
        Err(_) => RuneString::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_path_utilities() {
        let joined = path_join("/tmp", "test.txt");
        assert!(joined.contains("test.txt"));

        let base = basename("/path/to/file.txt");
        assert_eq!(base.as_str(), "file.txt");

        let dir = dirname("/path/to/file.txt");
        assert!(dir.contains("to"));

        let ext = extension("/path/to/file.txt");
        assert_eq!(ext.as_str(), "txt");
    }

    #[test]
    fn test_blocked_patterns() {
        assert!(!is_path_allowed(Path::new("/home/user/.env")));
        assert!(!is_path_allowed(Path::new("/home/user/.ssh/id_rsa")));
        assert!(!is_path_allowed(Path::new("/path/to/credentials.json")));
    }

    #[test]
    fn test_temp_dir_access() {
        // Create a temp file
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        writeln!(temp_file, "test content").unwrap();

        let path_str = temp_file.path().to_string_lossy().to_string();
        assert!(file_exists(&path_str), "file_exists failed for {}", path_str);
        assert!(is_file(&path_str), "is_file failed for {}", path_str);

        // read_file now returns JSON with "ok", "content", and optional "error" fields
        let result = read_file(&path_str);
        let result_str = result.as_str();
        assert!(result_str.contains("\"ok\":true") || result_str.contains("\"ok\": true"),
            "Expected ok:true in result, got: {}", result_str);
        assert!(result_str.contains("test content"),
            "Expected 'test content' in result, got: {}", result_str);
    }
}
