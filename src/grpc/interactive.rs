//! gRPC Interactive REPL
//!
//! Provides an interactive shell for exploring gRPC services and making calls.

use crate::errors::QuicpulseError;
use crate::grpc::client::GrpcClient;
use crate::grpc::reflection;
use std::io::{self, BufRead, Write};

/// Commands available in the REPL
#[derive(Debug)]
enum Command {
    List,
    Describe(String),
    Call { service: String, method: String, payload: String },
    Use(String),
    Help,
    Quit,
    History,
    Clear,
    Status,
    Unknown(String),
}

/// Interactive gRPC REPL state
pub struct GrpcRepl {
    client: GrpcClient,
    current_service: Option<String>,
    history: Vec<String>,
    verbose: bool,
}

impl GrpcRepl {
    /// Create a new gRPC REPL
    pub fn new(client: GrpcClient, verbose: bool) -> Self {
        Self {
            client,
            current_service: None,
            history: Vec::new(),
            verbose,
        }
    }

    /// Run the interactive REPL
    pub async fn run(&mut self) -> Result<(), QuicpulseError> {
        self.print_welcome();

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            // Print prompt
            let prompt = if let Some(ref svc) = self.current_service {
                format!("\x1b[36m{}\x1b[0m> ", svc)
            } else {
                "\x1b[33mgrpc\x1b[0m> ".to_string()
            };
            print!("{}", prompt);
            stdout.flush().map_err(QuicpulseError::Io)?;

            // Read input
            let mut input = String::new();
            match stdin.lock().read_line(&mut input) {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error reading input: {}", e);
                    continue;
                }
            }

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            // Add to history
            self.history.push(input.to_string());

