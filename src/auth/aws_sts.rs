//! AWS STS AssumeRole support
//!
//! Provides automatic role assumption for AWS profiles configured with
//! `role_arn` and `source_profile`.

use crate::errors::QuicpulseError;
use super::aws::AwsSigV4Config;
use super::aws_config::AwsProfile;

/// Temporary AWS credentials from STS
#[derive(Debug, Clone)]
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
    pub expiration: Option<String>,
}

/// Resolve an AssumeRole profile to actual credentials
pub async fn resolve_assume_role_profile(
    profile: &AwsProfile,
    region: String,
    service: String,
) -> Result<AwsSigV4Config, QuicpulseError> {
    let role_arn = profile.role_arn.as_ref()
        .ok_or_else(|| QuicpulseError::Config("Profile missing role_arn".to_string()))?;

    // Get source credentials
    let source_profile_name = profile.source_profile.as_deref().unwrap_or("default");
    let source_profile = super::aws_config::load_profile(source_profile_name)?;

    // Source profile must have static credentials (or be another resolvable profile)
    let source_creds = if source_profile.has_static_credentials() {
        AwsCredentials {
            access_key_id: source_profile.access_key_id.clone().unwrap(),
            secret_access_key: source_profile.secret_access_key.clone().unwrap(),
            session_token: source_profile.session_token.clone(),
            expiration: None,
        }
    } else if source_profile.is_sso_profile() {
        // Recursively resolve SSO profile
        let sso_config = super::aws_sso::resolve_sso_profile(&source_profile, region.clone(), "sts".to_string()).await?;
        AwsCredentials {
            access_key_id: sso_config.access_key_id,
            secret_access_key: sso_config.secret_access_key,
            session_token: sso_config.session_token,
            expiration: None,
        }
    } else {
        return Err(QuicpulseError::Config(format!(
            "Source profile '{}' has no valid credentials",
            source_profile_name
        )));
    };

    // Build AssumeRole request
    let role_session_name = profile.role_session_name.clone()
        .unwrap_or_else(|| format!("quicpulse-{}", std::process::id()));

    let assumed_creds = assume_role(
        &source_creds,
        role_arn,
        &role_session_name,
        profile.external_id.as_deref(),
        profile.duration_seconds,
        &region,
    ).await?;

    Ok(AwsSigV4Config {
        access_key_id: assumed_creds.access_key_id,
        secret_access_key: assumed_creds.secret_access_key,
        session_token: assumed_creds.session_token,
        region,
        service,
    })
}

