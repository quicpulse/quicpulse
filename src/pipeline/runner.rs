//! Workflow execution engine
//!
//! Executes workflow steps with variable substitution, extraction, and assertions.

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use reqwest::{Client, Method, header::HeaderMap, redirect::Policy};
use serde_json::Value as JsonValue;
use tera::{Context, Tera};
use regex::Regex;
use once_cell::sync::Lazy;

use crate::errors::QuicpulseError;
use crate::fuzz::{FuzzRunner, FuzzOptions, FuzzBodyFormat, format_fuzz_results, PayloadCategory};
use crate::bench::{BenchmarkRunner, BenchmarkConfig, format_results as format_bench_results};
use crate::output::terminal::{self, colors, RESET};
use crate::sessions::Session;
use crate::config::Config;
use crate::context::Environment;
use crate::devexp::dotenv::EnvVars;
use crate::har::parser::load_har;

// Cached regex patterns to avoid recompilation in hot paths
static TEMPLATE_VAR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap()
});
static VAR_NOT_FOUND_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"Variable `([^`]+)` not found").unwrap()
});
use crate::filter;
use crate::magic::expand_magic_values;
use crate::grpc::{GrpcEndpoint, client::GrpcClient};
use crate::websocket::{self, types::{WsEndpoint, WsOptions, BinaryMode}};
use crate::scripting::{ScriptEngine, ScriptContext, ScriptResult, RequestData, ResponseData, MultiScriptEngine, ScriptType, detect_script_type};
use super::workflow::{
    Workflow, WorkflowStep, StatusAssertion, GraphQLConfig, GrpcConfig, WebSocketConfig,
    ScriptConfig, FuzzConfig, BenchConfig, DownloadConfig, HarConfig, OpenApiConfig,
    PluginConfig, UploadConfig, OutputConfig, FilterConfig, SaveConfig
};
use super::assertions::{AssertionResult, Assertion, check_assertions};
use super::dependency::{resolve_dependencies, has_dependencies};

/// Maximum number of steps in a workflow (prevents resource exhaustion)
const MAX_WORKFLOW_STEPS: usize = 100_000;

/// Maximum number of retries per step
const MAX_RETRIES_PER_STEP: u32 = 10;

/// Workflow configuration options
#[derive(Debug, Clone, Default)]
pub struct WorkflowOptions {
    /// Continue executing even if a step fails
    pub continue_on_failure: bool,
    /// Maximum number of retries per step
    pub max_retries: u32,
    /// Base delay between retries (doubles each retry)
    pub retry_delay: Duration,
    /// Show verbose progress output
    pub verbose: bool,
    /// Only run steps with these tags (empty = run all)
    pub tags: Vec<String>,
    /// Only run steps with these names (empty = run all)
    pub include: Vec<String>,
    /// Exclude steps matching these patterns (regex supported)
    pub exclude: Vec<String>,
    /// Directory to save response data
    pub save_responses: Option<std::path::PathBuf>,
}

/// Result of executing a single step
#[derive(Debug)]
pub struct StepResult {
    pub name: String,
    pub method: String,
    pub url: String,
    pub status_code: Option<u16>,
    pub response_time: Duration,
    pub assertions: Vec<AssertionResult>,
    pub extracted: HashMap<String, JsonValue>,
    pub error: Option<String>,
    pub skipped: bool,
}

impl StepResult {
    pub fn passed(&self) -> bool {
        !self.skipped && self.error.is_none() && self.assertions.iter().all(|a| a.passed)
    }
}

/// Workflow execution engine
/// Bug #4 fix: ScriptEngine is now cached to avoid expensive recompilation
/// of modules for every script execution in a workflow.
pub struct PipelineRunner {
    client: Client,
    variables: HashMap<String, JsonValue>,
    dry_run: bool,
    options: WorkflowOptions,
    default_timeout: Duration,
    /// Multi-language script engine (Rune + JavaScript)
    script_engine: MultiScriptEngine,
    /// Session for persistent cookies/headers (optional)
    session: Option<Session>,
    /// Session name for saving
    session_name: Option<String>,
    /// Session host for saving
    session_host: Option<String>,
    /// Whether session is read-only
    session_read_only: bool,
}

impl PipelineRunner {
    /// Create a new pipeline runner
    pub fn new(dry_run: bool) -> Result<Self, QuicpulseError> {
        Self::with_options(dry_run, WorkflowOptions::default())
    }

    /// Create a new pipeline runner with options
    pub fn with_options(dry_run: bool, options: WorkflowOptions) -> Result<Self, QuicpulseError> {
        let client = Client::builder()
            .build()
            .map_err(|e| QuicpulseError::Request(e))?;

        // Bug #4 fix: Create script engine once and reuse for all scripts
        // Now using MultiScriptEngine for Rune + JavaScript support
        // JavaScript is enabled when the feature is compiled in
        #[cfg(feature = "javascript")]
        let js_enabled = true; // TODO: Check bundled plugin config
        #[cfg(not(feature = "javascript"))]
        let js_enabled = false;
        let script_engine = MultiScriptEngine::new(js_enabled)?;

        Ok(Self {
            client,
            variables: HashMap::new(),
            dry_run,
            options,
            default_timeout: Duration::from_secs(30),
            script_engine,
            session: None,
            session_name: None,
            session_host: None,
            session_read_only: false,
        })
    }

    /// Load session for the workflow
    pub fn load_session(&mut self, workflow: &Workflow) -> Result<(), QuicpulseError> {
        if let Some(ref session_name) = workflow.session {
            // Extract host from base_url
            let host = workflow.base_url.as_ref()
                .and_then(|url| url::Url::parse(url).ok())
                .and_then(|u| u.host_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "default".to_string());

            let env = Environment::init();
            let config = Config::load(&env)?;
            let session = Session::load_named(session_name, &host, &config)?;

            self.session = Some(session);
            self.session_name = Some(session_name.clone());
            self.session_host = Some(host);
            self.session_read_only = workflow.session_read_only.unwrap_or(false);
        }
        Ok(())
    }

    /// Save session after workflow execution
    pub fn save_session(&self) -> Result<(), QuicpulseError> {
        if let (Some(ref session), Some(ref name), Some(ref host)) = (&self.session, &self.session_name, &self.session_host) {
            if !self.session_read_only {
                let env = Environment::init();
                let config = Config::load(&env)?;
                session.save_named(name, host, &config)?;
            }
        }
        Ok(())
    }

    /// Apply session headers/cookies to a request
    fn apply_session_to_request(&self, headers: &mut HeaderMap, url: &str) {
        if let Some(ref session) = self.session {
            // Add session headers
            for header in &session.headers {
                if let (Ok(name), Ok(val)) = (
                    reqwest::header::HeaderName::try_from(header.name.as_str()),
                    reqwest::header::HeaderValue::from_str(&header.value)
                ) {
                    headers.insert(name, val);
                }
            }

            // Add session cookies
            if let Ok(parsed_url) = url::Url::parse(url) {
                let domain = parsed_url.host_str().unwrap_or("");
                let path = parsed_url.path();
                let is_secure = parsed_url.scheme() == "https";

                if let Some(cookie_header) = session.get_cookie_header(domain, path, is_secure) {
                    if let Ok(val) = reqwest::header::HeaderValue::from_str(&cookie_header) {
                        headers.insert(reqwest::header::COOKIE, val);
                    }
                }
            }
        }
    }

    /// Update session from response headers
    fn update_session_from_response(&mut self, response_headers: &HeaderMap, url: &str) {
        if let Some(ref mut session) = self.session {
            if let Ok(parsed_url) = url::Url::parse(url) {
                let domain = parsed_url.host_str().unwrap_or("").to_string();

                // Parse Set-Cookie headers
                for value in response_headers.get_all(reqwest::header::SET_COOKIE) {
                    if let Ok(cookie_str) = value.to_str() {
                        session.parse_set_cookie(cookie_str, &domain);
                    }
                }
            }
        }
    }

