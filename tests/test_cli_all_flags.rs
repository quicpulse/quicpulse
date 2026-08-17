//! Exhaustive CLI argument parser tests covering every field and option in Args

use clap::Parser;
use quicpulse::cli::args::{AuthType, LogFormat, PrettyOption, Shell};
use quicpulse::cli::Args;
use std::path::PathBuf;

macro_rules! parse_args {
    ($($arg:expr),*) => {{
        Args::try_parse_from(["quicpulse", $($arg),*]).expect("Failed to parse args")
    }};
}

#[test]
fn test_positional_and_content_type_flags() {
    let args = parse_args!(
        "POST",
        "https://api.example.com/v1",
        "name=john",
        "age:=30",
        "-j"
    );
    assert_eq!(args.method, Some("POST".to_string()));
    assert_eq!(args.url, Some("https://api.example.com/v1".to_string()));
    assert_eq!(args.request_items, vec!["name=john", "age:=30"]);
    assert!(args.json);

    let args_form = parse_args!(
        "-f",
        "--multipart",
        "--boundary=test_bound",
        "--raw=raw_body",
        "http://example.com"
    );
    assert!(args_form.form);
    assert!(args_form.multipart);
    assert_eq!(args_form.boundary, Some("test_bound".to_string()));
    assert_eq!(args_form.raw, Some("raw_body".to_string()));
}

#[test]
fn test_compression_and_chunked_flags() {
    let args = parse_args!("-x", "--chunked", "http://example.com");
    assert_eq!(args.compress, 1);
    assert!(args.chunked);

    let args_double = parse_args!("-xx", "http://example.com");
    assert_eq!(args_double.compress, 2);
}

#[test]
fn test_output_processing_and_format_flags() {
    let args = parse_args!(
        "--pretty=all",
        "--style=monokai",
        "--pager",
        "--pager-cmd=less -R",
        "--no-pager",
        "--unsorted",
        "--sorted",
        "--response-charset=utf-8",
        "--response-mime=application/json",
        "--format-options=json.indent:4",
        "http://example.com"
    );
    assert_eq!(args.pretty, Some(PrettyOption::All));
    assert_eq!(args.style, Some("monokai".to_string()));
    assert!(args.pager);
    assert_eq!(args.pager_cmd, Some("less -R".to_string()));
    assert!(args.no_pager);
    assert!(args.unsorted);
    assert!(args.sorted);
    assert_eq!(args.response_charset, Some("utf-8".to_string()));
    assert_eq!(args.response_mime, Some("application/json".to_string()));
    assert_eq!(args.format_options, vec!["json.indent:4"]);
}

#[test]
fn test_output_options_flags() {
    let args = parse_args!(
        "-p",
        "Hhb",
        "-m",
        "-vv",
        "--all",
        "-S",
        "-o",
        "output.json",
        "-d",
        "-c",
        "-qq",
        "http://example.com"
    );
    assert_eq!(args.print, Some("Hhb".to_string()));
    assert!(args.meta);
    assert_eq!(args.verbose, 2);
    assert!(args.all);
    assert!(args.stream);
    assert_eq!(args.output, Some(PathBuf::from("output.json")));
    assert!(args.download);
    assert!(args.continue_download);
    assert_eq!(args.quiet, 2);
}

