//! Demo: Observing CLI hooks from the SDK
//!
//! This example demonstrates how locally configured hooks (in settings.local.json)
//! affect SDK operations. It shows what the SDK receives when hooks:
//! - Allow operations (exit 0)
//! - Warn about operations (exit 1)
//! - Block operations (exit 2)
//!
//! The demo uses a PreToolUse hook on Read that limits line counts:
//! - ≤400 lines: allowed silently
//! - 401-600 lines: allowed with warning
//! - >600 lines: blocked

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, Message, SettingSource};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hooks Feedback Demo");
    println!("====================\n");
    println!("This demo shows how locally configured hooks affect SDK operations.\n");
    println!("Your settings.local.json has a PreToolUse hook on Read that:");
    println!("  - Allows: ≤400 lines");
    println!("  - Warns:  401-600 lines");
    println!("  - Blocks: >600 lines\n");

    // Load all setting sources including Local (which has the hooks config)
    let options = ClaudeAgentOptions::builder()
        .setting_sources(vec![
            SettingSource::User,
            SettingSource::Project,
            SettingSource::Local,
        ])
        .max_turns(3)
        .build();

    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Test 1: Ask Claude to read a small file (should be allowed)
    println!("─────────────────────────────────────────────────────────────────");
    println!("TEST 1: Read small file (Cargo.toml - should be ALLOWED)");
    println!("─────────────────────────────────────────────────────────────────\n");

    client
        .send_message("Read Cargo.toml and tell me the package name (just the name, nothing else)")
        .await?;

    process_response(&mut client, "Test 1").await;

    // Test 2: Ask Claude to read a medium file (might trigger warn)
    println!("\n─────────────────────────────────────────────────────────────────");
    println!("TEST 2: Read medium file (src/client.rs - might WARN)");
    println!("─────────────────────────────────────────────────────────────────\n");

    client
        .send_message(
            "Read the first 450 lines of src/client.rs and tell me the first struct name you see",
        )
        .await?;

    process_response(&mut client, "Test 2").await;

    // Test 3: Ask Claude to read a large file (should trigger block or warn)
    println!("\n─────────────────────────────────────────────────────────────────");
    println!("TEST 3: Read large file (entire src/transport/subprocess.rs)");
    println!("─────────────────────────────────────────────────────────────────\n");
    println!("This may trigger the hook's BLOCK or WARN depending on file size.\n");

    client
        .send_message(
            "Read the entire src/transport/subprocess.rs file and count how many functions it has",
        )
        .await?;

    process_response(&mut client, "Test 3").await;

    client.close().await?;

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("Demo complete!");
    println!("Check logs/tools/Read.jsonl for hook validation logs.");
    println!("═══════════════════════════════════════════════════════════════");

    Ok(())
}

async fn process_response(client: &mut ClaudeSDKClient, test_name: &str) {
    let mut messages = Box::pin(client.receive_response());
    let mut saw_tool_use = false;
    let mut saw_error = false;

    while let Some(msg) = messages.next().await {
        match msg {
            Ok(Message::Assistant { message, .. }) => {
                for block in &message.content {
                    match block {
                        anthropic_agent_sdk::ContentBlock::ToolUse { name, input, .. } => {
                            saw_tool_use = true;
                            let input_preview = format!("{}", input);
                            let preview = if input_preview.len() > 100 {
                                format!("{}...", &input_preview[..100])
                            } else {
                                input_preview
                            };
                            println!("  [Tool Use] {}: {}", name, preview);
                        }
                        anthropic_agent_sdk::ContentBlock::Text { text } => {
                            // Only show non-empty text
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                println!("  [Claude] {}", trimmed);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::User { message, .. }) => {
                // Tool results come as User messages
                if let Some(anthropic_agent_sdk::UserContent::Blocks(blocks)) = &message.content {
                    for block in blocks {
                        if let anthropic_agent_sdk::ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                            ..
                        } = block
                        {
                            let status = if is_error.unwrap_or(false) {
                                saw_error = true;
                                "ERROR"
                            } else {
                                "OK"
                            };

                            // Check if content mentions hook or block
                            let content_str = format!("{:?}", content);
                            let is_hook_related = content_str.contains("blocked")
                                || content_str.contains("hook")
                                || content_str.contains("WARN")
                                || content_str.contains("Maximum");

                            if is_hook_related || is_error.unwrap_or(false) {
                                println!(
                                    "  [Tool Result {}] {} - Hook feedback detected!",
                                    status, tool_use_id
                                );
                                // Show the hook message
                                let preview = if content_str.len() > 300 {
                                    format!("{}...", &content_str[..300])
                                } else {
                                    content_str
                                };
                                println!("    Content: {}", preview);
                            } else {
                                println!("  [Tool Result {}] {}", status, tool_use_id);
                            }
                        }
                    }
                }
            }
            Ok(Message::System { subtype, data }) => {
                // Check for hook-related system messages
                if subtype.contains("hook") || subtype.contains("error") {
                    println!("  [System:{}] {:?}", subtype, data);
                }
            }
            Ok(Message::Result { .. }) => {
                println!("\n  {} Summary:", test_name);
                println!("    Tool use attempted: {}", saw_tool_use);
                println!("    Error encountered: {}", saw_error);
                break;
            }
            Ok(_) => {
                // Ignore other message types (StreamEvent, etc.)
            }
            Err(e) => {
                println!("  [Error] {}", e);
                saw_error = true;
            }
        }
    }
}