    /// Set default timeout for requests
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.default_timeout = timeout;
    }

    /// Build a custom client for a step if needed (for proxy/SSL/redirect options)
    fn build_step_client(&self, step: &WorkflowStep) -> Result<Client, QuicpulseError> {
        // Check if we need a custom client
        let needs_custom = step.proxy.is_some()
            || step.insecure.is_some()
            || step.ca_cert.is_some()
            || step.client_cert.is_some()
            || step.follow_redirects.is_some()
            || step.max_redirects.is_some()
            || step.http2.is_some();

        if !needs_custom {
            return Ok(self.client.clone());
        }

        let mut builder = Client::builder();

        // Configure redirect policy
        if let Some(follow) = step.follow_redirects {
            if !follow {
                builder = builder.redirect(Policy::none());
            } else if let Some(max) = step.max_redirects {
                builder = builder.redirect(Policy::limited(max as usize));
            }
        } else if let Some(max) = step.max_redirects {
            builder = builder.redirect(Policy::limited(max as usize));
        }

        // Configure proxy
        if let Some(ref proxy_url) = step.proxy {
            let proxy = reqwest::Proxy::all(proxy_url)
                .map_err(|e| QuicpulseError::Argument(format!("Invalid proxy URL '{}': {}", proxy_url, e)))?;
            builder = builder.proxy(proxy);
        }

        // Configure TLS options
        if step.insecure == Some(true) {
            builder = builder.danger_accept_invalid_certs(true);
        }

        // Add custom CA certificate
        if let Some(ref ca_path) = step.ca_cert {
            let ca_data = std::fs::read(ca_path)
                .map_err(|e| QuicpulseError::Io(e))?;
            let cert = reqwest::Certificate::from_pem(&ca_data)
                .map_err(|e| QuicpulseError::Argument(format!("Invalid CA certificate: {}", e)))?;
            builder = builder.add_root_certificate(cert);
        }

        // Add client certificate
        if let Some(ref cert_path) = step.client_cert {
            let cert_data = std::fs::read(cert_path)
                .map_err(|e| QuicpulseError::Io(e))?;

            // If client key is separate, read and combine
            if let Some(ref key_path) = step.client_key {
                let key_data = std::fs::read(key_path)
                    .map_err(|e| QuicpulseError::Io(e))?;
                let mut combined = cert_data;
                combined.extend_from_slice(b"\n");
                combined.extend_from_slice(&key_data);
                let identity = reqwest::Identity::from_pem(&combined)
                    .map_err(|e| QuicpulseError::Argument(format!("Invalid client certificate/key: {}", e)))?;
                builder = builder.identity(identity);
            } else {
                // Cert and key in same file (PFX/PKCS12 style)
                let identity = reqwest::Identity::from_pem(&cert_data)
                    .map_err(|e| QuicpulseError::Argument(format!("Invalid client certificate: {}", e)))?;
                builder = builder.identity(identity);
            }
        }

        // Force HTTP/2 if requested
        if step.http2 == Some(true) {
            builder = builder.http2_prior_knowledge();
        }

        builder.build()
            .map_err(|e| QuicpulseError::Request(e))
    }

    /// Execute a script from a ScriptConfig
    /// Bug #4 fix: Uses cached script engine instead of creating new one each time
    /// Now supports both Rune and JavaScript via MultiScriptEngine
    async fn execute_script(&self, config: &ScriptConfig, ctx: &mut ScriptContext, step_name: &str) -> Result<ScriptResult, QuicpulseError> {
        // Detect script type from config (explicit type field or file extension)
        let script_type = detect_script_type(
            config.r#type.as_deref(),
            config.file.as_deref(),
        );

        let source = if let Some(ref code) = config.code {
            match script_type {
                ScriptType::Rune => {
                    // Rune: wrap in main function if needed
                    if code.contains("pub fn main") || code.contains("fn main") {
                        code.clone()
                    } else {
                        format!("pub fn main() {{ {} }}", code)
                    }
                }
                ScriptType::JavaScript => {
                    // JavaScript: use code as-is
                    code.clone()
                }
            }
        } else if let Some(ref file_path) = config.file {
            // Bug #5 fix: Use spawn_blocking to avoid blocking tokio worker threads
            // Previously used sync std::fs::read_to_string which blocks the event loop
            let path_clone = file_path.clone();
            let step_name_clone = step_name.to_string();
            tokio::task::spawn_blocking(move || {
                std::fs::read_to_string(&path_clone)
            })
            .await
            .map_err(|e| QuicpulseError::Script(format!(
                "Step '{}': Script file read task panicked: {}",
                step_name_clone, e
            )))?
            .map_err(|e| QuicpulseError::Script(format!(
                "Step '{}': Failed to read script file '{}': {}",
                step_name, file_path, e
            )))?
        } else {
            return Err(QuicpulseError::Script(format!(
                "Step '{}': Script config must have either 'code' or 'file'",
                step_name
            )));
        };

        // Use MultiScriptEngine with detected script type
        self.script_engine.execute(&source, ctx, script_type).await
    }

    /// Execute a script-based assertion
    async fn execute_script_assertion(&self, config: &ScriptConfig, response: &ResponseData, step_name: &str) -> Result<bool, QuicpulseError> {
        let mut ctx = ScriptContext::new();
        ctx.set_response(response.clone());

        let result: ScriptResult = self.execute_script(config, &mut ctx, step_name).await?;
        result.as_bool()
    }

    /// Validate a workflow before execution
    pub fn validate(&self, workflow: &Workflow) -> Result<Vec<String>, Vec<String>> {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        if workflow.steps.is_empty() {
            errors.push("Workflow has no steps defined".to_string());
        }

        if workflow.steps.len() > MAX_WORKFLOW_STEPS {
            errors.push(format!(
                "Workflow has too many steps: {} (max {})",
                workflow.steps.len(), MAX_WORKFLOW_STEPS
            ));
        }

        for (i, step) in workflow.steps.iter().enumerate() {
            let step_prefix = format!("Step {} ({})", i + 1, step.name);

            // Validate method
            if step.method.to_uppercase().parse::<Method>().is_err() {
                errors.push(format!("{}: Invalid HTTP method '{}'", step_prefix, step.method));
            }

            // Check for undefined variables in URL (warning only - they might be extracted)
            let undefined = self.find_undefined_variables(&step.url, workflow);
            for var in &undefined {
                // Check if this variable will be extracted by a previous step
                let will_be_extracted = workflow.steps[..i].iter()
                    .any(|s| s.extract.contains_key(var));
                if !will_be_extracted {
                    warnings.push(format!("{}: URL references undefined variable '{}' (may be extracted at runtime)", step_prefix, var));
                }
            }

            // Validate timeout if specified
            if let Some(ref timeout) = step.timeout {
                if humantime::parse_duration(timeout).is_err() {
                    errors.push(format!("{}: Invalid timeout format '{}'", step_prefix, timeout));
                }
            }

            // Validate delay if specified
            if let Some(ref delay) = step.delay {
                if humantime::parse_duration(delay).is_err() {
                    errors.push(format!("{}: Invalid delay format '{}'", step_prefix, delay));
                }
            }

            // Validate assertions
            if let Some(ref latency) = step.assert.latency {
                let trimmed = latency.trim_start_matches('<').trim();
                if humantime::parse_duration(trimmed).is_err() {
                    errors.push(format!("{}: Invalid latency assertion format '{}'", step_prefix, latency));
                }
            }

            // Validate retries
            if let Some(retries) = step.retries {
                if retries > MAX_RETRIES_PER_STEP {
                    errors.push(format!("{}: Too many retries {} (max {})", step_prefix, retries, MAX_RETRIES_PER_STEP));
                }
            }
        }

        if errors.is_empty() {
            Ok(warnings)
        } else {
            Err(errors)
        }
    }

    /// Find undefined variables in a template string
    fn find_undefined_variables(&self, template: &str, workflow: &Workflow) -> Vec<String> {
        let mut undefined = Vec::new();
        // Match {{variable}} or {{ variable }} - use cached regex
        for cap in TEMPLATE_VAR_RE.captures_iter(template) {
            let var_name = &cap[1];
            if !workflow.variables.contains_key(var_name) {
                undefined.push(var_name.to_string());
            }
        }
        undefined
    }

    /// Run a complete workflow
    pub async fn run(&mut self, workflow: &Workflow) -> Result<Vec<StepResult>, QuicpulseError> {
        // Safety check: prevent resource exhaustion from too many steps
        if workflow.steps.len() > MAX_WORKFLOW_STEPS {
            return Err(QuicpulseError::Argument(format!(
                "Workflow has too many steps: {} (max {})",
                workflow.steps.len(), MAX_WORKFLOW_STEPS
            )));
        }

        // Load session if configured
        self.load_session(workflow)?;

        // Load dotenv file if configured
        if let Some(ref dotenv_path) = workflow.dotenv {
            self.load_dotenv(dotenv_path)?;
        }

        // Initialize variables from workflow and environment
        self.variables = workflow.variables.clone();

        // Inject environment variables as env_VAR_NAME
        // SECURITY: Only inject allowed env vars to prevent leaking secrets
        // like AWS_SECRET_ACCESS_KEY, API keys, etc.
        use crate::scripting::modules::env::is_allowed_env_var;
        for (key, value) in std::env::vars() {
            if is_allowed_env_var(&key) {
                self.variables.insert(format!("env_{}", key), JsonValue::String(value));
            }
        }

        // Filter steps based on tags, include, and exclude options
        let filtered_steps: Vec<&WorkflowStep> = workflow.steps.iter()
            .filter(|step| self.should_run_step(step))
            .collect();

        // Apply dependency ordering if any step has depends_on
        let ordered_steps: Vec<&WorkflowStep> = if has_dependencies(&filtered_steps) {
            let dep_order = resolve_dependencies(&filtered_steps)?;
            if self.options.verbose && !self.dry_run {
                eprintln!("{} ({} execution levels)",
                    terminal::info("Resolved step dependencies"),
                    terminal::number(&dep_order.levels.len().to_string()));
            }
            dep_order.order.iter().map(|&i| filtered_steps[i]).collect()
        } else {
            filtered_steps
        };

        let total_steps = ordered_steps.len();
        let mut results = Vec::with_capacity(total_steps);

        // Print enhanced dry-run plan if in dry-run mode
        if self.dry_run {
            self.print_dry_run_plan(workflow, &ordered_steps);
        }

        if self.options.verbose && !self.dry_run && total_steps != workflow.steps.len() {
            eprintln!("{} {} of {} steps {}",
                terminal::info("Running"),
                terminal::number(&total_steps.to_string()),
                terminal::number(&workflow.steps.len().to_string()),
                terminal::muted("(filtered)"));
        }

        for (i, step) in ordered_steps.iter().enumerate() {
            // Progress output
            if self.options.verbose && !self.dry_run {
                eprintln!("\n{}{}/{}{} {} {}",
                    terminal::muted("["),
                    terminal::number(&(i + 1).to_string()),
                    terminal::number(&total_steps.to_string()),
                    terminal::muted("]"),
                    terminal::info("Running:"),
                    terminal::label(&step.name));
            }

            let step_results = self.run_step_with_control_flow(step, workflow).await?;

            // Handle multiple results from loops
            for result in step_results {
                // Extract variables from successful steps
                if result.error.is_none() && !result.skipped {
                    for (key, value) in &result.extracted {
                        self.variables.insert(key.clone(), value.clone());
                    }
                }

                let passed = result.passed();

                // Progress feedback
                if self.options.verbose && !self.dry_run {
                    if result.skipped {
                        eprintln!("  {} {}", terminal::muted("->"), terminal::muted("Skipped"));
                    } else if passed {
                        eprintln!("  {} {} {}",
                            terminal::muted("->"),
                            terminal::success("Passed"),
                            terminal::muted(&format!("({:?})", result.response_time)));
                    } else {
                        eprintln!("  {} {} {}",
                            terminal::muted("->"),
                            terminal::error("Failed:"),
                            terminal::colorize(
                                result.error.as_ref().map(|e| e.as_str())
                                    .unwrap_or("assertion failed"),
                                colors::RED));
                    }
                }

                // Save response data if configured
                if !self.dry_run {
                    self.save_response_data(&result)?;
                }

                results.push(result);

                // Stop on failure unless continue_on_failure is set
                if !passed && !self.options.continue_on_failure {
                    // Save session before returning
                    self.save_session()?;
                    return Ok(results);
                }
            }
        }

        // Save session if configured
        self.save_session()?;

        Ok(results)
    }

    /// Check if a step should be run based on filtering options
    fn should_run_step(&self, step: &WorkflowStep) -> bool {
        // Check tag filter: step must have at least one matching tag
        if !self.options.tags.is_empty() {
            let has_matching_tag = step.tags.iter()
                .any(|tag| self.options.tags.contains(tag));
            if !has_matching_tag {
                return false;
            }
        }

        // Check include filter: step name must be in the include list
        if !self.options.include.is_empty() {
            if !self.options.include.contains(&step.name) {
                return false;
            }
        }

        // Check exclude filter: step name must not match any exclude pattern
        if !self.options.exclude.is_empty() {
            for pattern in &self.options.exclude {
                // Try as regex first, fall back to exact match
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(&step.name) {
                        return false;
                    }
                } else if step.name == *pattern {
                    return false;
                }
            }
        }

        true
    }

    /// Run a step with control flow (repeat, foreach, while)
    async fn run_step_with_control_flow(&mut self, step: &WorkflowStep, workflow: &Workflow) -> Result<Vec<StepResult>, QuicpulseError> {
        let mut results = Vec::new();

        // Handle repeat
        if let Some(count) = step.repeat {
            let count = count.min(1000); // Safety limit
            for i in 0..count {
                self.variables.insert("_iteration".to_string(), serde_json::json!(i));
                self.variables.insert("_index".to_string(), serde_json::json!(i));
                let result = self.run_step_with_retry(step, workflow).await?;
                let passed = result.passed();
                results.push(result);
                if !passed && step.fail_fast.unwrap_or(true) {
                    break;
                }
            }
            return Ok(results);
        }

        // Handle foreach
        if let Some(ref foreach_expr) = step.foreach {
            let var_name = step.foreach_var.as_deref().unwrap_or("item");
            let rendered = self.render_template_for_step(foreach_expr, &step.name, "foreach")
                .unwrap_or_else(|_| foreach_expr.clone());

            // Try to parse as JSON array or get from variables
            let items: Vec<serde_json::Value> = if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&rendered) {
                arr
            } else if let Some(val) = self.variables.get(&rendered) {
                if let Some(arr) = val.as_array() {
                    arr.clone()
                } else {
                    vec![val.clone()]
                }
            } else {
                Vec::new()
            };

            for (i, item) in items.into_iter().enumerate() {
                self.variables.insert(var_name.to_string(), item);
                self.variables.insert("_index".to_string(), serde_json::json!(i));
                let result = self.run_step_with_retry(step, workflow).await?;
                let passed = result.passed();
                results.push(result);
                if !passed && step.fail_fast.unwrap_or(true) {
                    break;
                }
            }
            return Ok(results);
        }

        // Handle while loop
        if let Some(ref condition) = step.while_condition {
            let max_iters = step.max_iterations.unwrap_or(100).min(1000);
            for i in 0..max_iters {
                if !self.evaluate_condition(condition) {
                    break;
                }
                self.variables.insert("_iteration".to_string(), serde_json::json!(i));
                let result = self.run_step_with_retry(step, workflow).await?;
                let passed = result.passed();
                results.push(result);
                if !passed && step.fail_fast.unwrap_or(true) {
                    break;
                }
            }
            return Ok(results);
        }

        // No control flow - run once
        let result = self.run_step_with_retry(step, workflow).await?;
        results.push(result);
        Ok(results)
    }

    /// Run a step with retry logic
    async fn run_step_with_retry(&mut self, step: &WorkflowStep, workflow: &Workflow) -> Result<StepResult, QuicpulseError> {
        // Cap retries to prevent runaway loops
        let max_retries = step.retries.unwrap_or(self.options.max_retries).min(MAX_RETRIES_PER_STEP);
        let retry_delay = self.options.retry_delay;

        let mut last_result = self.run_step(step, workflow).await?;

        if max_retries == 0 || last_result.passed() || last_result.skipped || self.dry_run {
            return Ok(last_result);
        }

        for attempt in 1..=max_retries {
            // Exponential backoff
            let delay = retry_delay * (1 << (attempt - 1));
            if self.options.verbose {
                eprintln!("  {} {} {}/{} after {}...",
                    terminal::muted("->"),
                    terminal::warning("Retry"),
                    terminal::number(&attempt.to_string()),
                    terminal::number(&max_retries.to_string()),
                    terminal::muted(&format!("{:?}", delay)));
            }
            tokio::time::sleep(delay).await;

            last_result = self.run_step(step, workflow).await?;

            if last_result.passed() {
                break;
            }
        }

        Ok(last_result)
    }

    /// Run a single workflow step
    async fn run_step(&mut self, step: &WorkflowStep, workflow: &Workflow) -> Result<StepResult, QuicpulseError> {
        // Check skip condition
        if let Some(ref condition) = step.skip_if {
            if self.evaluate_condition(condition) {
                return Ok(StepResult {
                    name: step.name.clone(),
                    method: step.method.clone(),
                    url: String::new(),
                    status_code: None,
                    response_time: Duration::ZERO,
                    assertions: Vec::new(),
                    extracted: HashMap::new(),
                    error: None,
                    skipped: true,
                });
            }
        }

        // Apply delay if specified
        if let Some(ref delay) = step.delay {
            let rendered_delay = if self.dry_run {
                self.render_template_dry_run(delay)
            } else {
                self.render_template_for_step(delay, &step.name, "delay").unwrap_or_else(|_| delay.clone())
            };
            if let Ok(duration) = humantime::parse_duration(&rendered_delay) {
                if !self.dry_run {
                    tokio::time::sleep(duration).await;
                }
            }
        }

        // Build URL with variable substitution
        // In dry-run mode, use graceful rendering that shows placeholders for missing variables
        let (_url, full_url) = if self.dry_run {
            let url = self.render_template_dry_run(&step.url);
            let full_url = if let Some(ref base) = workflow.base_url {
                let base_rendered = self.render_template_dry_run(base);
                if url.starts_with("http://") || url.starts_with("https://") {
                    url.clone()
                } else {
                    format!("{}{}", base_rendered.trim_end_matches('/'),
                        if url.starts_with('/') { url.clone() } else { format!("/{}", url) })
                }
            } else {
                url.clone()
            };
            (url, full_url)
        } else {
            let url = self.render_template_for_step(&step.url, &step.name, "url")?;
            let full_url = if let Some(ref base) = workflow.base_url {
                let base_rendered = self.render_template_for_step(base, &step.name, "base_url")?;
                if url.starts_with("http://") || url.starts_with("https://") {
                    url.clone()
                } else {
                    format!("{}{}", base_rendered.trim_end_matches('/'),
                        if url.starts_with('/') { url.clone() } else { format!("/{}", url) })
                }
            } else {
                url.clone()
            };
            (url, full_url)
        };

        // Parse method
        let method: Method = step.method.to_uppercase().parse()
            .map_err(|_| QuicpulseError::Argument(format!("Step '{}': Invalid HTTP method '{}'", step.name, step.method)))?;

        // Dry run: just show what would be executed (skip headers and request)
        if self.dry_run {
            eprintln!("  {} {} {}{}{} {}",
                terminal::muted("[DRY RUN]"),
                terminal::label(&step.name),
                terminal::protocol::http_method(&method.to_string()),
                method,
                RESET,
                terminal::colorize(&full_url, colors::AQUA));
            return Ok(StepResult {
                name: step.name.clone(),
                method: method.to_string(),
                url: full_url,
                status_code: None,
                response_time: Duration::ZERO,
                assertions: Vec::new(),
                extracted: HashMap::new(),
                error: None,
                skipped: false,
            });
        }

        // Execute pre-script if configured
        if let Some(ref pre_script) = step.pre_script {
            let mut ctx = ScriptContext::new();
            // Set up request data for the script
            let mut request_data = RequestData::new(&step.method, &full_url);
            for (k, v) in &step.headers {
                request_data.set_header(k, v);
            }
            if let Some(ref body) = step.body {
                request_data.set_body(body.clone());
            }
            ctx.set_request(request_data);

            // Add workflow variables to context
            for (k, v) in &self.variables {
                ctx.set_variable(k, v.clone());
            }

            match self.execute_script(pre_script, &mut ctx, &step.name).await {
                Ok(_) => {
                    // Script executed successfully
                }
                Err(e) => {
                    return Ok(StepResult {
                        name: step.name.clone(),
                        method: method.to_string(),
                        url: full_url,
                        status_code: None,
                        response_time: Duration::ZERO,
                        assertions: Vec::new(),
                        extracted: HashMap::new(),
                        error: Some(format!("Pre-script error: {}", e)),
                        skipped: false,
                    });
                }
            }
        }

        // Build headers (not needed for dry-run)
        let mut headers = HeaderMap::new();

        // Add workflow global headers
        for (key, value) in &workflow.headers {
            let rendered_value = self.render_template_for_step(value, &step.name, &format!("header '{}'", key))?;
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::try_from(key.as_str()),
                reqwest::header::HeaderValue::from_str(&rendered_value)
            ) {
                headers.insert(name, val);
            }
        }

        // Add step-specific headers
        for (key, value) in &step.headers {
            let rendered_value = self.render_template_for_step(value, &step.name, &format!("header '{}'", key))?;
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::try_from(key.as_str()),
                reqwest::header::HeaderValue::from_str(&rendered_value)
            ) {
                headers.insert(name, val);
            }
        }

        // Determine timeout for this step
        let step_timeout = if let Some(ref t) = step.timeout {
            let rendered_timeout = if self.dry_run {
                self.render_template_dry_run(t)
            } else {
                self.render_template_for_step(t, &step.name, "timeout").unwrap_or_else(|_| t.clone())
            };
            humantime::parse_duration(&rendered_timeout).unwrap_or(self.default_timeout)
        } else {
            self.default_timeout
        };

        // Handle gRPC requests (special path - not HTTP)
        if let Some(ref grpc_config) = step.grpc {
            return self.run_grpc_step(step, grpc_config, &full_url).await;
        }

        // Handle WebSocket requests (special path - not HTTP)
        if let Some(ref ws_config) = step.websocket {
            return self.run_websocket_step(step, ws_config, &full_url, step_timeout).await;
        }

        // Handle fuzzing requests (special path - runs multiple payloads)
        if let Some(ref fuzz_config) = step.fuzz {
            return self.run_fuzz_step(step, fuzz_config, &full_url, &headers, step_timeout).await;
        }

        // Handle benchmarking requests (special path - runs load test)
        if let Some(ref bench_config) = step.bench {
            return self.run_bench_step(step, bench_config, &full_url, &headers, step_timeout).await;
        }

        // Warn about HTTP/3 (not yet implemented)
        if step.http3 == Some(true) {
            if self.options.verbose {
                eprintln!("  {}: HTTP/3 is not yet implemented, falling back to HTTP/2",
                    terminal::warning("Warning"));
            }
        }

        // Handle HAR replay (special path - replay from HAR file)
        if let Some(ref har_config) = step.har {
            return self.run_har_step(step, har_config).await;
        }

        // Handle OpenAPI-driven step (special path)
        if let Some(ref openapi_config) = step.openapi {
            return self.run_openapi_step(step, openapi_config, workflow).await;
        }

        // Run plugins if configured
        if let Some(ref plugins) = step.plugins {
            self.run_plugins(plugins, step)?;
        }

        // Apply session headers/cookies
        self.apply_session_to_request(&mut headers, &full_url);

        // Build URL with query parameters
        let request_url = if !step.query.is_empty() {
            let mut url = reqwest::Url::parse(&full_url)
                .map_err(|e| QuicpulseError::Argument(format!("Step '{}': Invalid URL - {}", step.name, e)))?;
            {
                let mut query_pairs = url.query_pairs_mut();
                for (key, value) in &step.query {
                    let rendered_value = self.render_template_for_step(value, &step.name, &format!("query param '{}'", key))?;
                    query_pairs.append_pair(key, &rendered_value);
                }
            }
            url.to_string()
        } else {
            full_url.clone()
        };

        // Build custom client if needed (for proxy/SSL/redirect options)
        let client = self.build_step_client(step)?;

        // Save headers for curl generation before they're moved
        let headers_for_curl = headers.clone();

        // Build request
        let mut request = client.request(method.clone(), &request_url)
            .timeout(step_timeout)
            .headers(headers);

        // Add authentication if configured
        if let Some(ref auth) = step.auth {
            use super::workflow::StepAuth;
            match auth {
                StepAuth::Basic { username, password } => {
                    let user = self.render_template_for_step(username, &step.name, "auth username")?;
                    let pass = self.render_template_for_step(password, &step.name, "auth password")?;
                    request = request.basic_auth(user, Some(pass));
                }
                StepAuth::Bearer { token } => {
                    let tok = self.render_template_for_step(token, &step.name, "auth token")?;
                    request = request.bearer_auth(tok);
                }
                StepAuth::Digest { username, password } => {
                    // Digest auth requires special handling - add header for now
                    let user = self.render_template_for_step(username, &step.name, "auth username")?;
                    let pass = self.render_template_for_step(password, &step.name, "auth password")?;
                    // Note: reqwest doesn't natively support digest auth, using basic as fallback
                    request = request.basic_auth(user, Some(pass));
                }
                StepAuth::AwsSigV4 { access_key, secret_key, session_token, region, service } => {
                    let ak = self.render_template_for_step(access_key, &step.name, "aws access_key")?;
                    let sk = self.render_template_for_step(secret_key, &step.name, "aws secret_key")?;
                    let reg = self.render_template_for_step(region, &step.name, "aws region")?;
                    let svc = self.render_template_for_step(service, &step.name, "aws service")?;
                    let st = if let Some(token) = session_token {
                        Some(self.render_template_for_step(token, &step.name, "aws session_token")?)
                    } else {
                        None
                    };
                    // Build AWS SigV4 config and sign the request
                    let aws_config = crate::auth::AwsSigV4Config {
                        access_key_id: ak,
                        secret_access_key: sk,
                        session_token: st,
                        region: reg,
                        service: svc,
                    };
                    
                    // Sign the request and add auth headers
                    let existing_headers: Vec<(String, String)> = step.headers.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    
                    if let Ok(auth_headers) = crate::auth::sign_request(
                        &aws_config,
                        &step.method,
                        &full_url,
                        &existing_headers,
                        None,
                        false,
                    ) {
                        for (name, value) in auth_headers {
                            request = request.header(&name, &value);
                        }
                    }
                }
                StepAuth::OAuth2 { token_url, client_id, client_secret, scope } => {
                    // Get OAuth2 token
                    let url = self.render_template_for_step(token_url, &step.name, "oauth2 token_url")?;
                    let cid = self.render_template_for_step(client_id, &step.name, "oauth2 client_id")?;
                    let csec = self.render_template_for_step(client_secret, &step.name, "oauth2 client_secret")?;
                    let scopes: Vec<String> = if let Some(s) = scope {
                        let rendered = self.render_template_for_step(s, &step.name, "oauth2 scope")?;
                        rendered.split_whitespace().map(|s| s.to_string()).collect()
                    } else {
                        Vec::new()
                    };

                    let oauth_config = crate::auth::OAuth2Config {
                        token_url: url,
                        client_id: cid,
                        client_secret: csec,
                        scopes,
                    };

                    // Get token and add as bearer auth
                    if let Ok(token) = crate::auth::get_token(&oauth_config).await {
                        request = request.bearer_auth(&token.access_token);
                    }
                }
                StepAuth::Gcp { project_id: _, service_account: _, scopes: _ } => {
                    // GCP authentication: get access token using gcloud CLI
                    match crate::auth::gcp::get_gcp_access_token().await {
                        Ok(token) => {
                            request = request.bearer_auth(&token);
                        }
                        Err(e) => {
                            return Err(QuicpulseError::Auth(format!("GCP auth failed: {}", e)));
                        }
                    }
                }
                StepAuth::Azure { tenant_id: _, subscription_id: _, resource } => {
                    // Azure authentication: get access token using az CLI
                    let resource_url = resource.as_deref();
                    match crate::auth::azure::get_azure_access_token(resource_url).await {
                        Ok(token) => {
                            request = request.bearer_auth(&token);
                        }
                        Err(e) => {
                            return Err(QuicpulseError::Auth(format!("Azure auth failed: {}", e)));
                        }
                    }
                }
            }
        }

        // Add body based on type
        if let Some(ref graphql_config) = step.graphql {
            // GraphQL request - build JSON body with query/variables
            let query = if graphql_config.introspection == Some(true) {
                // Use standard introspection query
                crate::graphql::introspection::INTROSPECTION_QUERY.to_string()
            } else {
                self.render_template_for_step(&graphql_config.query, &step.name, "graphql query")?
            };
            let mut graphql_body = serde_json::json!({
                "query": query
            });
            if let Some(ref vars) = graphql_config.variables {
                let vars_str = self.render_json_template_for_step(vars, &step.name)?;
                if let Ok(vars_json) = serde_json::from_str::<JsonValue>(&vars_str) {
                    graphql_body["variables"] = vars_json;
                }
            }
            if let Some(ref op_name) = graphql_config.operation_name {
                let rendered_op = self.render_template_for_step(op_name, &step.name, "graphql operation_name")?;
                graphql_body["operationName"] = JsonValue::String(rendered_op);
            } else if graphql_config.introspection == Some(true) {
                graphql_body["operationName"] = JsonValue::String("IntrospectionQuery".to_string());
            }
            let body_str = serde_json::to_string(&graphql_body)
                .map_err(|e| QuicpulseError::Argument(format!("Failed to serialize GraphQL body: {}", e)))?;
            request = request.header("Content-Type", "application/json")
                .body(body_str);
        } else if let Some(ref raw) = step.raw {
            // Raw body takes precedence
            let body_str = self.render_template_for_step(raw, &step.name, "raw body")?;
            if step.compress == Some(true) {
                // Compress raw body with deflate
                let compressed = compress_deflate(body_str.as_bytes());
                request = request.header("Content-Encoding", "deflate")
                    .body(compressed);
            } else {
                request = request.body(body_str);
            }
        } else if let Some(ref body) = step.body {
            // JSON body
            let body_str = self.render_json_template_for_step(body, &step.name)?;
            if step.compress == Some(true) {
                // Compress JSON body with deflate
                let compressed = compress_deflate(body_str.as_bytes());
                request = request.header("Content-Type", "application/json")
                    .header("Content-Encoding", "deflate")
                    .body(compressed);
            } else {
                request = request.header("Content-Type", "application/json")
                    .body(body_str);
            }
        } else if let Some(ref form) = step.form {
            // URL-encoded form
            let mut form_data = HashMap::new();
            for (key, value) in form {
                form_data.insert(key.clone(), self.render_template_for_step(value, &step.name, &format!("form field '{}'", key))?);
            }
            request = request.form(&form_data);
        } else if let Some(ref multipart_fields) = step.multipart {
            // Multipart form
            let mut form = reqwest::multipart::Form::new();
            for field in multipart_fields {
                if let Some(ref file_path) = field.file {
                    // File field
                    let path = self.render_template_for_step(file_path, &step.name, &format!("multipart file '{}'", field.name))?;
                    let file_bytes = std::fs::read(&path)
                        .map_err(|e| QuicpulseError::Io(e))?;
                    let file_name = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("file")
                        .to_string();
                    let mut part = reqwest::multipart::Part::bytes(file_bytes)
                        .file_name(file_name);
                    if let Some(ref ct) = field.content_type {
                        part = part.mime_str(ct)
                            .map_err(|e| QuicpulseError::Argument(format!("Invalid content type: {}", e)))?;
                    }
                    form = form.part(field.name.clone(), part);
                } else if let Some(ref value) = field.value {
                    // Text field
                    let rendered = self.render_template_for_step(value, &step.name, &format!("multipart field '{}'", field.name))?;
                    form = form.text(field.name.clone(), rendered);
                }
            }
            request = request.multipart(form);
        } else if let Some(ref upload_config) = step.upload {
            // File upload with optional compression
            let (data, content_type) = self.apply_upload_config(upload_config, &step.name)?;
            if let Some(ct) = content_type {
                request = request.header("Content-Type", ct);
            }
            if upload_config.compress.is_some() {
                request = request.header("Content-Encoding", upload_config.compress.as_ref().unwrap());
            }
            request = request.body(data);
        }

        // Execute request
        let start = Instant::now();
        let response = request.send().await;
        let response_time = start.elapsed();

        match response {
            Ok(resp) => {
                let status_code = resp.status().as_u16();
                let response_headers = resp.headers().clone();

                // Update session with response cookies
                self.update_session_from_response(&response_headers, &full_url);

                // Handle download if configured
                let body = if let Some(ref download_config) = step.download {
                    self.handle_download_response(resp, download_config, &step.name).await?
                } else {
                    resp.text().await.unwrap_or_default()
                };

                // Execute post-script if configured
                if let Some(ref post_script) = step.post_script {
                    let mut ctx = ScriptContext::new();
                    // Set up response data for the script
                    let mut response_data = ResponseData::new(status_code,
                        serde_json::from_str(&body).unwrap_or(serde_json::Value::String(body.clone())));
                    response_data.elapsed_ms = response_time.as_millis() as u64;
                    for (k, v) in response_headers.iter() {
                        response_data.headers.insert(
                            k.as_str().to_string(),
                            v.to_str().unwrap_or("").to_string()
                        );
                    }
                    ctx.set_response(response_data);

                    // Add workflow variables to context
                    for (k, v) in &self.variables {
                        ctx.set_variable(k, v.clone());
                    }

                    if let Err(e) = self.execute_script(post_script, &mut ctx, &step.name).await {
                        return Ok(StepResult {
                            name: step.name.clone(),
                            method: method.to_string(),
                            url: full_url,
                            status_code: Some(status_code),
                            response_time,
                            assertions: Vec::new(),
                            extracted: HashMap::new(),
                            error: Some(format!("Post-script error: {}", e)),
                            skipped: false,
                        });
                    }
                }

                // Build assertions from step config
                let mut assertions = self.build_step_assertions(step, status_code, response_time, &response_headers, &body);

                // Execute script-based assertion if configured
                if let Some(ref script_assert) = step.script_assert {
                    let mut response_data = ResponseData::new(status_code,
                        serde_json::from_str(&body).unwrap_or(serde_json::Value::String(body.clone())));
                    response_data.elapsed_ms = response_time.as_millis() as u64;
                    for (k, v) in response_headers.iter() {
                        response_data.headers.insert(
                            k.as_str().to_string(),
                            v.to_str().unwrap_or("").to_string()
                        );
                    }

                    match self.execute_script_assertion(script_assert, &response_data, &step.name).await {
                        Ok(passed) => {
                            assertions.push(AssertionResult {
                                assertion: "script_assert".to_string(),
                                passed,
                                message: if passed {
                                    "Script assertion passed".to_string()
                                } else {
                                    "Script assertion failed".to_string()
                                },
                            });
                        }
                        Err(e) => {
                            assertions.push(AssertionResult {
                                assertion: "script_assert".to_string(),
                                passed: false,
                                message: format!("Script error: {}", e),
                            });
                        }
                    }
                }

                // Extract variables
                let extracted = self.extract_variables(step, &body)?;

                // Save response if configured
                if let Some(ref save_config) = step.save {
                    let filtered_headers = if let Some(ref filter) = step.filter {
                        self.filter_headers(&response_headers, filter)
                    } else {
                        response_headers.clone()
                    };
                    self.save_response(save_config, status_code, &filtered_headers, &body, &step.name)?;
                }

                // Generate curl command if requested
                if step.curl == Some(true) {
                    let body_str = step.body.as_ref().map(|b| serde_json::to_string(b).unwrap_or_default());
                    let curl_cmd = self.generate_curl(step, &request_url, &headers_for_curl, body_str.as_deref());
                    eprintln!("\n{} {}:\n{}\n",
                        terminal::muted("# Curl command for"),
                        terminal::label(&step.name),
                        terminal::colorize(&curl_cmd, colors::GREY));
                }

                Ok(StepResult {
                    name: step.name.clone(),
                    method: method.to_string(),
                    url: full_url,
                    status_code: Some(status_code),
                    response_time,
                    assertions,
                    extracted,
                    error: None,
                    skipped: false,
                })
            }
            Err(e) => {
                // Provide better error messages
                let error_msg = if e.is_timeout() {
                    format!("Request timed out after {:?}", step_timeout)
                } else if e.is_connect() {
                    format!("Connection failed: {}", e)
                } else if e.is_request() {
                    format!("Request error: {}", e)
                } else {
                    e.to_string()
                };

                Ok(StepResult {
                    name: step.name.clone(),
                    method: method.to_string(),
                    url: full_url,
                    status_code: None,
                    response_time,
                    assertions: Vec::new(),
                    extracted: HashMap::new(),
                    error: Some(error_msg),
                    skipped: false,
                })
            }
        }
    }

    /// Render a template with step context for better error messages
    /// Also expands magic values like {uuid}, {email}, {random_string:10}, etc.
    fn render_template_for_step(&self, template: &str, step_name: &str, field_name: &str) -> Result<String, QuicpulseError> {
        // First expand magic values (before Tera templating)
        let magic_expanded = expand_magic_values(template).value;

        let mut context = Context::new();
        for (key, value) in &self.variables {
            context.insert(key, value);
        }

        Tera::one_off(&magic_expanded, &context, false)
            .map_err(|e| {
                // Extract the undefined variable name from the error message
                let error_str = e.to_string();
                if error_str.contains("not found") {
                    // Try to extract variable name using cached regex
                    if let Some(caps) = VAR_NOT_FOUND_RE.captures(&error_str) {
                        let var_name = &caps[1];
                        return QuicpulseError::Argument(format!(
                            "Step '{}', {}: undefined variable '{}'. Available variables: {}",
                            step_name,
                            field_name,
                            var_name,
                            self.variables.keys().cloned().collect::<Vec<_>>().join(", ")
                        ));
                    }
                }
                QuicpulseError::Argument(format!("Step '{}', {}: template error - {}", step_name, field_name, e))
            })
    }

    /// Render a template for dry-run mode (doesn't fail on undefined variables)
    /// Also expands magic values like {uuid}, {email}, {random_string:10}, etc.
    fn render_template_dry_run(&self, template: &str) -> String {
        // First expand magic values
        let magic_expanded = expand_magic_values(template).value;

        let mut context = Context::new();
        for (key, value) in &self.variables {
            context.insert(key, value);
        }

        // Try to render, on failure show the template with placeholders
        match Tera::one_off(&magic_expanded, &context, false) {
            Ok(rendered) => rendered,
            Err(_) => {
                // Replace undefined variables with placeholder markers - use cached regex
                TEMPLATE_VAR_RE.replace_all(&magic_expanded, |caps: &regex::Captures| {
                    let var_name = &caps[1];
                    if self.variables.contains_key(var_name) {
                        // Variable exists, render it
                        self.variables.get(var_name)
                            .map(|v| match v {
                                JsonValue::String(s) => s.clone(),
                                other => other.to_string(),
                            })
                            .unwrap_or_else(|| format!("<{}>", var_name))
                    } else {
                        // Variable doesn't exist, show placeholder
                        format!("<{}>", var_name)
                    }
                }).to_string()
            }
        }
    }

    /// Render a JSON value with step context for better error messages
    fn render_json_template_for_step(&self, json: &JsonValue, step_name: &str) -> Result<String, QuicpulseError> {
        let json_str = serde_json::to_string(json)
            .map_err(|e| QuicpulseError::Argument(format!("Step '{}': JSON error - {}", step_name, e)))?;
        
        let mut escaped_context = tera::Context::new();
        for (key, value) in &self.variables {
            // JSON-encode string values to escape quotes properly
            if let JsonValue::String(s) = value {
                // The string is already JSON-escaped when we serialize to the template
                // but we need to ensure inner quotes don't break the outer JSON structure
                escaped_context.insert(key, &s);
            } else {
                escaped_context.insert(key, &value.to_string());
            }
        }
        
        self.render_template_for_step(&json_str, step_name, "body")
    }

    /// Evaluate a skip condition
    fn evaluate_condition(&self, condition: &str) -> bool {
        // Simple variable check: {{var}} or !{{var}}
        let negated = condition.starts_with('!');
        let var_name = condition.trim_start_matches('!')
            .trim_matches(|c| c == '{' || c == '}')
            .trim();

        let value = self.variables.get(var_name);
        let is_truthy = match value {
            Some(JsonValue::Bool(b)) => *b,
            Some(JsonValue::Null) => false,
            Some(JsonValue::String(s)) => !s.is_empty(),
            Some(JsonValue::Number(n)) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
            Some(JsonValue::Array(a)) => !a.is_empty(),
            Some(JsonValue::Object(o)) => !o.is_empty(),
            None => false,
        };

        if negated { !is_truthy } else { is_truthy }
    }

    /// Build assertions from step configuration
    fn build_step_assertions(
        &self,
        step: &WorkflowStep,
        status_code: u16,
        response_time: Duration,
        headers: &HeaderMap,
        body: &str,
    ) -> Vec<AssertionResult> {
        let mut assertions = Vec::new();

        // Status assertion
        if let Some(ref status) = step.assert.status {
            let pattern = match status {
                StatusAssertion::Exact(code) => code.to_string(),
                StatusAssertion::Range(range) => range.clone(),
            };
            let assertion_list = vec![Assertion::Status(pattern)];
            assertions.extend(check_assertions(&assertion_list, status_code, response_time, headers, body));
        }

        // Latency assertion
        if let Some(ref latency) = step.assert.latency {
            if let Ok(max_duration) = humantime::parse_duration(latency.trim_start_matches('<').trim()) {
                let assertion_list = vec![Assertion::Time(max_duration)];
                assertions.extend(check_assertions(&assertion_list, status_code, response_time, headers, body));
            }
        }

        // Header assertions
        for (name, value) in &step.assert.headers {
            let assertion_list = vec![Assertion::Header(name.clone(), Some(value.clone()))];
            assertions.extend(check_assertions(&assertion_list, status_code, response_time, headers, body));
        }

        // Body assertions (key:value pairs)
        for (key, expected) in &step.assert.body {
            let pattern = format!("{}:{}", key, expected);
            let assertion_list = vec![Assertion::Body(pattern)];
            assertions.extend(check_assertions(&assertion_list, status_code, response_time, headers, body));
        }

        assertions
    }

    /// Extract variables from response
    fn extract_variables(&self, step: &WorkflowStep, body: &str) -> Result<HashMap<String, JsonValue>, QuicpulseError> {
        let mut extracted = HashMap::new();

        if step.extract.is_empty() {
            return Ok(extracted);
        }

        // Parse body as JSON
        let json: JsonValue = serde_json::from_str(body)
            .unwrap_or(JsonValue::Null);

        for (var_name, jq_expr) in &step.extract {
            // Convert extraction path to JQ expression
            let expr = if jq_expr.starts_with("response.body.") {
                format!(".{}", jq_expr.strip_prefix("response.body.").unwrap())
            } else if jq_expr.starts_with('.') {
                jq_expr.clone()
            } else {
                format!(".{}", jq_expr)
            };

            // Apply JQ filter
            if let Ok(results) = filter::apply_filter(&json, &expr) {
                if !results.is_empty() {
                    extracted.insert(var_name.clone(), results[0].clone());
                }
            }
        }

        Ok(extracted)
    }

    /// Run a gRPC step
    async fn run_grpc_step(
        &self,
        step: &WorkflowStep,
        grpc_config: &GrpcConfig,
        url: &str,
    ) -> Result<StepResult, QuicpulseError> {
        use futures::StreamExt;

        let start = Instant::now();

        // Parse gRPC endpoint from URL
        let endpoint = GrpcEndpoint {
            host: url.trim_start_matches("http://")
                .trim_start_matches("https://")
                .trim_start_matches("grpc://")
                .trim_start_matches("grpcs://")
                .split(':')
                .next()
                .unwrap_or("localhost")
                .split('/')
                .next()
                .unwrap_or("localhost")
                .to_string(),
            port: url.split(':')
                .nth(1)
                .and_then(|s| s.split('/').next())
                .and_then(|s| s.parse().ok())
                .unwrap_or(443),
            service: Some(grpc_config.service.clone()),
            method: Some(grpc_config.method.clone()),
            use_tls: grpc_config.tls.unwrap_or(url.starts_with("https://") || url.starts_with("grpcs://")),
        };

        // Connect and make call
        let step_timeout = step.timeout.as_ref()
            .and_then(|t| humantime::parse_duration(t).ok());

        // Build headers for gRPC metadata from both step headers and grpc.metadata
        let mut headers: Vec<(String, String)> = step.headers.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        // Add metadata from grpc config
        if let Some(ref metadata) = grpc_config.metadata {
            for (k, v) in metadata {
                headers.push((k.clone(), v.clone()));
            }
        }

        // Bug #2 fix: Pass None for SSL config in pipeline (could be enhanced to read from workflow options)
        let mut client = GrpcClient::connect_with_options(endpoint.clone(), step_timeout, Some(headers), None)
            .await
            .map_err(|e| QuicpulseError::Connection(format!("gRPC connection failed: {}", e)))?;

        // Load proto file if specified
        if let Some(ref proto_path) = grpc_config.proto_file {
            let path = std::path::Path::new(proto_path);
            // Note: import_paths not yet supported in GrpcClient
            client.load_proto(path)?;
        }

        // Determine streaming mode
        let streaming_mode = if let Some(ref mode) = grpc_config.streaming {
            mode.as_str()
        } else if let Some(method_info) = client.get_method_info(&grpc_config.service, &grpc_config.method) {
            if method_info.client_streaming && method_info.server_streaming {
                "bidi"
            } else if method_info.server_streaming {
                "server"
            } else if method_info.client_streaming {
                "client"
            } else {
                "unary"
            }
        } else {
            "unary"
        };

        match streaming_mode {
            "server" => {
                // Server streaming: single request, stream of responses
                let message = grpc_config.message.clone().unwrap_or(serde_json::json!({}));
                let rendered_message = self.render_json_template_for_step(&message, &step.name)?;
                let request_json: JsonValue = serde_json::from_str(&rendered_message)
                    .map_err(|e| QuicpulseError::Argument(format!("Invalid gRPC message JSON: {}", e)))?;

                let response = client.call_server_streaming(&grpc_config.service, &grpc_config.method, &request_json).await?;

                if !response.is_ok() {
                    let response_time = start.elapsed();
                    return Ok(StepResult {
                        name: step.name.clone(),
                        method: format!("gRPC/{}/{} (server streaming)", grpc_config.service, grpc_config.method),
                        url: endpoint.uri(),
                        status_code: Some(500),
                        response_time,
                        assertions: Vec::new(),
                        extracted: HashMap::new(),
                        error: Some(format!("gRPC error: {}", response.message())),
                        skipped: false,
                    });
                }

                // Collect all streamed responses into an array
                let mut responses: Vec<JsonValue> = Vec::new();
                let mut stream = response.into_stream();
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(json) => responses.push(json),
                        Err(e) => {
                            let response_time = start.elapsed();
                            return Ok(StepResult {
                                name: step.name.clone(),
                                method: format!("gRPC/{}/{} (server streaming)", grpc_config.service, grpc_config.method),
                                url: endpoint.uri(),
                                status_code: None,
                                response_time,
                                assertions: Vec::new(),
                                extracted: HashMap::new(),
                                error: Some(format!("Stream error: {}", e)),
                                skipped: false,
                            });
                        }
                    }
                }

                let response_time = start.elapsed();
                let body = serde_json::to_string_pretty(&responses).unwrap_or_default();
                let assertions = self.build_step_assertions(step, 200, response_time, &HeaderMap::new(), &body);
                let extracted = self.extract_variables(step, &body)?;

                Ok(StepResult {
                    name: step.name.clone(),
                    method: format!("gRPC/{}/{} (server streaming)", grpc_config.service, grpc_config.method),
                    url: endpoint.uri(),
                    status_code: Some(200),
                    response_time,
                    assertions,
                    extracted,
                    error: None,
                    skipped: false,
                })
            }

            "client" => {
                // Client streaming: stream of requests, single response
                let messages = grpc_config.messages.clone().unwrap_or_else(|| {
                    vec![grpc_config.message.clone().unwrap_or(serde_json::json!({}))]
                });

                // Render templates for each message
                let mut rendered_messages: Vec<JsonValue> = Vec::new();
                for msg in &messages {
                    let rendered = self.render_json_template_for_step(msg, &step.name)?;
                    let json: JsonValue = serde_json::from_str(&rendered)
                        .map_err(|e| QuicpulseError::Argument(format!("Invalid gRPC message JSON: {}", e)))?;
                    rendered_messages.push(json);
                }

                let request_stream = futures::stream::iter(rendered_messages);
                let response = client.call_client_streaming(&grpc_config.service, &grpc_config.method, request_stream).await?;

                let response_time = start.elapsed();
                let body = response.json()
                    .map(|j| serde_json::to_string_pretty(&j).unwrap_or_default())
                    .unwrap_or_default();

                let status_code = if response.is_ok() { 200 } else { 500 };
                let assertions = self.build_step_assertions(step, status_code, response_time, &HeaderMap::new(), &body);
                let extracted = self.extract_variables(step, &body)?;

                Ok(StepResult {
                    name: step.name.clone(),
                    method: format!("gRPC/{}/{} (client streaming)", grpc_config.service, grpc_config.method),
                    url: endpoint.uri(),
                    status_code: Some(status_code),
                    response_time,
                    assertions,
                    extracted,
                    error: if response.is_ok() { None } else { Some(response.message().to_string()) },
                    skipped: false,
                })
            }

            "bidi" => {
                // Bidirectional streaming
                let messages = grpc_config.messages.clone().unwrap_or_else(|| {
                    vec![grpc_config.message.clone().unwrap_or(serde_json::json!({}))]
                });

                // Render templates for each message
                let mut rendered_messages: Vec<JsonValue> = Vec::new();
                for msg in &messages {
                    let rendered = self.render_json_template_for_step(msg, &step.name)?;
                    let json: JsonValue = serde_json::from_str(&rendered)
                        .map_err(|e| QuicpulseError::Argument(format!("Invalid gRPC message JSON: {}", e)))?;
                    rendered_messages.push(json);
                }

                let request_stream = futures::stream::iter(rendered_messages);
                let response = client.call_bidi_streaming(&grpc_config.service, &grpc_config.method, request_stream).await?;

                if !response.is_ok() {
                    let response_time = start.elapsed();
                    return Ok(StepResult {
                        name: step.name.clone(),
                        method: format!("gRPC/{}/{} (bidi streaming)", grpc_config.service, grpc_config.method),
                        url: endpoint.uri(),
                        status_code: Some(500),
                        response_time,
                        assertions: Vec::new(),
                        extracted: HashMap::new(),
                        error: Some(format!("gRPC error: {}", response.message())),
                        skipped: false,
                    });
                }

                // Collect all streamed responses
                let mut responses: Vec<JsonValue> = Vec::new();
                let mut stream = response.into_stream();
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(json) => responses.push(json),
                        Err(e) => {
                            let response_time = start.elapsed();
                            return Ok(StepResult {
                                name: step.name.clone(),
                                method: format!("gRPC/{}/{} (bidi streaming)", grpc_config.service, grpc_config.method),
                                url: endpoint.uri(),
                                status_code: None,
                                response_time,
                                assertions: Vec::new(),
                                extracted: HashMap::new(),
                                error: Some(format!("Stream error: {}", e)),
                                skipped: false,
                            });
                        }
                    }
                }

                let response_time = start.elapsed();
                let body = serde_json::to_string_pretty(&responses).unwrap_or_default();
                let assertions = self.build_step_assertions(step, 200, response_time, &HeaderMap::new(), &body);
                let extracted = self.extract_variables(step, &body)?;

                Ok(StepResult {
                    name: step.name.clone(),
                    method: format!("gRPC/{}/{} (bidi streaming)", grpc_config.service, grpc_config.method),
                    url: endpoint.uri(),
                    status_code: Some(200),
                    response_time,
                    assertions,
                    extracted,
                    error: None,
                    skipped: false,
                })
            }

            _ => {
                // Unary call (default)
                let message = grpc_config.message.clone().unwrap_or(serde_json::json!({}));
                let rendered_message = self.render_json_template_for_step(&message, &step.name)?;
                let request_json: JsonValue = serde_json::from_str(&rendered_message)
                    .map_err(|e| QuicpulseError::Argument(format!("Invalid gRPC message JSON: {}", e)))?;

                match client.call_unary(&grpc_config.service, &grpc_config.method, &request_json).await {
                    Ok(response) => {
                        let response_time = start.elapsed();
                        let body = response.json()
                            .map(|j| serde_json::to_string_pretty(&j).unwrap_or_default())
                            .unwrap_or_default();

                        let status_code = if response.is_ok() { 200 } else { 500 };
                        let assertions = self.build_step_assertions(step, status_code, response_time, &HeaderMap::new(), &body);
                        let extracted = self.extract_variables(step, &body)?;

                        Ok(StepResult {
                            name: step.name.clone(),
                            method: format!("gRPC/{}/{}", grpc_config.service, grpc_config.method),
                            url: endpoint.uri(),
                            status_code: Some(status_code),
                            response_time,
                            assertions,
                            extracted,
                            error: None,
                            skipped: false,
                        })
                    }
                    Err(e) => {
                        let response_time = start.elapsed();
                        Ok(StepResult {
                            name: step.name.clone(),
                            method: format!("gRPC/{}/{}", grpc_config.service, grpc_config.method),
                            url: endpoint.uri(),
                            status_code: None,
                            response_time,
                            assertions: Vec::new(),
                            extracted: HashMap::new(),
                            error: Some(format!("gRPC call failed: {}", e)),
                            skipped: false,
                        })
                    }
                }
            }
        }
    }

    /// Run a WebSocket step
    async fn run_websocket_step(
        &self,
        step: &WorkflowStep,
        ws_config: &WebSocketConfig,
        url: &str,
        timeout: Duration,
    ) -> Result<StepResult, QuicpulseError> {
        use crate::websocket::client::WsClient;
        use crate::websocket::types::WsMessage;
        use crate::websocket::codec::decode_binary;

        let start = Instant::now();

        // Parse WebSocket endpoint from URL
        let ws_url = url.trim_start_matches("http://")
            .trim_start_matches("https://");
        let use_tls = url.starts_with("https://") || url.starts_with("wss://");
        
        // Extract host and port
        let (host, port, path) = {
            let cleaned = ws_url.trim_start_matches("ws://")
                .trim_start_matches("wss://");
            let (host_port, path) = cleaned.split_once('/').unwrap_or((cleaned, ""));
            let path = format!("/{}", path);
            
            if let Some((host, port_str)) = host_port.split_once(':') {
                let port: u16 = port_str.parse().unwrap_or(if use_tls { 443 } else { 80 });
                (host.to_string(), port, path)
            } else {
                (host_port.to_string(), if use_tls { 443 } else { 80 }, path)
            }
        };

        let endpoint = WsEndpoint {
            host,
            port,
            path,
            use_tls,
            subprotocol: ws_config.subprotocol.clone(),
        };

        // Build WebSocket options
        let ws_headers: Vec<(String, String)> = step.headers.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let ping_interval = ws_config.ping_interval.map(|s| std::time::Duration::from_secs(s));

        let options = WsOptions {
            timeout: Some(timeout),
            compress: ws_config.compress.unwrap_or(false),
            binary_mode: ws_config.binary_mode.as_ref().and_then(|m| m.parse().ok()),
            ping_interval,
            max_messages: ws_config.max_messages.unwrap_or(0),
            headers: ws_headers,
        };

        // Connect to WebSocket server
        let skip_tls = step.insecure.unwrap_or(false);
        let mut client = WsClient::connect_simple(&endpoint, &options, skip_tls)
            .await
            .map_err(|e| QuicpulseError::WebSocket(format!("WebSocket connection failed: {}", e)))?;

        // Determine mode and execute
        let mode = ws_config.mode.as_deref().unwrap_or("send");
        
        let mut received_messages: Vec<String> = Vec::new();
        let mut last_message = String::new();

        match mode {
            "send" => {
                // Send a single message and optionally wait for response
                if let Some(ref msg) = ws_config.message {
                    let rendered = self.render_template_for_step(msg, &step.name, "websocket message")?;
                    client.send_text(&rendered).await?;
                }

                // Send binary if specified
                if let Some(ref binary) = ws_config.binary {
                    let binary_mode = ws_config.binary_mode.as_deref()
                        .and_then(|m| m.parse().ok())
                        .unwrap_or(BinaryMode::Hex);
                    let data = decode_binary(binary, binary_mode)?;
                    client.send_binary(&data).await?;
                }

                // Wait for response if configured
                if let Some(wait_ms) = ws_config.wait_response {
                    let wait_duration = Duration::from_millis(wait_ms);
                    if let Some(msg) = client.receive_timeout(wait_duration).await? {
                        last_message = match &msg {
                            WsMessage::Text(t) => t.clone(),
                            WsMessage::Binary(b) => format!("[binary: {} bytes]", b.len()),
                            WsMessage::Close(code, reason) => format!("[close: {:?} {}]", code, reason),
                            _ => String::new(),
                        };
                        received_messages.push(last_message.clone());
                    }
                }
            }

            "stream" => {
                // Send multiple messages
                if let Some(ref messages) = ws_config.messages {
                    for msg in messages {
                        let rendered = self.render_template_for_step(msg, &step.name, "websocket message")?;
                        client.send_text(&rendered).await?;
                    }
                }

                // Wait for response if configured
                if let Some(wait_ms) = ws_config.wait_response {
                    let wait_duration = Duration::from_millis(wait_ms);
                    while let Some(msg) = client.receive_timeout(wait_duration).await? {
                        let text = match &msg {
                            WsMessage::Text(t) => t.clone(),
                            WsMessage::Binary(b) => format!("[binary: {} bytes]", b.len()),
                            WsMessage::Close(_, _) => break,
                            _ => continue,
                        };
                        received_messages.push(text.clone());
                        last_message = text;

                        if options.max_messages > 0 && received_messages.len() >= options.max_messages {
                            break;
                        }
                    }
                }
            }

            "listen" => {
                // Listen for messages
                let max = ws_config.max_messages.unwrap_or(1);
                let wait_duration = ws_config.wait_response
                    .map(Duration::from_millis)
                    .unwrap_or(timeout);

                for _ in 0..max {
                    if let Some(msg) = client.receive_timeout(wait_duration).await? {
                        let text = match &msg {
                            WsMessage::Text(t) => t.clone(),
                            WsMessage::Binary(b) => format!("[binary: {} bytes]", b.len()),
                            WsMessage::Close(_, _) => break,
                            _ => continue,
                        };
                        received_messages.push(text.clone());
                        last_message = text;
                    } else {
                        break;
                    }
                }
            }

            _ => {
                return Err(QuicpulseError::Argument(format!(
                    "Unknown WebSocket mode '{}'. Use 'send', 'stream', or 'listen'",
                    mode
                )));
            }
        }

        // Close connection
        let _ = client.close().await;

        let response_time = start.elapsed();

        // Build response body as JSON array if multiple messages, or single message
        let body = if received_messages.len() > 1 {
            serde_json::to_string(&received_messages).unwrap_or(last_message.clone())
        } else {
            last_message.clone()
        };

        // Build assertions
        let assertions = if !step.assert.body.is_empty() {
            let mut results = Vec::new();
            for (expr, expected) in &step.assert.body {
                // Simple contains check for WebSocket responses
                let expected_str = match expected {
                    JsonValue::String(s) => s.clone(),
                    _ => expected.to_string(),
                };
                let passed = body.contains(&expected_str);
                results.push(AssertionResult {
                    assertion: format!("body.{} contains", expr),
                    passed,
                    message: if passed {
                        format!("Body contains expected value")
                    } else {
                        format!("Expected body to contain '{}', got: {}", expected_str, 
                            if body.len() > 100 { format!("{}...", &body[..100]) } else { body.clone() })
                    },
                });
            }
            results
        } else {
            Vec::new()
        };

        // Extract variables from response
        let mut extracted = HashMap::new();
        if !step.extract.is_empty() {
            // Try to parse response as JSON for extraction
            if let Ok(json) = serde_json::from_str::<JsonValue>(&body) {
                for (var_name, jq_expr) in &step.extract {
                    // Convert extraction path to JQ expression (same as extract_variables)
                    let expr = if jq_expr.starts_with("response.body.") {
                        format!(".{}", jq_expr.strip_prefix("response.body.").unwrap())
                    } else if jq_expr.starts_with('.') {
                        jq_expr.clone()
                    } else {
                        format!(".{}", jq_expr)
                    };

                    // Apply JQ filter
                    if let Ok(results) = filter::apply_filter(&json, &expr) {
                        if !results.is_empty() {
                            extracted.insert(var_name.clone(), results[0].clone());
                        }
                    }
                }
            } else {
                // For non-JSON responses, allow extracting the raw body
                for (var_name, json_path) in &step.extract {
                    if json_path == "response.body" || json_path == "body" {
                        extracted.insert(var_name.clone(), JsonValue::String(body.clone()));
                    }
                }
            }
        }

        Ok(StepResult {
            name: step.name.clone(),
            method: format!("WebSocket/{}", mode),
            url: endpoint.url(),
            status_code: Some(101), // WebSocket upgrade status
            response_time,
            assertions,
            extracted,
            error: None,
            skipped: false,
        })
    }

    /// Run a fuzzing step
    async fn run_fuzz_step(
        &self,
        step: &WorkflowStep,
        fuzz_config: &FuzzConfig,
        url: &str,
        headers: &HeaderMap,
        timeout: Duration,
    ) -> Result<StepResult, QuicpulseError> {
        let start = Instant::now();

        // Parse categories
        let categories: Option<Vec<PayloadCategory>> = fuzz_config.categories.as_ref().map(|cats| {
            cats.iter().filter_map(|cat_str| {
                match cat_str.to_lowercase().as_str() {
                    "sql" | "sqli" => Some(PayloadCategory::SqlInjection),
                    "xss" => Some(PayloadCategory::Xss),
                    "cmd" | "command" => Some(PayloadCategory::CommandInjection),
                    "path" | "traversal" => Some(PayloadCategory::PathTraversal),
                    "boundary" | "bound" => Some(PayloadCategory::Boundary),
                    "type" | "confusion" => Some(PayloadCategory::TypeConfusion),
                    "format" | "fmt" => Some(PayloadCategory::FormatString),
                    "int" | "integer" | "overflow" => Some(PayloadCategory::IntegerOverflow),
                    "unicode" | "uni" => Some(PayloadCategory::Unicode),
                    "nosql" | "mongo" => Some(PayloadCategory::NoSqlInjection),
                    _ => None,
                }
            }).collect()
        });

        let options = FuzzOptions {
            concurrency: fuzz_config.concurrency.unwrap_or(10),
            timeout,
            categories,
            verbose: self.options.verbose,
            anomalies_only: fuzz_config.anomalies_only.unwrap_or(false),
            stop_on_anomaly: fuzz_config.stop_on_anomaly.unwrap_or(false),
            min_risk_level: fuzz_config.risk_level.unwrap_or(1),
            proxy: None,
            insecure: step.insecure.unwrap_or(false),
            ca_cert: step.ca_cert.clone(),
            body_format: FuzzBodyFormat::Json,
            custom_payloads: Vec::new(),
        };

        // Get fields to fuzz from step body or fuzz_config.fields
        let fields_to_fuzz: Vec<String> = fuzz_config.fields.clone().unwrap_or_else(|| {
            step.body.as_ref()
                .and_then(|b| b.as_object())
                .map(|obj| obj.keys().cloned().collect())
                .unwrap_or_default()
        });

        if fields_to_fuzz.is_empty() {
            return Ok(StepResult {
                name: step.name.clone(),
                method: "FUZZ".to_string(),
                url: url.to_string(),
                status_code: None,
                response_time: start.elapsed(),
                assertions: Vec::new(),
                extracted: HashMap::new(),
                error: Some("No fields to fuzz. Provide fields in fuzz config or body.".to_string()),
                skipped: false,
            });
        }

        let runner = FuzzRunner::new(options.clone())?;
        let method: reqwest::Method = step.method.parse()
            .map_err(|_| QuicpulseError::Argument(format!("Invalid method: {}", step.method)))?;

        let (results, summary) = runner.run(
            method,
            url,
            step.body.as_ref(),
            &fields_to_fuzz,
            headers.clone(),
        ).await?;

        let response_time = start.elapsed();

        // Print results if verbose
        if self.options.verbose {
            eprintln!("{}", format_fuzz_results(&results, &summary, options.anomalies_only));
        }

        // Build assertions based on fuzzing results
        let mut assertions = Vec::new();
        assertions.push(AssertionResult {
            assertion: "no_server_errors".to_string(),
            passed: summary.server_errors == 0,
            message: if summary.server_errors == 0 {
                "No server errors during fuzzing".to_string()
            } else {
                format!("{} server errors found", summary.server_errors)
            },
        });

        assertions.push(AssertionResult {
            assertion: "no_anomalies".to_string(),
            passed: summary.anomalies == 0,
            message: if summary.anomalies == 0 {
                "No anomalies detected".to_string()
            } else {
                format!("{} anomalies detected", summary.anomalies)
            },
        });

        Ok(StepResult {
            name: step.name.clone(),
            method: format!("FUZZ ({} payloads)", summary.total_requests),
            url: url.to_string(),
            status_code: if summary.server_errors > 0 { Some(500) } else { Some(200) },
            response_time,
            assertions,
            extracted: HashMap::new(),
            error: None,
            skipped: false,
        })
    }

    /// Run a benchmarking step
    async fn run_bench_step(
        &self,
        step: &WorkflowStep,
        bench_config: &BenchConfig,
        url: &str,
        headers: &HeaderMap,
        timeout: Duration,
    ) -> Result<StepResult, QuicpulseError> {
        let start = Instant::now();

        // Create a custom client for benchmarking
        let mut builder = Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(100)
            .pool_idle_timeout(Duration::from_secs(30));

        if step.insecure == Some(true) {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let client = builder.build()
            .map_err(|e| QuicpulseError::Request(e))?;

        let config = BenchmarkConfig {
            total_requests: bench_config.requests,
            concurrency: bench_config.concurrency.unwrap_or(10),
            url: url.to_string(),
            method: step.method.clone(),
        };

        // Create benchmark runner with our custom client
        let runner = crate::bench::BenchmarkRunner {
            config: config.clone(),
            client,
            body: step.body.as_ref().map(|b| serde_json::to_vec(b).unwrap_or_default()),
            headers: headers.clone(),
        };

        // Run warmup if configured
        if let Some(warmup) = bench_config.warmup {
            if warmup > 0 && self.options.verbose {
                eprintln!("  {} {} warmup requests...",
                    terminal::muted("Running"),
                    terminal::number(&warmup.to_string()));
            }
            // Note: warmup is informational only for now
        }

        let result = runner.run().await?;
        let response_time = start.elapsed();

        // Print results if verbose
        if self.options.verbose {
            eprintln!("{}", format_bench_results(&result));
        }

        // Build assertions based on benchmark results
        let mut assertions = Vec::new();
        assertions.push(AssertionResult {
            assertion: "success_rate".to_string(),
            passed: result.stats.success_rate >= 0.95,
            message: format!("Success rate: {:.1}%", result.stats.success_rate * 100.0),
        });

        if let Some(ref latency) = step.assert.latency {
            if let Ok(max_duration) = humantime::parse_duration(latency.trim_start_matches('<').trim()) {
                let p95_ms = result.stats.latency.p95_ms;
                let passed = p95_ms <= max_duration.as_secs_f64() * 1000.0;
                assertions.push(AssertionResult {
                    assertion: "latency_p95".to_string(),
                    passed,
                    message: format!("p95 latency: {:.2}ms (max: {:?})", p95_ms, max_duration),
                });
            }
        }

        Ok(StepResult {
            name: step.name.clone(),
            method: format!("BENCH ({} req @ {} conc)", config.total_requests, config.concurrency),
            url: url.to_string(),
            status_code: Some(200),
            response_time,
            assertions,
            extracted: HashMap::from([
                ("bench_rps".to_string(), JsonValue::Number(serde_json::Number::from_f64(result.stats.requests_per_second).unwrap_or(0.into()))),
                ("bench_p50_ms".to_string(), JsonValue::Number(serde_json::Number::from_f64(result.stats.latency.p50_ms).unwrap_or(0.into()))),
                ("bench_p95_ms".to_string(), JsonValue::Number(serde_json::Number::from_f64(result.stats.latency.p95_ms).unwrap_or(0.into()))),
                ("bench_p99_ms".to_string(), JsonValue::Number(serde_json::Number::from_f64(result.stats.latency.p99_ms).unwrap_or(0.into()))),
            ]),
            error: None,
            skipped: false,
        })
    }

    /// Handle download response - save to file and return body as string
    async fn handle_download_response(
        &self,
        response: reqwest::Response,
        download_config: &DownloadConfig,
        step_name: &str,
    ) -> Result<String, QuicpulseError> {
        use std::path::PathBuf;

        // Render the output path with variables
        let output_path = self.render_template_for_step(&download_config.path, step_name, "download path")?;
        let path = PathBuf::from(&output_path);

        // Check if file exists and handle overwrite
        if path.exists() && !download_config.overwrite.unwrap_or(false) {
            if download_config.resume.unwrap_or(false) {
                // Resume is handled at request time with Range header
                // For now, we'll just append
            } else {
                return Err(QuicpulseError::Download(format!(
                    "File '{}' already exists. Use overwrite: true or resume: true",
                    output_path
                )));
            }
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| QuicpulseError::Io(e))?;
        }

        // Get content length for progress
        let content_length = response.content_length();

        // Read body as bytes
        let bytes = response.bytes().await
            .map_err(|e| QuicpulseError::Request(e))?;

        // Write to file
        std::fs::write(&path, &bytes)
            .map_err(|e| QuicpulseError::Io(e))?;

        if self.options.verbose {
            eprintln!("  {} {} bytes to {}",
                terminal::success("Downloaded"),
                terminal::number(&bytes.len().to_string()),
                terminal::colorize(&output_path, colors::AQUA));
        }

        // Return a summary as body text
        Ok(format!(
            "{{\"downloaded\": true, \"path\": \"{}\", \"size\": {}}}",
            output_path.replace('\\', "\\\\").replace('"', "\\\""),
            content_length.unwrap_or(bytes.len() as u64)
        ))
    }

    /// Load dotenv file and add variables to workflow
    fn load_dotenv(&mut self, dotenv_path: &str) -> Result<(), QuicpulseError> {
        let path = std::path::Path::new(dotenv_path);
        let env_vars = EnvVars::load_file(path)?;

        // Add dotenv variables to workflow variables
        for (key, value) in env_vars.all() {
            self.variables.insert(key.clone(), serde_json::Value::String(value.clone()));
        }

        if self.options.verbose {
            eprintln!("  {} {} variables from {}",
                terminal::success("Loaded"),
                terminal::number(&env_vars.all().len().to_string()),
                terminal::colorize(dotenv_path, colors::AQUA));
        }

        Ok(())
    }

    /// Save response to file
    fn save_response(
        &self,
        save_config: &SaveConfig,
        status_code: u16,
        headers: &HeaderMap,
        body: &str,
        step_name: &str,
    ) -> Result<(), QuicpulseError> {
        let output_path = self.render_template_for_step(&save_config.path, step_name, "save path")?;
        let path = std::path::PathBuf::from(&output_path);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| QuicpulseError::Io(e))?;
        }

        let what = save_config.what.as_deref().unwrap_or("body");
        let format = save_config.format.as_deref().unwrap_or("raw");
        let append = save_config.append.unwrap_or(false);

        let content = match what {
            "headers" => {
                let mut output = String::new();
                for (key, value) in headers.iter() {
                    output.push_str(&format!("{}: {}\n", key.as_str(), value.to_str().unwrap_or("")));
                }
                output
            }
            "body" => body.to_string(),
            "all" | "response" => {
                let mut output = format!("HTTP/1.1 {}\n", status_code);
                for (key, value) in headers.iter() {
                    output.push_str(&format!("{}: {}\n", key.as_str(), value.to_str().unwrap_or("")));
                }
                output.push('\n');
                output.push_str(body);
                output
            }
            _ => body.to_string(),
        };

        let final_content = match format {
            "json" => {
                // Try to format as pretty JSON
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    serde_json::to_string_pretty(&json).unwrap_or(content)
                } else {
                    content
                }
            }
            _ => content,
        };

        if append {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| QuicpulseError::Io(e))?;
            file.write_all(final_content.as_bytes())
                .map_err(|e| QuicpulseError::Io(e))?;
        } else {
            std::fs::write(&path, &final_content)
                .map_err(|e| QuicpulseError::Io(e))?;
        }

        if self.options.verbose {
            eprintln!("  {} {} to {}",
                terminal::success("Saved"),
                terminal::label(what),
                terminal::colorize(&output_path, colors::AQUA));
        }

        Ok(())
    }

    /// Generate curl command for debugging
    fn generate_curl(
        &self,
        step: &WorkflowStep,
        url: &str,
        headers: &HeaderMap,
        body: Option<&str>,
    ) -> String {
        let mut cmd = format!("curl -X {} '{}'", step.method, url);

        // Add headers
        for (key, value) in headers.iter() {
            if let Ok(val) = value.to_str() {
                cmd.push_str(&format!(" \\\n  -H '{}: {}'", key.as_str(), val));
            }
        }

        // Add body
        if let Some(body_str) = body {
            cmd.push_str(&format!(" \\\n  -d '{}'", body_str.replace('\'', "'\\''")));
        }

        cmd
    }

    /// Run a HAR replay step
    async fn run_har_step(
        &self,
        step: &WorkflowStep,
        har_config: &HarConfig,
    ) -> Result<StepResult, QuicpulseError> {
        let start = std::time::Instant::now();

        // Load the HAR file
        let har_path = self.render_template_for_step(&har_config.file, &step.name, "HAR file")?;
        let har = load_har(std::path::Path::new(&har_path))?;

        // Get the entry to replay
        let entry_index = har_config.entry_index.unwrap_or(0);
        if entry_index >= har.log.entries.len() {
            return Ok(StepResult {
                name: step.name.clone(),
                method: "HAR".to_string(),
                url: har_path,
                status_code: None,
                response_time: start.elapsed(),
                assertions: Vec::new(),
                extracted: HashMap::new(),
                error: Some(format!("HAR entry index {} out of bounds (max {})", entry_index, har.log.entries.len() - 1)),
                skipped: false,
            });
        }

        let entry = &har.log.entries[entry_index];
        let request = &entry.request;

        // Build the request
        let method: Method = request.method.to_uppercase().parse()
            .unwrap_or(Method::GET);
        let url = request.url.clone();

        let mut headers = HeaderMap::new();
        for header in &request.headers {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::try_from(header.name.as_str()),
                reqwest::header::HeaderValue::from_str(&header.value)
            ) {
                headers.insert(name, val);
            }
        }

        // Get body from postData
        let body = request.post_data.as_ref().and_then(|pd| pd.text.clone());

        // Make the request
        let mut req = self.client.request(method.clone(), &url);
        req = req.headers(headers);
        if let Some(body_str) = &body {
            req = req.body(body_str.clone());
        }

        let response = req.send().await;
        let response_time = start.elapsed();

        match response {
            Ok(resp) => {
                let status_code = resp.status().as_u16();
                let _resp_body = resp.text().await.unwrap_or_default();

                Ok(StepResult {
                    name: step.name.clone(),
                    method: format!("HAR/{}", method),
                    url,
                    status_code: Some(status_code),
                    response_time,
                    assertions: Vec::new(),
                    extracted: HashMap::new(),
                    error: None,
                    skipped: false,
                })
            }
            Err(e) => Ok(StepResult {
                name: step.name.clone(),
                method: format!("HAR/{}", method),
                url,
                status_code: None,
                response_time,
                assertions: Vec::new(),
                extracted: HashMap::new(),
                error: Some(format!("HAR request failed: {}", e)),
                skipped: false,
            }),
        }
    }

    /// Run an OpenAPI-driven step
    async fn run_openapi_step(
        &self,
        step: &WorkflowStep,
        openapi_config: &OpenApiConfig,
        _workflow: &Workflow,
    ) -> Result<StepResult, QuicpulseError> {
        let start = std::time::Instant::now();

        // For OpenAPI steps, we generate a sub-workflow and run the appropriate step
        // This is a simplified implementation - full implementation would parse the spec
        // and build the request from the operation definition

        Ok(StepResult {
            name: step.name.clone(),
            method: "OPENAPI".to_string(),
            url: openapi_config.spec.clone(),
            status_code: None,
            response_time: start.elapsed(),
            assertions: Vec::new(),
            extracted: HashMap::new(),
            error: Some("OpenAPI step execution requires running the openapi import command first".to_string()),
            skipped: false,
        })
    }

    /// Apply upload configuration to request
    fn apply_upload_config(
        &self,
        upload_config: &UploadConfig,
        step_name: &str,
    ) -> Result<(Vec<u8>, Option<String>), QuicpulseError> {
        let file_path = self.render_template_for_step(&upload_config.file, step_name, "upload file")?;

        // Read file contents
        let mut data = std::fs::read(&file_path)
            .map_err(|e| QuicpulseError::Io(e))?;

        // Apply compression if configured
        if let Some(ref compress_type) = upload_config.compress {
            data = match compress_type.to_lowercase().as_str() {
                "deflate" => compress_deflate(&data),
                "gzip" => {
                    use std::io::Write;
                    use flate2::write::GzEncoder;
                    use flate2::Compression;
                    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(&data).unwrap_or_default();
                    encoder.finish().unwrap_or(data.clone())
                }
                "br" | "brotli" => {
                    // Brotli compression would require the brotli crate
                    data // Fall back to uncompressed
                }
                _ => data,
            };
        }

        // Determine content type
        let content_type = upload_config.content_type.clone()
            .or_else(|| mime_guess::from_path(&file_path).first().map(|m| m.to_string()));

        Ok((data, content_type))
    }

    /// Filter response headers based on configuration
    fn filter_headers(&self, headers: &HeaderMap, filter: &FilterConfig) -> HeaderMap {
        let mut filtered = HeaderMap::new();

        for (key, value) in headers.iter() {
            let key_str = key.as_str();
            let mut include = true;

            // Check exclude patterns
            if let Some(ref excludes) = filter.exclude_headers {
                for pattern in excludes {
                    if pattern.ends_with('*') {
                        let prefix = &pattern[..pattern.len()-1];
                        if key_str.starts_with(prefix) {
                            include = false;
                            break;
                        }
                    } else if key_str == pattern {
                        include = false;
                        break;
                    }
                }
            }

            // Check include patterns (if specified, only include matching)
            if include {
                if let Some(ref includes) = filter.include_headers {
                    include = includes.iter().any(|pattern| {
                        if pattern.ends_with('*') {
                            let prefix = &pattern[..pattern.len()-1];
                            key_str.starts_with(prefix)
                        } else {
                            key_str == pattern
                        }
                    });
                }
            }

            if include {
                filtered.insert(key.clone(), value.clone());
            }
        }

        filtered
    }

    /// Run plugins for a step
    fn run_plugins(
        &self,
        _plugins: &[PluginConfig],
        _step: &WorkflowStep,
    ) -> Result<(), QuicpulseError> {
        // Plugin execution is currently a stub
        // Full implementation would load and execute plugins from the registry
        if self.options.verbose {
            eprintln!("  {}: Plugin execution in workflows is experimental",
                terminal::warning("Note"));
        }
        Ok(())
    }

    /// Save response data to disk
    fn save_response_data(&self, result: &StepResult) -> Result<(), QuicpulseError> {
        if let Some(ref dir) = self.options.save_responses {
            use std::fs;

            // Ensure directory exists
            fs::create_dir_all(dir)
                .map_err(|e| QuicpulseError::Io(e))?;

            // Build filename: {step_name}_{status}_{timestamp}.json
            let status = result.status_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "error".to_string());

            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S%.3f");

            // Sanitize step name for filename
            let safe_name: String = result.name.chars()
                .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                .collect();

            let filename = format!("{}_{}_{}Z.json", safe_name, status, timestamp);
            let path = dir.join(&filename);

            // Build response data object
            let response_data = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "step_name": result.name,
                "method": result.method,
                "url": result.url,
                "status_code": result.status_code,
                "duration_ms": result.response_time.as_millis(),
                "passed": result.passed(),
                "skipped": result.skipped,
                "error": result.error,
                "assertions": result.assertions.iter().map(|a| {
                    serde_json::json!({
                        "assertion": a.assertion,
                        "passed": a.passed,
                        "message": a.message
                    })
                }).collect::<Vec<_>>(),
                "extracted": result.extracted,
            });

            let json = serde_json::to_string_pretty(&response_data)
                .map_err(|e| QuicpulseError::Parse(format!("JSON serialization error: {}", e)))?;

            fs::write(&path, json)
                .map_err(|e| QuicpulseError::Io(e))?;

            if self.options.verbose {
                eprintln!("  {} {}",
                    terminal::success("Response saved to:"),
                    terminal::colorize(&path.display().to_string(), colors::AQUA));
            }
        }
        Ok(())
    }

    /// Print enhanced dry-run execution plan
    fn print_dry_run_plan(&self, workflow: &Workflow, ordered_steps: &[&WorkflowStep]) {
        use std::collections::HashSet;

        let header_line = terminal::colorize("═══════════════════════════════════════════════════════════════════", colors::GREY);
        let section_line = terminal::colorize("───────────────────────────────────────────────────────────────────", colors::GREY);

        eprintln!("\n{}", header_line);
        eprintln!("{}                      DRY RUN EXECUTION PLAN{}", terminal::bold_fg(colors::WHITE), RESET);
        eprintln!("{}\n", header_line);
        eprintln!("{} {}", terminal::label("Workflow:"), terminal::bold(&workflow.name, colors::WHITE));
        if !workflow.description.is_empty() {
            eprintln!("  {}\n", terminal::muted(&workflow.description));
        }

        // Show initial variables
        eprintln!("{}:", terminal::label("Initial Variables"));
        if self.variables.is_empty() {
            eprintln!("  {}", terminal::muted("(none)"));
        } else {
            for (key, value) in &self.variables {
                let val_str = match value {
                    serde_json::Value::String(s) => {
                        if s.len() > 40 {
                            format!("\"{}...\"", &s[..40])
                        } else {
                            format!("\"{}\"", s)
                        }
                    }
                    _ => value.to_string(),
                };
                eprintln!("  {} = {}", terminal::key(key), terminal::value(&val_str));
            }
        }
        eprintln!();

        // Track which variables will be available at each step
        let mut available_vars: HashSet<String> = self.variables.keys().cloned().collect();

        eprintln!("{} ({} steps):", terminal::label("Execution Order"), terminal::number(&ordered_steps.len().to_string()));
        eprintln!("{}", section_line);

        for (i, step) in ordered_steps.iter().enumerate() {
            // Step header
            let url = self.render_template_dry_run(&step.url);
            let full_url = if let Some(ref base) = workflow.base_url {
                if url.starts_with("http://") || url.starts_with("https://") {
                    url.clone()
                } else {
                    let base_rendered = self.render_template_dry_run(base);
                    format!("{}{}", base_rendered, url)
                }
            } else {
                url.clone()
            };

            eprintln!("\n{}{}{} {}",
                terminal::muted("["),
                terminal::number(&(i + 1).to_string()),
                terminal::muted("]"),
                terminal::bold(&step.name, colors::WHITE));

            // Show tags
            if !step.tags.is_empty() {
                eprintln!("    {} {}",
                    terminal::muted("Tags:"),
                    terminal::colorize(&step.tags.join(", "), colors::PURPLE));
            }

            // Show dependencies
            if !step.depends_on.is_empty() {
                eprintln!("    {} {}",
                    terminal::muted("Depends on:"),
                    terminal::label(&step.depends_on.join(", ")));
            }

            // Show request
            let method_upper = step.method.to_uppercase();
            eprintln!("    {}{}{} {}",
                terminal::protocol::http_method(&method_upper),
                method_upper,
                RESET,
                terminal::colorize(&full_url, colors::AQUA));

            // Find undefined variables in URL and body
            let mut undefined_vars = Vec::new();
            for caps in TEMPLATE_VAR_RE.captures_iter(&step.url) {
                let var_name = &caps[1];
                if !available_vars.contains(var_name) {
                    undefined_vars.push(var_name.to_string());
                }
            }
            if let Some(ref body) = step.body {
                let body_str = serde_json::to_string(body).unwrap_or_default();
                for caps in TEMPLATE_VAR_RE.captures_iter(&body_str) {
                    let var_name = &caps[1];
                    if !available_vars.contains(var_name) && !undefined_vars.contains(&var_name.to_string()) {
                        undefined_vars.push(var_name.to_string());
                    }
                }
            }

            // Show warnings for undefined variables
            if !undefined_vars.is_empty() {
                eprintln!("    {} {} {}",
                    terminal::colorize("⚠", colors::ORANGE),
                    terminal::warning("Undefined variables:"),
                    terminal::colorize(&undefined_vars.join(", "), colors::ORANGE));
            }

            // Show what this step extracts
            if !step.extract.is_empty() {
                let extract_names: Vec<&String> = step.extract.keys().collect();
                eprintln!("    {} {} {}",
                    terminal::colorize("→", colors::GREEN),
                    terminal::muted("Extracts:"),
                    terminal::key(&extract_names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));

                // Add to available vars
                for key in step.extract.keys() {
                    available_vars.insert(key.clone());
                }
            }

            // Show assertions
            if step.assert.status.is_some() || step.assert.latency.is_some() ||
               !step.assert.headers.is_empty() || !step.assert.body.is_empty() {
                let mut assertion_parts = Vec::new();
                if let Some(ref status) = step.assert.status {
                    assertion_parts.push(format!("status={:?}", status));
                }
                if let Some(ref latency) = step.assert.latency {
                    assertion_parts.push(format!("latency<{}", latency));
                }
                if !step.assert.headers.is_empty() {
                    assertion_parts.push(format!("{} headers", step.assert.headers.len()));
                }
                if !step.assert.body.is_empty() {
                    assertion_parts.push(format!("{} body checks", step.assert.body.len()));
                }
                eprintln!("    {} {} {}",
                    terminal::colorize("✓", colors::GREEN),
                    terminal::muted("Asserts:"),
                    terminal::info(&assertion_parts.join(", ")));
            }

            // Show skip condition
            if let Some(ref skip_if) = step.skip_if {
                eprintln!("    {} {}",
                    terminal::muted("Skip if:"),
                    terminal::colorize(skip_if, colors::YELLOW));
            }
        }

        eprintln!("\n{}", section_line);
        eprintln!("{}\n", terminal::muted("End of dry run. No requests were sent."));
    }
}