#[test]
fn test_session_and_auth_flags() {
    let args = parse_args!(
        "--session=my_session",
        "--session-read-only=read_session",
        "-a",
        "admin:password123",
        "-A",
        "digest",
        "--ignore-netrc",
        "--aws-region=us-west-2",
        "--aws-service=s3",
        "--aws-profile=prod",
        "--oauth-token-url=https://auth.example.com/token",
        "--oauth-auth-url=https://auth.example.com/auth",
        "--oauth-device-url=https://auth.example.com/device",
        "--oauth-redirect-port=8888",
        "--oauth-pkce",
        "--oauth-scope=read",
        "--oauth-scope=write",
        "http://example.com"
    );
    assert_eq!(args.session, Some("my_session".to_string()));
    assert_eq!(args.session_read_only, Some("read_session".to_string()));
    assert_eq!(
        args.auth.as_ref().map(|s| s.as_str()),
        Some("admin:password123")
    );
    assert_eq!(args.auth_type, Some(AuthType::Digest));
    assert!(args.ignore_netrc);
    assert_eq!(args.aws_region, Some("us-west-2".to_string()));
    assert_eq!(args.aws_service, Some("s3".to_string()));
    assert_eq!(args.aws_profile, Some("prod".to_string()));
    assert_eq!(
        args.oauth_token_url,
        Some("https://auth.example.com/token".to_string())
    );
    assert_eq!(
        args.oauth_auth_url,
        Some("https://auth.example.com/auth".to_string())
    );
    assert_eq!(
        args.oauth_device_url,
        Some("https://auth.example.com/device".to_string())
    );
    assert_eq!(args.oauth_redirect_port, 8888);
    assert!(args.oauth_pkce);
    assert_eq!(args.oauth_scopes, vec!["read", "write"]);
}

#[test]
fn test_protocols_graphql_grpc_websocket_flags() {
    let args = parse_args!(
        "--http3",
        "--http-version=2",
        "-G",
        "--graphql-query=query { users { id } }",
        "--graphql-operation=GetUsers",
        "--graphql-schema",
        "--grpc",
        "--proto=service.proto",
        "--grpc-list",
        "--grpc-describe=UserService",
        "--grpc-interactive",
        "--grpc-plaintext",
        "--ws",
        "--ws-subprotocol=chat",
        "--ws-send=hello",
        "--ws-interactive",
        "--ws-listen",
        "--ws-binary=hex",
        "--ws-compress",
        "--ws-max-messages=50",
        "--ws-ping-interval=15",
        "http://example.com"
    );
    assert!(args.http3);
    assert_eq!(args.http_version, Some("2".to_string()));
    assert!(args.graphql);
    assert_eq!(
        args.graphql_query,
        Some("query { users { id } }".to_string())
    );
    assert_eq!(args.graphql_operation, Some("GetUsers".to_string()));
    assert!(args.graphql_schema);
    assert!(args.grpc);
    assert_eq!(args.proto, Some(PathBuf::from("service.proto")));
    assert!(args.grpc_list);
    assert_eq!(args.grpc_describe, Some("UserService".to_string()));
    assert!(args.grpc_interactive);
    assert!(args.grpc_plaintext);
    assert!(args.ws);
    assert_eq!(args.ws_subprotocol, Some("chat".to_string()));
    assert_eq!(args.ws_send, Some("hello".to_string()));
    assert!(args.ws_interactive);
    assert!(args.ws_listen);
    assert_eq!(args.ws_binary, Some("hex".to_string()));
    assert!(args.ws_compress);
    assert_eq!(args.ws_max_messages, 50);
    assert_eq!(args.ws_ping_interval, Some(15));
}

