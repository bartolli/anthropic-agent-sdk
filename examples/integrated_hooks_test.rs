//! Test integrated hooks - hooks fire automatically via options.hooks
//!
//! This example demonstrates hooks that fire automatically when messages flow
//! through the client, without needing to manually call `process_message()`.
//!
//! Run with: cargo run --example `integrated_hooks_test`
//! Run with debug: `RUST_LOG=debug` cargo run --example `integrated_hooks_test`

use anthropic_agent_sdk::ClaudeSDKClient;
use anthropic_agent_sdk::hooks::{HookManager, HookMatcherBuilder};
use anthropic_agent_sdk::types::{ClaudeAgentOptions, HookEvent, HookOutput, Message};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

/// State to track subagent lifecycle
#[derive(Debug, Default)]
struct SubagentTracker {
    started: Vec<String>,
    stopped: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_ansi(true)
        .init();

    info!("=== Integrated Hooks Test ===");

    // Shared state to track subagents
    let tracker = Arc::new(Mutex::new(SubagentTracker::default()));

    // Create SubagentStart hook - ctx provides session info and cancellation
    let tracker_start = tracker.clone();
    let subagent_start_hook = HookManager::callback(move |input, _tool_name, ctx| {
        let tracker = tracker_start.clone();
        async move {
            let agent_id = input
                .get("agent_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let agent_type = input
                .get("agent_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            info!(
                agent_id = %agent_id,
                agent_type = %agent_type,
                session = ?ctx.session_id,
                "Subagent started"
            );

            {
                let mut t = tracker.lock().unwrap();
                t.started.push(agent_id);
            }

            Ok(HookOutput::default())
        }
    });

    // Create SubagentStop hook
    let tracker_stop = tracker.clone();
    let subagent_stop_hook = HookManager::callback(move |input, _tool_name, ctx| {
        let tracker = tracker_stop.clone();
        async move {
            let agent_id = input
                .get("agent_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            info!(agent_id = %agent_id, cwd = ?ctx.cwd, "Subagent stopped");

            {
                let mut t = tracker.lock().unwrap();
                t.stopped.push(agent_id);
            }

            Ok(HookOutput::default())
        }
    });

    // Create PreToolUse hook for logging - can check ctx.is_cancelled()
    let pre_tool_hook = HookManager::callback(move |input, tool_name, ctx| {
        async move {
            // Early return if cancelled
            if ctx.is_cancelled() {
                return Ok(HookOutput::default());
            }
            let tool = tool_name.unwrap_or_else(|| "unknown".to_string());
            let tool_input = input
                .get("tool_input")
                .map(std::string::ToString::to_string)
                .unwrap_or_default();
            let preview: String = if tool_input.chars().count() > 50 {
                tool_input.chars().take(50).collect::<String>() + "..."
            } else {
                tool_input
            };
            debug!(tool = %tool, input = %preview, "PreToolUse hook triggered");
            Ok(HookOutput::default())
        }
    });

    // Build hooks configuration
    let mut hooks = HashMap::new();
    hooks.insert(
        HookEvent::SubagentStart,
        vec![
            HookMatcherBuilder::new(None::<String>)
                .add_hook(subagent_start_hook)
                .build(),
        ],
    );
    hooks.insert(
        HookEvent::SubagentStop,
        vec![
            HookMatcherBuilder::new(None::<String>)
                .add_hook(subagent_stop_hook)
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

    // Create client with hooks in options - hooks fire automatically!
    let options = ClaudeAgentOptions::builder()
        .max_turns(3)
        .system_prompt("You are a helpful assistant.")
        .hooks(hooks)
        .build();

    match ClaudeSDKClient::new(options, None).await {
        Ok(mut client) => {
            info!("Client connected with integrated hooks");

            // Send a message that will trigger Task tool
            client
                .send_message(
                    "Use the Task tool with subagent_type 'general-purpose' and model 'haiku' to run: date. Return the result.",
                )
                .await?;

            // Read responses - hooks fire automatically in the background!
            loop {
                match tokio::time::timeout(Duration::from_secs(30), client.next_message()).await {
                    Ok(Some(message)) => {
                        match message? {
                            Message::Assistant { message, .. } => {
                                // Check for text content
                                for block in &message.content {
                                    if let anthropic_agent_sdk::types::ContentBlock::Text { text } =
                                        block
                                    {
                                        if text.len() > 100 {
                                            info!("Assistant: {}...", &text[..100]);
                                        } else {
                                            info!("Assistant: {}", text);
                                        }
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
                        }
                    }
                    Ok(None) => {
                        debug!("Stream ended");
                        break;
                    }
                    Err(_) => {
                        warn!("Timeout after 30s");
                        break;
                    }
                }
            }

            client.close().await?;

            // Print final summary
            let t = tracker.lock().unwrap();
            info!(
                started = t.started.len(),
                stopped = t.stopped.len(),
                "=== Final Summary ==="
            );
            for (i, id) in t.started.iter().enumerate() {
                info!(index = i + 1, agent_id = %id, "Tracked subagent");
            }
        }
        Err(e) => {
            warn!(error = %e, "Could not connect to Claude CLI");
        }
    }

    info!("=== Test Complete ===");
    Ok(())
}