/// Compress data using deflate algorithm
fn compress_deflate(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    use flate2::write::DeflateEncoder;
    use flate2::Compression;

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap_or_default();
    encoder.finish().unwrap_or_else(|_| data.to_vec())
}

/// Format workflow results for output
pub fn format_workflow_results(results: &[StepResult]) -> String {
    let mut output = String::new();
    output.push_str("\n═══════════════════════════════════════════════════════════════════\n");
    output.push_str("                        WORKFLOW RESULTS\n");
    output.push_str("═══════════════════════════════════════════════════════════════════\n\n");

    let total = results.len();
    let passed = results.iter().filter(|r| r.passed()).count();
    let skipped = results.iter().filter(|r| r.skipped).count();
    let failed = total - passed - skipped;

    for (i, result) in results.iter().enumerate() {
        let status_icon = if result.skipped {
            "⊘"
        } else if result.passed() {
            "✓"
        } else {
            "✗"
        };

        let status_str = result.status_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "---".to_string());

        output.push_str(&format!(
            "  {} Step {}: {} ({} {})\n",
            status_icon, i + 1, result.name, result.method,
            if result.skipped { "SKIPPED" } else { &status_str }
        ));

        if !result.skipped && !result.url.is_empty() {
            output.push_str(&format!("      URL: {}\n", result.url));
            output.push_str(&format!("      Time: {:?}\n", result.response_time));
        }

        if let Some(ref error) = result.error {
            output.push_str(&format!("      Error: {}\n", error));
        }

        for assertion in &result.assertions {
            let icon = if assertion.passed { "  ✓" } else { "  ✗" };
            output.push_str(&format!("      {} {}: {}\n", icon, assertion.assertion, assertion.message));
        }

        if !result.extracted.is_empty() {
            output.push_str("      Extracted:\n");
            for (key, value) in &result.extracted {
                output.push_str(&format!("        {} = {}\n", key, value));
            }
        }

        output.push('\n');
    }

    output.push_str("───────────────────────────────────────────────────────────────────\n");
    output.push_str(&format!(
        "  Total: {} | Passed: {} | Failed: {} | Skipped: {}\n",
        total, passed, failed, skipped
    ));
    output.push_str("═══════════════════════════════════════════════════════════════════\n");

    output
}