#[test]
fn test_network_tls_troubleshooting_flags() {
    let args = parse_args!(
        "--offline",
        "--unix-socket=/var/run/test.sock",
        "--proxy=http://127.0.0.1:8080",
        "--socks=socks5://127.0.0.1:1080",
        "--resolve=api.com:443:127.0.0.1",
        "--interface=eth0",
        "--local-port=40000-50000",
        "--tcp-fastopen",
        "--local-address=192.168.1.50",
        "-F",
        "--max-redirects=5",
        "--max-headers=100",
        "--timeout=3.5",
        "--check-status",
        "--path-as-is",
        "--verify=no",
        "--ssl=tls1.3",
        "--ciphers=ECDHE-RSA-AES128-GCM-SHA256",
        "--cert=client.crt",
        "--cert-key=client.key",
        "--cert-key-pass=secretpass",
        "--no-color",
        "--strict-tty",
        "--log-format=json",
        "-I",
        "--default-scheme=https",
        "--traceback",
        "--debug",
        "--debug-json",
        "--update",
        "http://example.com"
    );
    assert!(args.offline);
    assert_eq!(args.unix_socket, Some(PathBuf::from("/var/run/test.sock")));
    assert_eq!(args.proxy[0].as_str(), "http://127.0.0.1:8080");
    assert_eq!(
        args.socks_proxy.as_ref().map(|s| s.as_str()),
        Some("socks5://127.0.0.1:1080")
    );
    assert_eq!(args.resolve, vec!["api.com:443:127.0.0.1"]);
    assert_eq!(args.interface, Some("eth0".to_string()));
    assert_eq!(args.local_port, Some("40000-50000".to_string()));
    assert!(args.tcp_fastopen);
    assert_eq!(args.local_address, Some("192.168.1.50".to_string()));
    assert!(args.follow);
    assert_eq!(args.max_redirects, 5);
    assert_eq!(args.max_headers, Some(100));
    assert_eq!(args.timeout, Some(3.5));
    assert!(args.check_status);
    assert!(args.path_as_is);
    assert_eq!(args.verify, "no");
    assert_eq!(args.ssl, Some("tls1.3".to_string()));
    assert_eq!(
        args.ciphers,
        Some("ECDHE-RSA-AES128-GCM-SHA256".to_string())
    );
    assert_eq!(args.cert, Some(PathBuf::from("client.crt")));
    assert_eq!(args.cert_key, Some(PathBuf::from("client.key")));
    assert_eq!(args.cert_key_pass, Some("secretpass".to_string()));
    assert!(args.no_color);
    assert!(args.strict_tty);
    assert_eq!(args.log_format, Some(LogFormat::Json));
    assert!(args.ignore_stdin);
    assert_eq!(args.default_scheme, "https");
    assert!(args.traceback);
    assert!(args.debug);
    assert!(args.debug_json);
    assert!(args.update);
}

#[test]
fn test_bench_filter_assertions_workflow_flags() {
    let args = parse_args!(
        "--bench",
        "--requests=500",
        "--concurrency=25",
        "-J",
        ".data[] | .id",
        "--table",
        "--csv",
        "--assert-status=200",
        "--assert-time=<500ms",
        "--assert-body=success",
        "--assert-header=Content-Type:application/json",
        "--script-allow-dir=/tmp",
        "--run=pipeline.yaml",
        "--env=staging",
        "--var=foo=bar",
        "--dry-run",
        "--continue-on-failure",
        "--workflow-retries=3",
        "--workflow-verbose",
        "--validate",
        "--tags=smoke,core",
        "--include=step1,step2",
        "--exclude=step3",
        "--save-responses=/tmp/responses",
        "--report-junit=report.xml",
        "--report-json=report.json",
        "--report-tap=report.tap",
        "http://example.com"
    );
    assert!(args.bench);
    assert_eq!(args.bench_requests, 500);
    assert_eq!(args.bench_concurrency, 25);
    assert_eq!(args.filter, Some(".data[] | .id".to_string()));
    assert!(args.table);
    assert!(args.csv);
    assert_eq!(args.assert_status, Some("200".to_string()));
    assert_eq!(args.assert_time, Some("<500ms".to_string()));
    assert_eq!(args.assert_body, Some("success".to_string()));
    assert_eq!(args.assert_header, vec!["Content-Type:application/json"]);
    assert_eq!(args.script_allow_dirs, vec![PathBuf::from("/tmp")]);
    assert_eq!(args.run_workflow, Some(PathBuf::from("pipeline.yaml")));
    assert_eq!(args.workflow_env, Some("staging".to_string()));
    assert_eq!(args.workflow_vars, vec!["foo=bar"]);
    assert!(args.dry_run);
    assert!(args.continue_on_failure);
    assert_eq!(args.workflow_retries, 3);
    assert!(args.workflow_verbose);
    assert!(args.validate_workflow);
    assert_eq!(args.workflow_step_tags, vec!["smoke", "core"]);
    assert_eq!(args.workflow_include, vec!["step1", "step2"]);
    assert_eq!(args.workflow_exclude, vec!["step3"]);
    assert_eq!(args.save_responses, Some(PathBuf::from("/tmp/responses")));
    assert_eq!(args.report_junit, Some(PathBuf::from("report.xml")));
    assert_eq!(args.report_json, Some(PathBuf::from("report.json")));
    assert_eq!(args.report_tap, Some(PathBuf::from("report.tap")));
}

