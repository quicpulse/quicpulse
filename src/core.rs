use clap::Parser;

fn form_urlencode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut buf = [0u8; 4];
    for c in s.chars() {
        match c {
            ' ' => result.push('+'),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            _ => {
                let encoded = c.encode_utf8(&mut buf);
                for b in encoded.as_bytes() {
                    use std::fmt::Write;
                    let _ = write!(result, "%{:02X}", b);
                }
            }
        }
    }
    result
}

/// Serialize JSON to a compact string format
fn json_to_deterministic_format(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

use std::time::Instant;

use crate::bench::run_benchmark;
use crate::cli::{Args, process_args};
use crate::devexp::{generate_code, generate_curl_command, format_curl_pretty, import_curl, EnvVars};
use crate::fuzz::run_fuzz;
use crate::grpc::run_grpc;
use crate::har::run_har_replay;
use crate::openapi::run_openapi_import;
use crate::pipeline::{run_workflow, handle_workflow_commands};
use crate::websocket::{is_ws_request, run_websocket};
// PrettyOption is now shared between cli::args and output::writer
use crate::client::{send_request_with_session, check_status, IntermediateResponse, run_http3};
use crate::config::Config;
use crate::context::Environment;
use crate::downloads::Downloader;
use crate::errors::QuicpulseError;
use crate::filter;
use crate::internal;
use crate::output::formatters::{ColorFormatter, ColorStyle, format_json, JsonFormatterOptions};
use crate::output::pager::{PagerConfig, write_with_pager};
use crate::output::writer::{OutputOptions, ProcessingOptions, PrettyOption};
use crate::pipeline;
use crate::pipeline::assertions::{build_assertions, check_assertions};
use crate::sessions::Session;
use crate::status::ExitStatus;
use crate::table;
use crate::utils::url_as_host;

/// Main entry point for the CLI.
///
/// Handles argument parsing, configuration loading, and dispatches
/// to the appropriate handler (HTTP request, workflow, gRPC, etc.).
pub fn run(args: Vec<String>, mut env: Environment) -> ExitStatus {
    if let Some(name) = args.first() {
        if let Some(basename) = std::path::Path::new(name).file_stem() {
            env.program_name = basename.to_string_lossy().to_string();
        }
    }

    let config = match Config::load(&env) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Warning: Failed to load config: {}", e);
            Config::default()
        }
    };

    let merged_args = merge_default_options(args, &config);

    let debug = merged_args.iter().any(|a| a == "--debug");
    let traceback = merged_args.iter().any(|a| a == "--traceback") || debug;

    let parsed = match Args::try_parse_from(&merged_args) {
        Ok(args) => args,
        Err(e) => {
            e.print().ok();
            return if e.kind() == clap::error::ErrorKind::DisplayHelp
                || e.kind() == clap::error::ErrorKind::DisplayVersion {
                ExitStatus::Success
            } else {
                ExitStatus::Error
            };
        }
    };

    if let Some(shell) = &parsed.generate_completions {
        generate_completions(shell);
        return ExitStatus::Success;
    }

    if parsed.generate_manpage {
        generate_manpage();
        return ExitStatus::Success;
    }

    if parsed.update {
        match internal::self_update() {
            Ok(_version) => {
                eprintln!("Please restart quicpulse to use the new version.");
                return ExitStatus::Success;
            }
            Err(internal::UpdateError::AlreadyUpToDate) => {
                eprintln!("Already running the latest version ({})", env!("CARGO_PKG_VERSION"));
                return ExitStatus::Success;
            }
            Err(e) => {
                eprintln!("Update failed: {}", e);
                return ExitStatus::Error;
            }
        }
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    match runtime.block_on(program(parsed, env)) {
        Ok(status) => status,
        Err(e) => handle_error(e, traceback),
    }
}

pub async fn program(args: Args, env: Environment) -> Result<ExitStatus, QuicpulseError> {
    if args.debug {
        eprintln!("Debug: {:?}", args);
    }

    for dir in &args.script_allow_dirs {
        crate::scripting::modules::fs::allow_directory(dir);
    }

    // Handle workflow sharing commands
    if let Some(status) = handle_workflow_commands(&args, &env).await? {
        return Ok(status);
    }

    if let Some(ref workflow_path) = args.run_workflow {
        return run_workflow(&args, workflow_path, &env).await;
    }

    if let Some(ref har_path) = args.import_har {
        return run_har_replay(&args, har_path).await;
    }

    if let Some(ref openapi_path) = args.import_openapi {
        return run_openapi_import(&args, openapi_path, &env);
    }

    // Handle curl import
    if let Some(ref curl_cmd) = args.import_curl {
        let imported_args = import_curl(curl_cmd)?;
        // Merge with any additional args from the command line
        let merged = merge_curl_args(args.clone(), imported_args);
        // Recurse with the new args (but without import_curl to avoid infinite loop)
        let mut clean_args = merged;
        clean_args.import_curl = None;
        return Box::pin(program(clean_args, env)).await;
    }

    // Handle .http/.rest file import
    if let Some(ref http_file_path) = args.http_file {
        return handle_http_file(&args, http_file_path, &env).await;
    }

    // Handle mock server
    if args.mock_server {
        return run_mock_server(&args).await;
    }

    // Handle plugin commands
    if args.plugin_list {
        return handle_plugin_list(&args).await;
    }
    if let Some(ref query) = args.plugin_search {
        return handle_plugin_search(query, &args).await;
    }
    if let Some(ref name) = args.plugin_install {
        return handle_plugin_install(name, &args).await;
    }
    if let Some(ref name) = args.plugin_uninstall {
        return handle_plugin_uninstall(name, &args).await;
    }
    if args.plugin_update {
        return handle_plugin_update(&args).await;
    }

    if args.method.is_none() {
        eprintln!("usage: {} [METHOD] URL [REQUEST_ITEM ...]", env.program_name);
        eprintln!("\nFor more information, run: {} --help", env.program_name);
        return Ok(ExitStatus::Error);
    }

    let env_vars = load_env_vars(&args)?;

    let args = expand_args_variables(args, &env_vars)?;

    let processed = process_args(&args)?;

    if args.curl {
        let curl_cmd = generate_curl_command(&args, &processed);
        if env.stdout_isatty {
            println!("{}", format_curl_pretty(&curl_cmd));
        } else {
            println!("{}", curl_cmd);
        }
        return Ok(ExitStatus::Success);
    }

    // Handle code generation
    if let Some(ref lang) = args.generate_code {
        match generate_code(lang, &args, &processed) {
            Ok(code) => {
                println!("{}", code);
                return Ok(ExitStatus::Success);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                return Ok(ExitStatus::Error);
            }
        }
    }

    // Handle Unix socket requests
    #[cfg(unix)]
    if let Some(ref socket_path) = args.unix_socket {
        return handle_unix_socket_request(&args, &processed, socket_path, &env).await;
    }

    #[cfg(not(unix))]
    if args.unix_socket.is_some() {
        eprintln!("Error: Unix domain sockets are not supported on this platform");
        return Ok(ExitStatus::Error);
    }

    let config = Config::load(&env).unwrap_or_default();
    let host = url_as_host(&processed.url);
    let mut session = load_session(&args, &host, &config)?;

    let mut args = args;
    prompt_auth_if_needed(&mut args, &host)?;

    if args.offline {
        print_offline_request(&processed, &args, &env);
        return Ok(ExitStatus::Success);
    }

    if args.bench {
        return run_benchmark(args, processed, env).await;
    }

    if args.fuzz {
        return run_fuzz(args, processed, env).await;
    }

    if crate::grpc::is_grpc_request(&args) {
        return run_grpc(&args, &processed, &env).await;
    }

    if is_ws_request(&args) {
        return run_websocket(&args, &processed, &env).await;
    }

    // Use dedicated HTTP/3 client when --http3 is specified
    if args.http3 && processed.url.starts_with("https://") {
        return run_http3(&args, &processed, &env, session.as_ref()).await;
    }

    let proc_opts = build_processing_options(&args, &env);

    if args.verbose > 0 {
        print_request(&processed, &args, &env, &proc_opts);
    }

    let mut downloader = if args.download {
        Some(Downloader::new(args.output.clone(), args.continue_download))
    } else {
        None
    };

    let download_headers = if let Some(ref dl) = downloader {
        let mut headers = reqwest::header::HeaderMap::new();
        dl.pre_request(&mut headers);
        Some(headers)
    } else {
        None
    };

    let request_start = Instant::now();

    let result = send_request_with_session(
        &args,
        &processed,
        &env,
        session.as_ref(),
        download_headers.as_ref(),
    ).await?;

    if args.all && !result.intermediate_responses.is_empty() {
        for intermediate in &result.intermediate_responses {
            print_intermediate_response(intermediate, &args, &env);
        }
    }

    let response_time = request_start.elapsed();

    let status_code = result.response.status().as_u16();
    let response_headers = result.response.headers().clone();

    if let Some(ref mut sess) = session {
        update_session_from_response(sess, &response_headers, &host);

        if let Some(ref auth_str) = args.auth {
            let auth_type = match args.auth_type {
                Some(crate::cli::args::AuthType::Bearer) => "bearer",
                Some(crate::cli::args::AuthType::Ntlm) => "ntlm",
                Some(crate::cli::args::AuthType::Negotiate) => "negotiate",
                Some(crate::cli::args::AuthType::Kerberos) => "kerberos",
                Some(crate::cli::args::AuthType::Digest) => "digest",
                _ => "basic",
            };
            sess.set_auth(auth_type, auth_str);
        }
    }

    let response_body = if let Some(ref mut dl) = downloader {
        download_response(result.response, dl, &args, &env).await?;
        String::new()
    } else {
        print_response_with_body(result.response, &args, &env).await?
    };

    save_session(&args, session, &host, &config)?;

    if pipeline::has_assertions(&args) {
        let assertions = build_assertions(&args);

        let is_download_mode = downloader.is_some();
        if is_download_mode {
            let has_body_assertions = assertions.iter().any(|a| {
                matches!(a, pipeline::Assertion::Body(_))
            });
            if has_body_assertions {
                eprintln!("\x1b[33mWarning: Body assertions are skipped in download mode (--download).\x1b[0m");
                eprintln!("The response body is streamed to disk and not available for assertions.");
            }
        }

        let results = check_assertions(&assertions, status_code, response_time, &response_headers, &response_body);

        let results: Vec<_> = if is_download_mode {
            results.into_iter().filter(|r| {
                r.passed || !r.assertion.contains("body")
            }).collect()
        } else {
            results
        };

        let all_passed = results.iter().all(|r| r.passed);
        if !all_passed || args.verbose > 0 {
            eprintln!("\nAssertions:");
            for result in &results {
                let icon = if result.passed { "✓" } else { "✗" };
                eprintln!("  {} {}: {}", icon, result.assertion, result.message);
            }
        }

        if !all_passed {
            return Ok(ExitStatus::from_code(pipeline::EXIT_ASSERTION_FAILED));
        }
    }

    let exit_status = check_status(status_code, args.check_status);

    Ok(exit_status)
}

