//! Environment struct (stdin/stdout/etc.)

use std::io::{self, Stdin, Stdout, Stderr};

/// Execution environment
pub struct Environment {
    pub stdin: Stdin,
    pub stdout: Stdout,
    pub stderr: Stderr,
    pub stdin_isatty: bool,
    pub stdout_isatty: bool,
    pub stderr_isatty: bool,
    pub colors: u32,
    pub program_name: String,
}

impl Environment {
    /// Initialize the environment with Windows ANSI support
    pub fn init() -> Self {
        // Enable ANSI escape codes on Windows 10+
        #[cfg(windows)]
        {
            // crossterm handles enabling virtual terminal processing
            let _ = crossterm::execute!(io::stdout(), crossterm::terminal::SetTitle("QuicPulse"));
        }

        Self::default()
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            stdin: io::stdin(),
            stdout: io::stdout(),
            stderr: io::stderr(),
            stdin_isatty: atty::is(atty::Stream::Stdin),
            stdout_isatty: atty::is(atty::Stream::Stdout),
            stderr_isatty: atty::is(atty::Stream::Stderr),
            colors: detect_color_support(),
            program_name: "http".to_string(),
        }
    }
}

/// Detect color support level
fn detect_color_support() -> u32 {
    if !atty::is(atty::Stream::Stdout) {
        return 0;
    }

    // Check for NO_COLOR environment variable
    if std::env::var("NO_COLOR").is_ok() {
        return 0;
    }

    // Check COLORTERM for truecolor support
    if let Ok(colorterm) = std::env::var("COLORTERM") {
        if colorterm == "truecolor" || colorterm == "24bit" {
            return 16777216; // 24-bit color
        }
    }

    // Check TERM for 256 color support
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("256color") || term.contains("256") {
            return 256;
        }
        if term == "dumb" {
            return 0;
        }
    }

    // Default to 256 colors for modern terminals
    256
}
