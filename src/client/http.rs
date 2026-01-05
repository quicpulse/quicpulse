//! HTTP request building and sending
//!
//! This module provides the core HTTP client functionality using reqwest.

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, COOKIE};
use reqwest::{Client, Method, Response};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

use crate::auth::{AwsSigV4Config, OAuth2Config, get_token, sign_request as aws_sign_request, sha256_hex};
use crate::cli::args::{Args, AuthType};
use crate::cli::parser::ProcessedArgs;
use crate::input::InputItem;
use crate::models::types::RequestType;
use crate::client::ssl::SslConfig;
use crate::context::Environment;
use crate::errors::QuicpulseError;
use crate::graphql;
use crate::middleware::auth::{Auth, DigestAuth, DigestChallenge};
use crate::sessions::Session;
use crate::status::ExitStatus;

pub const USER_AGENT_STRING: &str = concat!("QuicPulse/", env!("CARGO_PKG_VERSION"));

fn create_chunked_stream(data: Vec<u8>) -> reqwest::Body {
    use crate::uploads::chunked::{ChunkedReader, CHUNK_SIZE};
    use std::io::Cursor;

    // Wrap bytes in Cursor to make it readable
    let cursor = Cursor::new(data);
    let chunked_reader = ChunkedReader::with_chunk_size(cursor, CHUNK_SIZE);

    // Convert iterator to async stream
    let stream = futures::stream::iter(chunked_reader);
    reqwest::Body::wrap_stream(stream)
}

/// Serialize JSON to a compact string format
fn json_to_deterministic_format(value: &JsonValue) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Form-style URL encode a string (uses + for spaces)
fn form_urlencode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            ' ' => result.push('+'),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            _ => {
                for b in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    result
}

/// Intermediate response captured during redirect chain (for --all flag)
#[derive(Debug)]
pub struct IntermediateResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HeaderMap,
    /// Request method that was used
    pub method: String,
    /// Request URL
    pub url: String,
}

/// Result of an HTTP request
#[derive(Debug)]
pub struct HttpResult {
    /// The response
    pub response: Response,
    /// Request method
    pub method: String,
    /// Request URL
    pub url: String,
    /// Intermediate responses from redirects (populated when --all is used)
    pub intermediate_responses: Vec<IntermediateResponse>,
}

/// Build and send an HTTP request
pub async fn send_request(
    args: &Args,
    processed: &ProcessedArgs,
    _env: &Environment,
) -> Result<HttpResult, QuicpulseError> {
    send_request_with_session(args, processed, _env, None, None).await
}

