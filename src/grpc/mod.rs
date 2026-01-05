//! gRPC support module

pub mod client;
pub mod reflection;
pub mod codec;
pub mod dynamic;
pub mod interactive;
pub mod proto_parser;

pub use proto_parser::ProtoSchema;
pub use dynamic::{GrpcSchema, MethodInfo};
pub use interactive::run_interactive;

use crate::cli::Args;
use crate::cli::parser::ProcessedArgs;
use crate::context::Environment;
use crate::errors::QuicpulseError;
use crate::output::terminal::{self, colors, RESET};
use crate::output::formatters::{ColorFormatter, ColorStyle};
use crate::status::ExitStatus;

/// Check if request should be treated as gRPC
pub fn is_grpc_request(args: &Args) -> bool {
    args.grpc || args.grpc_list || args.grpc_describe.is_some() || args.grpc_interactive
}

/// Parse gRPC endpoint from URL
///
/// Accepts formats like:
/// - grpc://host:port/package.Service/Method
/// - host:port/package.Service/Method (when --grpc flag is used)
pub fn parse_grpc_endpoint(url: &str) -> Result<GrpcEndpoint, QuicpulseError> {
    let url = url.trim();

    // Remove grpc:// prefix if present
    let url = url.strip_prefix("grpc://")
        .or_else(|| url.strip_prefix("grpcs://"))
        .unwrap_or(url);

    // Split host:port from path
    let (host_port, path) = if let Some(idx) = url.find('/') {
        (&url[..idx], &url[idx + 1..])
    } else {
        (url, "")
    };

    // Parse host and port
    let (host, port) = if let Some(idx) = host_port.rfind(':') {
        let port_str = &host_port[idx + 1..];
        if let Ok(port) = port_str.parse::<u16>() {
            (&host_port[..idx], port)
        } else {
            (host_port, 443) // Default to 443 for gRPC
        }
    } else {
        (host_port, 443)
    };

    // Parse service and method from path
    let (service, method) = if !path.is_empty() {
        if let Some(idx) = path.rfind('/') {
            (Some(path[..idx].to_string()), Some(path[idx + 1..].to_string()))
        } else {
            (Some(path.to_string()), None)
        }
    } else {
        (None, None)
    };

    Ok(GrpcEndpoint {
        host: host.to_string(),
        port,
        service,
        method,
        use_tls: url.starts_with("grpcs://") || port == 443,
    })
}

/// Parsed gRPC endpoint
#[derive(Debug, Clone)]
pub struct GrpcEndpoint {
    pub host: String,
    pub port: u16,
    pub service: Option<String>,
    pub method: Option<String>,
    pub use_tls: bool,
}

impl GrpcEndpoint {
    /// Get the full address for connection
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Get the URI for the endpoint
    pub fn uri(&self) -> String {
        let scheme = if self.use_tls { "https" } else { "http" };
        format!("{}://{}:{}", scheme, self.host, self.port)
    }

