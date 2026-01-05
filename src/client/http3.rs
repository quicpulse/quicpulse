//! HTTP/3 client using h3-quinn
//!
//! Provides HTTP/3 support via QUIC transport using the h3 and quinn crates.
//! This is used when --http3 flag is specified.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Buf;
use http::{Request, StatusCode};
use http::header::HeaderMap;
use tokio::net::lookup_host;

use crate::auth::{AwsSigV4Config, sign_request as aws_sign_request};
use crate::errors::QuicpulseError;

/// HTTP/3 response with body
#[derive(Debug)]
pub struct Http3Response {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    pub version: http::Version,
}

/// Send an HTTP/3 request
pub async fn send_http3_request(
    method: &str,
    url: &str,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    timeout: Duration,
) -> Result<Http3Response, QuicpulseError> {
    send_http3_request_with_options(method, url, headers, body, timeout, true, None, None).await
}

/// Send an HTTP/3 request with TLS options
pub async fn send_http3_request_with_options(
    method: &str,
    url: &str,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    timeout: Duration,
    verify_tls: bool,
    client_cert: Option<&std::path::Path>,
    client_key: Option<&std::path::Path>,
) -> Result<Http3Response, QuicpulseError> {
    let timeout_secs = timeout.as_secs_f64();

    // Parse URL
    let parsed_url = url::Url::parse(url)
        .map_err(|e| QuicpulseError::Parse(format!("Invalid URL: {}", e)))?;

    if parsed_url.scheme() != "https" {
        return Err(QuicpulseError::Config(
            "HTTP/3 requires HTTPS".to_string()
        ));
    }

    let host = parsed_url.host_str()
        .ok_or_else(|| QuicpulseError::Parse("URL missing host".to_string()))?;
    let port = parsed_url.port().unwrap_or(443);

    // Resolve hostname
    let addr = resolve_host(host, port).await?;

    // Build TLS config for QUIC
    let tls_config = build_tls_config(verify_tls, client_cert, client_key)?;

    // Create QUIC endpoint
    let mut endpoint = h3_quinn::quinn::Endpoint::client("[::]:0".parse().unwrap())
        .map_err(|e| QuicpulseError::Config(format!("Failed to create QUIC endpoint: {}", e)))?;

    let client_config = h3_quinn::quinn::ClientConfig::new(Arc::new(
        h3_quinn::quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)
            .map_err(|e| QuicpulseError::Config(format!("TLS config error: {}", e)))?
    ));
    endpoint.set_default_client_config(client_config);

    // Connect with timeout
    let connect_fut = endpoint.connect(addr, host);
    let conn = tokio::time::timeout(timeout, async {
        connect_fut
            .map_err(|e| QuicpulseError::Connection(format!("QUIC connect error: {}", e)))?
            .await
            .map_err(|e| QuicpulseError::Connection(format!("QUIC connection failed: {}", e)))
    })
    .await
    .map_err(|_| QuicpulseError::Timeout(timeout_secs))??;

    // Create HTTP/3 connection
    let quinn_conn = h3_quinn::Connection::new(conn);
    let (mut driver, mut send_request) = h3::client::new(quinn_conn)
        .await
        .map_err(|e| QuicpulseError::Connection(format!("HTTP/3 handshake failed: {}", e)))?;

    // Spawn driver task
    tokio::spawn(async move {
        let _ = futures::future::poll_fn(|cx| driver.poll_close(cx)).await;
    });

    // Build request
    let path = if let Some(query) = parsed_url.query() {
        format!("{}?{}", parsed_url.path(), query)
    } else {
        parsed_url.path().to_string()
    };

    let mut req_builder = Request::builder()
        .method(method)
        .uri(&path);

    // Add headers
    for (name, value) in headers.iter() {
        req_builder = req_builder.header(name, value);
    }

    // Add required headers if not present
    if !headers.contains_key("host") {
        req_builder = req_builder.header("host", host);
    }

    let req = req_builder.body(())
        .map_err(|e| QuicpulseError::Parse(format!("Failed to build request: {}", e)))?;

    // Send request with timeout
    let response = tokio::time::timeout(timeout, async {
        let mut stream = send_request.send_request(req)
            .await
            .map_err(|e| QuicpulseError::Connection(format!("Failed to send request: {}", e)))?;

        // Send body if present
        if let Some(body_bytes) = body {
            stream.send_data(bytes::Bytes::from(body_bytes))
                .await
                .map_err(|e| QuicpulseError::Connection(format!("Failed to send body: {}", e)))?;
        }

        stream.finish()
            .await
            .map_err(|e| QuicpulseError::Connection(format!("Failed to finish request: {}", e)))?;

        // Receive response
        let resp = stream.recv_response()
            .await
            .map_err(|e| QuicpulseError::Connection(format!("Failed to receive response: {}", e)))?;

        // Read body
        let mut body_data = Vec::new();
        while let Some(chunk) = stream.recv_data()
            .await
            .map_err(|e| QuicpulseError::Connection(format!("Failed to receive body: {}", e)))?
        {
            body_data.extend_from_slice(chunk.chunk());
        }

        Ok::<_, QuicpulseError>((resp, body_data))
    })
    .await
    .map_err(|_| QuicpulseError::Timeout(timeout_secs))??;

    let (resp, body) = response;

    Ok(Http3Response {
        status: resp.status(),
        headers: resp.headers().clone(),
        body,
        version: http::Version::HTTP_3,
    })
}

