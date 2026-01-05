//! AWS configuration file parser
//!
//! Parses `~/.aws/credentials` and `~/.aws/config` files to load AWS profiles.
//! Supports:
//! - Static credentials (access_key_id, secret_access_key, session_token)
//! - SSO profiles (sso_start_url, sso_region, sso_account_id, sso_role_name)
//! - AssumeRole profiles (role_arn, source_profile, external_id)

use std::collections::HashMap;
use std::path::PathBuf;
use crate::errors::QuicpulseError;

/// AWS profile configuration
#[derive(Debug, Clone, Default)]
pub struct AwsProfile {
    /// Profile name
    pub name: String,

    // Static credentials
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,

    // Region configuration
    pub region: Option<String>,

    // SSO configuration (Gap 2)
    pub sso_start_url: Option<String>,
    pub sso_region: Option<String>,
    pub sso_account_id: Option<String>,
    pub sso_role_name: Option<String>,
    pub sso_session: Option<String>,

    // AssumeRole configuration (Gap 3)
    pub role_arn: Option<String>,
    pub source_profile: Option<String>,
    pub external_id: Option<String>,
    pub role_session_name: Option<String>,
    pub duration_seconds: Option<u32>,

    // Credential process (external command)
    pub credential_process: Option<String>,
}

impl AwsProfile {
    /// Check if this profile has static credentials
    pub fn has_static_credentials(&self) -> bool {
        self.access_key_id.is_some() && self.secret_access_key.is_some()
    }

    /// Check if this profile uses SSO
    pub fn is_sso_profile(&self) -> bool {
        self.sso_start_url.is_some() || self.sso_session.is_some()
    }

    /// Check if this profile uses AssumeRole
    pub fn is_assume_role_profile(&self) -> bool {
        self.role_arn.is_some()
    }

    /// Check if this profile uses a credential process
    pub fn has_credential_process(&self) -> bool {
        self.credential_process.is_some()
    }
}

/// Load an AWS profile by name
///
/// Searches in order:
/// 1. `~/.aws/credentials` for static credentials
/// 2. `~/.aws/config` for additional settings (region, SSO, AssumeRole)
///
/// The profile name in `~/.aws/config` is prefixed with "profile " (except for "default").
pub fn load_profile(name: &str) -> Result<AwsProfile, QuicpulseError> {
    let mut profile = AwsProfile {
        name: name.to_string(),
        ..Default::default()
    };

    // Load from credentials file first
    if let Ok(credentials) = parse_credentials_file() {
        if let Some(creds) = credentials.get(name) {
            profile.access_key_id = creds.access_key_id.clone();
            profile.secret_access_key = creds.secret_access_key.clone();
            profile.session_token = creds.session_token.clone();
        }
    }

    // Load from config file (may override or add settings)
    if let Ok(config) = parse_config_file() {
        if let Some(cfg) = config.get(name) {
            // Merge config settings
            if cfg.region.is_some() {
                profile.region = cfg.region.clone();
            }
            if cfg.sso_start_url.is_some() {
                profile.sso_start_url = cfg.sso_start_url.clone();
            }
            if cfg.sso_region.is_some() {
                profile.sso_region = cfg.sso_region.clone();
            }
            if cfg.sso_account_id.is_some() {
                profile.sso_account_id = cfg.sso_account_id.clone();
            }
            if cfg.sso_role_name.is_some() {
                profile.sso_role_name = cfg.sso_role_name.clone();
            }
            if cfg.sso_session.is_some() {
                profile.sso_session = cfg.sso_session.clone();
            }
            if cfg.role_arn.is_some() {
                profile.role_arn = cfg.role_arn.clone();
            }
            if cfg.source_profile.is_some() {
                profile.source_profile = cfg.source_profile.clone();
            }
            if cfg.external_id.is_some() {
                profile.external_id = cfg.external_id.clone();
            }
            if cfg.role_session_name.is_some() {
                profile.role_session_name = cfg.role_session_name.clone();
            }
            if cfg.duration_seconds.is_some() {
                profile.duration_seconds = cfg.duration_seconds;
            }
            if cfg.credential_process.is_some() {
                profile.credential_process = cfg.credential_process.clone();
            }
            // Config file can also have credentials
            if cfg.access_key_id.is_some() && profile.access_key_id.is_none() {
                profile.access_key_id = cfg.access_key_id.clone();
            }
            if cfg.secret_access_key.is_some() && profile.secret_access_key.is_none() {
                profile.secret_access_key = cfg.secret_access_key.clone();
            }
            if cfg.session_token.is_some() && profile.session_token.is_none() {
                profile.session_token = cfg.session_token.clone();
            }
        }
    }

    // Validate that we found something useful
    if !profile.has_static_credentials()
        && !profile.is_sso_profile()
        && !profile.is_assume_role_profile()
        && !profile.has_credential_process()
    {
        return Err(QuicpulseError::Config(format!(
            "AWS profile '{}' not found or has no valid credentials configuration",
            name
        )));
    }

    Ok(profile)
}