/// Build and send an HTTP request with optional session and download headers
pub async fn send_request_with_session(
    args: &Args,
    processed: &ProcessedArgs,
    _env: &Environment,
    session: Option<&Session>,
    download_headers: Option<&HeaderMap>,
) -> Result<HttpResult, QuicpulseError> {
    // Build the client (pass URL for HTTP version configuration)
    let client = build_client(args, &processed.url)?;

    // Parse the method
    let method: Method = processed.method.parse()
        .map_err(|_| QuicpulseError::Parse(format!("Invalid HTTP method: {}", processed.method)))?;

    // Parse the URL
    let mut url = Url::parse(&processed.url)
        .map_err(|e| QuicpulseError::Parse(format!("Invalid URL: {}", e)))?;

    // Extract credentials from URL (user:pass@host) if present and no -a auth specified
    let url_credentials = if args.auth.is_none() && !url.username().is_empty() {
        let username = url.username().to_string();
        let password = url.password().map(|p| p.to_string());
        // Clear credentials from URL so they're not sent in the request
        let _ = url.set_username("");
        let _ = url.set_password(None);
        Some((username, password))
    } else {
        None
    };

    // Handle OAuth2 flows - obtain token before building headers
    let oauth2_token = match args.auth_type {
        Some(AuthType::OAuth2) => {
            // Client Credentials flow
            if let Some(ref auth_str) = args.auth {
                let token_url = args.oauth_token_url.clone().unwrap_or_else(|| {
                    std::env::var("OAUTH_TOKEN_URL").unwrap_or_default()
                });

                if token_url.is_empty() {
                    return Err(QuicpulseError::Auth(
                        "OAuth2 requires --oauth-token-url or OAUTH_TOKEN_URL environment variable".to_string()
                    ));
                }

                let config = OAuth2Config::from_credentials(
                    auth_str,
                    token_url,
                    args.oauth_scopes.clone(),
                )?;

                Some(get_token(&config).await?)
            } else {
                let token_url = args.oauth_token_url.clone().unwrap_or_else(|| {
                    std::env::var("OAUTH_TOKEN_URL").unwrap_or_default()
                });

                if token_url.is_empty() {
                    return Err(QuicpulseError::Auth(
                        "OAuth2 requires credentials (--auth client_id:client_secret) or environment variables".to_string()
                    ));
                }

                let config = OAuth2Config::from_env(token_url, args.oauth_scopes.clone())?;
                Some(get_token(&config).await?)
            }
        }
        Some(AuthType::OAuth2AuthCode) => {
            // Authorization Code flow (with optional PKCE)
            use crate::auth::oauth2_flows::{AuthCodeConfig, authorization_code_flow};

            let auth_url = args.oauth_auth_url.clone()
                .or_else(|| std::env::var("OAUTH_AUTH_URL").ok())
                .ok_or_else(|| QuicpulseError::Auth(
                    "Authorization Code flow requires --oauth-auth-url".to_string()
                ))?;

            let token_url = args.oauth_token_url.clone()
                .or_else(|| std::env::var("OAUTH_TOKEN_URL").ok())
                .ok_or_else(|| QuicpulseError::Auth(
                    "Authorization Code flow requires --oauth-token-url".to_string()
                ))?;

            let client_id = if let Some(ref auth_str) = args.auth {
                auth_str.split(':').next().unwrap_or(auth_str).to_string()
            } else {
                std::env::var("OAUTH_CLIENT_ID").map_err(|_| QuicpulseError::Auth(
                    "Authorization Code flow requires --auth client_id[:client_secret] or OAUTH_CLIENT_ID".to_string()
                ))?
            };

            let client_secret = if let Some(ref auth_str) = args.auth {
                auth_str.split(':').nth(1).map(String::from)
            } else {
                std::env::var("OAUTH_CLIENT_SECRET").ok()
            };

            let redirect_uri = format!("http://localhost:{}/callback", args.oauth_redirect_port);

            let config = AuthCodeConfig {
                client_id,
                client_secret,
                auth_url,
                token_url,
                redirect_uri,
                scopes: args.oauth_scopes.clone(),
                use_pkce: args.oauth_pkce,
            };

            Some(authorization_code_flow(&config).await?)
        }
        Some(AuthType::OAuth2Device) => {
            // Device Authorization flow
            use crate::auth::oauth2_flows::{DeviceFlowConfig, device_flow};

            let device_auth_url = args.oauth_device_url.clone()
                .or_else(|| std::env::var("OAUTH_DEVICE_URL").ok())
                .ok_or_else(|| QuicpulseError::Auth(
                    "Device flow requires --oauth-device-url".to_string()
                ))?;

            let token_url = args.oauth_token_url.clone()
                .or_else(|| std::env::var("OAUTH_TOKEN_URL").ok())
                .ok_or_else(|| QuicpulseError::Auth(
                    "Device flow requires --oauth-token-url".to_string()
                ))?;

            let client_id = if let Some(ref auth_str) = args.auth {
                auth_str.split(':').next().unwrap_or(auth_str).to_string()
            } else {
                std::env::var("OAUTH_CLIENT_ID").map_err(|_| QuicpulseError::Auth(
                    "Device flow requires --auth client_id or OAUTH_CLIENT_ID".to_string()
                ))?
            };

            let config = DeviceFlowConfig {
                client_id,
                device_auth_url,
                token_url,
                scopes: args.oauth_scopes.clone(),
            };

            Some(device_flow(&config).await?)
        }
        _ => None,
    };

    // Build headers (includes session headers)
    let mut headers = build_headers_with_session(args, processed, &url, session)?;

    if let Some((username, password)) = url_credentials {
        let auth_str = if let Some(pass) = password {
            format!("{}:{}", username, pass)
        } else {
            username
        };
        // Use the same auth type specified by --auth-type (defaults to Basic)
        apply_auth(&mut headers, &auth_str, args.auth_type.as_ref())?;
    }

    // Apply OAuth2 token if obtained
    if let Some(ref token) = oauth2_token {
        if let Ok(value) = HeaderValue::try_from(token.authorization_header()) {
            headers.insert(reqwest::header::AUTHORIZATION, value);
        }
    }

    // Add query parameters
    add_query_params(&mut url, &processed.items)?;

    // Build request body - use spawn_blocking to avoid blocking async runtime
    // This is important because build_body does synchronous file I/O for:
    // - Reading files for @path syntax
    // - Reading files for multipart uploads
    // - Reading files for header values
    let args_clone = args.clone();
    let processed_clone = processed.clone();
    let body = tokio::task::spawn_blocking(move || {
        build_body(&args_clone, &processed_clone)
    })
    .await
    .map_err(|e| QuicpulseError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??;

    let is_multipart = matches!(body.as_ref(), Some(RequestBody::Multipart(_)));
    let use_aws_sigv4 = matches!(args.auth_type, Some(AuthType::AwsSigv4));

    // Prepare body bytes and track if compressed (for non-multipart bodies)
    let (final_body_bytes, body_was_compressed): (Option<Vec<u8>>, bool) = if is_multipart {
        (None, false) // Multipart uses UNSIGNED-PAYLOAD
    } else if let Some(ref b) = body {
        let raw_bytes = match b {
            RequestBody::Json(j) => json_to_deterministic_format(j).into_bytes(),
            RequestBody::Form(f) => serde_urlencoded::to_string(f).unwrap_or_default().into_bytes(),
            RequestBody::Raw(r) => r.as_bytes().to_vec(),
            RequestBody::Multipart(_) => unreachable!(),
        };

        // Apply compression if requested
        if args.compress > 0 {
            let (compressed, was_compressed) = crate::uploads::compress_request(
                &raw_bytes,
                args.compress > 1, // Force compression with -xx
            )?;
            if was_compressed {
                (Some(compressed), true)
            } else {
                (Some(raw_bytes), false)
            }
        } else {
            (Some(raw_bytes), false)
        }
    } else {
        (None, false)
    };

    // Build AWS config outside the signing block so we can reuse it for redirects
    // AWS SigV4 signatures are URL-specific, so we must re-sign after each redirect
    let aws_config = if use_aws_sigv4 {
        let region = args.aws_region.clone();

        let service = args.aws_service.clone().unwrap_or_else(|| {
            infer_aws_service(&url).unwrap_or_else(|| "execute-api".to_string())
        });

        // AWS credential resolution priority:
        // 1. --auth flag (explicit credentials: ACCESS_KEY:SECRET_KEY[:SESSION_TOKEN])
        // 2. --aws-profile flag (named profile from ~/.aws/credentials)
        // 3. AWS_PROFILE env var (default profile name)
        // 4. Environment variables (AWS_ACCESS_KEY_ID, etc.)
        let config = if let Some(ref auth_str) = args.auth {
            // Explicit credentials take highest priority
            let region = region.unwrap_or_else(|| {
                std::env::var("AWS_REGION")
                    .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
                    .unwrap_or_else(|_| "us-east-1".to_string())
            });
            AwsSigV4Config::from_credentials(auth_str, region, service)?
        } else if let Some(ref profile_name) = args.aws_profile {
            // Named profile from --aws-profile flag
            AwsSigV4Config::from_profile(profile_name, region, service).await?
        } else if let Ok(profile_name) = std::env::var("AWS_PROFILE") {
            // AWS_PROFILE env var specifies default profile name
            AwsSigV4Config::from_profile(&profile_name, region, service).await?
        } else {
            // Fall back to environment variables
            let region = region.unwrap_or_else(|| {
                std::env::var("AWS_REGION")
                    .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
                    .unwrap_or_else(|_| "us-east-1".to_string())
            });
            AwsSigV4Config::from_env(region, service)?
        };
        Some(config)
    } else {
        None
    };

    // Handle AWS SigV4 signing - must be done on FINAL body bytes (after compression)
    let aws_sig_headers = if let Some(ref config) = aws_config {
        // Collect current headers for signing (include Content-Encoding if compressed)
        let mut current_headers: Vec<(String, String)> = headers.iter()
            .filter_map(|(name, value)| {
                value.to_str().ok().map(|v| (name.to_string(), v.to_string()))
            })
            .collect();

        // Bug #1 fix: Include Content-Encoding header in signature if body was compressed
        if body_was_compressed {
            current_headers.push(("content-encoding".to_string(), "deflate".to_string()));
        }

        // Sign the request with FINAL body bytes (use unsigned payload for multipart uploads)
        let mut sig_headers = aws_sign_request(
            config,
            &method.to_string(),
            url.as_str(),
            &current_headers,
            final_body_bytes.as_deref(),
            is_multipart, // unsigned_payload flag
        )?;

        // Add content hash header for S3 compatibility
        // For multipart, use UNSIGNED-PAYLOAD as the content hash
        let content_hash = if is_multipart {
            "UNSIGNED-PAYLOAD".to_string()
        } else if let Some(ref bytes) = final_body_bytes {
            sha256_hex(bytes)
        } else {
            sha256_hex(&[])
        };
        sig_headers.push(("x-amz-content-sha256".to_string(), content_hash));

        Some(sig_headers)
    } else {
        None
    };

    let has_content_type = headers.contains_key(CONTENT_TYPE);
    let original_content_type = headers.get(CONTENT_TYPE).cloned();

    // Build the request
    let mut request_builder = client.request(method.clone(), url.clone());
    request_builder = request_builder.headers(headers);

    // Set HTTP/3 version if requested
    if args.http3 || args.http_version.as_deref() == Some("3") {
        if !processed.url.starts_with("http://") {
            request_builder = request_builder.version(http::Version::HTTP_3);
        }
    }

    // Add AWS SigV4 headers if present
    if let Some(sig_headers) = aws_sig_headers {
        for (name, value) in sig_headers {
            if let (Ok(header_name), Ok(header_value)) = (
                HeaderName::try_from(name.as_str()),
                HeaderValue::try_from(value.as_str()),
            ) {
                request_builder = request_builder.header(header_name, header_value);
            }
        }
    }

    // Add download headers if present (for Range header, Accept-Encoding, etc.)
    if let Some(dl_headers) = download_headers {
        for (key, value) in dl_headers.iter() {
            request_builder = request_builder.header(key, value);
        }
    }

    // Add body if present
    // Bug #1 fix: Use pre-computed final_body_bytes (already compressed if needed)
    // This ensures the same bytes that were signed are sent
    if let Some(body_content) = body {
        match body_content {
            RequestBody::Json(_) => {
                // Body bytes already computed above (with compression if applicable)
                if let Some(bytes) = final_body_bytes.clone() {
                    if body_was_compressed {
                        request_builder = request_builder
                            .header(CONTENT_TYPE, "application/json")
                            .header("Content-Encoding", "deflate")
                            .body(bytes);
                    } else if args.chunked {
                        if !has_content_type {
                            request_builder = request_builder.header(CONTENT_TYPE, "application/json");
                        }
                        request_builder = request_builder
                            .header("Transfer-Encoding", "chunked")
                            .body(create_chunked_stream(bytes));
                    } else {
                        request_builder = request_builder
                            .header(CONTENT_TYPE, "application/json")
                            .body(bytes);
                    }
                }
            }
            RequestBody::Form(form) => {
                // Body bytes already computed above (with compression if applicable)
                // IMPORTANT: Always use pre-calculated bytes when available to ensure
                // byte-for-byte consistency required for AWS SigV4 signing.
                // Re-serializing with .form() could produce different ordering.
                if let Some(bytes) = final_body_bytes.clone() {
                    if body_was_compressed {
                        request_builder = request_builder
                            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                            .header("Content-Encoding", "deflate")
                            .body(bytes);
                    } else {
                        // Use the exact bytes we calculated and signed
                        request_builder = request_builder
                            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                            .body(bytes);
                    }
                } else {
                    // Fallback: let reqwest serialize (only if no body was computed)
                    request_builder = request_builder.form(&form);
                }
            }
            RequestBody::Raw(_) => {
                // Body bytes already computed above (with compression if applicable)
                if let Some(bytes) = final_body_bytes.clone() {
                    if body_was_compressed {
                        request_builder = request_builder
                            .header("Content-Encoding", "deflate")
                            .body(bytes);
                    } else if args.chunked {
                        request_builder = request_builder
                            .header("Transfer-Encoding", "chunked")
                            .body(create_chunked_stream(bytes));
                    } else {
                        request_builder = request_builder.body(bytes);
                    }
                }
            }
            RequestBody::Multipart(form) => {
                // Multipart doesn't support compression in the same way
                request_builder = request_builder.multipart(form);
            }
        }
    }

    // Send the request
    let mut response = request_builder.send().await
        .map_err(QuicpulseError::Request)?;

    // Handle Digest auth challenge-response (401 retry)
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        if matches!(args.auth_type, Some(AuthType::Digest)) {
            if let Some(www_auth) = response.headers().get(reqwest::header::WWW_AUTHENTICATE) {
                if let Ok(www_auth_str) = www_auth.to_str() {
                    // Check if it's a Digest challenge
                    if www_auth_str.to_lowercase().starts_with("digest ") {
                        if let Ok(challenge) = DigestChallenge::parse(www_auth_str) {
                            // Get credentials
                            if let Some(ref auth_secret) = args.auth {
                                let auth_str = auth_secret.as_str();
                                let digest_auth = DigestAuth::from_credentials(auth_str)
                                    .map_err(|e| QuicpulseError::Auth(e.to_string()))?;

                                // Get URI path for digest response
                                let uri_path = url.path();
                                let uri_with_query = if let Some(query) = url.query() {
                                    format!("{}?{}", uri_path, query)
                                } else {
                                    uri_path.to_string()
                                };

                                // Generate Authorization header
                                let auth_header = digest_auth.respond_to_challenge(
                                    &challenge,
                                    &method.to_string(),
                                    &uri_with_query,
                                ).map_err(|e| QuicpulseError::Auth(e.to_string()))?;

                                // Rebuild headers with Authorization
                                let mut retry_headers = build_headers_with_session(args, processed, &url, session)?;
                                retry_headers.insert(
                                    reqwest::header::AUTHORIZATION,
                                    HeaderValue::try_from(auth_header)
                                        .map_err(|e| QuicpulseError::Parse(format!("Invalid auth header: {}", e)))?,
                                );

                                // Rebuild request with auth header
                                let mut retry_builder = client.request(method.clone(), url.clone())
                                    .headers(retry_headers);

                                // Set HTTP version if specified
                                if args.http3 || args.http_version.as_deref() == Some("3") {
                                    if !processed.url.starts_with("http://") {
                                        retry_builder = retry_builder.version(http::Version::HTTP_3);
                                    }
                                }

                                // Add download headers if present
                                if let Some(dl_headers) = download_headers {
                                    for (key, value) in dl_headers.iter() {
                                        retry_builder = retry_builder.header(key, value);
                                    }
                                }

                                // Add body if present (use same body bytes)
                                if let Some(ref body_bytes) = final_body_bytes {
                                    if body_was_compressed {
                                        retry_builder = retry_builder
                                            .header("Content-Encoding", "deflate")
                                            .body(body_bytes.clone());
                                    } else {
                                        retry_builder = retry_builder.body(body_bytes.clone());
                                    }
                                    // Restore Content-Type
                                    if let Some(ref ct) = original_content_type {
                                        retry_builder = retry_builder.header(CONTENT_TYPE, ct.clone());
                                    }
                                }

                                // Send retry request
                                response = retry_builder.send().await
                                    .map_err(QuicpulseError::Request)?;
                            }
                        }
                    }
                }
            }
        }
    }

    // Manually handle redirects when:
    // 1. --all is specified (to capture intermediate responses)
    // 2. Using AWS SigV4 (must re-sign for each redirect URL)
    let mut intermediate_responses = Vec::new();
    let handle_redirects_manually = (args.all && args.follow) || (use_aws_sigv4 && args.follow);
    
    if handle_redirects_manually {
        let mut redirect_count = 0;
        let mut current_url = url.clone();
        let mut current_method = method.clone();

        while response.status().is_redirection() && redirect_count < args.max_redirects as usize {
            // Capture intermediate response
            let status = response.status().as_u16();
            let resp_headers = response.headers().clone();

            // Get the Location header for the next request
            let location = match response.headers().get(reqwest::header::LOCATION) {
                Some(loc) => loc.to_str().unwrap_or("").to_string(),
                None => break, // No Location header, stop redirecting
            };

            // Store intermediate response (only if --all is set)
            if args.all {
                intermediate_responses.push(IntermediateResponse {
                    status,
                    headers: resp_headers,
                    method: current_method.to_string(),
                    url: current_url.to_string(),
                });
            }

            // Resolve relative URL against current URL
            let next_url = current_url.join(&location)
                .map_err(|e| QuicpulseError::Parse(format!("Invalid redirect URL '{}': {}", location, e)))?;

            // HTTP spec: POST -> GET on 301/302/303 redirects (except 307/308 which preserve method)
            let next_method = match response.status().as_u16() {
                301 | 302 | 303 => {
                    if current_method == Method::POST {
                        Method::GET
                    } else {
                        current_method.clone()
                    }
                }
                _ => current_method.clone(),
            };

            // Build new request for redirect
            let mut redirect_request = client.request(next_method.clone(), next_url.clone())
                .header(reqwest::header::USER_AGENT, USER_AGENT_STRING);

            let status_code = response.status().as_u16();
            
            // For 307/308, preserve body
            let redirect_body_bytes = if (status_code == 307 || status_code == 308) && final_body_bytes.is_some() {
                if let Some(ref body_bytes) = final_body_bytes {
                    redirect_request = redirect_request.body(body_bytes.clone());
                    if let Some(ref ct) = original_content_type {
                        redirect_request = redirect_request.header(CONTENT_TYPE, ct.clone());
                    }
                }
                final_body_bytes.as_deref()
            } else {
                // No body for GET after 301/302/303
                None
            };

            // AWS SigV4: Re-sign the request for the new URL
            // The signature is calculated for a specific URL, so we MUST re-sign after redirect
            if let Some(ref config) = aws_config {
                let redirect_sig_headers = aws_sign_request(
                    config,
                    &next_method.to_string(),
                    next_url.as_str(),
                    &[], // No custom headers needed for redirect
                    redirect_body_bytes,
                    is_multipart,
                )?;

                for (name, value) in redirect_sig_headers {
                    if let (Ok(header_name), Ok(header_value)) = (
                        HeaderName::try_from(name.as_str()),
                        HeaderValue::try_from(value.as_str()),
                    ) {
                        redirect_request = redirect_request.header(header_name, header_value);
                    }
                }

                // Add content hash header
                let content_hash = if is_multipart {
                    "UNSIGNED-PAYLOAD".to_string()
                } else if let Some(bytes) = redirect_body_bytes {
                    sha256_hex(bytes)
                } else {
                    sha256_hex(&[])
                };
                redirect_request = redirect_request.header("x-amz-content-sha256", content_hash);
            }

            if let Some(session_ref) = session {
                let domain = next_url.host_str().unwrap_or("");
                let path = next_url.path();
                let is_secure = next_url.scheme() == "https";
                if let Some(cookie_header) = session_ref.get_cookie_header(domain, path, is_secure) {
                    if !cookie_header.is_empty() {
                        redirect_request = redirect_request.header(reqwest::header::COOKIE, cookie_header);
                    }
                }
            }

            // Send redirect request
            response = redirect_request.send().await
                .map_err(QuicpulseError::Request)?;

            current_url = next_url;
            current_method = next_method;
            redirect_count += 1;
        }
    }

    Ok(HttpResult {
        response,
        method: method.to_string(),
        url: url.to_string(),
        intermediate_responses,
    })
}