    /// Get the full service path
    pub fn service_path(&self) -> Option<String> {
        self.service.as_ref().map(|s| {
            if let Some(ref m) = self.method {
                format!("/{}/{}", s, m)
            } else {
                format!("/{}", s)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_grpc_endpoint_full() {
        let endpoint = parse_grpc_endpoint("grpc://localhost:50051/mypackage.MyService/MyMethod").unwrap();
        assert_eq!(endpoint.host, "localhost");
        assert_eq!(endpoint.port, 50051);
        assert_eq!(endpoint.service, Some("mypackage.MyService".to_string()));
        assert_eq!(endpoint.method, Some("MyMethod".to_string()));
    }

    #[test]
    fn test_parse_grpc_endpoint_simple() {
        let endpoint = parse_grpc_endpoint("localhost:50051").unwrap();
        assert_eq!(endpoint.host, "localhost");
        assert_eq!(endpoint.port, 50051);
        assert!(endpoint.service.is_none());
        assert!(endpoint.method.is_none());
    }

    #[test]
    fn test_parse_grpc_endpoint_service_only() {
        let endpoint = parse_grpc_endpoint("localhost:50051/grpc.health.v1.Health").unwrap();
        assert_eq!(endpoint.host, "localhost");
        assert_eq!(endpoint.service, Some("grpc.health.v1.Health".to_string()));
        assert!(endpoint.method.is_none());
    }

    #[test]
    fn test_endpoint_uri() {
        let endpoint = GrpcEndpoint {
            host: "example.com".to_string(),
            port: 443,
            service: None,
            method: None,
            use_tls: true,
        };
        assert_eq!(endpoint.uri(), "https://example.com:443");
    }
}

pub async fn run_grpc(
    args: &Args,
    processed: &ProcessedArgs,
    _env: &Environment,
) -> Result<ExitStatus, QuicpulseError> {
    use crate::input::InputItem;

    let endpoint = parse_grpc_endpoint(&processed.url)?;

    let headers: Vec<(String, String)> = processed.items.iter()
        .filter_map(|item| {
            match item {
                InputItem::Header { name, value } => Some((name.clone(), value.clone())),
                InputItem::EmptyHeader { name } => Some((name.clone(), String::new())),
                InputItem::HeaderFile { name, path } => {
                    std::fs::read_to_string(path).ok().map(|v| (name.clone(), v.trim().to_string()))
                }
                _ => None,
            }
        })
        .collect();

    let timeout = args.timeout.map(|t| std::time::Duration::from_secs_f64(t));

    use crate::client::ssl::SslConfig;
    let ssl_config = SslConfig::from_args(
        &args.verify,
        args.ssl.as_deref(),
        args.ciphers.as_deref(),
        args.cert.as_ref().and_then(|p| p.to_str()),
        args.cert_key.as_ref().and_then(|p| p.to_str()),
        args.cert_key_pass.as_deref(),
    );

    let mut client = client::GrpcClient::connect_with_options(endpoint.clone(), timeout, Some(headers), Some(&ssl_config)).await?;

    if let Some(ref proto_path) = args.proto {
        client.load_proto(proto_path)?;
        if args.verbose > 0 {
            eprintln!("{}: {}", terminal::info("Loaded proto schema from"), proto_path.display());
            if let Some(schema) = client.schema() {
                eprintln!("  {}: {}", terminal::label("Package"), schema.package);
                eprintln!("  {}: {}", terminal::label("Messages"), schema.messages.len());
                eprintln!("  {}: {}", terminal::label("Services"), schema.services.len());
            }
        }
    }

    // Handle interactive mode
    if args.grpc_interactive {
        eprintln!("{} {}...", terminal::info("Starting gRPC interactive REPL for"), terminal::label(&endpoint.uri()));
        run_interactive(client, args.verbose > 0).await?;
        return Ok(ExitStatus::Success);
    }

    if args.grpc_list {
        eprintln!("{} {}...", terminal::info("Discovering services on"), terminal::label(&endpoint.uri()));

        match reflection::list_services(client.channel()).await {
            Ok(services) => {
                println!("{}", terminal::bold("Available services:", colors::WHITE));
                for service in services {
                    println!("  {}", terminal::label(&service));
                }
                return Ok(ExitStatus::Success);
            }
            Err(e) => {
                if let Some(schema) = client.schema() {
                    println!("{}", terminal::bold("Services from proto file:", colors::WHITE));
                    for service in &schema.services {
                        println!("\n  {}", terminal::label(&service.full_name));
                        for method in &service.methods {
                            let stream_info = match (method.client_streaming, method.server_streaming) {
                                (true, true) => terminal::muted(" (bidirectional streaming)"),
                                (true, false) => terminal::muted(" (client streaming)"),
                                (false, true) => terminal::muted(" (server streaming)"),
                                (false, false) => String::new(),
                            };
                            println!("    {} {} ({}) -> {}{}",
                                terminal::colorize("rpc", colors::PURPLE),
                                terminal::value(&method.name),
                                terminal::key(&method.input_type),
                                terminal::key(&method.output_type),
                                stream_info);
                        }
                    }
                    return Ok(ExitStatus::Success);
                }
                return Err(e);
            }
        }
    }

    if let Some(ref service_name) = args.grpc_describe {
        if let Some(schema) = client.schema() {
            for service in &schema.services {
                if service.name == *service_name || service.full_name == *service_name {
                    println!("{} {} {{", terminal::colorize("service", colors::PURPLE), terminal::label(&service.name));
                    for method in &service.methods {
                        let client_stream = if method.client_streaming { "stream " } else { "" };
                        let server_stream = if method.server_streaming { "stream " } else { "" };
                        println!("  {} {}({}{}) returns ({}{});",
                            terminal::colorize("rpc", colors::PURPLE),
                            terminal::value(&method.name),
                            terminal::muted(client_stream), terminal::key(&method.input_type),
                            terminal::muted(server_stream), terminal::key(&method.output_type));
                    }
                    println!("}}");
                    return Ok(ExitStatus::Success);
                }
            }
        }

        match reflection::describe_service(client.channel(), service_name).await {
            Ok(service) => {
                println!("{}", service.format_display());
                return Ok(ExitStatus::Success);
            }
            Err(e) => {
                return Err(QuicpulseError::Argument(format!(
                    "Could not describe service '{}': {}", service_name, e
                )));
            }
        }
    }

    let (service, method) = endpoint.service
        .as_ref()
        .and_then(|s| endpoint.method.as_ref().map(|m| (s.clone(), m.clone())))
        .ok_or_else(|| QuicpulseError::Argument(
            "gRPC call requires service and method. Use format: grpc://host:port/package.Service/Method".to_string()
        ))?;

    let request_json = if let Some(ref raw_body) = args.raw {
        serde_json::from_str(raw_body)
            .map_err(|e| QuicpulseError::Argument(format!("Invalid JSON body: {}", e)))?
    } else {
        let mut obj = serde_json::Map::new();
        for item in &processed.items {
            let (key, value) = match item {
                InputItem::DataField { key, value } => {
                    (key.clone(), serde_json::json!(value))
                }
                InputItem::DataFieldFile { key, path } => {
                    let content = std::fs::read_to_string(path).unwrap_or_default();
                    (key.clone(), serde_json::json!(content.trim()))
                }
                InputItem::JsonField { key, value } => {
                    (key.clone(), value.clone())
                }
                InputItem::JsonFieldFile { key, path } => {
                    let content = std::fs::read_to_string(path).unwrap_or_default();
                    let json_val = serde_json::from_str(&content).unwrap_or(serde_json::json!(content));
                    (key.clone(), json_val)
                }
                _ => continue,
            };
            obj.insert(key, value);
        }
        serde_json::Value::Object(obj)
    };

    if args.verbose > 0 {
        eprintln!("{}: {}/{}", terminal::info("gRPC Call"), terminal::label(&service), terminal::value(&method));
        let formatter = ColorFormatter::new(ColorStyle::Auto);
        let req_str = serde_json::to_string_pretty(&request_json).unwrap_or_default();
        eprintln!("{}: {}", terminal::info("Request"), formatter.format_json(&req_str));
    }

    let method_info = client.get_method_info(&service, &method);
    let is_server_streaming = method_info.as_ref().map(|m| m.server_streaming).unwrap_or(false);
    let is_client_streaming = method_info.as_ref().map(|m| m.client_streaming).unwrap_or(false);

    if is_client_streaming && is_server_streaming {
        run_grpc_bidi_streaming(&client, &service, &method, args).await
    } else if is_server_streaming {
        run_grpc_server_streaming(&client, &service, &method, &request_json, args).await
    } else if is_client_streaming {
        run_grpc_client_streaming(&client, &service, &method, args).await
    } else {
        let response = client.call_unary(&service, &method, &request_json).await?;

        if response.is_ok() {
            println!("{}: {}", terminal::label("Status"), terminal::success(&format!("{:?}", response.code())));
            if let Ok(json) = response.json() {
                let formatter = ColorFormatter::new(ColorStyle::Auto);
                let pretty = serde_json::to_string_pretty(&json).unwrap_or_default();
                println!("{}", formatter.format_json(&pretty));
            }
            Ok(ExitStatus::Success)
        } else {
            eprintln!("{}: {} - {}", terminal::error("gRPC Error"),
                terminal::warning(&format!("{:?}", response.code())), response.message());
            Ok(ExitStatus::Error)
        }
    }
}

async fn run_grpc_server_streaming(
    client: &client::GrpcClient,
    service: &str,
    method: &str,
    request_json: &serde_json::Value,
    args: &Args,
) -> Result<ExitStatus, QuicpulseError> {
    use futures::StreamExt;

    if args.verbose > 0 {
        eprintln!("{}...", terminal::info("Server streaming call"));
    }

    let response = client.call_server_streaming(service, method, request_json).await?;

    if !response.is_ok() {
        eprintln!("{}: {} - {}", terminal::error("gRPC Error"),
            terminal::warning(&format!("{:?}", response.code())), response.message());
        return Ok(ExitStatus::Error);
    }

    println!("{}: {}", terminal::label("Status"), terminal::success(&format!("{:?}", response.code())));

    let formatter = ColorFormatter::new(ColorStyle::Auto);
    let mut stream = response.into_stream();
    let mut count = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(json) => {
                count += 1;
                let output = serde_json::to_string(&json).unwrap_or_default();
                println!("{}", formatter.format_json(&output));
            }
            Err(e) => {
                eprintln!("{}: {}", terminal::error("Stream error"), e);
                return Ok(ExitStatus::Error);
            }
        }
    }

    if args.verbose > 0 {
        eprintln!("{} {} messages", terminal::info("Received"), terminal::number(&count.to_string()));
    }

    Ok(ExitStatus::Success)
}

async fn run_grpc_client_streaming(
    client: &client::GrpcClient,
    service: &str,
    method: &str,
    args: &Args,
) -> Result<ExitStatus, QuicpulseError> {
    use futures::stream;

    if args.verbose > 0 {
        eprintln!("{} - reading NDJSON from stdin...", terminal::info("Client streaming call"));
    }

    let lines: Vec<serde_json::Value> = tokio::task::spawn_blocking(|| {
        use std::io::BufRead;
        let stdin = std::io::stdin();
        stdin.lock()
            .lines()
            .filter_map(|line| line.ok())
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect()
    }).await.unwrap_or_default();

    if args.verbose > 0 {
        eprintln!("{} {} messages from stdin", terminal::info("Read"), terminal::number(&lines.len().to_string()));
    }

    let request_stream = stream::iter(lines);

    let response = client.call_client_streaming(service, method, request_stream).await?;

    if response.is_ok() {
        println!("{}: {}", terminal::label("Status"), terminal::success(&format!("{:?}", response.code())));
        if let Ok(json) = response.json() {
            let formatter = ColorFormatter::new(ColorStyle::Auto);
            let pretty = serde_json::to_string_pretty(&json).unwrap_or_default();
            println!("{}", formatter.format_json(&pretty));
        }
        Ok(ExitStatus::Success)
    } else {
        eprintln!("{}: {} - {}", terminal::error("gRPC Error"),
            terminal::warning(&format!("{:?}", response.code())), response.message());
        Ok(ExitStatus::Error)
    }
}

async fn run_grpc_bidi_streaming(
    client: &client::GrpcClient,
    service: &str,
    method: &str,
    args: &Args,
) -> Result<ExitStatus, QuicpulseError> {
    use futures::{stream, StreamExt};

    if args.verbose > 0 {
        eprintln!("{} - reading NDJSON from stdin...", terminal::info("Bidirectional streaming call"));
    }

    let lines: Vec<serde_json::Value> = tokio::task::spawn_blocking(|| {
        use std::io::BufRead;
        let stdin = std::io::stdin();
        stdin.lock()
            .lines()
            .filter_map(|line| line.ok())
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect()
    }).await.unwrap_or_default();

    if args.verbose > 0 {
        eprintln!("{} {} messages from stdin", terminal::info("Read"), terminal::number(&lines.len().to_string()));
    }

    let request_stream = stream::iter(lines);

    let response = client.call_bidi_streaming(service, method, request_stream).await?;

    if !response.is_ok() {
        eprintln!("{}: {} - {}", terminal::error("gRPC Error"),
            terminal::warning(&format!("{:?}", response.code())), response.message());
        return Ok(ExitStatus::Error);
    }

    println!("{}: {}", terminal::label("Status"), terminal::success(&format!("{:?}", response.code())));

    let formatter = ColorFormatter::new(ColorStyle::Auto);
    let mut stream = response.into_stream();
    let mut count = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(json) => {
                count += 1;
                let output = serde_json::to_string(&json).unwrap_or_default();
                println!("{}", formatter.format_json(&output));
            }
            Err(e) => {
                eprintln!("{}: {}", terminal::error("Stream error"), e);
                return Ok(ExitStatus::Error);
            }
        }
    }

    if args.verbose > 0 {
        eprintln!("{} {} messages", terminal::info("Received"), terminal::number(&count.to_string()));
    }

    Ok(ExitStatus::Success)
}
