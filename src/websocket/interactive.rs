//! WebSocket interactive REPL mode

use std::io::{self, Write};
use futures::StreamExt;
use tokio_tungstenite::tungstenite::protocol::Message;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};

use crate::context::Environment;
use crate::errors::QuicpulseError;
use crate::status::ExitStatus;
use super::client::WsClient;
use super::codec::{decode_binary, format_text_message, format_binary_message};
use super::stream::print_message;
use super::types::{BinaryMode, WsMessage, WsOptions};

const PROMPT: &str = "ws> ";

/// Run interactive WebSocket REPL mode
pub async fn run_interactive_mode(
    client: &mut WsClient,
    options: &WsOptions,
    env: &Environment,
) -> Result<ExitStatus, QuicpulseError> {
    // Check if we can do interactive mode
    if !env.stdin_isatty || !env.stdout_isatty {
        eprintln!("Interactive mode requires a TTY. Use --ws-listen or --ws-send instead.");
        return Ok(ExitStatus::Error);
    }

    eprintln!("WebSocket connected. Type messages to send, or use commands:");
    eprintln!("  /quit, /q    - Close connection and exit");
    eprintln!("  /ping [msg]  - Send a ping frame");
    eprintln!("  /binary <hex|base64> <data> - Send binary message");
    eprintln!("  /close [code] [reason] - Send close frame");
    eprintln!("  Ctrl+C       - Exit");
    eprintln!();

    // Use line-based input for simpler implementation
    // (Raw mode with crossterm is complex with async)
    run_line_based_repl(client, options).await
}

/// Line-based REPL implementation
async fn run_line_based_repl(
    client: &mut WsClient,
    options: &WsOptions,
) -> Result<ExitStatus, QuicpulseError> {
    use futures::SinkExt;

    // Channel for user input
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);

    // Spawn stdin reader thread
    std::thread::spawn(move || {
        let stdin = io::stdin();
        loop {
            print!("{}", PROMPT);
            let _ = io::stdout().flush();

            let mut line = String::new();
            match stdin.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if tx.blocking_send(line.trim().to_string()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut ping_interval = options.ping_interval.map(tokio::time::interval);

    loop {
        tokio::select! {
            // Handle user input
            Some(line) = rx.recv() => {
                if line.is_empty() {
                    continue;
                }

                // Handle commands
                if line.starts_with('/') {
                    match handle_command(&line, client, options).await {
                        Ok(true) => break, // Exit requested
                        Ok(false) => continue,
                        Err(e) => eprintln!("Command error: {}", e),
                    }
                    continue;
                }

                // Send as text message
                if let Err(e) = client.send_text(&line).await {
                    eprintln!("Send error: {}", e);
                }
            }

            // Handle incoming messages
            msg = client.stream_mut().next() => {
                // Clear the current prompt line
                print!("\r\x1b[K");

                match msg {
                    Some(Ok(Message::Text(text))) => {
                        print_message(&WsMessage::Text(text.to_string()), options);
                    }
                    Some(Ok(Message::Binary(data))) => {
                        print_message(&WsMessage::Binary(data.to_vec()), options);
                    }
                    Some(Ok(Message::Ping(data))) => {
                        eprintln!("[ping received]");
                        let _ = client.stream_mut().send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(data))) => {
                        print_message(&WsMessage::Pong(data.to_vec()), options);
                    }
                    Some(Ok(Message::Close(frame))) => {
                        let (code, reason) = frame
                            .map(|f| (Some(f.code.into()), f.reason.to_string()))
                            .unwrap_or((None, String::new()));
                        print_message(&WsMessage::Close(code, reason), options);
                        eprintln!("Connection closed by server");
                        break;
                    }
                    Some(Ok(Message::Frame(_))) => {}
                    Some(Err(e)) => {
                        eprintln!("Receive error: {}", e);
                        break;
                    }
                    None => {
                        eprintln!("Connection closed");
                        break;
                    }
                }

                // Reprint prompt
                print!("{}", PROMPT);
                let _ = io::stdout().flush();
            }

            // Send periodic pings
            _ = async {
                if let Some(ref mut interval) = ping_interval {
                    interval.tick().await
                } else {
                    std::future::pending().await
                }
            } => {
                if let Err(e) = client.send_ping(b"keepalive").await {
                    eprintln!("Ping error: {}", e);
                }
            }
        }
    }

    client.close().await?;
    Ok(ExitStatus::Success)
}

/// Handle a REPL command. Returns Ok(true) if should exit.
async fn handle_command(
    line: &str,
    client: &mut WsClient,
    _options: &WsOptions,
) -> Result<bool, QuicpulseError> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let cmd = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();

    match cmd.as_str() {
        "/quit" | "/q" | "/exit" => {
            eprintln!("Closing connection...");
            Ok(true)
        }

        "/ping" => {
            let data = if parts.len() > 1 {
                parts[1..].join(" ").into_bytes()
            } else {
                vec![]
            };
            client.send_ping(&data).await?;
            eprintln!("[ping sent]");
            Ok(false)
        }

        "/binary" | "/bin" => {
            if parts.len() < 3 {
                eprintln!("Usage: /binary <hex|base64> <data>");
                return Ok(false);
            }

            let mode: BinaryMode = parts[1].parse()
                .map_err(|e: String| QuicpulseError::Argument(e))?;
            let data = parts[2..].join(" ");
            let bytes = decode_binary(&data, mode)?;
            client.send_binary(&bytes).await?;
            eprintln!("[binary sent: {} bytes]", bytes.len());
            Ok(false)
        }

        "/close" => {
            // Parse optional close code and reason
            eprintln!("Sending close frame...");
            Ok(true)
        }

        "/help" | "/?" => {
            eprintln!("Commands:");
            eprintln!("  /quit, /q    - Close connection and exit");
            eprintln!("  /ping [msg]  - Send a ping frame");
            eprintln!("  /binary <hex|base64> <data> - Send binary message");
            eprintln!("  /close       - Send close frame and exit");
            eprintln!("  /help        - Show this help");
            Ok(false)
        }

        _ => {
            eprintln!("Unknown command: {}. Type /help for available commands.", cmd);
            Ok(false)
        }
    }
}