/// Infer AWS service name from URL
fn infer_aws_service(url: &Url) -> Option<String> {
    let host = url.host_str()?;

    // Common patterns: servicename.region.amazonaws.com
    // or servicename-region.amazonaws.com
    if host.ends_with(".amazonaws.com") {
        let parts: Vec<&str> = host.trim_end_matches(".amazonaws.com").split('.').collect();
        if !parts.is_empty() {
            let service = parts[0];
            // Handle some common service names
            return Some(match service {
                "s3" | "s3-accelerate" => "s3",
                "execute-api" => "execute-api",
                "lambda" => "lambda",
                "dynamodb" => "dynamodb",
                "sqs" => "sqs",
                "sns" => "sns",
                "sts" => "sts",
                "iam" => "iam",
                "ec2" => "ec2",
                "rds" => "rds",
                "secretsmanager" => "secretsmanager",
                "ssm" => "ssm",
                "kinesis" => "kinesis",
                "firehose" => "firehose",
                "logs" => "logs",
                "events" => "events",
                "apigateway" => "apigateway",
                _ => service,
            }.to_string());
        }
    }

    // For API Gateway custom domains, default to execute-api
    if host.contains("execute-api") {
        return Some("execute-api".to_string());
    }

    None
}

/// Build the HTTP client with appropriate configuration
fn build_client(args: &Args, url: &str) -> Result<Client, QuicpulseError> {
    let mut builder = Client::builder()
        .user_agent(USER_AGENT_STRING);

    // Set timeout
    if let Some(timeout) = args.timeout {
        builder = builder.timeout(Duration::from_secs_f64(timeout));
    }

    // Configure HTTP version (needs URL to determine if http:// or https://)
    builder = configure_http_version(builder, args, url);

    // Build and apply SSL configuration
    let ssl_config = SslConfig::from_args(
        &args.verify,
        args.ssl.as_deref(),
        args.ciphers.as_deref(),
        args.cert.as_ref().and_then(|p| p.to_str()),
        args.cert_key.as_ref().and_then(|p| p.to_str()),
        args.cert_key_pass.as_deref(),
    );
    builder = ssl_config.apply_to_builder(builder)?;

    // Handle redirects
    // When --all is used, we disable automatic redirects to capture intermediate responses
    // and manually follow redirects in send_request_with_session
    if args.all {
        // Always disable auto-redirects when --all is specified so we can capture intermediates
        builder = builder.redirect(reqwest::redirect::Policy::none());
    } else if args.follow {
        builder = builder.redirect(reqwest::redirect::Policy::limited(args.max_redirects as usize));
    } else {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }

    // Disable automatic Referer header on redirects
    builder = builder.referer(false);

    // Add SOCKS proxy if specified via --socks
    if let Some(ref socks_proxy) = args.socks_proxy {
        let socks_url = socks_proxy.to_string();
        // Ensure URL has a protocol
        let socks_url = if !socks_url.contains("://") {
            format!("socks5://{}", socks_url)
        } else {
            socks_url
        };
        if let Ok(p) = reqwest::Proxy::all(&socks_url) {
            builder = builder.proxy(p);
        }
    }

    // Add proxy if specified
    for proxy_str in &args.proxy {
        // Parse proxy format: protocol:url (e.g., "http:http://proxy.example.com:8080")
        // Or SOCKS format: socks4://host:port, socks5://host:port
        let proxy_str_lower = proxy_str.to_lowercase();

        // Handle SOCKS proxies directly with socks4://, socks4a://, socks5://, socks5h:// URLs
        if proxy_str_lower.starts_with("socks4://")
            || proxy_str_lower.starts_with("socks4a://")
            || proxy_str_lower.starts_with("socks5://")
            || proxy_str_lower.starts_with("socks5h://")
        {
            // SOCKS proxy - use as-is for all traffic
            if let Ok(p) = reqwest::Proxy::all(proxy_str.as_str()) {
                builder = builder.proxy(p);
            }
            continue;
        }

        if let Some((protocol, url)) = proxy_str.split_once(':') {
            // Handle case where URL contains "://"
            let url = if url.starts_with("//") {
                format!("{}:{}", protocol, url)
            } else {
                url.to_string()
            };

            let proxy = match protocol.to_lowercase().as_str() {
                "http" => reqwest::Proxy::http(&url),
                "https" => reqwest::Proxy::https(&url),
                "all" => reqwest::Proxy::all(&url),
                "socks4" | "socks4a" | "socks5" | "socks5h" => {
                    // Build SOCKS URL: socks5://host:port
                    let socks_url = format!("{}://{}", protocol.to_lowercase(), url.trim_start_matches("//"));
                    reqwest::Proxy::all(&socks_url)
                }
                _ => continue,
            };
            if let Ok(p) = proxy {
                builder = builder.proxy(p);
            }
        }
    }

    // Apply custom DNS resolution (--resolve)
    for resolve_entry in &args.resolve {
        // Format: HOST:PORT:ADDRESS
        if let Some((host_port, address)) = resolve_entry.rsplit_once(':') {
            if let Some((host, port_str)) = host_port.rsplit_once(':') {
                if let Ok(port) = port_str.parse::<u16>() {
                    if let Ok(addr) = address.parse::<std::net::IpAddr>() {
                        let socket_addr = std::net::SocketAddr::new(addr, port);
                        builder = builder.resolve(host, socket_addr);
                    }
                }
            }
        }
    }

    // Apply local address binding (--local-address or --interface as IP)
    if let Some(ref addr_str) = args.local_address.as_ref().or(args.interface.as_ref()) {
        if let Ok(addr) = addr_str.parse::<std::net::IpAddr>() {
            builder = builder.local_address(addr);
        }
    }

    // TCP Fast Open is typically enabled at the OS level
    // Reqwest doesn't have direct TFO support, but we can document the flag
    if args.tcp_fastopen {
        // Note: TCP Fast Open requires OS-level support and configuration
        // On Linux: echo 3 > /proc/sys/net/ipv4/tcp_fastopen
        // On macOS: sysctl -w net.inet.tcp.fastopen=3
        // The application just needs to enable it in socket options
        // reqwest doesn't expose this directly, but the underlying hyper/tokio may support it
    }

    // Local port binding is not directly supported by reqwest
    // Would require custom connector implementation
    if args.local_port.is_some() {
        // Note: Local port range binding requires a custom connector
        // This feature would need socket-level access before connection
    }

    builder.build().map_err(QuicpulseError::Request)
}

