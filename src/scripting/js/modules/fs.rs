//! Filesystem module for JavaScript
//!
//! Provides sandboxed file system access.
//! For security, file operations are restricted to specific directories.

use rquickjs::{Ctx, Object, Function};
use std::path::{Path, PathBuf};
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let fs = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create fs object: {}", e)))?;

    fs.set("read", Function::new(ctx.clone(), fs_read)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    fs.set("exists", Function::new(ctx.clone(), fs_exists)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    fs.set("is_file", Function::new(ctx.clone(), fs_is_file)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    fs.set("is_dir", Function::new(ctx.clone(), fs_is_dir)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    fs.set("size", Function::new(ctx.clone(), fs_size)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    fs.set("list", Function::new(ctx.clone(), fs_list)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    fs.set("basename", Function::new(ctx.clone(), fs_basename)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    fs.set("dirname", Function::new(ctx.clone(), fs_dirname)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    fs.set("extension", Function::new(ctx.clone(), fs_extension)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    fs.set("join", Function::new(ctx.clone(), fs_join)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("fs", fs)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set fs global: {}", e)))?;

    Ok(())
}

/// Validate that a path is safe to access.
/// For security, we only allow:
/// - Paths within current working directory
/// - Paths within user's home directory under specific subdirs
fn is_safe_path(path: &Path) -> bool {
    // Canonicalize to resolve .. and symlinks
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If path doesn't exist, check parent
            if let Some(parent) = path.parent() {
                match parent.canonicalize() {
                    Ok(p) => p.join(path.file_name().unwrap_or_default()),
                    Err(_) => return false,
                }
            } else {
                return false;
            }
        }
    };

    // Allow current working directory
    if let Ok(cwd) = std::env::current_dir() {
        if canonical.starts_with(&cwd) {
            return true;
        }
    }

    // Allow specific directories under home
    if let Some(home) = dirs::home_dir() {
        let allowed_subdirs = [
            ".config/quicpulse",
            ".quicpulse",
            "quicpulse",
        ];

        for subdir in &allowed_subdirs {
            let allowed = home.join(subdir);
            if canonical.starts_with(&allowed) {
                return true;
            }
        }
    }

    false
}

fn resolve_path(path_str: &str) -> Option<PathBuf> {
    let path = Path::new(path_str);

    // Expand ~ to home directory
    let expanded = if path_str.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(&path_str[2..])
        } else {
            return None;
        }
    } else {
        path.to_path_buf()
    };

    if is_safe_path(&expanded) {
        Some(expanded)
    } else {
        None
    }
}

fn fs_read(path: String) -> Option<String> {
    let resolved = resolve_path(&path)?;
    std::fs::read_to_string(resolved).ok()
}

fn fs_exists(path: String) -> bool {
    resolve_path(&path)
        .map(|p| p.exists())
        .unwrap_or(false)
}

fn fs_is_file(path: String) -> bool {
    resolve_path(&path)
        .map(|p| p.is_file())
        .unwrap_or(false)
}

fn fs_is_dir(path: String) -> bool {
    resolve_path(&path)
        .map(|p| p.is_dir())
        .unwrap_or(false)
}

fn fs_size(path: String) -> Option<i64> {
    let resolved = resolve_path(&path)?;
    std::fs::metadata(resolved)
        .ok()
        .map(|m| m.len() as i64)
}

fn fs_list(path: String) -> String {
    let entries: Vec<String> = resolve_path(&path)
        .and_then(|p| std::fs::read_dir(p).ok())
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();

    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
}

fn fs_basename(path: String) -> Option<String> {
    Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
}

fn fs_dirname(path: String) -> Option<String> {
    Path::new(&path)
        .parent()
        .and_then(|p| p.to_str())
        .map(String::from)
}

fn fs_extension(path: String) -> Option<String> {
    Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .map(String::from)
}

fn fs_join(base: String, path: String) -> String {
    Path::new(&base).join(&path).to_string_lossy().to_string()
}