/// Resolve hostname to socket address
async fn resolve_host(host: &str, port: u16) -> Result<SocketAddr, QuicpulseError> {
    let addr_str = format!("{}:{}", host, port);
    let mut addrs = lookup_host(&addr_str)
        .await
        .map_err(|e| QuicpulseError::Config(format!("DNS lookup failed: {}", e)))?;

    addrs.next()
        .ok_or_else(|| QuicpulseError::Config(format!("No addresses found for {}", host)))
}

/// Build TLS config for QUIC with HTTP/3 ALPN
fn build_tls_config(
    verify: bool,
    client_cert: Option<&std::path::Path>,
    client_key: Option<&std::path::Path>,
) -> Result<rustls::ClientConfig, QuicpulseError> {
    // Install default crypto provider if not already installed
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Load root certificates
    let root_store = if verify {
        let mut store = rustls::RootCertStore::empty();
        let certs_result = rustls_native_certs::load_native_certs();
        for cert in certs_result.certs {
            let _ = store.add(cert);
        }
        if store.is_empty() {
            return Err(QuicpulseError::Config(
                "No root certificates found".to_string()
            ));
        }
        store
    } else {
        rustls::RootCertStore::empty()
    };

    // Load client certificate if provided
    let client_auth = if let Some(cert_path) = client_cert {
        load_client_cert(cert_path, client_key)?
    } else {
        None
    };

    let mut config = if verify {
        let builder = rustls::ClientConfig::builder()
            .with_root_certificates(root_store);
        if let Some((certs, key)) = client_auth {
            builder.with_client_auth_cert(certs, key)
                .map_err(|e| QuicpulseError::Ssl(format!("Failed to set client certificate: {}", e)))?
        } else {
            builder.with_no_client_auth()
        }
    } else {
        let builder = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoopServerCertVerifier));
        if let Some((certs, key)) = client_auth {
            builder.with_client_auth_cert(certs, key)
                .map_err(|e| QuicpulseError::Ssl(format!("Failed to set client certificate: {}", e)))?
        } else {
            builder.with_no_client_auth()
        }
    };

    // Set ALPN protocols for HTTP/3
    config.alpn_protocols = vec![b"h3".to_vec()];

    Ok(config)
}

/// Load client certificate and key from PEM files
fn load_client_cert(
    cert_path: &std::path::Path,
    key_path: Option<&std::path::Path>,
) -> Result<Option<(Vec<rustls::pki_types::CertificateDer<'static>>, rustls::pki_types::PrivateKeyDer<'static>)>, QuicpulseError> {
    use std::io::BufReader;

    // Read certificate file
    let cert_data = std::fs::read(cert_path)
        .map_err(|e| QuicpulseError::Ssl(format!("Failed to read certificate '{}': {}", cert_path.display(), e)))?;

    // Parse certificates
    let mut certs = Vec::new();
    for cert in rustls_pemfile::certs(&mut BufReader::new(&cert_data[..])) {
        match cert {
            Ok(c) => certs.push(c),
            Err(e) => return Err(QuicpulseError::Ssl(format!("Failed to parse certificate: {}", e))),
        }
    }

    if certs.is_empty() {
        return Err(QuicpulseError::Ssl("No certificates found in file".to_string()));
    }

    // Read key file (either separate or from cert file)
    let key_data = if let Some(kp) = key_path {
        std::fs::read(kp)
            .map_err(|e| QuicpulseError::Ssl(format!("Failed to read key file '{}': {}", kp.display(), e)))?
    } else {
        cert_data.clone()
    };

    // Parse private key (try different formats)
    let key = {
        // Try PKCS#8 first
        let pkcs8_keys: Vec<_> = rustls_pemfile::pkcs8_private_keys(&mut BufReader::new(&key_data[..]))
            .filter_map(|k| k.ok())
            .collect();

        if let Some(key) = pkcs8_keys.into_iter().next() {
            rustls::pki_types::PrivateKeyDer::Pkcs8(key)
        } else {
            // Try RSA key
            let rsa_keys: Vec<_> = rustls_pemfile::rsa_private_keys(&mut BufReader::new(&key_data[..]))
                .filter_map(|k| k.ok())
                .collect();

            if let Some(key) = rsa_keys.into_iter().next() {
                rustls::pki_types::PrivateKeyDer::Pkcs1(key)
            } else {
                // Try EC key
                let ec_keys: Vec<_> = rustls_pemfile::ec_private_keys(&mut BufReader::new(&key_data[..]))
                    .filter_map(|k| k.ok())
                    .collect();

                if let Some(key) = ec_keys.into_iter().next() {
                    rustls::pki_types::PrivateKeyDer::Sec1(key)
                } else {
                    return Err(QuicpulseError::Ssl("No private key found in file".to_string()));
                }
            }
        }
    };

    Ok(Some((certs, key)))
}