/// Configure HTTP version based on args
fn configure_http_version(mut builder: reqwest::ClientBuilder, args: &Args, url: &str) -> reqwest::ClientBuilder {
    // Check URL scheme - http2_prior_knowledge() only works with http://, not https://
    // For https://, HTTP/2 is negotiated via ALPN during TLS handshake
    let is_plain_http = url.starts_with("http://");

    // Check for explicit HTTP/3 flag
    if args.http3 {
        if is_plain_http {
            // HTTP/3 requires HTTPS (runs over QUIC which requires TLS)
            eprintln!("Warning: HTTP/3 requires HTTPS. Falling back to HTTP/1.1 for plaintext connection.");
        } else {
            // Enable HTTP/3 with QUIC transport
            // This uses the http3 feature which requires RUSTFLAGS='--cfg reqwest_unstable'
            builder = builder.http3_prior_knowledge();
            return builder;
        }
    }

    // Check for explicit version specification
    if let Some(ref version) = args.http_version {
        match version.as_str() {
            "1" | "1.0" | "1.1" => {
                builder = builder.http1_only();
            }
            "2" => {
                // http2_prior_knowledge() sends HTTP/2 preface immediately without TLS negotiation
                // This only works for http:// (h2c - HTTP/2 cleartext)
                // For https://, HTTP/2 is negotiated via ALPN - don't use prior_knowledge
                if is_plain_http {
                    builder = builder.http2_prior_knowledge();
                }
                // For https://, reqwest will automatically use HTTP/2 via ALPN if server supports it
            }
            "3" => {
                if is_plain_http {
                    // HTTP/3 requires HTTPS (runs over QUIC which requires TLS)
                    eprintln!("Warning: HTTP/3 requires HTTPS. Falling back to HTTP/1.1 for plaintext connection.");
                } else {
                    // Enable HTTP/3 with QUIC transport
                    builder = builder.http3_prior_knowledge();
                }
            }
            _ => {
                // Default behavior - let reqwest negotiate
            }
        }
    }

    builder
}

