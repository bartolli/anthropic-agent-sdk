//! Session Binding Demo
//!
//! Demonstrates the SDK's automatic session binding for security.
//! Sessions are auto-bound on first Result - secure by default.
//!
//! Run with: `RUST_LOG=info` cargo run --example `session_binding_demo`

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("=== Session Binding Demo ===\n");

    let options = ClaudeAgentOptions::builder().max_turns(5).build();

    let mut client = ClaudeSDKClient::new(options, None).await?;
    info!("Client connected");
    info!(
        "Bound session (initially None): {:?}",
        client.bound_session()
    );

    // Send initial message
    info!("\n--- Sending initial message ---");
    client
        .send_message("What is 2 + 2? One word answer.")
        .await?;

    // Read until Result - session is auto-bound on first Result
    while let Some(msg) = client.next_message().await {
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
                info!("\n--- First Result received ---");
                info!("Session ID: {}", session_id);
                // Session is now AUTO-BOUND (secure by default)
                info!("Bound session (auto-bound): {:?}", client.bound_session());
                break;
            }
            _ => {}
        }
    }

    // Send follow-up message - validation happens automatically
    info!("\n--- Sending follow-up (session will be validated) ---");
    match client.send_message("What is 3 + 3? One word answer.").await {
        Ok(()) => info!("Message sent successfully (session validated)"),
        Err(e) => info!("Send failed: {}", e),
    }

    // Read response
    while let Some(msg) = client.next_message().await {
        match msg? {
            Message::Assistant { message, .. } => {
                info!("[Claude]");
                for block in &message.content {
                    if let ContentBlock::Text { text } = block {
                        println!("{text}");
                    }
                }
            }
            Message::Result { .. } => {
                info!("[Result received]");
                break;
            }
            _ => {}
        }
    }

    // Demonstrate manual validation
    info!("\n--- Manual validation ---");
    match client.validate_session() {
        Ok(()) => info!("Session validated: current matches bound"),
        Err(e) => info!("Validation failed: {}", e),
    }

    // Demonstrate unbind (useful for multi-session scenarios)
    info!("\n--- Unbinding session ---");
    client.unbind_session();
    info!("Bound session after unbind: {:?}", client.bound_session());

    // Demonstrate explicit bind (override auto-bind)
    info!("\n--- Explicit bind (for override scenarios) ---");
    let session = client.get_session_id().unwrap();
    client.bind_session(session.clone());
    info!("Re-bound to: {:?}", client.bound_session());

    client.close().await?;

    info!("\n=== Demo Complete ===");
    info!("Key points:");
    info!("1. Auto-bind on first Result (secure by default)");
    info!("2. send_message() validates session automatically");
    info!("3. unbind_session() disables validation");
    info!("4. bind_session() for explicit binding/override");
    info!("5. validate_session() for manual checks");

    Ok(())
}