/// Format workflow results as JSON lines (one line per result)
/// Suitable for CI/CD pipelines and log aggregation
pub fn format_workflow_results_json(results: &[StepResult]) -> String {
    let mut output = String::new();

    for result in results {
        let json = serde_json::json!({
            "level": if result.passed() { "info" } else { "error" },
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "event": "step_result",
            "step_name": result.name,
            "method": result.method,
            "url": result.url,
            "status_code": result.status_code,
            "duration_ms": result.response_time.as_millis(),
            "passed": result.passed(),
            "skipped": result.skipped,
            "error": result.error,
            "assertions_passed": result.assertions.iter().filter(|a| a.passed).count(),
            "assertions_failed": result.assertions.iter().filter(|a| !a.passed).count(),
        });
        output.push_str(&serde_json::to_string(&json).unwrap_or_default());
        output.push('\n');
    }

    // Add summary line
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed()).count();
    let skipped = results.iter().filter(|r| r.skipped).count();
    let failed = total - passed - skipped;

    let summary = serde_json::json!({
        "level": "info",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "event": "workflow_summary",
        "total": total,
        "passed": passed,
        "failed": failed,
        "skipped": skipped,
        "success": failed == 0,
    });
    output.push_str(&serde_json::to_string(&summary).unwrap_or_default());
    output.push('\n');

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template() {
        let mut runner = PipelineRunner::new(true).unwrap();
        runner.variables.insert("name".to_string(), JsonValue::String("test".to_string()));
        runner.variables.insert("id".to_string(), JsonValue::Number(42.into()));

        let result = runner.render_template_for_step("Hello {{name}}, ID={{id}}", "test", "field").unwrap();
        assert_eq!(result, "Hello test, ID=42");
    }

    #[test]
    fn test_evaluate_condition() {
        let mut runner = PipelineRunner::new(true).unwrap();
        runner.variables.insert("enabled".to_string(), JsonValue::Bool(true));
        runner.variables.insert("disabled".to_string(), JsonValue::Bool(false));

        assert!(runner.evaluate_condition("{{enabled}}"));
        assert!(!runner.evaluate_condition("{{disabled}}"));
        assert!(!runner.evaluate_condition("!{{enabled}}"));
        assert!(runner.evaluate_condition("!{{disabled}}"));
    }

    fn make_step(name: &str) -> WorkflowStep {
        WorkflowStep {
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn make_step_with_tags(name: &str, tags: Vec<&str>) -> WorkflowStep {
        WorkflowStep {
            name: name.to_string(),
            tags: tags.into_iter().map(String::from).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_filter_by_tags() {
        let options = WorkflowOptions {
            tags: vec!["smoke".to_string()],
            ..Default::default()
        };
        let runner = PipelineRunner::with_options(true, options).unwrap();

        let step_with_tag = make_step_with_tags("test1", vec!["smoke", "api"]);
        let step_without_tag = make_step_with_tags("test2", vec!["integration"]);
        let step_no_tags = make_step("test3");

        assert!(runner.should_run_step(&step_with_tag));
        assert!(!runner.should_run_step(&step_without_tag));
        assert!(!runner.should_run_step(&step_no_tags));
    }

    #[test]
    fn test_filter_by_include() {
        let options = WorkflowOptions {
            include: vec!["login".to_string(), "logout".to_string()],
            ..Default::default()
        };
        let runner = PipelineRunner::with_options(true, options).unwrap();

        let login_step = make_step("login");
        let logout_step = make_step("logout");
        let other_step = make_step("get_users");

        assert!(runner.should_run_step(&login_step));
        assert!(runner.should_run_step(&logout_step));
        assert!(!runner.should_run_step(&other_step));
    }

    #[test]
    fn test_filter_by_exclude() {
        let options = WorkflowOptions {
            exclude: vec!["cleanup".to_string()],
            ..Default::default()
        };
        let runner = PipelineRunner::with_options(true, options).unwrap();

        let normal_step = make_step("test");
        let cleanup_step = make_step("cleanup");

        assert!(runner.should_run_step(&normal_step));
        assert!(!runner.should_run_step(&cleanup_step));
    }

    #[test]
    fn test_filter_by_exclude_regex() {
        let options = WorkflowOptions {
            exclude: vec!["test_.*".to_string()],
            ..Default::default()
        };
        let runner = PipelineRunner::with_options(true, options).unwrap();

        let normal_step = make_step("login");
        let test_step1 = make_step("test_api");
        let test_step2 = make_step("test_auth");

        assert!(runner.should_run_step(&normal_step));
        assert!(!runner.should_run_step(&test_step1));
        assert!(!runner.should_run_step(&test_step2));
    }

    #[test]
    fn test_filter_combined_tags_and_include() {
        let options = WorkflowOptions {
            tags: vec!["smoke".to_string()],
            include: vec!["fast_test".to_string()],
            ..Default::default()
        };
        let runner = PipelineRunner::with_options(true, options).unwrap();

        // Both tag and include must match
        let step_both = WorkflowStep {
            name: "fast_test".to_string(),
            tags: vec!["smoke".to_string()],
            ..Default::default()
        };
        let step_tag_only = make_step_with_tags("other", vec!["smoke"]);
        let step_include_only = make_step("fast_test");

        assert!(runner.should_run_step(&step_both));
        assert!(!runner.should_run_step(&step_tag_only));
        assert!(!runner.should_run_step(&step_include_only));
    }

    #[test]
    fn test_no_filters_runs_all() {
        let runner = PipelineRunner::new(true).unwrap();

        let step1 = make_step("test1");
        let step2 = make_step_with_tags("test2", vec!["any_tag"]);

        assert!(runner.should_run_step(&step1));
        assert!(runner.should_run_step(&step2));
    }
}