fn print_request(
    processed: &crate::cli::parser::ProcessedArgs,
    args: &Args,
    _env: &Environment,
    proc_opts: &ProcessingOptions,
) {
    use crate::input::InputItem;

    let mut request_str = format!("{} {} HTTP/1.1\n", processed.method, processed.url);

    request_str.push_str(&format!("User-Agent: {}\n", crate::client::USER_AGENT_STRING));

    if !args.form && !args.multipart {
        request_str.push_str("Accept: application/json, */*;q=0.5\n");
    }

    if processed.has_data {
        if args.form {
            request_str.push_str("Content-Type: application/x-www-form-urlencoded; charset=utf-8\n");
        } else if !args.multipart {
            request_str.push_str("Content-Type: application/json\n");
        }
    }

    for item in &processed.items {
        match item {
            InputItem::Header { name, value } => {
                request_str.push_str(&format!("{}: {}\n", name, value));
            }
            InputItem::EmptyHeader { name } => {
                request_str.push_str(&format!("{}:\n", name));
            }
            _ => {}
        }
    }

    let formatter = if proc_opts.colors && proc_opts.pretty != PrettyOption::None {
        Some(ColorFormatter::new(proc_opts.style.clone()))
    } else {
        None
    };

    let formatted = if let Some(ref fmt) = formatter {
        fmt.format_headers(&request_str)
    } else {
        request_str
    };

    print!("{}", formatted);

    if args.verbose > 1 && processed.has_data {
        let data_items: Vec<_> = processed.items.iter()
            .filter(|item| item.is_data())
            .collect();

        if !data_items.is_empty() {
            println!();

            if args.form {
                let form_body: Vec<String> = data_items.iter()
                    .filter_map(|item| {
                        match item {
                            InputItem::DataField { key, value } => Some(format!("{}={}", key, value)),
                            InputItem::DataFieldFile { key, path } => {
                                std::fs::read_to_string(path).ok().map(|v| format!("{}={}", key, v.trim()))
                            }
                            _ => None,
                        }
                    })
                    .collect();
                let body_str = form_body.join("&");
                if let Some(fmt) = formatter {
                    println!("{}", fmt.format_by_mime(&body_str, "application/x-www-form-urlencoded"));
                } else {
                    println!("{}", body_str);
                }
            } else {
                let mut body = serde_json::json!({});
                for item in data_items {
                    if let Some(map) = body.as_object_mut() {
                        let (key, value) = match item {
                            InputItem::DataField { key, value } => {
                                (key.clone(), serde_json::Value::String(value.clone()))
                            }
                            InputItem::DataFieldFile { key, path } => {
                                let content = std::fs::read_to_string(path).unwrap_or_default();
                                (key.clone(), serde_json::Value::String(content.trim().to_string()))
                            }
                            InputItem::JsonField { key, value } => {
                                (key.clone(), value.clone())
                            }
                            InputItem::JsonFieldFile { key, path } => {
                                let content = std::fs::read_to_string(path).unwrap_or_default();
                                let json_val = serde_json::from_str(&content).unwrap_or(serde_json::Value::String(content));
                                (key.clone(), json_val)
                            }
                            _ => continue,
                        };
                        map.insert(key, value);
                    }
                }

                if let Ok(formatted_body) = format_json(&body.to_string(), &proc_opts.json) {
                    let colored = if let Some(fmt) = formatter {
                        fmt.format_json(&formatted_body)
                    } else {
                        formatted_body
                    };
                    println!("{}", colored);
                }
            }
        }
    }

    println!();
}

fn print_intermediate_response(
    intermediate: &IntermediateResponse,
    args: &Args,
    env: &Environment,
) {
    let output_opts = build_output_options(args, env);
    let proc_opts = build_processing_options(args, env);

    let formatter = if proc_opts.colors && proc_opts.pretty != PrettyOption::None {
        Some(ColorFormatter::new(proc_opts.style.clone()))
    } else {
        None
    };

    let status_text = reqwest::StatusCode::from_u16(intermediate.status)
        .map(|s| s.canonical_reason().unwrap_or(""))
        .unwrap_or("");

    if output_opts.response_headers {
        let status_line = format!("HTTP/1.1 {} {}", intermediate.status, status_text);

        let mut headers_str = status_line;
        headers_str.push('\n');

        let mut header_vec: Vec<(&reqwest::header::HeaderName, &reqwest::header::HeaderValue)> =
            intermediate.headers.iter().collect();
        header_vec.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

        for (name, value) in header_vec {
            let value_str = value.to_str().unwrap_or("<binary>");
            headers_str.push_str(&format!("{}: {}\n", name.as_str(), value_str));
        }

        let formatted = if let Some(ref fmt) = formatter {
            fmt.format_headers(&headers_str)
        } else {
            headers_str
        };

        print!("{}", formatted);
        println!();
    }
}