/// Call STS AssumeRole API
async fn assume_role(
    source_creds: &AwsCredentials,
    role_arn: &str,
    role_session_name: &str,
    external_id: Option<&str>,
    duration_seconds: Option<u32>,
    region: &str,
) -> Result<AwsCredentials, QuicpulseError> {
    use std::time::SystemTime;
    use aws_sigv4::http_request::{sign, SignableBody, SignableRequest, SigningSettings};
    use aws_sigv4::sign::v4;
    use aws_credential_types::Credentials;

    // Build the STS request
    let sts_endpoint = format!("https://sts.{}.amazonaws.com", region);

    // Build query parameters
    let mut params = vec![
        ("Action", "AssumeRole"),
        ("Version", "2011-06-15"),
        ("RoleArn", role_arn),
        ("RoleSessionName", role_session_name),
    ];

    let external_id_owned: String;
    if let Some(ext_id) = external_id {
        external_id_owned = ext_id.to_string();
        params.push(("ExternalId", &external_id_owned));
    }

    let duration_str: String;
    if let Some(duration) = duration_seconds {
        duration_str = duration.to_string();
        params.push(("DurationSeconds", &duration_str));
    }

    // Build query string
    let query_string: String = params.iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let full_url = format!("{}/?{}", sts_endpoint, query_string);

    // Create credentials for signing
    let credentials = Credentials::new(
        &source_creds.access_key_id,
        &source_creds.secret_access_key,
        source_creds.session_token.clone(),
        None,
        "quicpulse-sts",
    );
    let identity = credentials.into();

    // Sign the request
    let settings = SigningSettings::default();
    let signing_params = v4::SigningParams::builder()
        .identity(&identity)
        .region(region)
        .name("sts")
        .time(SystemTime::now())
        .settings(settings)
        .build()
        .map_err(|e| QuicpulseError::Auth(format!("Failed to build STS signing params: {}", e)))?;

    // Parse URL to get path and query
    let parsed_url = url::Url::parse(&full_url)
        .map_err(|e| QuicpulseError::Parse(format!("Invalid STS URL: {}", e)))?;

    let uri = format!(
        "{}{}",
        parsed_url.path(),
        parsed_url.query().map(|q| format!("?{}", q)).unwrap_or_default()
    );

    // Build headers
    let host = parsed_url.host_str().unwrap_or("sts.amazonaws.com");
    let mut header_map = http::HeaderMap::new();
    header_map.insert(
        http::header::HOST,
        http::header::HeaderValue::from_str(host).unwrap(),
    );

    let signable_request = SignableRequest::new(
        "GET",
        &uri,
        header_map.iter().map(|(k, v)| (k.as_str(), v.to_str().unwrap_or(""))),
        SignableBody::empty(),
    ).map_err(|e| QuicpulseError::Auth(format!("Failed to create STS signable request: {}", e)))?;

    let signing_output = sign(signable_request, &signing_params.into())
        .map_err(|e| QuicpulseError::Auth(format!("Failed to sign STS request: {}", e)))?;

    // Build reqwest request with signed headers
    let client = reqwest::Client::new();
    let mut request = client.get(&full_url);

    let (signing_instructions, _) = signing_output.into_parts();
    for (name, value) in signing_instructions.headers() {
        request = request.header(name, value);
    }

    // Make the request
    let response = request.send().await
        .map_err(|e| QuicpulseError::Config(format!("STS AssumeRole request failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body: String = response.text().await.unwrap_or_default();
        return Err(QuicpulseError::Auth(format!(
            "STS AssumeRole failed ({}): {}",
            status, body
        )));
    }

    // Parse XML response
    let body: String = response.text().await
        .map_err(|e| QuicpulseError::Config(format!("Failed to read STS response: {}", e)))?;

    parse_assume_role_response(&body)
}

/// Parse STS AssumeRole XML response
fn parse_assume_role_response(xml: &str) -> Result<AwsCredentials, QuicpulseError> {
    // Simple XML parsing - look for credential fields
    fn extract_tag(xml: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        let start = xml.find(&start_tag)? + start_tag.len();
        let end = xml[start..].find(&end_tag)? + start;

        Some(xml[start..end].to_string())
    }

    let access_key = extract_tag(xml, "AccessKeyId")
        .ok_or_else(|| QuicpulseError::Auth("STS response missing AccessKeyId".to_string()))?;

    let secret_key = extract_tag(xml, "SecretAccessKey")
        .ok_or_else(|| QuicpulseError::Auth("STS response missing SecretAccessKey".to_string()))?;

    let session_token = extract_tag(xml, "SessionToken");
    let expiration = extract_tag(xml, "Expiration");

    Ok(AwsCredentials {
        access_key_id: access_key,
        secret_access_key: secret_key,
        session_token,
        expiration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_assume_role_response() {
        let xml = r#"
        <AssumeRoleResponse>
            <AssumeRoleResult>
                <Credentials>
                    <AccessKeyId>ASIATESTACCESSKEY</AccessKeyId>
                    <SecretAccessKey>testsecretkey123</SecretAccessKey>
                    <SessionToken>testtoken456</SessionToken>
                    <Expiration>2024-01-01T12:00:00Z</Expiration>
                </Credentials>
            </AssumeRoleResult>
        </AssumeRoleResponse>
        "#;

        let creds = parse_assume_role_response(xml).unwrap();
        assert_eq!(creds.access_key_id, "ASIATESTACCESSKEY");
        assert_eq!(creds.secret_access_key, "testsecretkey123");
        assert_eq!(creds.session_token, Some("testtoken456".to_string()));
        assert_eq!(creds.expiration, Some("2024-01-01T12:00:00Z".to_string()));
    }
}
