//! Pipeline and workflow execution module

pub mod assertions;
pub mod dependency;
pub mod runner;
pub mod sharing;
pub mod workflow;
pub mod report;

pub use assertions::Assertion;
pub use runner::{PipelineRunner, WorkflowOptions, StepResult, format_workflow_results, format_workflow_results_json};
pub use sharing::handle_workflow_commands;
pub use workflow::{load_workflow, apply_environment, apply_cli_variables};
pub use report::{ReportConfig, ReportFormat, generate_report, WorkflowSummary};

use std::time::Duration;
use crate::cli::Args;
use crate::context::Environment;
use crate::errors::QuicpulseError;
use crate::status::ExitStatus;

pub const EXIT_ASSERTION_FAILED: i32 = 10;

pub fn has_assertions(args: &crate::cli::Args) -> bool {
    args.assert_status.is_some()
        || args.assert_time.is_some()
        || args.assert_body.is_some()
        || !args.assert_header.is_empty()
}

pub async fn run_workflow(
    args: &Args,
    workflow_path: &std::path::Path,
    _env: &Environment,
) -> Result<ExitStatus, QuicpulseError> {
    let mut workflow = load_workflow(workflow_path)?;

    if let Some(ref env_name) = args.workflow_env {
        apply_environment(&mut workflow, env_name)?;
    }

    apply_cli_variables(&mut workflow, &args.workflow_vars)?;

    let options = WorkflowOptions {
        continue_on_failure: args.continue_on_failure,
        max_retries: args.workflow_retries,
        retry_delay: Duration::from_millis(500),
        verbose: args.workflow_verbose,
        tags: args.workflow_step_tags.clone(),
        include: args.workflow_include.clone(),
        exclude: args.workflow_exclude.clone(),
        save_responses: args.save_responses.clone(),
    };

    let mut runner = PipelineRunner::with_options(args.dry_run, options)?;

    if let Some(timeout_secs) = args.timeout {
        runner.set_timeout(Duration::from_secs_f64(timeout_secs));
    }

    if args.validate_workflow {
        eprintln!("Validating workflow: {}", workflow.name);
        match runner.validate(&workflow) {
            Ok(warnings) => {
                if warnings.is_empty() {
                    eprintln!("  Workflow is valid");
                } else {
                    eprintln!("  Workflow is valid with {} warning(s):", warnings.len());
                    for warning in &warnings {
                        eprintln!("    - {}", warning);
                    }
                }
                return Ok(ExitStatus::Success);
            }
            Err(errors) => {
                eprintln!("  Workflow validation failed with {} error(s):", errors.len());
                for error in &errors {
                    eprintln!("    - {}", error);
                }
                return Ok(ExitStatus::Error);
            }
        }
    }

    eprintln!("Running workflow: {}", workflow.name);
    if !workflow.description.is_empty() {
        eprintln!("  {}", workflow.description);
    }
    eprintln!("  Steps: {}", workflow.steps.len());
    if args.continue_on_failure {
        eprintln!("  Continue on failure: enabled");
    }
    if args.workflow_retries > 0 {
        eprintln!("  Retries: {}", args.workflow_retries);
    }
    eprintln!();

    let results = runner.run(&workflow).await?;

    // Use JSON format if specified, otherwise use pretty format
    if matches!(args.log_format, Some(crate::cli::LogFormat::Json)) {
        print!("{}", format_workflow_results_json(&results));
    } else {
        print!("{}", format_workflow_results(&results));
    }

    generate_workflow_reports(args, &workflow.name, &results)?;

    let all_passed = results.iter().all(|r| r.passed() || r.skipped);

    if all_passed {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::from_code(EXIT_ASSERTION_FAILED))
    }
}

fn generate_workflow_reports(
    args: &Args,
    workflow_name: &str,
    results: &[StepResult],
) -> Result<(), QuicpulseError> {
    if let Some(ref path) = args.report_junit {
        let config = ReportConfig {
            output_path: path.to_string_lossy().to_string(),
            format: ReportFormat::JUnit,
            workflow_name: workflow_name.to_string(),
            include_timing: true,
            include_response_details: true,
        };
        generate_report(results, &config)?;
        eprintln!("JUnit report written to: {}", path.display());
    }

    if let Some(ref path) = args.report_json {
        let config = ReportConfig {
            output_path: path.to_string_lossy().to_string(),
            format: ReportFormat::Json,
            workflow_name: workflow_name.to_string(),
            include_timing: true,
            include_response_details: true,
        };
        generate_report(results, &config)?;
        eprintln!("JSON report written to: {}", path.display());
    }

    if let Some(ref path) = args.report_tap {
        let config = ReportConfig {
            output_path: path.to_string_lossy().to_string(),
            format: ReportFormat::Tap,
            workflow_name: workflow_name.to_string(),
            include_timing: true,
            include_response_details: true,
        };
        generate_report(results, &config)?;
        eprintln!("TAP report written to: {}", path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_assertions() {
        let mut args = crate::cli::Args::default();
        assert!(!has_assertions(&args));

        args.assert_status = Some("200".to_string());
        assert!(has_assertions(&args));
    }
}