fn print_offline_request(processed: &crate::cli::parser::ProcessedArgs, args: &Args, env: &Environment) {
    use crate::input::InputItem;
    use std::collections::HashMap;

    if args.quiet > 0 {
        return;
    }

    let output_opts = build_output_options(args, env);

    let mut url = processed.url.clone();
    let query_items: Vec<_> = processed.items.iter()
        .filter(|item| item.is_query())
        .collect();

    if !query_items.is_empty() {
        let query_string: String = query_items.iter()
            .filter_map(|item| {
                match item {
                    InputItem::QueryParam { name, value } => Some(format!("{}={}",
                        form_urlencode(name),
                        form_urlencode(value))),
                    InputItem::QueryParamFile { name, path } => {
                        std::fs::read_to_string(path).ok().map(|v| format!("{}={}",
                            form_urlencode(name),
                            form_urlencode(v.trim())))
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>()
            .join("&");

        if url.contains('?') {
            url = format!("{}&{}", url, query_string);
        } else {
            url = format!("{}?{}", url, query_string);
        }
    }

    if output_opts.request_headers {
        let (path_and_query, url_credentials) = if let Ok(parsed) = url::Url::parse(&url) {
             let path = if args.path_as_is {
                 let url_without_scheme = url.split("://").nth(1).unwrap_or(&url);
                 let path_start = url_without_scheme.find('/');
                 let query_start = url_without_scheme.find('?');
                 match (path_start, query_start) {
                     (Some(ps), Some(qs)) if ps < qs => url_without_scheme[ps..qs].to_string() + &url_without_scheme[qs..],
                     (Some(ps), _) => url_without_scheme[ps..].to_string(),
                     (None, Some(qs)) => "/".to_string() + &url_without_scheme[qs..],
                     (None, None) => "/".to_string(),
                 }
             } else {
                 let mut p = parsed.path().to_string();
                 if let Some(q) = parsed.query() {
                     p.push('?');
                     p.push_str(q);
                 }
                 if p.is_empty() { "/".to_string() } else { p }
             };

             let creds = if args.auth.is_none() && !parsed.username().is_empty() {
                 Some((parsed.username().to_string(), parsed.password().map(|p| p.to_string())))
             } else {
                 None
             };
             (path, creds)
        } else {
            (url.clone(), None)
        };

        let should_sort = match &args.pretty {
            Some(crate::cli::args::PrettyOption::All) | Some(crate::cli::args::PrettyOption::Format) => !args.unsorted,
            _ => false,
        };

        // Build processing options for colorization
        let proc_opts = build_processing_options(args, env);
        let formatter = if proc_opts.colors && proc_opts.pretty != PrettyOption::None {
            Some(ColorFormatter::new(proc_opts.style.clone()))
        } else {
            None
        };

        // Build request line as part of headers string for consistent formatting
        let request_line = format!("{} {} HTTP/1.1", processed.method, path_and_query);

        let mut final_headers_list: Vec<(String, String)> = Vec::new();
        let mut custom_headers: HashMap<String, String> = HashMap::new();

        for item in &processed.items {
            match item {
                InputItem::Header { name, value } => {
                    let key_lower = name.to_lowercase();
                    if value.is_empty() {
                        final_headers_list.retain(|(k, _)| k.to_lowercase() != key_lower);
                        custom_headers.insert(key_lower, String::new());
                    } else {
                        final_headers_list.push((name.clone(), value.clone()));
                        custom_headers.insert(key_lower, value.clone());
                    }
                }
                InputItem::EmptyHeader { name } => {
                    let key_lower = name.to_lowercase();
                    final_headers_list.push((name.clone(), String::new()));
                    custom_headers.insert(key_lower, String::new());
                }
                InputItem::HeaderFile { name, path } => {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        let key_lower = name.to_lowercase();
                        final_headers_list.push((name.clone(), content.trim().to_string()));
                        custom_headers.insert(key_lower, content.trim().to_string());
                    }
                }
                _ => {}
            }
        }

        let mut all_headers: Vec<(String, String)> = Vec::new();

        let accept_overridden = custom_headers.contains_key("accept");
        let is_form_mode = args.form || args.multipart;
        let is_empty_raw = args.raw.as_ref().map_or(false, |r| r.is_empty());
        let has_nonempty_raw = args.raw.as_ref().map_or(false, |r| !r.is_empty());
        let has_raw = args.raw.is_some();
        let is_json_mode = !is_form_mode && !is_empty_raw && ((!has_raw && processed.has_data) || has_nonempty_raw);
        let method_upper = processed.method.to_uppercase();
        let needs_content_length_zero = !matches!(method_upper.as_str(), "GET" | "HEAD" | "OPTIONS");

        let accept_encoding_overridden = custom_headers.contains_key("accept-encoding");
        let connection_overridden = custom_headers.contains_key("connection");

        if !accept_encoding_overridden {
            all_headers.push(("Accept-Encoding".to_string(), "gzip, deflate".to_string()));
        }

        if !is_json_mode && !accept_overridden {
            all_headers.push(("Accept".to_string(), "*/*".to_string()));
        }

        if !connection_overridden {
            all_headers.push(("Connection".to_string(), "keep-alive".to_string()));
        }

        if !custom_headers.contains_key("content-length") {
            if processed.has_data {
                all_headers.push(("Content-Length".to_string(), "<LEN>".to_string()));
            } else if needs_content_length_zero {
                all_headers.push(("Content-Length".to_string(), "0".to_string()));
            }
        }

        if !custom_headers.contains_key("authorization") {
            if let Some(ref auth) = args.auth {
                match args.auth_type {
                    Some(crate::cli::args::AuthType::Bearer) => {
                        all_headers.push(("Authorization".to_string(), format!("Bearer {}", auth)));
                    },
                    Some(crate::cli::args::AuthType::Digest) => {
                    },
                    _ => {
                        let mut parts = auth.splitn(2, ':');
                        let user = parts.next().unwrap_or("");
                        let pass = parts.next().unwrap_or("");
                        let creds = format!("{}:{}", user, pass);
                        use base64::Engine;
                        let encoded = base64::engine::general_purpose::STANDARD.encode(creds);
                        all_headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
                    }
                }
            } else if let Some((username, password)) = &url_credentials {
                use base64::Engine;
                let creds = format!("{}:{}", username, password.as_deref().unwrap_or(""));
                let encoded = base64::engine::general_purpose::STANDARD.encode(creds);
                all_headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
            }
        }

        if let Some(val) = custom_headers.get("user-agent") {
            if !val.is_empty() { all_headers.push(("User-Agent".to_string(), val.clone())); }
        } else {
            all_headers.push(("User-Agent".to_string(), crate::client::USER_AGENT_STRING.to_string()));
        }

        if accept_encoding_overridden {
            if let Some(val) = custom_headers.get("accept-encoding") {
                if !val.is_empty() { all_headers.push(("Accept-Encoding".to_string(), val.clone())); }
            }
        }
        if accept_overridden {
            if let Some(val) = custom_headers.get("accept") {
                if !val.is_empty() { all_headers.push(("Accept".to_string(), val.clone())); }
            }
        }
        if connection_overridden {
            if let Some(val) = custom_headers.get("connection") {
                if !val.is_empty() { all_headers.push(("Connection".to_string(), val.clone())); }
            }
        }
        if custom_headers.contains_key("authorization") && args.auth.is_none() {
            if let Some(val) = custom_headers.get("authorization") {
                if !val.is_empty() { all_headers.push(("Authorization".to_string(), val.clone())); }
            }
        }

        if !accept_overridden && is_json_mode {
            all_headers.push(("Accept".to_string(), "application/json, */*;q=0.5".to_string()));
        }

        if let Some(ct) = custom_headers.get("content-type") {
            if !ct.is_empty() { all_headers.push(("Content-Type".to_string(), ct.clone())); }
        } else if is_form_mode {
            if args.form {
                all_headers.push(("Content-Type".to_string(), "application/x-www-form-urlencoded; charset=utf-8".to_string()));
            } else if args.multipart {
                all_headers.push(("Content-Type".to_string(), "multipart/form-data; boundary=----WebKitFormBoundary".to_string()));
            }
        } else if is_json_mode {
            all_headers.push(("Content-Type".to_string(), "application/json".to_string()));
        }

        if args.chunked && !custom_headers.contains_key("transfer-encoding") {
            all_headers.push(("Transfer-Encoding".to_string(), "chunked".to_string()));
        }

        let standard_headers = ["accept-encoding", "accept", "connection", "content-length", "authorization", "user-agent", "content-type", "transfer-encoding", "host"];
        for (key, value) in &final_headers_list {
            let key_lower = key.to_lowercase();
            if !standard_headers.contains(&key_lower.as_str()) {
                all_headers.push((key.clone(), value.clone()));
            }
        }

        if let Some(val) = custom_headers.get("host") {
            if !val.is_empty() { all_headers.push(("Host".to_string(), val.clone())); }
        } else if let Ok(parsed_url) = url::Url::parse(&url) {
            if let Some(host) = parsed_url.host_str() {
                let host_val = if let Some(port) = parsed_url.port() {
                    format!("{}:{}", host, port)
                } else {
                    host.to_string()
                };
                all_headers.push(("Host".to_string(), host_val));
            }
        }

        if should_sort {
            all_headers.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        }

        // Build complete headers string for ColorFormatter
        let mut headers_str = request_line;
        headers_str.push('\n');

        for (key, value) in &all_headers {
            headers_str.push_str(&format!("{}: {}\n", key, value));
        }

        // Apply color formatting
        let formatted = if let Some(ref fmt) = formatter {
            fmt.format_headers(&headers_str)
        } else {
            headers_str
        };

        print!("{}", formatted);
        println!();
    }

    if output_opts.request_body && processed.has_data {
        let data_items: Vec<_> = processed.items.iter()
            .filter(|item| item.is_data())
            .collect();
        if !data_items.is_empty() {
            // Build processing options for body colorization
            let proc_opts = build_processing_options(args, env);
            let body_formatter = if proc_opts.colors && proc_opts.pretty != PrettyOption::None {
                Some(ColorFormatter::new(proc_opts.style.clone()))
            } else {
                None
            };

            let pretty = if let Some(p) = &args.pretty {
                 match p {
                     crate::cli::args::PrettyOption::All |
                     crate::cli::args::PrettyOption::Format |
                     crate::cli::args::PrettyOption::Colors => true,
                     crate::cli::args::PrettyOption::None => false,
                 }
            } else {
                 env.stdout_isatty
            };

            if args.form {
                let form_body: String = data_items.iter()
                    .filter_map(|item| {
                        match item {
                            InputItem::DataField { key, value } => {
                                Some(format!("{}={}",
                                    form_urlencode(key),
                                    form_urlencode(value)))
                            }
                            InputItem::DataFieldFile { key, path } => {
                                let value = std::fs::read_to_string(path).unwrap_or_default();
                                Some(format!("{}={}",
                                    form_urlencode(key),
                                    form_urlencode(value.trim())))
                            }
                            InputItem::JsonField { key, value } => {
                                // In form mode, JSON values are converted to their string representation
                                let value_str = value.to_string();
                                Some(format!("{}={}",
                                    form_urlencode(key),
                                    form_urlencode(&value_str)))
                            }
                            InputItem::JsonFieldFile { key, path } => {
                                let content = std::fs::read_to_string(path).unwrap_or_default();
                                Some(format!("{}={}",
                                    form_urlencode(key),
                                    form_urlencode(content.trim())))
                            }
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("&");
                if pretty {
                    println!("{}", form_body);
                } else {
                    print!("{}", form_body);
                }
            } else if args.multipart {
                for item in &data_items {
                    let (key, value) = match item {
                        InputItem::DataField { key, value } => (key.clone(), value.clone()),
                        InputItem::DataFieldFile { key, path } => {
                            let value = std::fs::read_to_string(path).unwrap_or_default();
                            (key.clone(), value.trim().to_string())
                        }
                        _ => continue,
                    };
                    println!("--<BOUNDARY>");
                    println!("Content-Disposition: form-data; name=\"{}\"", key);
                    println!();
                    println!("{}", value);
                }
                println!("--<BOUNDARY>");
            } else {
                let mut pairs: Vec<(String, serde_json::Value)> = Vec::new();
                for item in data_items {
                    let (key, value) = match item {
                        InputItem::DataField { key, value } => {
                            (key.clone(), serde_json::Value::String(value.clone()))
                        }
                        InputItem::DataFieldFile { key, path } => {
                            let content = std::fs::read_to_string(path).unwrap_or_default();
                            (key.clone(), serde_json::Value::String(content.trim().to_string()))
                        }
                        InputItem::JsonField { key, value } => {
                            (key.clone(), value.clone())
                        }
                        InputItem::JsonFieldFile { key, path } => {
                            let content = std::fs::read_to_string(path).unwrap_or_default();
                            let json_val = serde_json::from_str(&content)
                                .unwrap_or(serde_json::Value::String(content));
                            (key.clone(), json_val)
                        }
                        _ => continue,
                    };
                    pairs.push((key, value));
                }

                let sort_keys = match &args.pretty {
                    Some(crate::cli::args::PrettyOption::All) | Some(crate::cli::args::PrettyOption::Format) => !args.unsorted,
                    _ => false,
                };
                if sort_keys {
                    pairs.sort_by(|a, b| a.0.cmp(&b.0));
                }

                let mut body = serde_json::Map::new();
                for (key, value) in pairs {
                    body.insert(key, value);
                }

                let json_body = serde_json::Value::Object(body);
                let formatted = if matches!(&args.pretty, Some(crate::cli::args::PrettyOption::All) | Some(crate::cli::args::PrettyOption::Format)) {
                    let mut buf = Vec::new();
                    let json_formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
                    let mut ser = serde_json::Serializer::with_formatter(&mut buf, json_formatter);
                    use serde::Serialize;
                    json_body.serialize(&mut ser).ok();
                    String::from_utf8(buf).unwrap_or_else(|_| json_to_deterministic_format(&json_body))
                } else {
                    json_to_deterministic_format(&json_body)
                };
                // Apply color formatting to JSON
                let colored = if let Some(ref fmt) = body_formatter {
                    fmt.format_json(&formatted)
                } else {
                    formatted
                };
                if pretty {
                    println!("{}", colored);
                } else {
                    print!("{}", colored);
                }
            }
        }

        if let Some(ref raw_data) = args.raw {
            print!("{}", raw_data);
        }
    }
}

#[allow(dead_code)]
async fn print_response(
    mut response: reqwest::Response,
    args: &Args,
    env: &Environment,
) -> Result<(), QuicpulseError> {
    let status = response.status();
    let headers = response.headers().clone();
    let content_type = headers.get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let output_opts = build_output_options(args, env);

    let proc_opts = build_processing_options(args, env);

    let formatter = if proc_opts.colors && proc_opts.pretty != PrettyOption::None {
        Some(ColorFormatter::new(proc_opts.style.clone()))
    } else {
        None
    };

    if output_opts.response_headers && args.quiet == 0 {
        let status_line = format!(
            "HTTP/1.1 {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        );

        let mut headers_str = status_line;
        headers_str.push('\n');

        for (name, value) in headers.iter() {
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

        if output_opts.response_body {
            println!();
        }
    }

    if output_opts.response_body && args.quiet < 2 {
        if args.stream {
            use std::io::Write;
            let mut stdout = std::io::stdout();
            let mut total_bytes: u64 = 0;
            const STREAM_MAX_SIZE: u64 = 100 * 1024 * 1024;

            while let Some(chunk) = response.chunk().await.map_err(QuicpulseError::Request)? {
                total_bytes += chunk.len() as u64;
                if total_bytes > STREAM_MAX_SIZE {
                    eprintln!("\n\x1b[33mWarning: Stream exceeded {} bytes, stopping.\x1b[0m", STREAM_MAX_SIZE);
                    break;
                }
                let _ = stdout.write_all(&chunk);
                let _ = stdout.flush();
            }
            println!();
        } else {
            const MAX_BODY_SIZE: u64 = 100 * 1024 * 1024;

            let content_length = response.content_length();
            if let Some(len) = content_length {
                if len > MAX_BODY_SIZE {
                    eprintln!("\x1b[33mWarning: Response body too large ({} bytes) to print to stdout.\x1b[0m", len);
                    eprintln!("Use --download to save large responses to a file.");
                    return Ok(());
                }
            }

            let body = read_body_with_limit(response, MAX_BODY_SIZE, env.stdout_isatty).await?;

            if !body.is_empty() {
                let output = process_response_body(
                    &body,
                    content_type.as_deref(),
                    args,
                    &proc_opts,
                    formatter.as_ref(),
                )?;
                println!("{}", output);
            }
        }
    }

    Ok(())
}

async fn read_body_with_limit(mut response: reqwest::Response, max_size: u64, stdout_isatty: bool) -> Result<String, QuicpulseError> {
    let mut body_bytes = Vec::new();
    let mut total_bytes: u64 = 0;

    while let Some(chunk) = response.chunk().await.map_err(QuicpulseError::Request)? {
        total_bytes += chunk.len() as u64;

        if total_bytes > max_size {
            eprintln!("\x1b[33mWarning: Response body too large (>{} bytes), truncating output.\x1b[0m", max_size);
            eprintln!("Use --download to save large responses to a file.");
            body_bytes.extend_from_slice(&chunk[..(max_size as usize - (total_bytes - chunk.len() as u64) as usize).min(chunk.len())]);
            let body = String::from_utf8_lossy(&body_bytes).into_owned();
            return Ok(format!("{}\n... [truncated, use --download for full response]", body));
        }

        body_bytes.extend_from_slice(&chunk);
    }

    if stdout_isatty && crate::utils::is_binary(&body_bytes) {
        eprintln!("\x1b[33mWarning: Binary content detected ({} bytes).\x1b[0m", body_bytes.len());
        eprintln!("Use --download to save binary responses to a file.");
        return Ok(format!("[Binary data, {} bytes - use --download to save]", body_bytes.len()));
    }

    Ok(String::from_utf8_lossy(&body_bytes).into_owned())
}

async fn download_response(
    response: reqwest::Response,
    downloader: &mut Downloader,
    args: &Args,
    env: &Environment,
) -> Result<(), QuicpulseError> {
    let status = response.status();
    let headers = response.headers().clone();
    let url = response.url().to_string();

    if args.quiet == 0 {
        let proc_opts = build_processing_options(args, env);
        let formatter = if proc_opts.colors && proc_opts.pretty != PrettyOption::None {
            Some(ColorFormatter::new(proc_opts.style.clone()))
        } else {
            None
        };

        let status_line = format!(
            "HTTP/1.1 {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        );

        let mut headers_str = status_line;
        headers_str.push('\n');

        for (name, value) in headers.iter() {
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

    let output_path = downloader.start(&url, &response).await?;

    if args.quiet == 0 {
        eprintln!("Downloading to \"{}\"", output_path.display());
        if let Some(size) = downloader.total_size {
            let size_str = format_size(size);
            if downloader.resumed_from > 0 {
                let resumed_str = format_size(downloader.resumed_from);
                eprintln!("Resuming from {} ({} total)", resumed_str, size_str);
            } else {
                eprintln!("Size: {}", size_str);
            }
        }
    }

    let bytes_downloaded = downloader.download_body(response).await?;

    if args.quiet == 0 {
        eprintln!("Done. {} downloaded.", format_size(bytes_downloaded + downloader.resumed_from));
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn build_output_options(args: &Args, env: &Environment) -> OutputOptions {
    if let Some(ref print_str) = args.print {
        return OutputOptions::from_str(print_str);
    }

    if args.headers_only {
        return OutputOptions::headers_only();
    }

    if args.body {
        return OutputOptions::body_only();
    }

    if args.verbose > 0 {
        return OutputOptions::verbose();
    }

    if args.offline {
        return OutputOptions {
            request_headers: true,
            request_body: true,
            response_headers: false,
            response_body: false,
            metadata: false,
        };
    }

    if env.stdout_isatty {
        OutputOptions::default_output()
    } else {
        OutputOptions::body_only()
    }
}

fn build_processing_options(args: &Args, env: &Environment) -> ProcessingOptions {
    // Determine colors:
    // 1. --no-color always disables colors
    // 2. Explicit --pretty=colors or --pretty=all forces colors (even without TTY)
    // 3. Otherwise use TTY detection
    let colors = if args.no_color {
        false
    } else {
        match args.pretty {
            // Explicit color request forces colors
            Some(PrettyOption::Colors) | Some(PrettyOption::All) => true,
            // Explicit no-color request
            Some(PrettyOption::None) | Some(PrettyOption::Format) => false,
            // Default: use TTY detection
            None => env.stdout_isatty,
        }
    };

    let pretty = match args.pretty {
        Some(opt) => opt,
        None => {
            if env.stdout_isatty {
                PrettyOption::All
            } else {
                PrettyOption::Format
            }
        }
    };

    let style = match &args.style {
        Some(s) => ColorStyle::parse(s),
        None => ColorStyle::Auto,
    };

    let mut json_opts = JsonFormatterOptions::default();
    for opt in &args.format_options {
        if let Some((key, value)) = opt.split_once(':') {
            match key {
                "json.indent" => {
                    if let Ok(n) = value.parse() {
                        json_opts.indent = n;
                    }
                }
                "json.sort_keys" => {
                    json_opts.sort_keys = value == "true";
                }
                _ => {}
            }
        }
    }

    if args.sorted {
        json_opts.sort_keys = true;
    }
    if args.unsorted {
        json_opts.sort_keys = false;
    }

    ProcessingOptions {
        style,
        json: json_opts,
        colors,
        pretty,
    }
}

fn format_response_body(
    body: &str,
    content_type: Option<&str>,
    opts: &ProcessingOptions,
    formatter: Option<&ColorFormatter>,
) -> String {
    let mime = content_type.unwrap_or("text/plain");
    let base_mime = mime.split(';').next().unwrap_or(mime).trim();

    if base_mime == "application/json" || base_mime.ends_with("+json") {
        if matches!(opts.pretty, PrettyOption::All | PrettyOption::Format) {
            if let Ok(formatted) = format_json(body, &opts.json) {
                if let Some(fmt) = formatter {
                    return fmt.format_json(&formatted);
                }
                return formatted;
            }
        }
    }

    if let Some(fmt) = formatter {
        fmt.format_by_mime(body, base_mime)
    } else {
        body.to_string()
    }
}

fn merge_default_options(args: Vec<String>, config: &Config) -> Vec<String> {
    if config.default_options.is_empty() {
        return args;
    }

    let (flags, positional): (Vec<_>, Vec<_>) = config.default_options.iter()
        .partition(|opt| opt.starts_with('-'));

    if !positional.is_empty() {
        eprintln!("\x1b[33mWarning: Positional arguments in default_options are ignored: {:?}\x1b[0m", positional);
        eprintln!("Only flags (starting with -) can be used in default_options.");
        eprintln!("For default URLs, use --base-url or environment variables instead.");
    }

    if flags.is_empty() {
        return args;
    }

    let mut merged = Vec::with_capacity(args.len() + flags.len());

    if let Some(program) = args.first() {
        merged.push(program.clone());
    }

    merged.extend(flags.into_iter().cloned());
    merged.extend(args.into_iter().skip(1));

    merged
}

fn handle_error(error: QuicpulseError, traceback: bool) -> ExitStatus {
    if traceback {
        eprintln!("Error: {:?}", error);
    } else {
        eprintln!("Error: {}", error);
    }

    // All errors return the same exit code (1) following Unix conventions
    ExitStatus::Error
}

fn prompt_auth_if_needed(args: &mut Args, host: &str) -> Result<(), QuicpulseError> {
    use crate::cli::args::AuthType;
    use crate::auth::Netrc;

    if args.auth.is_none() && !args.ignore_netrc {
        if let Some(netrc) = Netrc::load() {
            if let Some((login, password)) = netrc.get_credentials(host) {
                args.auth = Some(format!("{}:{}", login, password).into());
                if args.auth_type.is_none() {
                    args.auth_type = Some(AuthType::Basic);
                }
            }
        }
    }

    let auth_str = match &args.auth {
        Some(a) => a.clone(),
        None => return Ok(()),
    };

    let auth_type = args.auth_type.clone().unwrap_or(AuthType::Basic);

    if matches!(auth_type, AuthType::Bearer) {
        return Ok(());
    }

    if !auth_str.contains(':') {
        use std::io::{self, Write};
        eprint!("http: password for {}@{}: ", auth_str, host);
        io::stderr().flush().map_err(QuicpulseError::Io)?;

        let password = rpassword::read_password()
            .map_err(|e| QuicpulseError::Auth(format!("Failed to read password: {}", e)))?;

        args.auth = Some(format!("{}:{}", auth_str, password).into());
    }

    Ok(())
}

fn load_session(
    args: &Args,
    host: &str,
    config: &Config,
) -> Result<Option<Session>, QuicpulseError> {
    let session_name = match &args.session {
        Some(name) => name,
        None => return Ok(None),
    };

    if session_name.contains('/') || session_name.contains('\\') {
        let path = std::path::Path::new(session_name);
        if path.exists() {
            Ok(Some(Session::load(path)?))
        } else {
            Ok(Some(Session::new()))
        }
    } else {
        Ok(Some(Session::load_named(session_name, host, config)?))
    }
}

fn update_session_from_response(
    session: &mut Session,
    headers: &reqwest::header::HeaderMap,
    host: &str,
) {
    for value in headers.get_all("set-cookie") {
        if let Ok(cookie_str) = value.to_str() {
            session.parse_set_cookie(cookie_str, host);
        }
    }
}

fn save_session(
    args: &Args,
    session: Option<Session>,
    host: &str,
    config: &Config,
) -> Result<(), QuicpulseError> {
    let session = match session {
        Some(s) => s,
        None => return Ok(()),
    };

    if args.session_read_only.is_some() {
        return Ok(());
    }

    let session_name = match &args.session {
        Some(name) => name,
        None => return Ok(()),
    };

    if session_name.contains('/') || session_name.contains('\\') {
        let path = std::path::Path::new(session_name);
        session.save(path)
    } else {
        session.save_named(session_name, host, config)
    }
}

fn generate_completions(shell: &crate::cli::args::Shell) {
    use clap::CommandFactory;
    use clap_complete::{generate, Shell as ClapShell};

    let mut cmd = Args::command();
    let shell = match shell {
        crate::cli::args::Shell::Bash => ClapShell::Bash,
        crate::cli::args::Shell::Zsh => ClapShell::Zsh,
        crate::cli::args::Shell::Fish => ClapShell::Fish,
        crate::cli::args::Shell::PowerShell => ClapShell::PowerShell,
        crate::cli::args::Shell::Elvish => ClapShell::Elvish,
    };

    generate(shell, &mut cmd, "http", &mut std::io::stdout());
}

fn generate_manpage() {
    use clap::CommandFactory;

    let cmd = Args::command();
    let man = clap_mangen::Man::new(cmd);
    man.render(&mut std::io::stdout()).expect("Failed to generate man page");
}

fn process_response_body(
    body: &str,
    content_type: Option<&str>,
    args: &Args,
    proc_opts: &ProcessingOptions,
    formatter: Option<&ColorFormatter>,
) -> Result<String, QuicpulseError> {
    let mime = content_type.unwrap_or("text/plain");
    let base_mime = mime.split(';').next().unwrap_or(mime).trim();
    let is_json = base_mime == "application/json" || base_mime.ends_with("+json");

    if is_json || args.filter.is_some() || args.table || args.csv {
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(ref filter_expr) = args.filter {
                let results = filter::apply_filter(&json, filter_expr)?;

                if results.len() == 1 {
                    json = results.into_iter().next().unwrap();
                } else {
                    json = serde_json::Value::Array(results);
                }
            }

            if args.table {
                return table::format_as_table(&json);
            }

            if args.csv {
                return table::format_as_csv(&json);
            }

            if matches!(proc_opts.pretty, PrettyOption::All | PrettyOption::Format) {
                // Format JSON with indentation
                let formatted = serde_json::to_string_pretty(&json)
                    .unwrap_or_else(|_| json.to_string());
                if let Some(fmt) = formatter {
                    return Ok(fmt.format_json(&formatted));
                }
                return Ok(formatted);
            }

            // No formatting (compact JSON), but still apply colors if requested
            let compact = json.to_string();
            if let Some(fmt) = formatter {
                return Ok(fmt.format_json(&compact));
            }
            return Ok(compact);
        }
    }

    Ok(format_response_body(body, content_type, proc_opts, formatter))
}

async fn print_response_with_body(
    response: reqwest::Response,
    args: &Args,
    env: &Environment,
) -> Result<String, QuicpulseError> {
    let status = response.status();
    let headers = response.headers().clone();
    let content_type = headers.get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let output_opts = build_output_options(args, env);

    let proc_opts = build_processing_options(args, env);

    let formatter = if proc_opts.colors && proc_opts.pretty != PrettyOption::None {
        Some(ColorFormatter::new(proc_opts.style.clone()))
    } else {
        None
    };

    // Build pager config
    let pager_config = PagerConfig {
        enabled: args.pager && !args.no_pager && env.stdout_isatty,
        command: None, // Use $PAGER or default
    };

    // Buffer output if paging is enabled
    let mut output_buffer = String::new();

    if output_opts.response_headers && args.quiet == 0 {
        let status_line = format!(
            "HTTP/1.1 {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        );

        let mut headers_str = status_line;
        headers_str.push('\n');

        for (name, value) in headers.iter() {
            if let Ok(v) = value.to_str() {
                headers_str.push_str(&format!("{}: {}\n", name, v));
            }
        }

        let formatted = if let Some(ref fmt) = formatter {
            fmt.format_headers(&headers_str)
        } else {
            headers_str
        };

        if pager_config.enabled {
            output_buffer.push_str(&formatted);
            if output_opts.response_body {
                output_buffer.push('\n');
            }
        } else {
            print!("{}", formatted);
            if output_opts.response_body {
                println!();
            }
        }
    }

    const MAX_BODY_SIZE: u64 = 100 * 1024 * 1024;
    let body = read_body_with_limit(response, MAX_BODY_SIZE, env.stdout_isatty).await?;

    if output_opts.response_body && args.quiet < 2 {
        if !body.is_empty() {
            let output = process_response_body(
                &body,
                content_type.as_deref(),
                args,
                &proc_opts,
                formatter.as_ref(),
            )?;
            if pager_config.enabled {
                output_buffer.push_str(&output);
                output_buffer.push('\n');
            } else {
                println!("{}", output);
            }
        }
    }

    // Write through pager if enabled
    if pager_config.enabled && !output_buffer.is_empty() {
        let mut stdout = std::io::stdout();
        if let Err(e) = write_with_pager(&mut stdout, &output_buffer, &pager_config, env.stdout_isatty) {
            // Fallback to direct output if pager fails
            eprintln!("Warning: Pager failed ({}), falling back to direct output", e);
            print!("{}", output_buffer);
        }
    }

    Ok(body)
}

/// Handle HTTP requests over Unix domain sockets
#[cfg(unix)]
async fn handle_unix_socket_request(
    args: &Args,
    processed: &crate::cli::parser::ProcessedArgs,
    socket_path: &std::path::Path,
    env: &Environment,
) -> Result<ExitStatus, QuicpulseError> {
    use crate::client::unix_socket;
    use std::time::Duration;

    // Build headers
    let mut headers: Vec<(String, String)> = Vec::new();

    // Add headers from processed args
    for item in &processed.items {
        if let crate::input::InputItem::Header { name, value } = item {
            headers.push((name.clone(), value.clone()));
        }
    }

    // Add auth header if specified
    if let Some(ref auth_str) = args.auth {
        use crate::cli::args::AuthType;
        let auth_header = match args.auth_type {
            Some(AuthType::Bearer) => format!("Bearer {}", auth_str),
            _ => {
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    auth_str.as_bytes()
                );
                format!("Basic {}", encoded)
            }
        };
        headers.push(("Authorization".to_string(), auth_header));
    }

    // Build body
    let body: Option<Vec<u8>> = if processed.has_data {
        if args.form {
            // Form-encoded body
            let form_data: Vec<String> = processed.items.iter()
                .filter_map(|item| {
                    if let crate::input::InputItem::DataField { key, value } = item {
                        Some(format!("{}={}", form_urlencode(key), form_urlencode(value)))
                    } else {
                        None
                    }
                })
                .collect();
            headers.push(("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string()));
            Some(form_data.join("&").into_bytes())
        } else {
            // JSON body
            let mut json_body = serde_json::json!({});
            for item in &processed.items {
                match item {
                    crate::input::InputItem::DataField { key, value } => {
                        if let Some(obj) = json_body.as_object_mut() {
                            obj.insert(key.clone(), serde_json::Value::String(value.clone()));
                        }
                    }
                    crate::input::InputItem::JsonField { key, value } => {
                        if let Some(obj) = json_body.as_object_mut() {
                            obj.insert(key.clone(), value.clone());
                        }
                    }
                    _ => {}
                }
            }
            headers.push(("Content-Type".to_string(), "application/json".to_string()));
            Some(json_body.to_string().into_bytes())
        }
    } else if let Some(ref raw) = args.raw {
        Some(raw.as_bytes().to_vec())
    } else {
        None
    };

    // Parse URL to get path
    let url = url::Url::parse(&processed.url)
        .map_err(|e| QuicpulseError::Argument(format!("Invalid URL: {}", e)))?;
    let path = if url.query().is_some() {
        format!("{}?{}", url.path(), url.query().unwrap())
    } else {
        url.path().to_string()
    };

    // Add Host header from URL
    if let Some(host) = url.host_str() {
        headers.push(("Host".to_string(), host.to_string()));
    }

    let timeout = args.timeout.map(|t| Duration::from_secs_f64(t));

    let request_start = std::time::Instant::now();

    // Print request if verbose or print-request-body
    if args.verbose > 0 || args.print.as_ref().map(|p| p.contains('H') || p.contains('h')).unwrap_or(false) {
        eprintln!("{} {} (via Unix socket {})", processed.method, path, socket_path.display());
        for (name, value) in &headers {
            eprintln!("{}: {}", name, value);
        }
        if let Some(ref body_bytes) = body {
            if let Ok(body_str) = String::from_utf8(body_bytes.clone()) {
                eprintln!();
                eprintln!("{}", body_str);
            }
        }
        eprintln!();
    }

    // Send request
    let response = unix_socket::send_request(
        socket_path,
        &processed.method,
        &path,
        &headers,
        body.as_deref(),
        timeout,
    ).await?;

    let response_time = request_start.elapsed();

    // Print response
    let output_opts = build_output_options(args, env);
    let proc_opts = build_processing_options(args, env);

    let formatter = if proc_opts.colors && proc_opts.pretty != PrettyOption::None {
        Some(ColorFormatter::new(proc_opts.style.clone()))
    } else {
        None
    };

    // Print status line and headers
    if output_opts.response_headers {
        let status_line = format!("{} {} {}", response.http_version, response.status, response.status_text);

        let mut headers_str = status_line;
        headers_str.push('\n');

        for (name, value) in &response.headers {
            headers_str.push_str(&format!("{}: {}\n", name, value));
        }

        let formatted = if let Some(ref fmt) = formatter {
            fmt.format_headers(&headers_str)
        } else {
            headers_str
        };

        print!("{}", formatted);
        println!();
    }

    // Print body
    if output_opts.response_body {
        if let Ok(body_str) = response.text() {
            let content_type = response.content_type().unwrap_or("text/plain");
            let base_mime = content_type.split(';').next().unwrap_or(content_type).trim();
            let is_json = base_mime == "application/json" || base_mime.contains("json");

            let output = if is_json {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    let formatted = if matches!(proc_opts.pretty, PrettyOption::All | PrettyOption::Format) {
                        serde_json::to_string_pretty(&json).unwrap_or_else(|_| body_str.clone())
                    } else {
                        json.to_string()
                    };

                    if let Some(ref fmt) = formatter {
                        fmt.format_json(&formatted)
                    } else {
                        formatted
                    }
                } else {
                    body_str.clone()
                }
            } else if let Some(ref fmt) = formatter {
                fmt.format_by_mime(&body_str, base_mime)
            } else {
                body_str.clone()
            };
            println!("{}", output);
        } else {
            // Binary data
            eprintln!("(binary data: {} bytes)", response.body.len());
        }
    }

    // Print timing if verbose
    if args.verbose > 0 {
        eprintln!("\nCompleted in {:?}", response_time);
    }

    // Return status based on response code
    if response.status >= 400 {
        Ok(ExitStatus::Error)
    } else {
        Ok(ExitStatus::Success)
    }
}

/// Handle .http/.rest file import and execution
async fn handle_http_file(
    args: &Args,
    path: &std::path::Path,
    env: &Environment,
) -> Result<ExitStatus, QuicpulseError> {
    use crate::devexp::http_file::{parse_http_file, parse_variables, request_to_args, list_requests};

    // List mode - just show the requests
    if args.http_list {
        let requests = list_requests(path)?;
        if requests.is_empty() {
            eprintln!("No requests found in {}", path.display());
            return Ok(ExitStatus::Error);
        }

        println!("Requests in {}:", path.display());
        for (idx, name, method, url) in &requests {
            println!("  [{}] {} {} - {}", idx, method, name, url);
        }
        return Ok(ExitStatus::Success);
    }

    // Parse the file
    let requests = parse_http_file(path)?;
    if requests.is_empty() {
        eprintln!("No requests found in {}", path.display());
        return Ok(ExitStatus::Error);
    }

    // Parse variables from the file content
    let content = std::fs::read_to_string(path)
        .map_err(|e| QuicpulseError::Io(e))?;
    let variables = parse_variables(&content);

    // Filter requests if --http-request is specified
    let requests_to_run: Vec<_> = if let Some(ref request_filter) = args.http_request {
        // Try parsing as index first
        if let Ok(idx) = request_filter.parse::<usize>() {
            if idx == 0 || idx > requests.len() {
                eprintln!("Request index {} out of range (1-{})", idx, requests.len());
                return Ok(ExitStatus::Error);
            }
            vec![requests[idx - 1].clone()]
        } else {
            // Filter by name (case-insensitive substring match)
            let filter_lower = request_filter.to_lowercase();
            let matched: Vec<_> = requests.iter()
                .filter(|r| {
                    r.name.as_ref()
                        .map(|n| n.to_lowercase().contains(&filter_lower))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            if matched.is_empty() {
                eprintln!("No requests matching '{}' found", request_filter);
                eprintln!("Available requests:");
                for (i, req) in requests.iter().enumerate() {
                    let name = req.name.as_deref().unwrap_or("(unnamed)");
                    eprintln!("  [{}] {} {} - {}", i + 1, req.method, name, req.url);
                }
                return Ok(ExitStatus::Error);
            }
            matched
        }
    } else {
        // Run all requests
        requests.clone()
    };

    let mut all_success = true;

    for (i, request) in requests_to_run.iter().enumerate() {
        let default_name = format!("Request {}", i + 1);
        let name = request.name.as_deref().unwrap_or(&default_name);

        if env.stdout_isatty && requests_to_run.len() > 1 {
            println!("\n\x1b[1m=== {} ===\x1b[0m", name);
            println!("  {} {}", request.method, request.url);
        }

        // Convert request to Args and execute
        let mut request_args = request_to_args(request, &variables);

        // Inherit some settings from the original args
        request_args.verbose = args.verbose;
        request_args.follow = args.follow;
        request_args.verify = args.verify.clone();
        request_args.timeout = args.timeout;
        request_args.proxy = args.proxy.clone();

        // Create a fresh environment for each sub-request
        let sub_env = Environment::default();
        let result = Box::pin(program(request_args, sub_env)).await;

        match result {
            Ok(ExitStatus::Success) => {}
            Ok(_) => all_success = false,
            Err(e) => {
                eprintln!("Error in {}: {}", name, e);
                all_success = false;
            }
        }
    }

    if all_success {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::Error)
    }
}

fn load_env_vars(args: &Args) -> Result<EnvVars, QuicpulseError> {
    let mut env_vars = EnvVars::new();

    if let Some(ref env_file) = args.env_file {
        env_vars = EnvVars::load_file(env_file)?;
    } else if !args.no_env {
        env_vars = EnvVars::try_load_default();
    }

    env_vars.merge_with_system();

    Ok(env_vars)
}

/// Merge curl import args with command line args
/// Command line args take precedence over imported curl args
fn merge_curl_args(cli_args: Args, curl_args: Args) -> Args {
    Args {
        // Core request - use curl_args as base, cli overrides
        method: cli_args.method.or(curl_args.method),
        url: cli_args.url.or(curl_args.url),
        request_items: if cli_args.request_items.is_empty() {
            curl_args.request_items
        } else {
            let mut items = curl_args.request_items;
            items.extend(cli_args.request_items);
            items
        },

        // Content type
        json: cli_args.json || curl_args.json,
        form: cli_args.form || curl_args.form,
        multipart: cli_args.multipart || curl_args.multipart,
        boundary: cli_args.boundary.or(curl_args.boundary),
        raw: cli_args.raw.or(curl_args.raw),

        // Auth - CLI takes precedence
        auth: cli_args.auth.or(curl_args.auth),
        auth_type: cli_args.auth_type.or(curl_args.auth_type),

        // Network options
        follow: cli_args.follow || curl_args.follow,
        max_redirects: if cli_args.max_redirects != 30 { cli_args.max_redirects } else { curl_args.max_redirects },
        timeout: cli_args.timeout.or(curl_args.timeout),
        verify: if cli_args.verify != "yes" { cli_args.verify } else { curl_args.verify },
        cert: cli_args.cert.or(curl_args.cert),
        cert_key: cli_args.cert_key.or(curl_args.cert_key),

        // Proxy
        proxy: if cli_args.proxy.is_empty() { curl_args.proxy } else { cli_args.proxy },

        // Output
        verbose: cli_args.verbose.max(curl_args.verbose),
        quiet: cli_args.quiet.max(curl_args.quiet),
        output: cli_args.output.or(curl_args.output),
        download: cli_args.download || curl_args.download,

        // HTTP version
        http_version: cli_args.http_version.or(curl_args.http_version),
        http3: cli_args.http3 || curl_args.http3,

        // Use all other CLI args
        ..cli_args
    }
}

fn expand_args_variables(mut args: Args, env_vars: &EnvVars) -> Result<Args, QuicpulseError> {
    if let Some(ref url) = args.url {
        args.url = Some(env_vars.expand(url)?);
    }

    if let Some(ref method) = args.method {
        if crate::devexp::has_variables(method) {
            args.method = Some(env_vars.expand(method)?);
        }
    }

    let mut expanded_items = Vec::with_capacity(args.request_items.len());
    for item in &args.request_items {
        expanded_items.push(env_vars.expand(item)?);
    }
    args.request_items = expanded_items;

    if let Some(ref auth) = args.auth {
        args.auth = Some(env_vars.expand(auth)?.into());
    }

    if let Some(ref raw) = args.raw {
        args.raw = Some(env_vars.expand(raw)?);
    }

    use crate::cli::args::SensitiveUrl;
    let mut expanded_proxies = Vec::with_capacity(args.proxy.len());
    for proxy in &args.proxy {
        expanded_proxies.push(SensitiveUrl(env_vars.expand(proxy)?));
    }
    args.proxy = expanded_proxies;

    if let Some(ref session) = args.session {
        args.session = Some(env_vars.expand(session)?);
    }

    if let Some(ref query) = args.graphql_query {
        args.graphql_query = Some(env_vars.expand(query)?);
    }

    Ok(args)
}

/// Run the built-in mock server
async fn run_mock_server(args: &Args) -> Result<ExitStatus, QuicpulseError> {
    use crate::mock::{MockServerConfig, MockServer};
    use crate::mock::routes::{RouteConfig, HttpMethod, ResponseConfig};

    // Load config from file if specified
    let mut config = if let Some(ref config_path) = args.mock_config {
        MockServerConfig::load(config_path)?
    } else {
        MockServerConfig::default()
    };

    // Override port if specified
    if let Some(port) = args.mock_port {
        config.port = port;
    }

    // Enable CORS if specified
    if args.mock_cors {
        config.cors = true;
    }

    // Parse latency if specified
    if let Some(ref latency) = args.mock_latency {
        if let Some((min_str, max_str)) = latency.split_once('-') {
            let min: u64 = min_str.parse()
                .map_err(|_| QuicpulseError::Argument(format!("Invalid latency min: {}", min_str)))?;
            let max: u64 = max_str.parse()
                .map_err(|_| QuicpulseError::Argument(format!("Invalid latency max: {}", max_str)))?;
            config.latency = Some((min, max));
        } else {
            let ms: u64 = latency.parse()
                .map_err(|_| QuicpulseError::Argument(format!("Invalid latency: {}", latency)))?;
            config.latency = Some((ms, ms));
        }
    }

    // Parse route arguments: "METHOD:PATH:BODY" or "METHOD:PATH:@FILE"
    for route_arg in &args.mock_routes {
        let parts: Vec<&str> = route_arg.splitn(3, ':').collect();
        if parts.len() < 2 {
            return Err(QuicpulseError::Argument(
                format!("Invalid route format '{}'. Expected METHOD:PATH or METHOD:PATH:BODY", route_arg)
            ));
        }

        let method = match parts[0].to_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            "*" => HttpMethod::Any,
            _ => return Err(QuicpulseError::Argument(format!("Unknown HTTP method: {}", parts[0]))),
        };

        let path = parts[1].to_string();

        let response = if parts.len() > 2 {
            let body = parts[2];
            if body.starts_with('@') {
                // Load from file
                let file_path = &body[1..];
                ResponseConfig {
                    body_file: Some(file_path.to_string()),
                    ..Default::default()
                }
            } else if body.starts_with('{') || body.starts_with('[') {
                // JSON body
                let json: serde_json::Value = serde_json::from_str(body)
                    .map_err(|e| QuicpulseError::Argument(format!("Invalid JSON: {}", e)))?;
                ResponseConfig::json_body(json)
            } else {
                ResponseConfig::text(body)
            }
        } else {
            ResponseConfig::text("OK")
        };

        config.routes.push(RouteConfig {
            method,
            path,
            response,
            priority: 0,
            enabled: true,
            name: None,
        });
    }

    // If no routes configured, add a default echo endpoint
    if config.routes.is_empty() {
        eprintln!("No routes configured. Use --mock-route to add routes.");
        eprintln!("Example: --mock-route 'GET:/api/hello:{{\"message\":\"Hello, World!\"}}'");
        eprintln!("         --mock-route 'POST:/api/echo:{{{{body}}}}'");
    }

    let server = MockServer::new(config)?;
    server.run().await?;

    // Server runs until interrupted
    Ok(ExitStatus::Success)
}

/// List installed plugins
async fn handle_plugin_list(args: &Args) -> Result<ExitStatus, QuicpulseError> {
    use crate::plugins::PluginLoader;

    let mut loader = PluginLoader::default();

    // Add custom plugin directory if specified
    if let Some(ref dir) = args.plugin_dir {
        loader.add_search_path(dir.clone());
    }

    // Discover plugins
    let plugins = loader.discover()?;

    if plugins.is_empty() {
        println!("No plugins installed.");
        println!("\nPlugin directories searched:");
        for dir in loader.search_paths() {
            println!("  - {}", dir.display());
        }
        println!("\nUse --plugin-install <name> to install plugins.");
    } else {
        println!("Installed plugins:\n");
        for plugin in &plugins {
            println!("  {} v{}", plugin.manifest.name, plugin.manifest.version);
            if !plugin.manifest.description.is_empty() {
                println!("    {}", plugin.manifest.description);
            }
            println!("    Hooks: {:?}", plugin.hooks);
            println!();
        }
    }

    Ok(ExitStatus::Success)
}

/// Search for plugins in the registry
async fn handle_plugin_search(query: &str, _args: &Args) -> Result<ExitStatus, QuicpulseError> {
    use crate::plugins::PluginRegistry;

    let registry = PluginRegistry::default();

    println!("Searching for plugins matching '{}'...\n", query);

    match registry.search(query).await {
        Ok(results) => {
            if results.is_empty() {
                println!("No plugins found matching '{}'.", query);
            } else {
                println!("Found {} plugin(s):\n", results.len());
                for plugin in results {
                    println!("  {} v{}", plugin.name, plugin.version);
                    if !plugin.description.is_empty() {
                        println!("    {}", plugin.description);
                    }
                    if let Some(ref author) = plugin.author {
                        println!("    Author: {}", author);
                    }
                    println!();
                }
            }
            Ok(ExitStatus::Success)
        }
        Err(e) => {
            eprintln!("Failed to search registry: {}", e);
            Ok(ExitStatus::Error)
        }
    }
}

/// Install a plugin from registry or URL
async fn handle_plugin_install(name: &str, _args: &Args) -> Result<ExitStatus, QuicpulseError> {
    use crate::plugins::PluginRegistry;

    let registry = PluginRegistry::default();
    let dest = PluginRegistry::ensure_plugins_dir()?.join(name);

    println!("Installing plugin '{}'...", name);

    match registry.install(name, &dest).await {
        Ok(()) => {
            println!("Successfully installed '{}' to {}", name, dest.display());
            Ok(ExitStatus::Success)
        }
        Err(e) => {
            eprintln!("Failed to install plugin '{}': {}", name, e);
            Ok(ExitStatus::Error)
        }
    }
}

/// Uninstall a plugin
async fn handle_plugin_uninstall(name: &str, _args: &Args) -> Result<ExitStatus, QuicpulseError> {
    use crate::plugins::PluginRegistry;

    println!("Uninstalling plugin '{}'...", name);

    match PluginRegistry::uninstall(name) {
        Ok(()) => {
            println!("Successfully uninstalled '{}'", name);
            Ok(ExitStatus::Success)
        }
        Err(e) => {
            eprintln!("Failed to uninstall plugin '{}': {}", name, e);
            Ok(ExitStatus::Error)
        }
    }
}

/// Update all installed plugins
async fn handle_plugin_update(args: &Args) -> Result<ExitStatus, QuicpulseError> {
    use crate::plugins::{PluginLoader, PluginRegistry};

    let registry = PluginRegistry::default();
    let mut loader = PluginLoader::default();

    // Add custom plugin directory if specified
    if let Some(ref dir) = args.plugin_dir {
        loader.add_search_path(dir.clone());
    }

    let plugins = loader.discover()?;

    if plugins.is_empty() {
        println!("No plugins installed to update.");
        return Ok(ExitStatus::Success);
    }

    println!("Checking {} plugin(s) for updates...\n", plugins.len());

    let mut checked = 0;

    for plugin in &plugins {
        print!("  {} v{}... ", plugin.manifest.name, plugin.manifest.version);

        // Check if there's a newer version in the registry
        match registry.get(&plugin.manifest.name).await {
            Ok(entry) => {
                if entry.version != plugin.manifest.version {
                    println!("update available: v{}", entry.version);
                    // Re-install to update
                    let dest = PluginRegistry::plugins_dir().join(&plugin.manifest.name);
                    if let Err(e) = std::fs::remove_dir_all(&dest) {
                        println!("    Warning: Failed to remove old version: {}", e);
                    }
                    if let Err(e) = registry.install(&plugin.manifest.name, &dest).await {
                        println!("    Failed to update: {}", e);
                    } else {
                        println!("    Updated to v{}", entry.version);
                    }
                } else {
                    println!("up to date");
                }
                checked += 1;
            }
            Err(_) => {
                println!("not in registry (local plugin)");
            }
        }
    }

    println!("\nChecked {} plugin(s)", checked);

    Ok(ExitStatus::Success)
}