/// Build headers for the request (without session)
fn build_headers(args: &Args, processed: &ProcessedArgs) -> Result<HeaderMap, QuicpulseError> {
    build_headers_with_session(args, processed, &Url::parse("http://localhost").unwrap(), None)
}

/// Build headers for the request with optional session support
fn build_headers_with_session(
    args: &Args,
    processed: &ProcessedArgs,
    url: &Url,
    session: Option<&Session>,
) -> Result<HeaderMap, QuicpulseError> {
    let mut headers = HeaderMap::new();

    // 1. First, add session headers (lowest priority - can be overridden)
    if let Some(sess) = session {
        for header in &sess.headers {
            if let (Ok(name), Ok(value)) = (
                HeaderName::try_from(header.name.as_str()),
                HeaderValue::try_from(header.value.as_str()),
            ) {
                headers.insert(name, value);
            }
        }

        // Add session cookies
        let domain = url.host_str().unwrap_or("localhost");
        let path = url.path();
        let is_secure = url.scheme() == "https";

        if let Some(cookie_header) = sess.get_cookie_header(domain, path, is_secure) {
            if let Ok(value) = HeaderValue::try_from(cookie_header) {
                headers.insert(COOKIE, value);
            }
        }

        // Apply session auth if no CLI auth specified
        if args.auth.is_none() {
            if let Some(ref auth) = sess.auth {
                apply_session_auth(&mut headers, auth);
            }
        }
    }

    // 2. Apply authentication from CLI args
    if let Some(ref auth_str) = args.auth {
        apply_auth(&mut headers, auth_str, args.auth_type.as_ref())?;
    }

    // 3. Set default Accept header for JSON mode
    if processed.has_data && !args.form && !args.multipart {
        headers.insert(ACCEPT, HeaderValue::from_static("application/json, */*;q=0.5"));
    }

    // 4. Set Content-Type based on mode
    if processed.has_data {
        match processed.request_type {
            RequestType::Json => {
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            }
            RequestType::Form => {
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/x-www-form-urlencoded; charset=utf-8"));
            }
            RequestType::Multipart => {
                // Content-Type is set automatically by reqwest for multipart
            }
        }
    }

    // 4.5. Add Content-Length: 0 for methods without body
    // We send Content-Length: 0 for all methods except GET/HEAD/OPTIONS
    let method_upper = processed.method.to_uppercase();
    let needs_content_length_zero = !matches!(method_upper.as_str(), "GET" | "HEAD" | "OPTIONS");
    if needs_content_length_zero && !processed.has_data && args.raw.is_none() {
        headers.insert(CONTENT_LENGTH, HeaderValue::from_static("0"));
    }

    // 5. Add headers from request items (highest priority - override everything)
    for item in &processed.items {
        match item {
            InputItem::Header { name, value } => {
                let header_name = HeaderName::try_from(name.as_str())
                    .map_err(|e| QuicpulseError::Parse(format!("Invalid header name '{}': {}", name, e)))?;
                let header_value = HeaderValue::try_from(value.as_str())
                    .map_err(|e| QuicpulseError::Parse(format!("Invalid header value '{}': {}", value, e)))?;
                // Bug #3 fix: Use append instead of insert to allow multiple headers with same name
                // HTTP allows multiple headers with the same name (e.g., Set-Cookie, Cache-Control)
                headers.append(header_name, header_value);
            }
            InputItem::EmptyHeader { name } => {
                let header_name = HeaderName::try_from(name.as_str())
                    .map_err(|e| QuicpulseError::Parse(format!("Invalid header name '{}': {}", name, e)))?;
                headers.append(header_name, HeaderValue::from_static(""));
            }
            InputItem::HeaderFile { name, path } => {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| QuicpulseError::Io(e))?;
                let header_name = HeaderName::try_from(name.as_str())
                    .map_err(|e| QuicpulseError::Parse(format!("Invalid header name '{}': {}", name, e)))?;
                let header_value = HeaderValue::try_from(content.trim())
                    .map_err(|e| QuicpulseError::Parse(format!("Invalid header value: {}", e)))?;
                headers.append(header_name, header_value);
            }
            _ => {}
        }
    }

    Ok(headers)
}

