//! Security fuzzing module

pub mod payloads;
pub mod runner;

pub use payloads::{PayloadCategory, load_custom_payloads_from_file, create_custom_payloads, FuzzPayload};
pub use runner::{FuzzRunner, FuzzOptions, FuzzBodyFormat, format_fuzz_results};

use std::time::Duration;
use crate::cli::Args;
use crate::cli::parser::ProcessedArgs;
use crate::context::Environment;
use crate::errors::QuicpulseError;
use crate::input::InputItem;
use crate::status::ExitStatus;

pub async fn run_fuzz(
    args: Args,
    processed: ProcessedArgs,
    _env: Environment,
) -> Result<ExitStatus, QuicpulseError> {
    // Load custom payloads from dictionary file and/or CLI args
    let mut custom_payloads: Vec<FuzzPayload> = Vec::new();

    if let Some(ref dict_path) = args.fuzz_dict {
        match load_custom_payloads_from_file(dict_path) {
            Ok(payloads) => {
                eprintln!("Loaded {} custom payloads from {}", payloads.len(), dict_path.display());
                custom_payloads.extend(payloads);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                return Ok(ExitStatus::Error);
            }
        }
    }

    if !args.fuzz_payloads.is_empty() {
        let cli_payloads = create_custom_payloads(&args.fuzz_payloads);
        eprintln!("Added {} custom payloads from CLI", cli_payloads.len());
        custom_payloads.extend(cli_payloads);
    }

    let categories: Option<Vec<PayloadCategory>> = if args.fuzz_categories.is_empty() {
        None
    } else {
        let mut cats = Vec::new();
        for cat_str in &args.fuzz_categories {
            let cat = match cat_str.to_lowercase().as_str() {
                "sql" | "sqli" => PayloadCategory::SqlInjection,
                "xss" => PayloadCategory::Xss,
                "cmd" | "command" => PayloadCategory::CommandInjection,
                "path" | "traversal" => PayloadCategory::PathTraversal,
                "boundary" | "bound" => PayloadCategory::Boundary,
                "type" | "confusion" => PayloadCategory::TypeConfusion,
                "format" | "fmt" => PayloadCategory::FormatString,
                "int" | "integer" | "overflow" => PayloadCategory::IntegerOverflow,
                "unicode" | "uni" => PayloadCategory::Unicode,
                "nosql" | "mongo" => PayloadCategory::NoSqlInjection,
                "custom" => PayloadCategory::Custom,
                _ => {
                    eprintln!("Warning: Unknown fuzz category '{}', skipping", cat_str);
                    continue;
                }
            };
            cats.push(cat);
        }
        if cats.is_empty() { None } else { Some(cats) }
    };

    let body_format = if args.form {
        FuzzBodyFormat::Form
    } else {
        FuzzBodyFormat::Json
    };

    let options = FuzzOptions {
        concurrency: args.fuzz_concurrency,
        timeout: Duration::from_secs(args.timeout.map(|t| t as u64).unwrap_or(10)),
        categories,
        verbose: args.verbose > 0,
        anomalies_only: args.fuzz_anomalies_only,
        stop_on_anomaly: args.fuzz_stop_on_anomaly,
        min_risk_level: args.fuzz_risk,
        proxy: args.proxy.first().map(|p| p.0.clone()),
        insecure: args.verify == "no",
        ca_cert: args.cert.as_ref().and_then(|p| p.to_str().map(String::from)),
        body_format,
        custom_payloads,
    };

    let fields_to_fuzz: Vec<String> = if args.fuzz_fields.is_empty() {
        processed.items.iter()
            .filter(|item| matches!(item, InputItem::DataField { .. } | InputItem::JsonField { .. }))
            .map(|item| item.key().to_string())
            .collect()
    } else {
        args.fuzz_fields.clone()
    };

    if fields_to_fuzz.is_empty() {
        eprintln!("Error: No fields to fuzz. Provide data fields (e.g., name=value) or use --fuzz-field");
        return Ok(ExitStatus::Error);
    }

    let base_body: Option<serde_json::Value> = if processed.has_data {
        let mut body = serde_json::json!({});
        for item in &processed.items {
            match item {
                InputItem::DataField { key, value } => {
                    if let Some(map) = body.as_object_mut() {
                        map.insert(key.clone(), serde_json::Value::String(value.clone()));
                    }
                }
                InputItem::JsonField { key, value } => {
                    if let Some(map) = body.as_object_mut() {
                        map.insert(key.clone(), value.clone());
                    }
                }
                _ => {}
            }
        }
        Some(body)
    } else {
        None
    };

    let mut headers = reqwest::header::HeaderMap::new();
    for item in &processed.items {
        if let InputItem::Header { name, value } = item {
            if let Ok(header_name) = reqwest::header::HeaderName::try_from(name.as_str()) {
                if let Ok(header_value) = reqwest::header::HeaderValue::from_str(value) {
                    headers.insert(header_name, header_value);
                }
            }
        }
    }

    if let Some(ref auth_str) = args.auth {
        use crate::cli::args::AuthType;
        match args.auth_type {
            Some(AuthType::Bearer) => {
                if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", auth_str)) {
                    headers.insert(reqwest::header::AUTHORIZATION, val);
                }
            }
            Some(AuthType::Digest) => {
                eprintln!("Warning: Digest auth not supported in fuzz mode, using Basic auth");
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    auth_str.as_bytes()
                );
                if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Basic {}", encoded)) {
                    headers.insert(reqwest::header::AUTHORIZATION, val);
                }
            }
            _ => {
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    auth_str.as_bytes()
                );
                if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Basic {}", encoded)) {
                    headers.insert(reqwest::header::AUTHORIZATION, val);
                }
            }
        }
    }

    if base_body.is_some() && !headers.contains_key(reqwest::header::CONTENT_TYPE) {
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
    }

    let method: reqwest::Method = processed.method.parse()
        .map_err(|_| QuicpulseError::Argument(format!("Invalid method: {}", processed.method)))?;

    eprintln!("Fuzzing {} {} with {} field(s)", method, processed.url, fields_to_fuzz.len());
    eprintln!("  Fields: {}", fields_to_fuzz.join(", "));
    eprintln!("  Concurrency: {}", options.concurrency);
    eprintln!("  Risk level: >= {}/5", options.min_risk_level);
    if options.categories.is_some() {
        eprintln!("  Categories: {:?}", args.fuzz_categories);
    }
    eprintln!();

    let runner = FuzzRunner::new(options.clone())?;
    let (results, summary) = runner.run(
        method,
        &processed.url,
        base_body.as_ref(),
        &fields_to_fuzz,
        headers,
    ).await?;

    print!("{}", format_fuzz_results(&results, &summary, options.anomalies_only));

    if summary.anomalies > 0 {
        Ok(ExitStatus::Error)
    } else {
        Ok(ExitStatus::Success)
    }
}