            // Parse and execute command
            let cmd = self.parse_command(input);
            match self.execute_command(cmd).await {
                Ok(true) => {} // Continue
                Ok(false) => break, // Quit
                Err(e) => {
                    eprintln!("\x1b[31mError: {}\x1b[0m", e);
                }
            }
        }

        println!("\nGoodbye!");
        Ok(())
    }

    fn print_welcome(&self) {
        println!("\x1b[1m");
        println!("╔═══════════════════════════════════════════════════════════════╗");
        println!("║           QuicPulse gRPC Interactive REPL                     ║");
        println!("╠═══════════════════════════════════════════════════════════════╣");
        println!("║  Commands:                                                    ║");
        println!("║    list         - List available services                     ║");
        println!("║    describe <s> - Describe a service                          ║");
        println!("║    use <s>      - Set default service                         ║");
        println!("║    call <m> {{}}  - Call a method with JSON payload            ║");
        println!("║    help         - Show this help                              ║");
        println!("║    quit         - Exit the REPL                               ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");
        println!("\x1b[0m");
    }

    fn parse_command(&self, input: &str) -> Command {
        let parts: Vec<&str> = input.splitn(2, char::is_whitespace).collect();
        let cmd = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();
        let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match cmd.as_str() {
            "list" | "ls" | "services" => Command::List,
            "describe" | "desc" | "d" => {
                if args.is_empty() {
                    if let Some(ref svc) = self.current_service {
                        Command::Describe(svc.clone())
                    } else {
                        Command::Unknown("describe requires a service name".to_string())
                    }
                } else {
                    Command::Describe(args.to_string())
                }
            }
            "use" | "service" => Command::Use(args.to_string()),
            "call" | "c" | "invoke" => {
                // Parse: call MethodName {"json": "payload"}
                // Or with current service: call Service.Method {"json"}
                if let Some(idx) = args.find('{') {
                    let method_part = args[..idx].trim();
                    let payload = args[idx..].trim();

                    // Determine service and method
                    if let Some(dot_idx) = method_part.rfind('.') {
                        let service = method_part[..dot_idx].to_string();
                        let method = method_part[dot_idx + 1..].to_string();
                        Command::Call { service, method, payload: payload.to_string() }
                    } else if let Some(ref svc) = self.current_service {
                        Command::Call {
                            service: svc.clone(),
                            method: method_part.to_string(),
                            payload: payload.to_string(),
                        }
                    } else {
                        Command::Unknown("No service selected. Use 'use <service>' first".to_string())
                    }
                } else {
                    Command::Unknown("call requires JSON payload: call Method {\"key\": \"value\"}".to_string())
                }
            }
            "help" | "h" | "?" => Command::Help,
            "quit" | "exit" | "q" => Command::Quit,
            "history" | "hist" => Command::History,
            "clear" | "cls" => Command::Clear,
            "status" => Command::Status,
            "" => Command::Unknown("".to_string()),
            _ => Command::Unknown(format!("Unknown command: {}", cmd)),
        }
    }

    async fn execute_command(&mut self, cmd: Command) -> Result<bool, QuicpulseError> {
        match cmd {
            Command::List => {
                self.list_services().await?;
            }
            Command::Describe(service) => {
                self.describe_service(&service).await?;
            }
            Command::Use(service) => {
                self.current_service = Some(service.clone());
                println!("Now using service: \x1b[36m{}\x1b[0m", service);
            }
            Command::Call { service, method, payload } => {
                self.call_method(&service, &method, &payload).await?;
            }
            Command::Help => {
                self.print_help();
            }
            Command::Quit => {
                return Ok(false);
            }
            Command::History => {
                println!("Command history:");
                for (i, cmd) in self.history.iter().enumerate() {
                    println!("  {}: {}", i + 1, cmd);
                }
            }
            Command::Clear => {
                print!("\x1b[2J\x1b[1;1H");
            }
            Command::Status => {
                println!("Connection: \x1b[32mConnected\x1b[0m");
                if let Some(ref svc) = self.current_service {
                    println!("Current service: \x1b[36m{}\x1b[0m", svc);
                } else {
                    println!("Current service: \x1b[33mnone\x1b[0m");
                }
                if let Some(schema) = self.client.schema() {
                    println!("Proto loaded: \x1b[32myes\x1b[0m ({})", schema.package);
                } else {
                    println!("Proto loaded: \x1b[33mno\x1b[0m (using reflection)");
                }
            }
            Command::Unknown(msg) => {
                if !msg.is_empty() {
                    eprintln!("\x1b[33m{}\x1b[0m", msg);
                }
            }
        }
        Ok(true)
    }

    async fn list_services(&self) -> Result<(), QuicpulseError> {
        println!("Discovering services...");

        // Try proto schema first
        if let Some(schema) = self.client.schema() {
            println!("\n\x1b[1mServices from proto file:\x1b[0m");
            for service in &schema.services {
                println!("  \x1b[36m{}\x1b[0m", service.full_name);
                for method in &service.methods {
                    let stream_marker = match (method.client_streaming, method.server_streaming) {
                        (true, true) => " ⇄",
                        (true, false) => " →",
                        (false, true) => " ←",
                        (false, false) => "",
                    };
                    println!("    • {}{}", method.name, stream_marker);
                }
            }
            return Ok(());
        }

        // Fall back to reflection
        match reflection::list_services(self.client.channel()).await {
            Ok(services) => {
                println!("\n\x1b[1mAvailable services (via reflection):\x1b[0m");
                for service in services {
                    println!("  \x1b[36m{}\x1b[0m", service);
                }
            }
            Err(e) => {
                eprintln!("\x1b[31mReflection failed: {}\x1b[0m", e);
                eprintln!("Try loading a proto file with --proto");
            }
        }

        Ok(())
    }

    async fn describe_service(&self, service: &str) -> Result<(), QuicpulseError> {
        // Try proto schema first
        if let Some(schema) = self.client.schema() {
            for svc in &schema.services {
                if svc.name == service || svc.full_name == service {
                    println!("\n\x1b[1mservice {} {{\x1b[0m", svc.name);
                    for method in &svc.methods {
                        let client_stream = if method.client_streaming { "stream " } else { "" };
                        let server_stream = if method.server_streaming { "stream " } else { "" };
                        println!(
                            "  \x1b[32mrpc\x1b[0m \x1b[36m{}\x1b[0m(\x1b[33m{}{}\x1b[0m) returns (\x1b[33m{}{}\x1b[0m);",
                            method.name,
                            client_stream, method.input_type,
                            server_stream, method.output_type
                        );
                    }
                    println!("\x1b[1m}}\x1b[0m");
                    return Ok(());
                }
            }
        }

        // Fall back to reflection
        match reflection::describe_service(self.client.channel(), service).await {
            Ok(desc) => {
                println!("\n{}", desc.format_display());
            }
            Err(e) => {
                eprintln!("\x1b[31mFailed to describe service: {}\x1b[0m", e);
            }
        }

        Ok(())
    }

    async fn call_method(&mut self, service: &str, method: &str, payload: &str) -> Result<(), QuicpulseError> {
        // Parse JSON payload
        let json: serde_json::Value = serde_json::from_str(payload)
            .map_err(|e| QuicpulseError::Argument(format!("Invalid JSON: {}", e)))?;

        if self.verbose {
            eprintln!("\x1b[90mCalling {}/{} with:\x1b[0m", service, method);
            eprintln!("\x1b[90m{}\x1b[0m", serde_json::to_string_pretty(&json).unwrap_or_default());
        }

        println!("Calling \x1b[36m{}.{}\x1b[0m...", service, method);

        let response = self.client.call_unary(service, method, &json).await?;

        if response.is_ok() {
            println!("\x1b[32mStatus: {:?}\x1b[0m", response.code());
            if let Ok(resp_json) = response.json() {
                let pretty = serde_json::to_string_pretty(&resp_json).unwrap_or_default();
                println!("{}", pretty);
            }
        } else {
            println!("\x1b[31mError: {:?} - {}\x1b[0m", response.code(), response.message());
        }

        Ok(())
    }

    fn print_help(&self) {
        println!("\n\x1b[1mAvailable commands:\x1b[0m");
        println!("  \x1b[36mlist\x1b[0m, \x1b[36mls\x1b[0m            List all available services");
        println!("  \x1b[36mdescribe\x1b[0m <service>  Show service methods and types");
        println!("  \x1b[36muse\x1b[0m <service>       Set default service for calls");
        println!("  \x1b[36mcall\x1b[0m <method> {{}}    Call a method with JSON payload");
        println!("  \x1b[36mstatus\x1b[0m              Show connection status");
        println!("  \x1b[36mhistory\x1b[0m             Show command history");
        println!("  \x1b[36mclear\x1b[0m               Clear the screen");
        println!("  \x1b[36mhelp\x1b[0m, \x1b[36m?\x1b[0m             Show this help");
        println!("  \x1b[36mquit\x1b[0m, \x1b[36mexit\x1b[0m         Exit the REPL");
        println!();
        println!("\x1b[1mExamples:\x1b[0m");
        println!("  list");
        println!("  use mypackage.MyService");
        println!("  describe");
        println!("  call GetUser {{\"id\": 123}}");
        println!("  call mypackage.OtherService.Method {{\"name\": \"test\"}}");
        println!();
    }
}

/// Run the gRPC interactive REPL
pub async fn run_interactive(client: GrpcClient, verbose: bool) -> Result<(), QuicpulseError> {
    let mut repl = GrpcRepl::new(client, verbose);
    repl.run().await
}
