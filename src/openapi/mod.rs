//! OpenAPI/Swagger Import and Workflow Generation

mod parser;
pub mod generator;
mod schema_mapper;

pub use parser::{OpenApiSpec, parse_spec};
pub use generator::{generate_workflow, GeneratorOptions, workflow_to_yaml};
pub use schema_mapper::SchemaMapper;

use crate::cli::Args;
use crate::context::Environment;
use crate::errors::QuicpulseError;
use crate::status::ExitStatus;

pub fn run_openapi_import(
    args: &Args,
    openapi_path: &std::path::Path,
    env: &Environment,
) -> Result<ExitStatus, QuicpulseError> {
    use std::fs;

    let spec = parse_spec(openapi_path)?;

    if args.openapi_list {
        eprintln!("OpenAPI Specification: {} v{}", spec.title, spec.version);
        if let Some(ref desc) = spec.description {
            eprintln!("  {}", desc);
        }
        eprintln!("\nServers:");
        for server in &spec.servers {
            if let Some(ref desc) = server.description {
                eprintln!("  {} ({})", server.url, desc);
            } else {
                eprintln!("  {}", server.url);
            }
        }
        eprintln!("\nEndpoints ({}):", spec.endpoints.len());
        for endpoint in &spec.endpoints {
            let deprecated = if endpoint.deprecated { " [DEPRECATED]" } else { "" };
            let tags = if !endpoint.tags.is_empty() {
                format!(" [{}]", endpoint.tags.join(", "))
            } else {
                String::new()
            };
            let summary = endpoint.summary.as_deref()
                .or(endpoint.operation_id.as_deref())
                .unwrap_or("");
            eprintln!("  {:7} {}{}{}", endpoint.method, endpoint.path, tags, deprecated);
            if !summary.is_empty() {
                eprintln!("          {}", summary);
            }
        }
        return Ok(ExitStatus::Success);
    }

    let options = GeneratorOptions {
        base_url: args.openapi_base_url.clone(),
        include_deprecated: args.openapi_include_deprecated,
        include_fuzz: args.openapi_fuzz,
        filter_tags: args.openapi_tags.clone(),
        exclude_tags: args.openapi_exclude_tags.clone(),
        filter_methods: Vec::new(),
        max_latency: Some("500ms".to_string()),
        enable_chaining: true,
        group_by_tag: false,
        include_comments: true,
    };

    let workflow = generate_workflow(&spec, &options);

    let yaml = workflow_to_yaml(&workflow)
        .map_err(|e| QuicpulseError::Argument(format!("Failed to serialize workflow: {}", e)))?;

    if let Some(ref output_path) = args.generate_workflow {
        fs::write(output_path, &yaml)
            .map_err(|e| QuicpulseError::Io(e))?;
        eprintln!("Generated workflow written to: {}", output_path.display());
        eprintln!("  API: {} v{}", spec.title, spec.version);
        eprintln!("  Endpoints: {}", spec.endpoints.len());
        eprintln!("  Steps: {}", workflow.steps.len());
    } else {
        if env.stdout_isatty {
            eprintln!("# Generated from: {}", openapi_path.display());
            eprintln!("# API: {} v{}", spec.title, spec.version);
            eprintln!("# Endpoints: {}, Steps: {}", spec.endpoints.len(), workflow.steps.len());
            eprintln!();
        }
        print!("{}", yaml);
    }

    Ok(ExitStatus::Success)
}