/// Apply authentication to headers based on auth string and type
/// Uses enum-based middleware for type-safe authentication
fn apply_auth(
    headers: &mut HeaderMap,
    auth_str: &str,
    auth_type: Option<&AuthType>,
) -> Result<(), QuicpulseError> {
    let auth_type = auth_type.cloned().unwrap_or(AuthType::Basic);

    // AWS SigV4, GCP, Azure, and OAuth2 flows are handled separately in send_request_with_session
    if matches!(auth_type, AuthType::AwsSigv4 | AuthType::Gcp | AuthType::Azure | AuthType::OAuth2 | AuthType::OAuth2AuthCode | AuthType::OAuth2Device) {
        return Ok(());
    }

    // Create appropriate auth middleware based on type
    let auth = match auth_type {
        AuthType::Basic => {
            let (username, password) = parse_auth_credentials(auth_str);
            Auth::basic(username, password.unwrap_or_default())
        }
        AuthType::Digest => {
            let (username, password) = parse_auth_credentials(auth_str);
            Auth::digest(username, password.unwrap_or_default())
        }
        AuthType::Bearer => {
            Auth::bearer(auth_str)
        }
        AuthType::Ntlm => {
            let (username, password) = parse_auth_credentials(auth_str);
            Auth::ntlm(username, password.unwrap_or_default())
        }
        AuthType::Negotiate => {
            let (username, password) = parse_auth_credentials(auth_str);
            Auth::negotiate(username, password.unwrap_or_default())
        }
        AuthType::Kerberos => {
            let (username, password) = parse_auth_credentials(auth_str);
            Auth::kerberos(username, password.unwrap_or_default())
        }
        AuthType::AwsSigv4 | AuthType::Gcp | AuthType::Azure | AuthType::OAuth2 | AuthType::OAuth2AuthCode | AuthType::OAuth2Device => unreachable!(),
    };

    // Apply authentication to headers
    auth.apply(headers).map_err(|e| QuicpulseError::Auth(e.to_string()))
}

