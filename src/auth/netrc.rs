//! Netrc file parsing for automatic authentication
//!
//! Supports reading credentials from ~/.netrc or ~/_netrc (Windows)

use std::fs;
use std::path::PathBuf;

/// Netrc file entry
#[derive(Debug, Clone)]
pub struct NetrcEntry {
    /// Machine/host name
    pub machine: String,
    /// Login/username
    pub login: Option<String>,
    /// Password
    pub password: Option<String>,
    /// Account (rarely used)
    pub account: Option<String>,
}

/// Parsed netrc file
#[derive(Debug, Default)]
pub struct Netrc {
    /// Entries indexed by machine name
    entries: Vec<NetrcEntry>,
    /// Default entry (if any)
    default: Option<NetrcEntry>,
}

impl Netrc {
    /// Load netrc from default location
    pub fn load() -> Option<Self> {
        let path = Self::default_path()?;
        Self::load_from(&path)
    }

    /// Get default netrc path
    fn default_path() -> Option<PathBuf> {
        let home = dirs::home_dir()?;

        // Try .netrc first (Unix style)
        let netrc = home.join(".netrc");
        if netrc.exists() {
            return Some(netrc);
        }

        // Try _netrc (Windows style)
        let netrc = home.join("_netrc");
        if netrc.exists() {
            return Some(netrc);
        }

        None
    }

    /// Load netrc from a specific path
    pub fn load_from(path: &PathBuf) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        Self::parse(&content)
    }

    /// Parse netrc file content using netrc-rs crate
    fn parse(content: &str) -> Option<Self> {
        // Preprocess: strip comments and handle quoted values
        // (netrc-rs doesn't support these common extensions)
        let preprocessed = Self::preprocess(content);
        let parsed = netrc_rs::Netrc::parse(&preprocessed, false).ok()?;

        let mut netrc = Netrc::default();

        for machine in parsed.machines {
            let entry = NetrcEntry {
                machine: Self::restore_spaces(machine.name.clone())
                    .unwrap_or_else(|| "default".to_string()),
                login: Self::restore_spaces(machine.login),
                password: Self::restore_spaces(machine.password),
                account: Self::restore_spaces(machine.account),
            };

            if machine.name.is_none() {
                // Default entry (name is None in netrc-rs)
                netrc.default = Some(entry);
            } else {
                netrc.entries.push(entry);
            }
        }

        Some(netrc)
    }

    /// Preprocess netrc content to handle extensions not supported by netrc-rs:
    /// - Strip comments (lines starting with # and inline # comments)
    /// - Handle quoted values containing spaces (using placeholder)
    ///
    /// Uses U+001E (Record Separator) as a placeholder for spaces inside quotes.
    fn preprocess(content: &str) -> String {
        const SPACE_PLACEHOLDER: char = '\u{001E}'; // Record Separator - ASCII control char, not whitespace

        let mut result = String::new();
        let mut prev_char = '\0';

        for line in content.lines() {
            let trimmed = line.trim_start();

            // Skip comment-only lines
            if trimmed.starts_with('#') {
                result.push('\n');
                continue;
            }

            // Process the line character by character
            let mut line_result = String::new();
            let mut in_quotes = false;

            for c in line.chars() {
                // Handle escaped quotes
                if c == '"' && prev_char != '\\' {
                    in_quotes = !in_quotes;
                    // Don't include quote chars in output - netrc-rs doesn't handle them
                    prev_char = c;
                    continue;
                }

                // Handle inline comments (only outside quotes)
                if c == '#' && !in_quotes {
                    break;
                }

                // Replace spaces inside quotes with placeholder
                if c == ' ' && in_quotes {
                    line_result.push(SPACE_PLACEHOLDER);
                } else {
                    line_result.push(c);
                }
                prev_char = c;
            }

            result.push_str(&line_result);
            result.push('\n');
        }

        result
    }

    /// Restore spaces from placeholder after parsing
    fn restore_spaces(value: Option<String>) -> Option<String> {
        const SPACE_PLACEHOLDER: char = '\u{001E}';
        value.map(|s| s.replace(SPACE_PLACEHOLDER, " "))
    }

    /// Get credentials for a host
    pub fn get(&self, host: &str) -> Option<&NetrcEntry> {
        // Try exact match first
        if let Some(entry) = self.entries.iter().find(|e| e.machine == host) {
            return Some(entry);
        }

        // Try without port
        if let Some((hostname, _)) = host.rsplit_once(':') {
            if let Some(entry) = self.entries.iter().find(|e| e.machine == hostname) {
                return Some(entry);
            }
        }

        // Fall back to default
        self.default.as_ref()
    }

    /// Get login and password for a host
    pub fn get_credentials(&self, host: &str) -> Option<(String, String)> {
        let entry = self.get(host)?;
        let login = entry.login.clone()?;
        let password = entry.password.clone().unwrap_or_default();
        Some((login, password))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_netrc() {
        let content = r#"
            machine example.com
            login user1
            password secret1

            machine api.example.com
            login user2
            password secret2

            default
            login anonymous
            password guest
        "#;

        let netrc = Netrc::parse(content).unwrap();

        // Check specific machine
        let entry = netrc.get("example.com").unwrap();
        assert_eq!(entry.login.as_deref(), Some("user1"));
        assert_eq!(entry.password.as_deref(), Some("secret1"));

        // Check another machine
        let entry = netrc.get("api.example.com").unwrap();
        assert_eq!(entry.login.as_deref(), Some("user2"));
        assert_eq!(entry.password.as_deref(), Some("secret2"));

        // Check default
        let entry = netrc.get("unknown.com").unwrap();
        assert_eq!(entry.login.as_deref(), Some("anonymous"));
        assert_eq!(entry.password.as_deref(), Some("guest"));
    }

    #[test]
    fn test_get_credentials() {
        let content = r#"
            machine example.com login testuser password testpass
        "#;

        let netrc = Netrc::parse(content).unwrap();
        let (login, password) = netrc.get_credentials("example.com").unwrap();

        assert_eq!(login, "testuser");
        assert_eq!(password, "testpass");
    }

    #[test]
    fn test_host_with_port() {
        let content = r#"
            machine example.com login user password pass
        "#;

        let netrc = Netrc::parse(content).unwrap();

        // Should match host with port
        let entry = netrc.get("example.com:8080");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().login.as_deref(), Some("user"));
    }

    #[test]
    fn test_quoted_password_with_spaces() {
        let content = r#"
            machine example.com
            login "user name"
            password "secret phrase with spaces"
        "#;

        let netrc = Netrc::parse(content).unwrap();
        let entry = netrc.get("example.com").unwrap();

        assert_eq!(entry.login.as_deref(), Some("user name"));
        assert_eq!(entry.password.as_deref(), Some("secret phrase with spaces"));
    }

    #[test]
    fn test_comments() {
        let content = r#"
            # This is a comment
            machine example.com
            login user # inline comment is not standard, but handle gracefully
            password secret
        "#;

        let netrc = Netrc::parse(content).unwrap();
        let entry = netrc.get("example.com").unwrap();

        assert_eq!(entry.login.as_deref(), Some("user"));
        assert_eq!(entry.password.as_deref(), Some("secret"));
    }
}
