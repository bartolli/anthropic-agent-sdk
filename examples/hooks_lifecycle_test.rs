//! Test hook lifecycle events with real CLI
//!
//! Demonstrates all hook events firing in sequence:
//! - `SessionStart` (on init)
//! - `UserPromptSubmit` (before sending message)
//! - `PreToolUse` / `PostToolUse` (during tool execution)
//! - Stop (on result)
//! - `SessionEnd` (on close)
//!
//! Run with: `RUST_LOG=debug` cargo run --example `hooks_lifecycle_test`

use anthropic_agent_sdk::ClaudeSDKClient;
use anthropic_agent_sdk::hooks::{HookManager, HookMatcherBuilder};
use anthropic_agent_sdk::types::{
    ClaudeAgentOptions, ContentBlock, HookEvent, HookOutput, Message,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with debug level
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .with_target(true)
        .with_ansi(true)
        .init();

    info!("=== Hook Lifecycle Test ===");

    // Counters for each hook event
    let session_start_count = Arc::new(AtomicUsize::new(0));
    let user_prompt_count = Arc::new(AtomicUsize::new(0));
    let pre_tool_count = Arc::new(AtomicUsize::new(0));
    let post_tool_count = Arc::new(AtomicUsize::new(0));
    let stop_count = Arc::new(AtomicUsize::new(0));
    let session_end_count = Arc::new(AtomicUsize::new(0));

    // SessionStart hook
    let count = session_start_count.clone();
    let session_start_hook = HookManager::callback(move |input, _tool_name, ctx| {
        let count = count.clone();
        async move {
            let n = count.fetch_add(1, Ordering::SeqCst) + 1;
            let source = input
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            info!(
                event = "SessionStart",
                count = n,
                source = source,
                session_id = ?ctx.session_id,
                cwd = ?ctx.cwd,
                "SessionStart hook fired"
            );
            Ok(HookOutput::default())
        }
    });

    // UserPromptSubmit hook
    let count = user_prompt_count.clone();
    let user_prompt_hook = HookManager::callback(move |input, _tool_name, ctx| {
        let count = count.clone();
        async move {
            let n = count.fetch_add(1, Ordering::SeqCst) + 1;
            let prompt = input.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
            let preview = if prompt.len() > 50 {
                format!("{}...", &prompt[..50])
            } else {
                prompt.to_string()
            };
            info!(
                event = "UserPromptSubmit",
                count = n,
                prompt_preview = preview,
                session_id = ?ctx.session_id,
                "UserPromptSubmit hook fired"
            );
            Ok(HookOutput::default())
        }
    });

    // PreToolUse hook
    let count = pre_tool_count.clone();
    let pre_tool_hook = HookManager::callback(move |input, tool_name, ctx| {
        let count = count.clone();
        async move {
            let n = count.fetch_add(1, Ordering::SeqCst) + 1;
            let tool = tool_name.unwrap_or_else(|| "unknown".to_string());

            // Extract tool_input from the hook input
            let tool_input = input.get("tool_input");
            let input_preview = tool_input
                .map(|v| {
                    let s = serde_json::to_string(v).unwrap_or_default();
                    if s.len() > 80 {
                        format!("{}...", &s[..80])
                    } else {
                        s
                    }
                })
                .unwrap_or_else(|| "none".to_string());

            info!(
                event = "PreToolUse",
                count = n,
                tool = %tool,
                tool_input = %input_preview,
                session_id = ?ctx.session_id,
                "PreToolUse hook fired"
            );
            Ok(HookOutput::default())
        }
    });

    // PostToolUse hook
    let count = post_tool_count.clone();
    let post_tool_hook = HookManager::callback(move |input, tool_name, ctx| {
        let count = count.clone();
        async move {
            let n = count.fetch_add(1, Ordering::SeqCst) + 1;
            let tool = tool_name.unwrap_or_else(|| "unknown".to_string());

            // Extract tool_response from the hook input
            let tool_response = input.get("tool_response");
            let response_preview = tool_response
                .map(|v| {
                    let s = if let Some(text) = v.as_str() {
                        text.to_string()
                    } else {
                        serde_json::to_string(v).unwrap_or_default()
                    };
                    if s.len() > 80 {
                        format!("{}...", &s[..80])
                    } else {
                        s
                    }
                })
                .unwrap_or_else(|| "none".to_string());

            // Extract tool_use_id
            let tool_use_id = input
                .get("tool_use_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            info!(
                event = "PostToolUse",
                count = n,
                tool = %tool,
                tool_use_id = %tool_use_id,
                response = %response_preview,
                session_id = ?ctx.session_id,
                "PostToolUse hook fired"
            );
            Ok(HookOutput::default())
        }
    });

    // Stop hook
    let count = stop_count.clone();
    let stop_hook = HookManager::callback(move |input, _tool_name, ctx| {
        let count = count.clone();
        async move {
            let n = count.fetch_add(1, Ordering::SeqCst) + 1;

            // Extract stop_hook_active from input
            let stop_hook_active = input
                .get("stop_hook_active")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);

            info!(
                event = "Stop",
                count = n,
                stop_hook_active = stop_hook_active,
                session_id = ?ctx.session_id,
                cwd = ?ctx.cwd,
                "Stop hook fired"
            );
            Ok(HookOutput::default())
        }
    });

    // SessionEnd hook
    let count = session_end_count.clone();
    let session_end_hook = HookManager::callback(move |input, _tool_name, ctx| {
        let count = count.clone();
        async move {
            let n = count.fetch_add(1, Ordering::SeqCst) + 1;
            let reason = input
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            info!(
                event = "SessionEnd",
                count = n,
                reason = reason,
                session_id = ?ctx.session_id,
                "SessionEnd hook fired"
            );
            Ok(HookOutput::default())
        }
    });

    // Build hooks configuration
    let mut hooks = HashMap::new();

    hooks.insert(
        HookEvent::SessionStart,
        vec![
            HookMatcherBuilder::new(None::<String>)
                .add_hook(session_start_hook)
                .build(),
        ],
    );

    hooks.insert(
        HookEvent::UserPromptSubmit,
        vec![
            HookMatcherBuilder::new(None::<String>)
                .add_hook(user_prompt_hook)
                .build(),
        ],
    );

    hooks.insert(
        HookEvent::PreToolUse,
        vec![
            HookMatcherBuilder::new(Some("*"))
                .add_hook(pre_tool_hook)
                .build(),
        ],
    );

    hooks.insert(
        HookEvent::PostToolUse,
        vec![
            HookMatcherBuilder::new(Some("*"))
                .add_hook(post_tool_hook)
                .build(),
        ],
    );

    hooks.insert(
        HookEvent::Stop,
        vec![
            HookMatcherBuilder::new(None::<String>)
                .add_hook(stop_hook)
                .build(),
        ],
    );

    hooks.insert(
        HookEvent::SessionEnd,
        vec![
            HookMatcherBuilder::new(None::<String>)
                .add_hook(session_end_hook)
                .build(),
        ],
    );

    // Create client with hooks
    let options = ClaudeAgentOptions::builder()
        .max_turns(3)
        .system_prompt("You are a helpful assistant. Be very concise.")
        .hooks(hooks)
        .build();

    info!("Creating client...");

    match ClaudeSDKClient::new(options, None).await {
        Ok(mut client) => {
            info!("Client created successfully");

            // Send a message that will trigger tool use (Bash)
            let query = "Run 'echo hello' and tell me the output. Be concise.";
            info!(query = query, "Sending message...");

            client.send_message(query).await?;

            // Read responses
            loop {
                match tokio::time::timeout(Duration::from_secs(30), client.next_message()).await {
                    Ok(Some(message)) => match message? {
                        Message::System { subtype, data } => {
                            // Show relevant system data based on subtype
                            let details = match subtype.as_str() {
                                "init" => {
                                    let model =
                                        data.get("model").and_then(|v| v.as_str()).unwrap_or("?");
                                    let tools_count = data
                                        .get("tools")
                                        .and_then(|v| v.as_array())
                                        .map_or(0, std::vec::Vec::len);
                                    format!("model={model}, tools={tools_count}")
                                }
                                _ => format!("{data:?}"),
                            };
                            info!(
                                subtype = %subtype,
                                details = %details,
                                "System message"
                            );
                        }
                        Message::Assistant { message, .. } => {
                            for block in &message.content {
                                match block {
                                    ContentBlock::Text { text } => {
                                        info!(text = %text, "Assistant response");
                                    }
                                    ContentBlock::ToolUse { name, .. } => {
                                        info!(tool = %name, "Tool use");
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Message::Result {
                            session_id,
                            duration_ms,
                            num_turns,
                            ..
                        } => {
                            info!(
                                session = %session_id,
                                duration_ms = duration_ms,
                                turns = num_turns,
                                "Session complete"
                            );
                            break;
                        }
                        _ => {}
                    },
                    Ok(None) => {
                        info!("Stream ended");
                        break;
                    }
                    Err(_) => {
                        warn!("Timeout after 30s");
                        break;
                    }
                }
            }

            info!("Closing client...");
            client.close().await?;
        }
        Err(e) => {
            warn!(error = %e, "Could not connect to Claude CLI");
        }
    }

    // Print summary
    info!("=== Hook Event Summary ===");
    info!(
        session_start = session_start_count.load(Ordering::SeqCst),
        user_prompt = user_prompt_count.load(Ordering::SeqCst),
        pre_tool = pre_tool_count.load(Ordering::SeqCst),
        post_tool = post_tool_count.load(Ordering::SeqCst),
        stop = stop_count.load(Ordering::SeqCst),
        session_end = session_end_count.load(Ordering::SeqCst),
        "Hook counts"
    );

    Ok(())
}
