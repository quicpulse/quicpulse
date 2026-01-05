//! Pager integration for long output
//!
//! Pipes output through a pager like `less` or `more` for easier reading.

use std::io::{self, Write};
use std::process::{Command, Stdio, Child};

/// Pager configuration
#[derive(Debug, Clone)]
pub struct PagerConfig {
    /// Whether paging is enabled
    pub enabled: bool,
    /// Custom pager command (overrides $PAGER)
    pub command: Option<String>,
}

impl Default for PagerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            command: None,
        }
    }
}

/// Get the pager command from environment or use default
pub fn get_pager_command() -> String {
    std::env::var("PAGER").unwrap_or_else(|_| {
        // Default to less with raw control codes enabled (for colors)
        "less -R".to_string()
    })
}

/// A writer that pipes output through a pager
pub struct PagerWriter {
    child: Child,
    stdin: Option<std::process::ChildStdin>,
}

impl PagerWriter {
    /// Create a new pager writer
    pub fn new() -> io::Result<Self> {
        Self::with_command(&get_pager_command())
    }

    /// Create a pager writer with a specific command
    pub fn with_command(cmd: &str) -> io::Result<Self> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Empty pager command",
            ));
        }

        let mut command = Command::new(parts[0]);
        if parts.len() > 1 {
            command.args(&parts[1..]);
        }

        let mut child = command
            .stdin(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take();

        Ok(Self { child, stdin })
    }

    /// Wait for the pager to finish
    pub fn wait(mut self) -> io::Result<()> {
        // Close stdin to signal EOF to the pager
        drop(self.stdin.take());
        self.child.wait()?;
        Ok(())
    }
}

impl Write for PagerWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(ref mut stdin) = self.stdin {
            stdin.write(buf)
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Pager stdin closed",
            ))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(ref mut stdin) = self.stdin {
            stdin.flush()
        } else {
            Ok(())
        }
    }
}

impl Drop for PagerWriter {
    fn drop(&mut self) {
        // Close stdin to signal EOF
        drop(self.stdin.take());
        // Wait for child to finish (ignore errors on drop)
        let _ = self.child.wait();
    }
}

/// Check if output should be paged based on content length and terminal
pub fn should_page(content: &str, is_tty: bool, forced: bool) -> bool {
    if forced {
        return true;
    }

    if !is_tty {
        return false;
    }

    // Get terminal height
    let terminal_height = terminal_height().unwrap_or(24);
    let line_count = content.lines().count();

    // Page if content exceeds terminal height
    line_count > terminal_height
}

/// Get terminal height
fn terminal_height() -> Option<usize> {
    // Try to get terminal size
    if let Ok((_, rows)) = crossterm::terminal::size() {
        Some(rows as usize)
    } else {
        None
    }
}

/// Write content through a pager if appropriate
pub fn write_with_pager<W: Write>(
    output: &mut W,
    content: &str,
    config: &PagerConfig,
    is_tty: bool,
) -> io::Result<()> {
    if !config.enabled || !is_tty {
        // No paging, write directly
        output.write_all(content.as_bytes())?;
        return Ok(());
    }

    // Determine pager command
    let cmd = config.command.clone().unwrap_or_else(get_pager_command);

    // Try to create pager
    match PagerWriter::with_command(&cmd) {
        Ok(mut pager) => {
            pager.write_all(content.as_bytes())?;
            pager.wait()?;
            Ok(())
        }
        Err(_) => {
            // Fallback to direct output if pager fails
            output.write_all(content.as_bytes())?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pager_command() {
        // Should return something (either $PAGER or default)
        let cmd = get_pager_command();
        assert!(!cmd.is_empty());
    }

    #[test]
    fn test_should_page_forced() {
        assert!(should_page("short", false, true));
        assert!(should_page("short", true, true));
    }

    #[test]
    fn test_should_page_not_tty() {
        // Long content but not a TTY
        let long_content = "line\n".repeat(100);
        assert!(!should_page(&long_content, false, false));
    }

    #[test]
    fn test_pager_config_default() {
        let config = PagerConfig::default();
        assert!(!config.enabled);
        assert!(config.command.is_none());
    }
}