/// Get the path to the AWS credentials file
fn credentials_file_path() -> Option<PathBuf> {
    // Check AWS_SHARED_CREDENTIALS_FILE env var first
    if let Ok(path) = std::env::var("AWS_SHARED_CREDENTIALS_FILE") {
        return Some(PathBuf::from(path));
    }

    // Default to ~/.aws/credentials
    dirs::home_dir().map(|h| h.join(".aws").join("credentials"))
}

/// Get the path to the AWS config file
fn config_file_path() -> Option<PathBuf> {
    // Check AWS_CONFIG_FILE env var first
    if let Ok(path) = std::env::var("AWS_CONFIG_FILE") {
        return Some(PathBuf::from(path));
    }

    // Default to ~/.aws/config
    dirs::home_dir().map(|h| h.join(".aws").join("config"))
}

/// Parse the AWS credentials file (~/.aws/credentials)
fn parse_credentials_file() -> Result<HashMap<String, AwsProfile>, QuicpulseError> {
    let path = credentials_file_path()
        .ok_or_else(|| QuicpulseError::Config("Could not determine home directory".to_string()))?;

    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| QuicpulseError::Config(format!("Failed to read credentials file: {}", e)))?;

    parse_ini_file(&content, false)
}

/// Parse the AWS config file (~/.aws/config)
fn parse_config_file() -> Result<HashMap<String, AwsProfile>, QuicpulseError> {
    let path = config_file_path()
        .ok_or_else(|| QuicpulseError::Config("Could not determine home directory".to_string()))?;

    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| QuicpulseError::Config(format!("Failed to read config file: {}", e)))?;

    parse_ini_file(&content, true)
}

/// Parse an INI-format AWS configuration file
///
/// In the config file, profile sections are prefixed with "profile " (except default).
/// In the credentials file, sections are just the profile name.
fn parse_ini_file(content: &str, is_config_file: bool) -> Result<HashMap<String, AwsProfile>, QuicpulseError> {
    let mut profiles: HashMap<String, AwsProfile> = HashMap::new();
    let mut current_profile: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // Check for section header
        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len() - 1].trim();

            // Extract profile name
            let profile_name = if is_config_file {
                // Config file uses "profile name" format (except for "default")
                if *section == "default" {
                    "default".to_string()
                } else if let Some(name) = section.strip_prefix("profile ") {
                    name.trim().to_string()
                } else if section.starts_with("sso-session ") {
                    // Skip SSO session blocks for now
                    current_profile = None;
                    continue;
                } else {
                    // Unknown section type, skip
                    current_profile = None;
                    continue;
                }
            } else {
                // Credentials file uses plain profile names
                section.to_string()
            };

            current_profile = Some(profile_name.clone());
            profiles.entry(profile_name.clone()).or_insert_with(|| AwsProfile {
                name: profile_name,
                ..Default::default()
            });
            continue;
        }

        // Parse key = value pairs
        if let Some(profile_name) = &current_profile {
            if let Some((key, value)) = parse_key_value(line) {
                if let Some(profile) = profiles.get_mut(profile_name) {
                    set_profile_field(profile, &key, &value);
                }
            }
        }
    }

    Ok(profiles)
}

/// Parse a key = value line
fn parse_key_value(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() == 2 {
        let key = parts[0].trim().to_lowercase();
        let value = parts[1].trim().to_string();
        Some((key, value))
    } else {
        None
    }
}

