pub mod colors {
    pub const GREY: u8 = 102;      // #7D7D7D - Punctuation, secondary
    pub const AQUA: u8 = 109;      // #7A9EB5 - Numbers, info
    pub const PURPLE: u8 = 134;    // #9E54D6 - Special
    pub const ORANGE: u8 = 208;    // #F2913D - Warnings, PUT/PATCH
    pub const RED: u8 = 167;       // #E34F45 - Errors, DELETE
    pub const BLUE: u8 = 68;       // #426BD1 - Names, labels
    pub const PINK: u8 = 176;      // #DE85DE - Keys
    pub const GREEN: u8 = 71;      // #63C27A - Success, GET
    pub const YELLOW: u8 = 185;    // #CCCC3D - POST, redirects
    pub const WHITE: u8 = 250;     // Primary text
}

/// ANSI escape code constants
pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

/// Generate foreground color escape code
#[inline]
pub fn fg(color: u8) -> String {
    format!("\x1b[38;5;{}m", color)
}

/// Generate bold foreground color escape code
#[inline]
pub fn bold_fg(color: u8) -> String {
    format!("\x1b[1;38;5;{}m", color)
}

/// Colorize text with a foreground color
#[inline]
pub fn colorize(text: &str, color: u8) -> String {
    format!("{}{}{}", fg(color), text, RESET)
}

/// Colorize text with bold foreground color
#[inline]
pub fn bold(text: &str, color: u8) -> String {
    format!("{}{}{}", bold_fg(color), text, RESET)
}

/// Success message (green)
#[inline]
pub fn success(text: &str) -> String {
    bold(text, colors::GREEN)
}

/// Error message (red)
#[inline]
pub fn error(text: &str) -> String {
    bold(text, colors::RED)
}

/// Warning message (orange)
#[inline]
pub fn warning(text: &str) -> String {
    bold(text, colors::ORANGE)
}

/// Info message (aqua)
#[inline]
pub fn info(text: &str) -> String {
    colorize(text, colors::AQUA)
}

/// Label/name (blue)
#[inline]
pub fn label(text: &str) -> String {
    colorize(text, colors::BLUE)
}

/// Key (pink)
#[inline]
pub fn key(text: &str) -> String {
    colorize(text, colors::PINK)
}

/// Value/string (green)
#[inline]
pub fn value(text: &str) -> String {
    colorize(text, colors::GREEN)
}

/// Number (aqua)
#[inline]
pub fn number(text: &str) -> String {
    colorize(text, colors::AQUA)
}

/// Secondary/muted text (grey)
#[inline]
pub fn muted(text: &str) -> String {
    colorize(text, colors::GREY)
}

/// Protocol-specific colors
pub mod protocol {
    use super::*;

    /// WebSocket message type label
    pub fn ws_label(label: &str) -> String {
        format!("{}[{}]{}", fg(colors::PURPLE), label, RESET)
    }

    /// gRPC status
    pub fn grpc_status(code: i32) -> String {
        let color = match code {
            0 => colors::GREEN,  // OK
            1 => colors::ORANGE, // CANCELLED
            2 => colors::RED,    // UNKNOWN
            3 => colors::ORANGE, // INVALID_ARGUMENT
            4 => colors::ORANGE, // DEADLINE_EXCEEDED
            5 => colors::RED,    // NOT_FOUND
            6 => colors::ORANGE, // ALREADY_EXISTS
            7 => colors::RED,    // PERMISSION_DENIED
            8 => colors::ORANGE, // RESOURCE_EXHAUSTED
            9 => colors::ORANGE, // FAILED_PRECONDITION
            10 => colors::ORANGE, // ABORTED
            11 => colors::ORANGE, // OUT_OF_RANGE
            12 => colors::RED,   // UNIMPLEMENTED
            13 => colors::RED,   // INTERNAL
            14 => colors::RED,   // UNAVAILABLE
            15 => colors::RED,   // DATA_LOSS
            16 => colors::RED,   // UNAUTHENTICATED
            _ => colors::GREY,
        };
        bold_fg(color)
    }

    /// HTTP status code color
    pub fn http_status(code: u16) -> String {
        let color = match code / 100 {
            1 => colors::AQUA,   // Informational
            2 => colors::GREEN,  // Success
            3 => colors::YELLOW, // Redirect
            4 => colors::ORANGE, // Client error
            5 => colors::RED,    // Server error
            _ => colors::GREY,
        };
        bold_fg(color)
    }

    /// HTTP method color
    pub fn http_method(method: &str) -> String {
        let color = match method.to_uppercase().as_str() {
            "GET" | "HEAD" | "OPTIONS" => colors::GREEN,
            "POST" => colors::YELLOW,
            "PUT" | "PATCH" => colors::ORANGE,
            "DELETE" => colors::RED,
            _ => colors::GREY,
        };
        bold_fg(color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fg_color() {
        assert_eq!(fg(71), "\x1b[38;5;71m");
    }

    #[test]
    fn test_bold_fg_color() {
        assert_eq!(bold_fg(71), "\x1b[1;38;5;71m");
    }

    #[test]
    fn test_colorize() {
        let result = colorize("test", colors::GREEN);
        assert!(result.contains("38;5;71m"));
        assert!(result.contains("test"));
        assert!(result.ends_with(RESET));
    }

    #[test]
    fn test_success() {
        let result = success("OK");
        assert!(result.contains("1;38;5;71m")); // bold green
    }

    #[test]
    fn test_error() {
        let result = error("FAIL");
        assert!(result.contains("1;38;5;167m")); // bold red
    }
}
