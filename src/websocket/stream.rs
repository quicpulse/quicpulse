//! WebSocket stream/listen mode implementation

use std::io::BufRead;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::errors::QuicpulseError;
use crate::status::ExitStatus;
use crate::output::terminal::{self, colors, RESET};
use super::client::WsClient;
use super::codec::{format_text_message, format_binary_message};
use super::types::{WsMessage, WsOptions};

/// Print a received WebSocket message with colors
pub fn print_message(msg: &WsMessage, options: &WsOptions) {
    match msg {
        WsMessage::Text(text) => {
            let formatted = format_text_message(text);
            // Try to colorize JSON if it looks like JSON
            if text.trim().starts_with('{') || text.trim().starts_with('[') {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                    let pretty = serde_json::to_string_pretty(&json).unwrap_or_else(|_| formatted.clone());
                    let formatter = crate::output::formatters::ColorFormatter::new(
                        crate::output::formatters::ColorStyle::Auto
                    );
                    println!("{}", formatter.format_json(&pretty));
                    return;
                }
            }
            println!("{}", formatted);
        }
        WsMessage::Binary(data) => {
            let formatted = format_binary_message(data, options.binary_mode);
            eprintln!("{} {}", terminal::protocol::ws_label("binary"), formatted);
        }
        WsMessage::Ping(data) => {
            if !data.is_empty() {
                eprintln!("{} {:?}", terminal::protocol::ws_label("ping"), String::from_utf8_lossy(data));
            } else {
                eprintln!("{}", terminal::protocol::ws_label("ping"));
            }
        }
        WsMessage::Pong(data) => {
            if !data.is_empty() {
                eprintln!("{} {:?}", terminal::protocol::ws_label("pong"), String::from_utf8_lossy(data));
            } else {
                eprintln!("{}", terminal::protocol::ws_label("pong"));
            }
        }
        WsMessage::Close(code, reason) => {
            let label = terminal::protocol::ws_label("close");
            if let Some(code) = code {
                let code_color = if *code >= 1000 && *code < 2000 {
                    terminal::colorize(&code.to_string(), colors::GREEN)
                } else {
                    terminal::colorize(&code.to_string(), colors::ORANGE)
                };
                if reason.is_empty() {
                    eprintln!("{} code={}", label, code_color);
                } else {
                    eprintln!("{} code={} reason={}{}{}", label, code_color,
                        terminal::fg(colors::WHITE), reason, RESET);
                }
            } else if !reason.is_empty() {
                eprintln!("{} {}", label, reason);
            } else {
                eprintln!("{}", label);
            }
        }
    }
}

/// Run listen mode - receive messages until connection closes
pub async fn run_listen_mode(
    client: &mut WsClient,
    options: &WsOptions,
) -> Result<ExitStatus, QuicpulseError> {
    let mut count = 0;
    let max = options.max_messages;

    // Set up ping interval if specified
    let mut ping_interval = options.ping_interval.map(tokio::time::interval);

    loop {
        tokio::select! {
            // Receive messages
            msg = client.stream_mut().next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        print_message(&WsMessage::Text(text.to_string()), options);
                        count += 1;
                        if max > 0 && count >= max {
                            break;
                        }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        print_message(&WsMessage::Binary(data.to_vec()), options);
                        count += 1;
                        if max > 0 && count >= max {
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // Auto-respond with pong
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
                        break;
                    }
                    Some(Ok(Message::Frame(_))) => {}
                    Some(Err(e)) => {
                        eprintln!("{}: {}", terminal::error("Error"), e);
                        return Ok(ExitStatus::Error);
                    }
                    None => break,
                }
            }

            // Send periodic pings if configured
            _ = async {
                if let Some(ref mut interval) = ping_interval {
                    interval.tick().await
                } else {
                    std::future::pending().await
                }
            } => {
                if let Err(e) = client.send_ping(b"").await {
                    eprintln!("{}: {}", terminal::warning("Ping error"), e);
                }
            }
        }
    }

    Ok(ExitStatus::Success)
}

/// Run stdin mode - read NDJSON from stdin and send as messages
pub async fn run_stdin_mode(
    client: &mut WsClient,
    options: &WsOptions,
) -> Result<ExitStatus, QuicpulseError> {
    use futures::SinkExt;

    // Spawn a task to read from stdin
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);

    tokio::task::spawn_blocking(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            if let Ok(line) = line {
                let line = line.trim();
                if !line.is_empty() {
                    if tx.blocking_send(line.to_string()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut count = 0;
    let max = options.max_messages;

    loop {
        tokio::select! {
            // Send messages from stdin
            Some(line) = rx.recv() => {
                client.send_text(&line).await?;
            }

            // Receive messages
            msg = client.stream_mut().next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        print_message(&WsMessage::Text(text.to_string()), options);
                        count += 1;
                        if max > 0 && count >= max {
                            break;
                        }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        print_message(&WsMessage::Binary(data.to_vec()), options);
                        count += 1;
                        if max > 0 && count >= max {
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
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
                        break;
                    }
                    Some(Ok(Message::Frame(_))) => {}
                    Some(Err(e)) => {
                        eprintln!("{}: {}", terminal::error("Error"), e);
                        return Ok(ExitStatus::Error);
                    }
                    None => break,
                }
            }
        }
    }

    client.close().await?;
    Ok(ExitStatus::Success)
}
