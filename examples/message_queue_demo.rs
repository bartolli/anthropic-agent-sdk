//! Message Queue Demo
//!
//! Demonstrates the SDK's built-in message buffering feature.
//! Queue messages anytime - they're sent automatically after each turn.
//!
//! Run with: RUST_LOG=info cargo run --example message_queue_demo

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("=== Message Queue Demo ===");
    info!("Using SDK's built-in message buffering\n");

    let options = ClaudeAgentOptions::builder().max_turns(10).build();

    let mut client = ClaudeSDKClient::new(options, None).await?;
    info!("Client connected\n");

    // Send first message directly
    info!("Sending: What is Python?");
    client
        .send_message("What is Python? One sentence only.")
        .await?;

    // Queue follow-up messages (sent automatically after each Result)
    client.queue_message("What is TypeScript? One sentence only.");
    client.queue_message("Compare Rust to both. One sentence only.");
    info!("Queued {} follow-up messages\n", client.queued_count());

    let mut turn = 0;
    // Track how many total messages we expect (initial + queued)
    let total_messages = 1 + client.queued_count(); // 1 initial + 2 queued = 3

    // Use next_buffered() - handles queue automatically
    while let Some(msg) = client.next_buffered().await {
        match msg? {
            Message::Assistant { message, .. } => {
                info!("[Claude]");
                for block in &message.content {
                    if let ContentBlock::Text { text } = block {
                        println!("{text}");
                    }
                }
            }
            Message::Result { session_id, .. } => {
                turn += 1;
                let remaining = client.queued_count();

                info!(
                    turn,
                    total = total_messages,
                    remaining,
                    session_id = %session_id,
                    "[Turn complete]"
                );

                // Exit when we've completed all expected turns
                if turn >= total_messages {
                    info!("All {} messages processed, exiting", total_messages);
                    break;
                }
                println!(); // blank line between turns
            }
            _ => {}
        }
    }

    client.close().await?;

    info!("\n=== Demo Complete ===");
    info!("Total turns: {turn}");
    info!("Claude's final answer should reference Python and TypeScript!");

    Ok(())
}