/// A certificate verifier that accepts all certificates (DANGEROUS - only for testing)
#[derive(Debug)]
struct NoopServerCertVerifier;

impl rustls::client::danger::ServerCertVerifier for NoopServerCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        // Return all commonly used signature schemes
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

/// Build URL with query parameters from InputItems
fn build_url_with_query(base_url: &str, items: &[crate::input::InputItem]) -> Result<String, QuicpulseError> {
    use crate::input::InputItem;

    let mut parsed = url::Url::parse(base_url)
        .map_err(|e| QuicpulseError::Parse(format!("Invalid URL: {}", e)))?;

    // Collect existing query params
    let mut params: Vec<(String, String)> = parsed.query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    // Add query params from InputItems
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

    // Build query string using form-style encoding
    if !params.is_empty() {
        let query = params.iter()
            .map(|(k, v)| format!("{}={}",
                percent_encoding::utf8_percent_encode(k, percent_encoding::NON_ALPHANUMERIC),
                percent_encoding::utf8_percent_encode(v, percent_encoding::NON_ALPHANUMERIC)))
            .collect::<Vec<_>>()
            .join("&");
        parsed.set_query(Some(&query));
    }

    Ok(parsed.to_string())
}

/// Build request body from InputItems
fn build_http3_body(
    args: &crate::cli::Args,
    processed: &crate::cli::process::ProcessedArgs,
    headers: &mut HeaderMap,
) -> Result<Option<Vec<u8>>, QuicpulseError> {
    use crate::input::InputItem;
    use crate::models::types::RequestType;
    use std::io::Write;

    // Raw body takes priority
    if let Some(ref raw_body) = args.raw {
        return Ok(Some(raw_body.as_bytes().to_vec()));
    }

    // Collect data items
    let data_items: Vec<_> = processed.items.iter()
        .filter(|item| matches!(item,
            InputItem::DataField { .. } |
            InputItem::DataFieldFile { .. } |
            InputItem::JsonField { .. } |
            InputItem::JsonFieldFile { .. }
        ))
        .collect();

    // Collect file upload items
    let file_items: Vec<_> = processed.items.iter()
        .filter(|item| matches!(item, InputItem::FileUpload { .. }))
        .collect();

    // Check if we should use multipart
    let use_multipart = processed.request_type == RequestType::Multipart || !file_items.is_empty();

    if data_items.is_empty() && file_items.is_empty() {
        return Ok(None);
    }

    // Multipart mode
    if use_multipart {
        let boundary = format!("----QuicPulse{}", uuid::Uuid::new_v4().simple());
        let mut body = Vec::new();

        // Add data fields as parts
        for item in &data_items {
            match item {
                InputItem::DataField { key, value } => {
                    write!(body, "--{}\r\n", boundary)
                        .map_err(|e| QuicpulseError::Io(e))?;
                    write!(body, "Content-Disposition: form-data; name=\"{}\"\r\n\r\n", key)
                        .map_err(|e| QuicpulseError::Io(e))?;
                    write!(body, "{}\r\n", value)
                        .map_err(|e| QuicpulseError::Io(e))?;
                }
                InputItem::DataFieldFile { key, path } => {
                    let value = std::fs::read_to_string(path)
                        .map_err(|e| QuicpulseError::Io(e))?
                        .trim()
                        .to_string();
                    write!(body, "--{}\r\n", boundary)
                        .map_err(|e| QuicpulseError::Io(e))?;
                    write!(body, "Content-Disposition: form-data; name=\"{}\"\r\n\r\n", key)
                        .map_err(|e| QuicpulseError::Io(e))?;
                    write!(body, "{}\r\n", value)
                        .map_err(|e| QuicpulseError::Io(e))?;
                }
                _ => {}
            }
        }

        // Add file uploads
        for item in &file_items {
            if let InputItem::FileUpload { field, path, mime_type, filename } = item {
                let contents = std::fs::read(path)
                    .map_err(|e| QuicpulseError::Io(e))?;
                let fname = filename.clone()
                    .or_else(|| path.file_name()?.to_str().map(String::from))
                    .unwrap_or_else(|| "file".to_string());
                let mime = mime_type.clone()
                    .unwrap_or_else(|| guess_mime_from_path(path));

                write!(body, "--{}\r\n", boundary)
                    .map_err(|e| QuicpulseError::Io(e))?;
                write!(body, "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                    field, fname)
                    .map_err(|e| QuicpulseError::Io(e))?;
                write!(body, "Content-Type: {}\r\n\r\n", mime)
                    .map_err(|e| QuicpulseError::Io(e))?;
                body.extend_from_slice(&contents);
                write!(body, "\r\n")
                    .map_err(|e| QuicpulseError::Io(e))?;
            }
        }

        write!(body, "--{}--\r\n", boundary)
            .map_err(|e| QuicpulseError::Io(e))?;

        let content_type = format!("multipart/form-data; boundary={}", boundary);
        if let Ok(v) = http::header::HeaderValue::try_from(content_type) {
            headers.insert("content-type", v);
        }

        return Ok(Some(body));
    }

    // Form mode
    if processed.request_type == RequestType::Form {
        let mut form_data = Vec::new();
        for item in &data_items {
            match item {
                InputItem::DataField { key, value } => {
                    form_data.push((key.clone(), value.clone()));
                }
                InputItem::DataFieldFile { key, path } => {
                    let value = std::fs::read_to_string(path)
                        .map_err(|e| QuicpulseError::Io(e))?
                        .trim()
                        .to_string();
                    form_data.push((key.clone(), value));
                }
                InputItem::JsonField { key, value } => {
                    form_data.push((key.clone(), value.to_string()));
                }
                InputItem::JsonFieldFile { key, path } => {
                    let value = std::fs::read_to_string(path)
                        .map_err(|e| QuicpulseError::Io(e))?
                        .trim()
                        .to_string();
                    form_data.push((key.clone(), value));
                }
                _ => {}
            }
        }
        let body = serde_urlencoded::to_string(&form_data)
            .map_err(|e| QuicpulseError::Parse(format!("Failed to encode form: {}", e)))?;
        if !headers.contains_key("content-type") {
            if let Ok(v) = http::header::HeaderValue::try_from("application/x-www-form-urlencoded") {
                headers.insert("content-type", v);
            }
        }
        return Ok(Some(body.into_bytes()));
    }

    // JSON mode (default)
    let mut json_obj = serde_json::Map::new();
    for item in &data_items {
        match item {
            InputItem::DataField { key, value } => {
                json_obj.insert(key.clone(), serde_json::Value::String(value.clone()));
            }
            InputItem::DataFieldFile { key, path } => {
                let value = std::fs::read_to_string(path)
                    .map_err(|e| QuicpulseError::Io(e))?
                    .trim()
                    .to_string();
                json_obj.insert(key.clone(), serde_json::Value::String(value));
            }
            InputItem::JsonField { key, value } => {
                json_obj.insert(key.clone(), value.clone());
            }
            InputItem::JsonFieldFile { key, path } => {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| QuicpulseError::Io(e))?;
                let value: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| QuicpulseError::Json(e))?;
                json_obj.insert(key.clone(), value);
            }
            _ => {}
        }
    }

    if !headers.contains_key("content-type") {
        if let Ok(v) = http::header::HeaderValue::try_from("application/json") {
            headers.insert("content-type", v);
        }
    }
    Ok(Some(serde_json::to_vec(&serde_json::Value::Object(json_obj)).unwrap_or_default()))
}