/// Apply session authentication to headers
fn apply_session_auth(headers: &mut HeaderMap, session_auth: &crate::sessions::SessionAuth) {
    let auth = match session_auth.auth_type.as_str() {
        "basic" => {
            let (username, password) = parse_auth_credentials(&session_auth.credentials);
            Auth::basic(username, password.unwrap_or_default())
        }
        "bearer" => {
            Auth::bearer(&session_auth.credentials)
        }
        "ntlm" => {
            let (username, password) = parse_auth_credentials(&session_auth.credentials);
            Auth::ntlm(username, password.unwrap_or_default())
        }
        "negotiate" => {
            let (username, password) = parse_auth_credentials(&session_auth.credentials);
            Auth::negotiate(username, password.unwrap_or_default())
        }
        "kerberos" => {
            let (username, password) = parse_auth_credentials(&session_auth.credentials);
            Auth::kerberos(username, password.unwrap_or_default())
        }
        _ => {
            // Unknown auth type, default to basic
            let (username, password) = parse_auth_credentials(&session_auth.credentials);
            Auth::basic(username, password.unwrap_or_default())
        }
    };

    let _ = auth.apply(headers);
}

/// Parse authentication credentials from "user:password" or "token" format
fn parse_auth_credentials(auth_str: &str) -> (String, Option<String>) {
    if let Some((user, pass)) = auth_str.split_once(':') {
        (user.to_string(), Some(pass.to_string()))
    } else {
        // No colon - treat as username without password
        (auth_str.to_string(), None)
    }
}

/// Add query parameters to the URL (using form-style encoding with + for spaces)
fn add_query_params(url: &mut Url, items: &[InputItem]) -> Result<(), QuicpulseError> {
    let mut params: Vec<(String, String)> = Vec::new();

    // Collect existing query params (if any)
    if let Some(existing) = url.query() {
        if !existing.is_empty() {
            for pair in existing.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    params.push((k.to_string(), v.to_string()));
                } else if !pair.is_empty() {
                    params.push((pair.to_string(), String::new()));
                }
            }
        }
    }

    for item in items {
        match item {
            InputItem::QueryParam { name, value } => {
                params.push((name.clone(), value.clone()));
            }
            InputItem::QueryParamFile { name, path } => {
                let value = std::fs::read_to_string(path)
                    .map_err(|e| QuicpulseError::Io(e))?
                    .trim()
                    .to_string();
                params.push((name.clone(), value));
            }
            _ => {}
        }
    }

    if params.is_empty() {
        url.set_query(None);
    } else {
        // Build query string with form-style encoding (+ for spaces)
        let query_string: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", form_urlencode(k), form_urlencode(v)))
            .collect::<Vec<_>>()
            .join("&");
        url.set_query(Some(&query_string));
    }

    Ok(())
}

/// Request body types
pub enum RequestBody {
    Json(JsonValue),
    /// Form data as Vec to preserve order and allow duplicate keys
    /// (e.g., id=1&id=2). Order preservation is critical for AWS SigV4 signing.
    Form(Vec<(String, String)>),
    Raw(String),
    Multipart(reqwest::multipart::Form),
}

impl std::fmt::Debug for RequestBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestBody::Json(v) => f.debug_tuple("Json").field(v).finish(),
            RequestBody::Form(v) => f.debug_tuple("Form").field(v).finish(),
            RequestBody::Raw(v) => f.debug_tuple("Raw").field(v).finish(),
            RequestBody::Multipart(_) => f.debug_tuple("Multipart").field(&"<form>").finish(),
        }
    }
}

