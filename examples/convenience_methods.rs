//! Example demonstrating the new convenience methods in `ClaudeSDKClient`
//!
//! This example shows how to use:
//! - `receive_response()` - Auto-terminating stream for single queries
//! - `is_connected()` - Check connection status
//! - `get_session_id()` - Retrieve the session ID after completion

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client with default options
    let options = ClaudeAgentOptions::default();
    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Check if connected
    if client.is_connected().await {
        println!("✓ Client is connected and ready");
    }

    // Send a message
    println!("\nSending message: 'What is 2+2?'");
    client.send_message("What is 2+2?").await?;

    // Use receive_response() for auto-terminating stream
    println!("Receiving response...\n");
    {
        let mut messages = Box::pin(client.receive_response());

        while let Some(msg) = messages.next().await {
            match msg? {
                Message::Assistant { message, .. } => {
                    // Print assistant responses
                    if let Some(ContentBlock::Text { text }) = message.content.first() {
                        println!("  Assistant: {text}");
                    }
                }
                Message::Result {
                    session_id,
                    duration_ms,
                    num_turns,
                    ..
                } => {
                    println!("\n✓ Query completed!");
                    println!("  • Session ID: {session_id}");
                    println!("  • Duration: {duration_ms}ms");
                    println!("  • Turns: {num_turns}");
                }
                _ => {}
            }
        }
        // Stream is dropped here, releasing the mutable borrow
    }

    // Get session ID after completion
    if let Some(session_id) = client.get_session_id() {
        println!("\n✓ Retrieved session ID: {session_id}");
    }

    // Check connection status before closing
    if client.is_connected().await {
        println!("\nStill connected, closing gracefully...");
        client.close().await?;
    }

    println!("\n✓ Done!");
    Ok(())
}
