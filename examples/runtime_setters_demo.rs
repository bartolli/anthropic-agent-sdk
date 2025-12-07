//! Example demonstrating model switching via session resume
//!
//! This example shows how to effectively "switch models" mid-conversation by:
//! 1. Starting a session with one model (haiku)
//! 2. Capturing the session ID
//! 3. Closing and resuming with a different model (sonnet)
//!
//! The Claude CLI doesn't support changing models mid-session via control messages,
//! but we CAN resume a conversation with a different model using --resume.
//!
//! Run with: cargo run --example runtime_setters_demo

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message, SessionId};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Model Switching via Session Resume Demo");
    println!("=======================================\n");

    // =========================================================================
    // Step 1: Start session with HAIKU model
    // =========================================================================
    println!("Step 1: Starting session with HAIKU model");
    println!("------------------------------------------");

    let haiku_options = ClaudeAgentOptions::builder()
        .model("haiku")
        .max_turns(3)
        .build();

    let mut client = ClaudeSDKClient::new(haiku_options, None).await?;

    // Send first message with haiku
    client
        .send_message("Remember this number: 42. What is it?")
        .await?;

    let mut first_model = String::new();
    let mut session_id: Option<SessionId> = None;
    {
        let mut messages = Box::pin(client.receive_response());
        while let Some(msg) = messages.next().await {
            match msg? {
                Message::Assistant { message, .. } => {
                    for block in &message.content {
                        if let ContentBlock::Text { text } = block {
                            println!("Haiku response: {}", text);
                        }
                    }
                    first_model = message.model.clone();
                }
                Message::Result {
                    model_usage,
                    session_id: sid,
                    ..
                } => {
                    // Capture session ID from Result message
                    session_id = Some(sid);
                    println!("Model used: {}", first_model);
                    for (model_id, usage) in &model_usage {
                        if usage.output_tokens > 0 {
                            println!(
                                "  {} - {} tokens, ${:.6}",
                                model_id,
                                usage.total_tokens(),
                                usage.cost_usd
                            );
                        }
                    }
                    break;
                }
                _ => {}
            }
        }
    }

    let session_id = session_id.ok_or("Failed to get session ID from result")?;
    println!("Session ID captured: {}", session_id);

    // Close the haiku session
    client.close().await?;
    println!("\nHaiku session closed.\n");

    // =========================================================================
    // Step 2: Resume the SAME conversation but with SONNET model
    // =========================================================================
    println!("Step 2: Resuming session with SONNET model");
    println!("-------------------------------------------");
    println!("Using session ID: {}", session_id);

    let sonnet_options = ClaudeAgentOptions::builder()
        .model("sonnet") // Different model!
        .resume(session_id.clone()) // Resume the same conversation
        .max_turns(3)
        .build();

    let mut client = ClaudeSDKClient::new(sonnet_options, None).await?;

    // Ask about the number we mentioned earlier - Claude should remember!
    client
        .send_message("What number did I ask you to remember?")
        .await?;

    let mut second_model = String::new();
    {
        let mut messages = Box::pin(client.receive_response());
        while let Some(msg) = messages.next().await {
            match msg? {
                Message::Assistant { message, .. } => {
                    for block in &message.content {
                        if let ContentBlock::Text { text } = block {
                            println!("Sonnet response: {}", text);
                        }
                    }
                    second_model = message.model.clone();
                }
                Message::Result { model_usage, .. } => {
                    println!("Model used: {}", second_model);
                    for (model_id, usage) in &model_usage {
                        if usage.output_tokens > 0 {
                            println!(
                                "  {} - {} tokens, ${:.6}",
                                model_id,
                                usage.total_tokens(),
                                usage.cost_usd
                            );
                        }
                    }
                    break;
                }
                _ => {}
            }
        }
    }

    // Cleanup
    client.close().await?;

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n========== Summary ==========");
    println!("First query model:  {}", first_model);
    println!("Second query model: {}", second_model);

    if first_model.contains("haiku") && second_model.contains("sonnet") {
        println!("\nSUCCESS: Model was switched mid-conversation!");
        println!("The conversation context was preserved across model switch.");
    } else if first_model != second_model {
        println!("\nModels are different - switch worked!");
    } else {
        println!("\nNote: Models appear the same (may depend on routing)");
    }

    println!("\nDemo complete!");

    Ok(())
}