/// Guess MIME type from file path
fn guess_mime_from_path(path: &std::path::Path) -> String {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
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
    }.to_string()
}

/// Run an HTTP/3 request (entry point from core.rs)
pub async fn run_http3(
    args: &crate::cli::Args,
    processed: &crate::cli::process::ProcessedArgs,
    env: &crate::context::Environment,
    session: Option<&crate::sessions::Session>,
) -> Result<crate::status::ExitStatus, QuicpulseError> {
    use std::io::Write;
    use crate::input::InputItem;
    use crate::cli::args::AuthType;
    use crate::middleware::auth::{Auth, DigestAuth, DigestChallenge};
    use crate::output::formatters::{ColorFormatter, ColorStyle, format_json, JsonFormatterOptions};
    use crate::output::writer::PrettyOption;

    let timeout = Duration::from_secs(args.timeout.unwrap_or(30.0) as u64);

    // Build URL with query parameters
    let url = build_url_with_query(&processed.url, &processed.items)?;

    // Parse URL for session cookies
    let parsed_url = url::Url::parse(&url)
        .map_err(|e| QuicpulseError::Parse(format!("Invalid URL: {}", e)))?;

    // Build headers from processed items
    let mut headers = HeaderMap::new();

    // First apply session headers (lowest priority - can be overridden)
    if let Some(sess) = session {
        for header in &sess.headers {
            if let (Ok(n), Ok(v)) = (
                http::header::HeaderName::try_from(header.name.as_str()),
                http::header::HeaderValue::try_from(header.value.as_str()),
            ) {
                headers.insert(n, v);
            }
        }

        // Add session cookies
        let domain = parsed_url.host_str().unwrap_or("localhost");
        let path = parsed_url.path();
        let is_secure = parsed_url.scheme() == "https";

        if let Some(cookie_header) = sess.get_cookie_header(domain, path, is_secure) {
            if !cookie_header.is_empty() {
                if let Ok(v) = http::header::HeaderValue::try_from(cookie_header) {
                    headers.insert("cookie", v);
                }
            }
        }

        // Apply session auth if no CLI auth specified
        if args.auth.is_none() {
            if let Some(ref auth) = sess.auth {
                match auth.auth_type.as_str() {
                    "basic" => {
                        let parts: Vec<&str> = auth.credentials.splitn(2, ':').collect();
                        let (user, pass) = (parts[0], parts.get(1).copied().unwrap_or(""));
                        let auth_obj = Auth::basic(user, pass);
                        let _ = auth_obj.apply(&mut headers);
                    }
                    "bearer" => {
                        let auth_obj = Auth::bearer(&auth.credentials);
                        let _ = auth_obj.apply(&mut headers);
                    }
                    _ => {}
                }
            }
        }
    }

    // Add headers from request items (higher priority - override session)
    for item in &processed.items {
        match item {
            InputItem::Header { name, value } => {
                if let (Ok(n), Ok(v)) = (
                    http::header::HeaderName::try_from(name.as_str()),
                    http::header::HeaderValue::try_from(value.as_str()),
                ) {
                    headers.insert(n, v);
                }
            }
            InputItem::EmptyHeader { name } => {
                if let Ok(n) = http::header::HeaderName::try_from(name.as_str()) {
                    headers.insert(n, http::header::HeaderValue::from_static(""));
                }
            }
            InputItem::HeaderFile { name, path } => {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let (Ok(n), Ok(v)) = (
                        http::header::HeaderName::try_from(name.as_str()),
                        http::header::HeaderValue::try_from(content.trim()),
                    ) {
                        headers.insert(n, v);
                    }
                }
            }
            _ => {}
        }
    }

    // Add user agent if not present
    if !headers.contains_key("user-agent") {
        if let Ok(v) = http::header::HeaderValue::try_from(crate::client::USER_AGENT_STRING) {
            headers.insert("user-agent", v);
        }
    }

    // Apply authentication (Basic/Bearer)
    if let Some(ref auth_secret) = args.auth {
        let auth_str = auth_secret.as_str();
        let auth_type = args.auth_type.clone().unwrap_or(AuthType::Basic);
        match auth_type {
            AuthType::Basic => {
                let parts: Vec<&str> = auth_str.splitn(2, ':').collect();
                let (user, pass) = (parts[0], parts.get(1).copied().unwrap_or(""));
                let auth = Auth::basic(user, pass);
                auth.apply(&mut headers)
                    .map_err(|e| QuicpulseError::Auth(format!("Basic auth failed: {}", e)))?;
            }
            AuthType::Bearer => {
                let auth = Auth::bearer(auth_str);
                auth.apply(&mut headers)
                    .map_err(|e| QuicpulseError::Auth(format!("Bearer auth failed: {}", e)))?;
            }
            AuthType::Digest => {
                // Digest auth requires challenge-response, handled separately
                // Will be implemented in Phase 2
            }
            _ => {
                return Err(QuicpulseError::Config(
                    format!("{:?} auth not yet supported with HTTP/3", auth_type)
                ));
            }
        }
    }

    // Build body from raw body or data items
    let body = build_http3_body(args, processed, &mut headers)?;

    // Apply compression if requested
    let body = if args.compress > 0 {
        if let Some(body_bytes) = body {
            let (compressed, was_compressed) = crate::uploads::compress_request(
                &body_bytes,
                args.compress > 1, // Force compression with -xx
            )?;
            if was_compressed {
                if let Ok(v) = http::header::HeaderValue::try_from("deflate") {
                    headers.insert("content-encoding", v);
                }
            }
            Some(compressed)
        } else {
            None
        }
    } else {
        body
    };

    // Determine TLS verification setting
    let verify_tls = !matches!(args.verify.to_lowercase().as_str(), "no" | "false");

    // Get client certificate paths
    let client_cert = args.cert.as_deref();
    let client_key = args.cert_key.as_deref();

    // Check if AWS SigV4 authentication is requested
    let use_aws_sigv4 = matches!(args.auth_type, Some(AuthType::AwsSigv4));
    let is_multipart = headers.get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.starts_with("multipart/"))
        .unwrap_or(false);

    // Build AWS config for SigV4 signing
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
        let config = if let Some(ref auth_secret) = args.auth {
            let auth_str = auth_secret.as_str();
            let region = region.unwrap_or_else(|| {
                std::env::var("AWS_REGION")
                    .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
                    .unwrap_or_else(|_| "us-east-1".to_string())
            });
            AwsSigV4Config::from_credentials(auth_str, region, service)?
        } else if let Some(ref profile_name) = args.aws_profile {
            AwsSigV4Config::from_profile(profile_name, region, service).await?
        } else if let Ok(profile_name) = std::env::var("AWS_PROFILE") {
            AwsSigV4Config::from_profile(&profile_name, region, service).await?
        } else {
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

    // Sign request with AWS SigV4 if configured
    if let Some(ref config) = aws_config {
        // Collect headers for signing
        let current_headers: Vec<(String, String)> = headers.iter()
            .filter_map(|(name, value)| {
                value.to_str().ok().map(|v| (name.to_string(), v.to_string()))
            })
            .collect();

        // Sign the request
        let sig_headers = aws_sign_request(
            config,
            &processed.method,
            &url,
            &current_headers,
            body.as_deref(),
            is_multipart,
        )?;

        // Add signature headers to request
        for (name, value) in sig_headers {
            if let (Ok(n), Ok(v)) = (
                http::header::HeaderName::try_from(name.as_str()),
                http::header::HeaderValue::try_from(value.as_str()),
            ) {
                headers.insert(n, v);
            }
        }
    }

    // Print request if verbose
    if args.verbose > 0 {
        eprintln!("{} {} HTTP/3", processed.method, url);
        for (name, value) in headers.iter() {
            eprintln!("{}: {}", name, value.to_str().unwrap_or("<binary>"));
        }
        eprintln!();
    }

    // Send HTTP/3 request
    let mut resp = send_http3_request_with_options(
        &processed.method,
        &url,
        headers.clone(),
        body.clone(),
        timeout,
        verify_tls,
        client_cert,
        client_key,
    ).await?;

    // Handle Digest auth challenge-response (401 retry)
    if resp.status == StatusCode::UNAUTHORIZED {
        if matches!(args.auth_type, Some(AuthType::Digest)) {
            if let Some(www_auth) = resp.headers.get("www-authenticate") {
                if let Ok(www_auth_str) = www_auth.to_str() {
                    // Check if it's a Digest challenge
                    if www_auth_str.to_lowercase().starts_with("digest ") {
                        if let Ok(challenge) = DigestChallenge::parse(www_auth_str) {
                            // Get credentials
                            if let Some(ref auth_secret) = args.auth {
                                let auth_str = auth_secret.as_str();
                                let digest_auth = DigestAuth::from_credentials(auth_str)
                                    .map_err(|e| QuicpulseError::Auth(e.to_string()))?;

                                // Parse URL for path
                                let parsed_url = url::Url::parse(&url)
                                    .map_err(|e| QuicpulseError::Parse(format!("Invalid URL: {}", e)))?;
                                let uri_with_query = if let Some(query) = parsed_url.query() {
                                    format!("{}?{}", parsed_url.path(), query)
                                } else {
                                    parsed_url.path().to_string()
                                };

                                // Generate Authorization header
                                let auth_header = digest_auth.respond_to_challenge(
                                    &challenge,
                                    &processed.method,
                                    &uri_with_query,
                                ).map_err(|e| QuicpulseError::Auth(e.to_string()))?;

                                // Rebuild headers with Authorization
                                let mut retry_headers = headers.clone();
                                retry_headers.insert(
                                    "authorization",
                                    http::header::HeaderValue::try_from(auth_header)
                                        .map_err(|e| QuicpulseError::Parse(format!("Invalid auth header: {}", e)))?,
                                );

                                // Retry with auth
                                resp = send_http3_request_with_options(
                                    &processed.method,
                                    &url,
                                    retry_headers,
                                    body.clone(),
                                    timeout,
                                    verify_tls,
                                    client_cert,
                                    client_key,
                                ).await?;
                            }
                        }
                    }
                }
            }
        }
    }

    // Handle redirects
    if args.follow {
        let mut current_url = url.clone();
        let mut current_method = processed.method.clone();
        let mut current_body = body;
        let mut redirect_count = 0;

        while resp.status.is_redirection() && redirect_count < args.max_redirects as usize {
            // Get Location header
            let location = match resp.headers.get("location") {
                Some(loc) => loc.to_str().unwrap_or("").to_string(),
                None => break, // No Location header, stop redirecting
            };

            if location.is_empty() {
                break;
            }

            // Resolve relative URL
            let base_url = url::Url::parse(&current_url)
                .map_err(|e| QuicpulseError::Parse(format!("Invalid URL: {}", e)))?;
            let next_url = base_url.join(&location)
                .map_err(|e| QuicpulseError::Parse(format!("Invalid redirect URL '{}': {}", location, e)))?;

            // HTTP/3 only works with HTTPS
            if next_url.scheme() != "https" {
                eprintln!("Warning: Cannot follow redirect to non-HTTPS URL: {}", next_url);
                break;
            }

            // HTTP spec: POST -> GET on 301/302/303 redirects (except 307/308 which preserve method)
            let next_method = match resp.status.as_u16() {
                301 | 302 | 303 => {
                    if current_method == "POST" {
                        current_body = None; // Clear body for GET
                        "GET".to_string()
                    } else {
                        current_method.clone()
                    }
                }
                307 | 308 => {
                    // 307/308 preserve method and body
                    current_method.clone()
                }
                _ => current_method.clone(),
            };

            // Print intermediate response if --all is set
            if args.all {
                println!("HTTP/3 {} {}", resp.status.as_u16(), resp.status.canonical_reason().unwrap_or(""));
                for (name, value) in resp.headers.iter() {
                    println!("{}: {}", name, value.to_str().unwrap_or("<binary>"));
                }
                println!();
                if !resp.body.is_empty() && !args.headers_only {
                    let stdout = std::io::stdout();
                    let mut handle = stdout.lock();
                    let _ = handle.write_all(&resp.body);
                    let _ = handle.write_all(b"\n");
                }
            }

            if args.verbose > 0 {
                eprintln!("Redirecting to: {}", next_url);
            }

            // Build headers for redirect, re-signing with AWS SigV4 if needed
            let mut redirect_headers = headers.clone();

            // Remove old signature headers before re-signing
            if aws_config.is_some() {
                redirect_headers.remove("authorization");
                redirect_headers.remove("x-amz-date");
                redirect_headers.remove("x-amz-content-sha256");
                redirect_headers.remove("x-amz-security-token");
            }

            // AWS SigV4: Re-sign the request for the new URL
            if let Some(ref config) = aws_config {
                let redirect_body_bytes = if next_method == "GET" {
                    None
                } else {
                    current_body.as_deref()
                };

                let redirect_sig_headers = aws_sign_request(
                    config,
                    &next_method,
                    next_url.as_str(),
                    &[], // No custom headers needed for redirect
                    redirect_body_bytes,
                    is_multipart,
                )?;

                for (name, value) in redirect_sig_headers {
                    if let (Ok(n), Ok(v)) = (
                        http::header::HeaderName::try_from(name.as_str()),
                        http::header::HeaderValue::try_from(value.as_str()),
                    ) {
                        redirect_headers.insert(n, v);
                    }
                }
            }

            // Send redirect request
            resp = send_http3_request_with_options(
                &next_method,
                next_url.as_str(),
                redirect_headers,
                current_body.clone(),
                timeout,
                verify_tls,
                client_cert,
                client_key,
            ).await?;

            current_url = next_url.to_string();
            current_method = next_method;
            redirect_count += 1;
        }
    }

    // Build processing options for colors/formatting
    let use_colors = if args.no_color {
        false
    } else {
        match args.pretty {
            Some(PrettyOption::Colors) | Some(PrettyOption::All) => true,
            Some(PrettyOption::None) | Some(PrettyOption::Format) => false,
            None => env.stdout_isatty,
        }
    };

    let pretty_option = match args.pretty {
        Some(opt) => opt,
        None => if env.stdout_isatty { PrettyOption::All } else { PrettyOption::Format },
    };

    let color_style = match &args.style {
        Some(s) => ColorStyle::parse(s),
        None => ColorStyle::Auto,
    };

    let formatter = if use_colors && pretty_option != PrettyOption::None {
        Some(ColorFormatter::new(color_style.clone()))
    } else {
        None
    };

    // Print response headers (unless body-only mode)
    if !args.body {
        let status_line = format!(
            "HTTP/3 {} {}",
            resp.status.as_u16(),
            resp.status.canonical_reason().unwrap_or("")
        );

        let mut headers_str = status_line;
        headers_str.push('\n');

        for (name, value) in resp.headers.iter() {
            if let Ok(v) = value.to_str() {
                headers_str.push_str(&format!("{}: {}\n", name, v));
            }
        }

        let formatted = if let Some(ref fmt) = formatter {
            fmt.format_headers(&headers_str)
        } else {
            headers_str
        };

        print!("{}", formatted);
        println!();
    }

    // Handle download mode
    if args.download {
        // Determine filename
        let filename = determine_download_filename(&url, &resp.headers, args.output.as_ref());

        // Write body to file
        std::fs::write(&filename, &resp.body)
            .map_err(|e| QuicpulseError::Io(e))?;

        eprintln!("Downloaded {} bytes to {:?}", resp.body.len(), filename);

        return Ok(crate::status::ExitStatus::from_http_status(resp.status.as_u16(), args.check_status));
    }

    // Print body (unless headers-only mode)
    if !args.headers_only {
        let body_str = String::from_utf8_lossy(&resp.body);

        // Detect content type for formatting
        let content_type = resp.headers.get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/plain");

        let base_mime = content_type.split(';').next().unwrap_or(content_type).trim();
        let is_json = base_mime == "application/json" || base_mime.ends_with("+json");

        let output = if is_json {
            // Try to parse and format JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_str) {
                let formatted = if matches!(pretty_option, PrettyOption::All | PrettyOption::Format) {
                    serde_json::to_string_pretty(&json).unwrap_or_else(|_| body_str.to_string())
                } else {
                    json.to_string()
                };

                if let Some(ref fmt) = formatter {
                    fmt.format_json(&formatted)
                } else {
                    formatted
                }
            } else {
                body_str.to_string()
            }
        } else if let Some(ref fmt) = formatter {
            fmt.format_by_mime(&body_str, base_mime)
        } else {
            body_str.to_string()
        };

        println!("{}", output);
    }

    // Check status for exit code
    Ok(crate::status::ExitStatus::from_http_status(resp.status.as_u16(), args.check_status))
}

