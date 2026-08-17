//! Unit tests for AWS Signature Version 4 configuration and credential parsing

use quicpulse::auth::aws::AwsSigV4Config;

#[test]
fn test_aws_sigv4_config_from_credentials_two_parts() {
    let creds = "AKIAIOSFODNN7EXAMPLE:wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
    let config =
        AwsSigV4Config::from_credentials(creds, "us-east-1".to_string(), "s3".to_string()).unwrap();

    assert_eq!(config.access_key_id, "AKIAIOSFODNN7EXAMPLE");
    assert_eq!(
        config.secret_access_key,
        "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
    );
    assert_eq!(config.session_token, None);
    assert_eq!(config.region, "us-east-1");
    assert_eq!(config.service, "s3");
}

#[test]
fn test_aws_sigv4_config_from_credentials_three_parts_with_token() {
    let creds = "ASIAIOSFODNN7EXAMPLE:wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY:AQoDYXdzEJr...";
    let config = AwsSigV4Config::from_credentials(
        creds,
        "eu-central-1".to_string(),
        "execute-api".to_string(),
    )
    .unwrap();

    assert_eq!(config.access_key_id, "ASIAIOSFODNN7EXAMPLE");
    assert_eq!(
        config.secret_access_key,
        "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
    );
    assert_eq!(config.session_token, Some("AQoDYXdzEJr...".to_string()));
    assert_eq!(config.region, "eu-central-1");
    assert_eq!(config.service, "execute-api");
}

#[test]
fn test_aws_sigv4_invalid_credentials_format() {
    let invalid = "ONLY_ONE_PART";
    assert!(
        AwsSigV4Config::from_credentials(invalid, "us-east-1".to_string(), "s3".to_string())
            .is_err()
    );
}

#[test]
fn test_aws_sigv4_sign_request_headers() {
    let creds = "AKIAIOSFODNN7EXAMPLE:wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
    let config =
        AwsSigV4Config::from_credentials(creds, "us-east-1".to_string(), "s3".to_string()).unwrap();

    let headers = vec![("Host".to_string(), "s3.amazonaws.com".to_string())];

    let result = quicpulse::auth::aws::sign_request(
        &config,
        "GET",
        "https://s3.amazonaws.com/my-bucket/file.txt",
        &headers,
        None,
        false,
    );
    assert!(result.is_ok());
    let signed_headers = result.unwrap();
    let header_names: Vec<String> = signed_headers
        .into_iter()
        .map(|(k, _)| k.to_lowercase())
        .collect();
    assert!(header_names.contains(&"authorization".to_string()));
    assert!(header_names.contains(&"x-amz-date".to_string()));
}