/// Build the request body
fn build_body(args: &Args, processed: &ProcessedArgs) -> Result<Option<RequestBody>, QuicpulseError> {
    use reqwest::multipart::{Form, Part};
    use std::io::Read;

    // Check for raw body
    if let Some(raw) = &args.raw {
        return Ok(Some(RequestBody::Raw(raw.clone())));
    }

    // Collect data items (excluding file uploads)
    let data_items: Vec<_> = processed.items.iter()
        .filter(|item| matches!(item, InputItem::DataField { .. } | InputItem::DataFieldFile { .. } | InputItem::JsonField { .. } | InputItem::JsonFieldFile { .. }))
        .collect();

    // Collect file uploads
    let file_items: Vec<_> = processed.items.iter()
        .filter(|item| matches!(item, InputItem::FileUpload { .. }))
        .collect();

    // If there are file uploads, force multipart mode
    let use_multipart = processed.request_type == RequestType::Multipart || !file_items.is_empty();

    // Handle GraphQL schema introspection without any data items
    if args.graphql_schema {
        let schema_query = graphql::build_schema_request();
        return Ok(Some(RequestBody::Json(schema_query)));
    }

    // For GraphQL with --graphql-query but no other data
    if data_items.is_empty() && file_items.is_empty() {
        if args.graphql_query.is_some() || args.graphql {
            // Build GraphQL body from query flag only
            let json_obj = json!({});
            let graphql_body = graphql::build_graphql_body(args, &json_obj)?;
            return Ok(Some(RequestBody::Json(graphql_body)));
        }
        return Ok(None);
    }

    if use_multipart {
        // Build multipart form
        let mut form = Form::new();

        // Add data fields
        for item in &data_items {
            let (key, value) = get_data_key_value(item)?;
            form = form.text(key, value);
        }

        // Add file uploads
        for item in &file_items {
            if let InputItem::FileUpload { field, path, mime_type, filename: fname } = item {
                // Read file contents
                let mut file = std::fs::File::open(path)
                    .map_err(|e| QuicpulseError::Io(e))?;
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)
                    .map_err(|e| QuicpulseError::Io(e))?;

                // Determine filename
                let filename = fname.clone()
                    .or_else(|| {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_else(|| "file".to_string());

                // Create part
                let mut part = Part::bytes(contents).file_name(filename);

                // Set content type if specified or guess from extension
                if let Some(ref content_type) = mime_type {
                    part = part.mime_str(content_type)
                        .map_err(|e| QuicpulseError::Parse(format!("Invalid MIME type: {}", e)))?;
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let mime = guess_mime_type(ext);
                    part = part.mime_str(mime)
                        .map_err(|e| QuicpulseError::Parse(format!("Invalid MIME type: {}", e)))?;
                }

                form = form.part(field.clone(), part);
            }
        }

        Ok(Some(RequestBody::Multipart(form)))
    } else {
        match processed.request_type {
            RequestType::Json => {
                let mut json_obj = json!({});

                for item in data_items {
                    let (key, json_value) = match item {
                        InputItem::DataField { key, value } => {
                            (key.clone(), JsonValue::String(value.clone()))
                        }
                        InputItem::DataFieldFile { key, path } => {
                            let content = std::fs::read_to_string(path)
                                .map_err(|e| QuicpulseError::Io(e))?;
                            (key.clone(), JsonValue::String(content.trim().to_string()))
                        }
                        InputItem::JsonField { key, value } => {
                            (key.clone(), value.clone())
                        }
                        InputItem::JsonFieldFile { key, path } => {
                            let content = std::fs::read_to_string(path)
                                .map_err(|e| QuicpulseError::Io(e))?;
                            let json_val = serde_json::from_str(&content)
                                .map_err(|e| QuicpulseError::Json(e))?;
                            (key.clone(), json_val)
                        }
                        _ => continue,
                    };

                    // Handle nested keys (e.g., user[name])
                    set_nested_value(&mut json_obj, &key, json_value)?;
                }

                // Wrap as GraphQL request if --graphql flag is set
                let final_json = if graphql::is_graphql_request(args) {
                    graphql::build_graphql_body(args, &json_obj)?
                } else {
                    json_obj
                };

                Ok(Some(RequestBody::Json(final_json)))
            }
            RequestType::Form => {
                // Use Vec to preserve insertion order and allow duplicate keys
                // (e.g., id=1&id=2). This is important for both HTTP semantics
                // and AWS SigV4 signing which requires deterministic body ordering.
                let mut form_data = Vec::new();

                for item in data_items {
                    let (key, value) = get_data_key_value(item)?;
                    form_data.push((key, value));
                }

                Ok(Some(RequestBody::Form(form_data)))
            }
            RequestType::Multipart => {
                // Already handled above
                unreachable!()
            }
        }
    }
}

/// Guess MIME type from file extension
fn guess_mime_type(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "gz" | "gzip" => "application/gzip",
        "tar" => "application/x-tar",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

/// Get the key and data value from an InputItem, reading from file if needed
fn get_data_key_value(item: &InputItem) -> Result<(String, String), QuicpulseError> {
    match item {
        InputItem::DataField { key, value } => {
            Ok((key.clone(), value.clone()))
        }
        InputItem::DataFieldFile { key, path } => {
            let content = std::fs::read_to_string(path)
                .map_err(|e| QuicpulseError::Io(e))?;
            Ok((key.clone(), content.trim().to_string()))
        }
        InputItem::JsonField { key, value } => {
            Ok((key.clone(), value.to_string()))
        }
        InputItem::JsonFieldFile { key, path } => {
            let content = std::fs::read_to_string(path)
                .map_err(|e| QuicpulseError::Io(e))?;
            Ok((key.clone(), content.trim().to_string()))
        }
        _ => Err(QuicpulseError::Parse("Not a data item".to_string())),
    }
}

/// Set a nested value in a JSON object (handles keys like "user[name]")
fn set_nested_value(obj: &mut JsonValue, key: &str, value: JsonValue) -> Result<(), QuicpulseError> {
    // Simple case: no nested path
    if !key.contains('[') {
        if let Some(map) = obj.as_object_mut() {
            map.insert(key.to_string(), value);
        }
        return Ok(());
    }

    // Parse nested path: user[name] -> ["user", "name"]
    let parts: Vec<&str> = key.split(|c| c == '[' || c == ']')
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return Ok(());
    }

    // For nested keys, we need to build the structure
    // e.g., user[name]=John -> {"user": {"name": "John"}}
    let first = parts[0];
    
    if parts.len() == 1 {
        // Just a regular key
        if let Some(map) = obj.as_object_mut() {
            map.insert(first.to_string(), value);
        }
        return Ok(());
    }

    // Build nested structure from the inside out
    let mut inner_value = value;
    for part in parts[1..].iter().rev() {
        let mut wrapper = json!({});
        if let Some(map) = wrapper.as_object_mut() {
            map.insert(part.to_string(), inner_value);
        }
        inner_value = wrapper;
    }

    // Merge into the object at the first key
    if let Some(map) = obj.as_object_mut() {
        if let Some(existing) = map.get_mut(first) {
            // Merge the nested structures
            merge_json(existing, inner_value);
        } else {
            map.insert(first.to_string(), inner_value);
        }
    }

    Ok(())
}

/// Merge two JSON values (used for nested key handling)
fn merge_json(base: &mut JsonValue, overlay: JsonValue) {
    match (base, overlay) {
        (JsonValue::Object(base_map), JsonValue::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(base_value) = base_map.get_mut(&key) {
                    merge_json(base_value, value);
                } else {
                    base_map.insert(key, value);
                }
            }
        }
        (base, overlay) => {
            *base = overlay;
        }
    }
}


/// Check HTTP status and return appropriate exit status
///
/// When check_status is true, non-2xx responses return Error.
/// When check_status is false, always returns Success.
pub fn check_status(status_code: u16, check_status: bool) -> ExitStatus {
    ExitStatus::from_http_status(status_code, check_status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_format_simple() {
        let json = json!({"name": "John", "age": 30});
        let result = json_to_deterministic_format(&json);
        // serde_json compact format (no spaces around : and ,)
        assert!(result.contains("\"name\":\"John\""));
        assert!(result.contains("\"age\":30"));
    }

    #[test]
    fn test_json_format_nested() {
        let json = json!({"user": {"name": "John", "email": "john@example.com"}});
        let result = json_to_deterministic_format(&json);
        assert!(result.contains("\"user\":{"));
        assert!(result.contains("\"name\":\"John\""));
    }

    #[test]
    fn test_json_format_array() {
        let json = json!({"items": [1, 2, 3]});
        let result = json_to_deterministic_format(&json);
        assert_eq!(result, "{\"items\":[1,2,3]}");
    }

    #[test]
    fn test_json_format_escape() {
        let json = json!({"message": "Hello\nWorld"});
        let result = json_to_deterministic_format(&json);
        assert!(result.contains("\\n"));
    }

    #[test]
    fn test_json_format_order_preserved() {
        // Build JSON with specific order
        let mut map = serde_json::Map::new();
        map.insert("name".to_string(), json!("John"));
        map.insert("age".to_string(), json!(30));
        let json = JsonValue::Object(map);

        let result = json_to_deterministic_format(&json);
        // With preserve_order feature, order should be maintained
        assert_eq!(result, "{\"name\":\"John\",\"age\":30}");
    }
}