/// Determine download filename from URL, headers, or user-specified path
fn determine_download_filename(url: &str, headers: &HeaderMap, output: Option<&std::path::PathBuf>) -> std::path::PathBuf {
    // Priority 1: User-specified output path
    if let Some(path) = output {
        return path.clone();
    }

    // Priority 2: Content-Disposition header
    if let Some(cd) = headers.get("content-disposition") {
        if let Ok(cd_str) = cd.to_str() {
            if let Some(filename) = extract_filename_from_cd(cd_str) {
                return std::path::PathBuf::from(filename);
            }
        }
    }

    // Priority 3: URL path
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(segments) = parsed.path_segments() {
            if let Some(last) = segments.last() {
                if !last.is_empty() {
                    let decoded = percent_encoding::percent_decode_str(last)
                        .decode_utf8_lossy()
                        .to_string();
                    if !decoded.is_empty() {
                        return std::path::PathBuf::from(decoded);
                    }
                }
            }
        }
    }

    // Fallback
    std::path::PathBuf::from("download")
}

/// Extract filename from Content-Disposition header
fn extract_filename_from_cd(header: &str) -> Option<String> {
    // Simple parsing: look for filename="..." or filename*=UTF-8''...
    for part in header.split(';') {
        let part = part.trim();
        if part.starts_with("filename*=") {
            // RFC 5987 encoded filename
            let value = &part[10..];
            if let Some(encoded) = value.strip_prefix("UTF-8''") {
                let decoded = percent_encoding::percent_decode_str(encoded)
                    .decode_utf8_lossy()
                    .to_string();
                return Some(decoded);
            }
        } else if part.starts_with("filename=") {
            let value = &part[9..];
            let value = value.trim_matches('"').trim_matches('\'');
            return Some(value.to_string());
        }
    }
    None
}