#[test]
fn test_workflow_sharing_fuzzing_har_openapi_flags() {
    let args = parse_args!(
        "--workflow-list",
        "--workflow-pull=my_wf",
        "--workflow-push=wf.yaml",
        "--workflow-public",
        "--workflow-tags=api,v1",
        "--workflow-description=A test workflow",
        "--workflow-registry=https://custom-registry.com",
        "--workflow-search=auth",
        "--fuzz",
        "--fuzz-field=username",
        "--fuzz-category=sql",
        "--fuzz-concurrency=15",
        "--fuzz-risk=3",
        "--fuzz-anomalies-only",
        "--fuzz-stop-on-anomaly",
        "--fuzz-dict=dict.txt",
        "--fuzz-payload=' OR 1=1 --",
        "--import-har=traffic.har",
        "--har-interactive",
        "--har-filter=api/.*",
        "--har-delay=50ms",
        "--har-list",
        "--har-index=1",
        "--har-index=2",
        "--import-openapi=swagger.json",
        "--generate-workflow=generated.yaml",
        "--openapi-base-url=https://api.io",
        "--openapi-include-deprecated",
        "--openapi-tag=users",
        "--openapi-exclude-tag=internal",
        "--openapi-fuzz",
        "--openapi-list",
        "http://example.com"
    );
    assert!(args.workflow_list);
    assert_eq!(args.workflow_pull, Some("my_wf".to_string()));
    assert_eq!(args.workflow_push, Some(PathBuf::from("wf.yaml")));
    assert!(args.workflow_public);
    assert_eq!(args.workflow_tags, Some("api,v1".to_string()));
    assert_eq!(
        args.workflow_description,
        Some("A test workflow".to_string())
    );
    assert_eq!(
        args.workflow_registry,
        Some("https://custom-registry.com".to_string())
    );
    assert_eq!(args.workflow_search, Some("auth".to_string()));
    assert!(args.fuzz);
    assert_eq!(args.fuzz_fields, vec!["username"]);
    assert_eq!(args.fuzz_categories, vec!["sql"]);
    assert_eq!(args.fuzz_concurrency, 15);
    assert_eq!(args.fuzz_risk, 3);
    assert!(args.fuzz_anomalies_only);
    assert!(args.fuzz_stop_on_anomaly);
    assert_eq!(args.fuzz_dict, Some(PathBuf::from("dict.txt")));
    assert_eq!(args.fuzz_payloads, vec!["' OR 1=1 --"]);
    assert_eq!(args.import_har, Some(PathBuf::from("traffic.har")));
    assert!(args.har_interactive);
    assert_eq!(args.har_filter, Some("api/.*".to_string()));
    assert_eq!(args.har_delay, Some("50ms".to_string()));
    assert!(args.har_list);
    assert_eq!(args.har_indices, vec![1, 2]);
    assert_eq!(args.import_openapi, Some(PathBuf::from("swagger.json")));
    assert_eq!(
        args.generate_workflow,
        Some(PathBuf::from("generated.yaml"))
    );
    assert_eq!(args.openapi_base_url, Some("https://api.io".to_string()));
    assert!(args.openapi_include_deprecated);
    assert_eq!(args.openapi_tags, vec!["users"]);
    assert_eq!(args.openapi_exclude_tags, vec!["internal"]);
    assert!(args.openapi_fuzz);
    assert!(args.openapi_list);
}

