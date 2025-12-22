//! Debug example to understand hook protocol
//!
//! This example captures and prints raw hook data from Claude CLI
//! to understand the actual protocol format.
//!
//! Run with: cargo run --example `hook_debug`

use anthropic_agent_sdk::ClaudeSDKClient;
use anthropic_agent_sdk::types::{ClaudeAgentOptions, Message};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Hook Debug Example ===\n");
    println!("This will trigger a Task tool to spawn a subagent");
    println!("and print any hook-related messages.\n");

    let options = ClaudeAgentOptions::builder()
        .max_turns(5)
        .system_prompt("You are a helpful assistant. When asked to explore, use the Task tool to spawn a subagent.")
        .build();

    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Take the hook receiver to see raw events
    if let Some(mut hook_rx) = client.take_hook_receiver() {
        tokio::spawn(async move {
            while let Some((hook_id, event)) = hook_rx.recv().await {
                println!("\n[HOOK EVENT RECEIVED]");
                println!("  Hook ID: {hook_id}");
                println!("  Event: {event:?}");
                println!();
            }
        });
    }

    println!("Sending prompt that should trigger subagent...\n");
    client
        .send_message("Use the Task tool to briefly explore what files exist in the current directory. Keep it very short.")
        .await?;

    // Read responses
    loop {
        match tokio::time::timeout(Duration::from_secs(60), client.next_message()).await {
            Ok(Some(message)) => match message {
                Ok(Message::Result { session_id, .. }) => {
                    println!("\n[RESULT] Session: {session_id}");
                    break;
                }
                Ok(Message::Assistant { message, .. }) => {
                    // Print tool uses if any
                    for block in &message.content {
                        if let anthropic_agent_sdk::types::ContentBlock::ToolUse { name, .. } =
                            block
                        {
                            println!("[TOOL USE] {name}");
                        }
                    }
                }
                Ok(msg) => {
                    println!("[OTHER] {:?}", std::mem::discriminant(&msg));
                }
                Err(e) => {
                    eprintln!("[ERROR] {e}");
                    break;
                }
            },
            Ok(None) => {
                println!("[STREAM END]");
                break;
            }
            Err(_) => {
                println!("[TIMEOUT]");
                break;
            }
        }
    }

    client.close().await?;
    println!("\n=== Debug Complete ===");

    Ok(())
}