/// Set a profile field based on key name
fn set_profile_field(profile: &mut AwsProfile, key: &str, value: &str) {
    match key {
        "aws_access_key_id" => profile.access_key_id = Some(value.to_string()),
        "aws_secret_access_key" => profile.secret_access_key = Some(value.to_string()),
        "aws_session_token" => profile.session_token = Some(value.to_string()),
        "region" => profile.region = Some(value.to_string()),
        "sso_start_url" => profile.sso_start_url = Some(value.to_string()),
        "sso_region" => profile.sso_region = Some(value.to_string()),
        "sso_account_id" => profile.sso_account_id = Some(value.to_string()),
        "sso_role_name" => profile.sso_role_name = Some(value.to_string()),
        "sso_session" => profile.sso_session = Some(value.to_string()),
        "role_arn" => profile.role_arn = Some(value.to_string()),
        "source_profile" => profile.source_profile = Some(value.to_string()),
        "external_id" => profile.external_id = Some(value.to_string()),
        "role_session_name" => profile.role_session_name = Some(value.to_string()),
        "duration_seconds" => {
            if let Ok(seconds) = value.parse::<u32>() {
                profile.duration_seconds = Some(seconds);
            }
        }
        "credential_process" => profile.credential_process = Some(value.to_string()),
        _ => {} // Ignore unknown keys
    }
}

/// Get the default profile name from environment or return "default"
pub fn get_default_profile_name() -> String {
    std::env::var("AWS_PROFILE")
        .or_else(|_| std::env::var("AWS_DEFAULT_PROFILE"))
        .unwrap_or_else(|_| "default".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_credentials_ini() {
        let content = r#"
[default]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

[dev]
aws_access_key_id = AKIAI44QH8DHBEXAMPLE
aws_secret_access_key = je7MtGbClwBF/2Zp9Utk/h3yCo8nvbEXAMPLEKEY
aws_session_token = AQoDYXdzEJr...
"#;

        let profiles = parse_ini_file(content, false).unwrap();

        assert!(profiles.contains_key("default"));
        assert!(profiles.contains_key("dev"));

        let default = profiles.get("default").unwrap();
        assert_eq!(default.access_key_id.as_deref(), Some("AKIAIOSFODNN7EXAMPLE"));
        assert_eq!(default.secret_access_key.as_deref(), Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"));

        let dev = profiles.get("dev").unwrap();
        assert!(dev.session_token.is_some());
    }

    #[test]
    fn test_parse_config_ini() {
        let content = r#"
[default]
region = us-east-1

[profile dev]
region = us-west-2
role_arn = arn:aws:iam::123456789012:role/DevRole
source_profile = default

[profile sso-user]
sso_start_url = https://my-sso-portal.awsapps.com/start
sso_region = us-east-1
sso_account_id = 123456789012
sso_role_name = ReadOnlyAccess
"#;

        let profiles = parse_ini_file(content, true).unwrap();

        assert!(profiles.contains_key("default"));
        assert!(profiles.contains_key("dev"));
        assert!(profiles.contains_key("sso-user"));

        let default = profiles.get("default").unwrap();
        assert_eq!(default.region.as_deref(), Some("us-east-1"));

        let dev = profiles.get("dev").unwrap();
        assert_eq!(dev.region.as_deref(), Some("us-west-2"));
        assert!(dev.is_assume_role_profile());
        assert_eq!(dev.source_profile.as_deref(), Some("default"));

        let sso = profiles.get("sso-user").unwrap();
        assert!(sso.is_sso_profile());
        assert_eq!(sso.sso_account_id.as_deref(), Some("123456789012"));
    }

    #[test]
    fn test_profile_type_detection() {
        let mut profile = AwsProfile::default();

        // Empty profile
        assert!(!profile.has_static_credentials());
        assert!(!profile.is_sso_profile());
        assert!(!profile.is_assume_role_profile());

        // Static credentials
        profile.access_key_id = Some("AKIA...".to_string());
        profile.secret_access_key = Some("secret".to_string());
        assert!(profile.has_static_credentials());

        // SSO profile
        let mut sso_profile = AwsProfile::default();
        sso_profile.sso_start_url = Some("https://example.awsapps.com/start".to_string());
        assert!(sso_profile.is_sso_profile());

        // AssumeRole profile
        let mut role_profile = AwsProfile::default();
        role_profile.role_arn = Some("arn:aws:iam::123:role/Test".to_string());
        assert!(role_profile.is_assume_role_profile());
    }
}
