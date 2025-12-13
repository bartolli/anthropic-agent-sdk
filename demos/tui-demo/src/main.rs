//! Claude TUI Demo
//!
//! An interactive terminal UI demo for the Claude Agent SDK.
//!
//! Features:
//! - Styled prompts and output using console
//! - Thinking spinner with random messages
//! - Markdown rendering with syntax highlighting
//! - History and readline-style editing
//! - Context usage visualization
//!
//! Run with: cargo run -p claude-tui-demo

mod config;
mod input;
mod output;
mod thinking;

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message};
use std::path::PathBuf;
use std::time::Instant;

/// Get the history file path
fn history_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claude-tui-demo")
        .join("history.txt")
}

/// Extract a useful detail from tool input for display
fn extract_tool_detail(tool_name: &str, input: &serde_json::Value) -> Option<String> {
    let get_str = |key: &str| -> Option<&str> {
        input.get(key).and_then(serde_json::Value::as_str)
    };

    match tool_name {
        "Read" | "Write" | "Edit" => get_str("file_path").map(shorten_path),
        "Glob" => get_str("pattern").map(String::from),
        "Grep" => get_str("pattern").map(|s| truncate_str(s, 30)),
        "Bash" => get_str("command").map(|s| truncate_str(s, 40)),
        "Task" => get_str("description").map(String::from),
        "WebFetch" => get_str("url").map(|s| truncate_str(s, 50)),
        _ => None,
    }
}

/// Truncate string with ellipsis if too long
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

/// Shorten a file path for display (e.g., /very/long/path/file.rs -> .../path/file.rs)
fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 3 {
        path.to_string()
    } else {
        // Keep last 2-3 components
        format!(".../{}", parts[parts.len()-2..].join("/"))
    }
}

/// Ensure history directory exists
fn ensure_history_dir() -> anyhow::Result<()> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Handle slash commands
async fn handle_command(cmd: &str, _client: &mut ClaudeSDKClient) -> anyhow::Result<bool> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let command = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();

    match command.as_str() {
        "/help" => {
            output::display_help();
            Ok(false)
        }
        "/clear" => {
            output::clear_screen();
            Ok(false)
        }
        "/test-spinner" => {
            // Test spinner for 3 seconds
            println!("Testing spinner for 3 seconds...");
            output::show_thinking();
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            output::hide_thinking();
            println!("Spinner test complete.");
            Ok(false)
        }
        "/quit" | "/exit" => Ok(true),
        _ => {
            output::display_warning(&format!("Unknown command: {}", cmd));
            output::display_help();
            Ok(false)
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    // Default: show info for demo, warn for deps (filters rustyline keystroke noise)
    // Override with RUST_LOG env var for more detail
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "claude_tui_demo=info,rustyline=info,warn".parse().unwrap()
    });
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Display greeting
    output::display_greeting();

    // Create client with reasonable limits
    let options = ClaudeAgentOptions::builder().max_turns(50).build();

    let mut client = match ClaudeSDKClient::new(options, None).await {
        Ok(client) => client,
        Err(e) => {
            output::display_error(&format!("Failed to connect to Claude: {}", e));
            return Err(e.into());
        }
    };

    // Display session info
    output::display_session_info(None);

    // Create input editor with history
    let mut editor = input::create_editor()?;
    let _ = ensure_history_dir();
    let _ = input::load_history(&mut editor, &history_path());

    // Track token usage (simplified - real implementation would track from responses)
    let mut total_tokens: usize = 0;
    let context_limit = config::DEFAULT_CONTEXT_LIMIT;

    // Main loop
    loop {
        // Display context usage before prompt
        if total_tokens > 0 {
            output::display_context_usage(total_tokens, context_limit);
        }

        // Get user input
        match input::get_input(&mut editor)? {
            input::InputResult::Message(msg) => {
                if msg.is_empty() {
                    continue;
                }

                // Send message to Claude
                if let Err(e) = client.send_message(&msg).await {
                    output::display_error(&format!("Failed to send message: {}", e));
                    continue;
                }

                // Show thinking spinner
                output::show_thinking();
                let start_time = Instant::now();

                // Read and display response
                let mut first_text = true;   // Track first text output for claude> prefix
                let mut after_tools = false; // Track tool→text transition for spacing
                let mut had_text = false;    // Track text→tool transition for newline

                while let Some(message) = client.next_message().await {
                    match message {
                        Ok(msg) => match &msg {
                            Message::Assistant { message, .. } => {
                                // Process each content block
                                for block in &message.content {
                                    match block {
                                        ContentBlock::Text { text } => {
                                            // Hide spinner before text output
                                            output::hide_thinking();

                                            // Show claude> prefix for first text
                                            if first_text {
                                                output::display_claude_prefix();
                                                first_text = false;
                                            } else if after_tools {
                                                // Add blank line after tools before response
                                                println!();
                                            }
                                            after_tools = false;
                                            had_text = true;

                                            // Render markdown with syntax highlighting
                                            output::print_markdown(text);
                                        }
                                        ContentBlock::ToolUse { name, input, .. } => {
                                            // Hide current spinner, show tool header
                                            output::hide_thinking();

                                            // Newline after text before first tool
                                            if had_text {
                                                println!();
                                                had_text = false;
                                            }

                                            // Extract a useful detail from input (e.g., file path)
                                            let detail = extract_tool_detail(name, input);
                                            output::display_tool_header(name, detail.as_deref());
                                            after_tools = true;

                                            // Re-show spinner while tool executes
                                            output::show_thinking();
                                        }
                                        ContentBlock::Thinking { thinking, .. } => {
                                            // Could display thinking if verbose mode
                                            tracing::debug!("Thinking: {}...", &thinking[..thinking.len().min(50)]);
                                        }
                                        _ => {
                                            // Other block types
                                        }
                                    }
                                }
                            }
                            Message::Result {
                                total_cost_usd,
                                ..
                            } => {
                                // Ensure spinner is hidden before result
                                output::hide_thinking();

                                println!(); // Newline after response

                                // Update token estimate (rough approximation)
                                total_tokens += 1000; // Placeholder

                                // Display cost if enabled
                                output::display_cost(*total_cost_usd);

                                // Display elapsed time
                                output::display_elapsed(start_time.elapsed());

                                println!(); // Extra spacing before next prompt
                                break;
                            }
                            Message::System { subtype, .. } => {
                                // System messages - keep spinner running
                                tracing::debug!("System message ({})", subtype);
                            }
                            Message::User { .. } => {
                                // User messages may contain tool results
                                // We don't need to process them - just continue
                                tracing::debug!("User message (may contain tool result)");
                            }
                            _ => {
                                // Other message types (StreamEvent, etc.)
                            }
                        },
                        Err(e) => {
                            output::hide_thinking();
                            output::display_error(&format!("Error: {}", e));
                            break;
                        }
                    }
                }
            }

            input::InputResult::Command(cmd) => {
                if handle_command(&cmd, &mut client).await? {
                    break; // Exit command
                }
            }

            input::InputResult::Exit => {
                break;
            }

            input::InputResult::Interrupt => {
                // Ctrl+C - just show a new prompt
                println!();
                continue;
            }
        }
    }

    // Save history
    let _ = input::save_history(&mut editor, &history_path());

    // Cleanup
    client.close().await?;
    output::display_goodbye();

    Ok(())
}