#[test]
fn test_devexp_mock_plugin_flags() {
    let args = parse_args!(
        "--curl",
        "--import-curl=curl -X POST http://example.com",
        "--http-file=requests.http",
        "--http-request=Login",
        "--http-list",
        "--generate=python",
        "--env-file=.env.test",
        "--no-env",
        "--mock",
        "--mock-config=mock.yaml",
        "--mock-port=9090",
        "--mock-route=GET:/test:{\"status\":\"ok\"}",
        "--mock-cors",
        "--mock-latency=100-200",
        "--mock-host=0.0.0.0",
        "--mock-log",
        "--mock-record=record.har",
        "--mock-tls-cert=cert.pem",
        "--mock-tls-key=key.pem",
        "--mock-proxy=https://upstream.com",
        "--plugin-list",
        "--plugin-install=auth-plugin",
        "--plugin-uninstall=old-plugin",
        "--plugin-search=jwt",
        "--plugin-update",
        "--plugin-dir=/opt/plugins",
        "--plugin=custom_plugin",
        "http://example.com"
    );
    assert!(args.curl);
    assert_eq!(
        args.import_curl,
        Some("curl -X POST http://example.com".to_string())
    );
    assert_eq!(args.http_file, Some(PathBuf::from("requests.http")));
    assert_eq!(args.http_request, Some("Login".to_string()));
    assert!(args.http_list);
    assert_eq!(args.generate_code, Some("python".to_string()));
    assert_eq!(args.env_file, Some(PathBuf::from(".env.test")));
    assert!(args.no_env);
    assert!(args.mock_server);
    assert_eq!(args.mock_config, Some(PathBuf::from("mock.yaml")));
    assert_eq!(args.mock_port, Some(9090));
    assert_eq!(args.mock_routes, vec!["GET:/test:{\"status\":\"ok\"}"]);
    assert!(args.mock_cors);
    assert_eq!(args.mock_latency, Some("100-200".to_string()));
    assert_eq!(args.mock_host, Some("0.0.0.0".to_string()));
    assert!(args.mock_log);
    assert_eq!(args.mock_record, Some(PathBuf::from("record.har")));
    assert_eq!(args.mock_tls_cert, Some(PathBuf::from("cert.pem")));
    assert_eq!(args.mock_tls_key, Some(PathBuf::from("key.pem")));
    assert_eq!(args.mock_proxy, Some("https://upstream.com".to_string()));
    assert!(args.plugin_list);
    assert_eq!(args.plugin_install, Some("auth-plugin".to_string()));
    assert_eq!(args.plugin_uninstall, Some("old-plugin".to_string()));
    assert_eq!(args.plugin_search, Some("jwt".to_string()));
    assert!(args.plugin_update);
    assert_eq!(args.plugin_dir, Some(PathBuf::from("/opt/plugins")));
    assert_eq!(args.enabled_plugins, vec!["custom_plugin"]);
}

#[test]
fn test_generate_completions_enum_and_default_args() {
    let default_args = Args::default();
    assert!(!default_args.offline);
    assert_eq!(default_args.default_scheme, "http");
    assert_eq!(default_args.verify, "yes");
    assert_eq!(default_args.max_redirects, 30);
    assert_eq!(default_args.oauth_redirect_port, 8080);
    assert_eq!(default_args.bench_requests, 100);
    assert_eq!(default_args.bench_concurrency, 10);
    assert_eq!(default_args.fuzz_concurrency, 10);
    assert_eq!(default_args.fuzz_risk, 1);

    let shells = [
        ("bash", Shell::Bash),
        ("zsh", Shell::Zsh),
        ("fish", Shell::Fish),
        ("powershell", Shell::PowerShell),
        ("elvish", Shell::Elvish),
    ];
    for (name, expected_shell) in shells {
        let args = parse_args!(format!("--generate-completions={}", name).as_str());
        assert_eq!(args.generate_completions, Some(expected_shell));
    }
}