/// Infer AWS service name from URL
fn infer_aws_service(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?;

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
                "cloudformation" => "cloudformation",
                "cloudwatch" => "monitoring",
                "elasticloadbalancing" => "elasticloadbalancing",
                "autoscaling" => "autoscaling",
                "elasticache" => "elasticache",
                "elasticbeanstalk" => "elasticbeanstalk",
                "glacier" => "glacier",
                "kms" => "kms",
                "redshift" => "redshift",
                "route53" => "route53",
                "ses" => "ses",
                "swf" => "swf",
                "cloudsearch" => "cloudsearch",
                "elastictranscoder" => "elastictranscoder",
                "importexport" => "importexport",
                "storagegateway" => "storagegateway",
                "datapipeline" => "datapipeline",
                "directconnect" => "directconnect",
                "opsworks" => "opsworks",
                "elasticmapreduce" => "elasticmapreduce",
                "support" => "support",
                "cognito-identity" => "cognito-identity",
                "cognito-idp" => "cognito-idp",
                "cognito-sync" => "cognito-sync",
                other => other,
            }.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_requires_https() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = send_http3_request(
                "GET",
                "http://example.com",
                HeaderMap::new(),
                None,
                Duration::from_secs(5),
            ).await;

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("HTTPS"));
        });
    }

    #[test]
    #[ignore = "requires network access to HTTP/3 endpoint"]
    fn test_http3_cloudflare_connection() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = send_http3_request(
                "GET",
                "https://cloudflare-quic.com/",
                HeaderMap::new(),
                None,
                Duration::from_secs(10),
            ).await;

            match result {
                Ok(resp) => {
                    assert_eq!(resp.version, http::Version::HTTP_3);
                    assert!(resp.status.is_success());
                    println!("HTTP/3 response: {} bytes", resp.body.len());
                }
                Err(e) => {
                    // Connection might fail due to network/firewall issues
                    println!("HTTP/3 connection failed (may be expected): {}", e);
                }
            }
        });
    }
}
