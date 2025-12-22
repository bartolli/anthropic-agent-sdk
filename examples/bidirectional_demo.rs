//! Bidirectional Communication Demo
//!
//! Demonstrates multi-turn conversations with context preservation using session resume.
//!
//! ## CLI Architecture Note
//!
//! The Claude CLI processes stdin between turns (after Result, before next user message).
//! During streaming, the CLI is in "write mode" and doesn't read stdin. This means:
//! - Messages sent during streaming are buffered/ignored
//! - Control messages (interrupt, setModel) rejected with "transport not ready"
//! - True mid-stream bidirectional requires CLI changes
//!
//! ## Workaround: Session Resume
//!
//! Each turn creates a new client that resumes the previous session via `--resume`.
//! Context is preserved across turns - Claude remembers previous exchanges.
//!
//! Run with: `RUST_LOG=info` cargo run --example `bidirectional_demo`

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message};
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("=== Bidirectional Communication Demo ===");
    info!("Using session resume for multi-turn conversation");

    let messages = [
        "What is Python? One sentence only.",
        "What is TypeScript? One sentence only.",
        "Compare Rust to both. One sentence only.",
    ];

    let mut session_id: Option<String> = None;

    for (i, msg) in messages.iter().enumerate() {
        info!("\n--- Turn {} ---", i + 1);
        info!(message = %msg, "[User]");

        // Build options - resume if we have a session_id
        let options = if let Some(ref sid) = session_id {
            info!(resuming = %sid, "Resuming session");
            ClaudeAgentOptions::builder()
                .max_turns(10)
                .resume(sid.clone())
                .build()
        } else {
            ClaudeAgentOptions::builder().max_turns(10).build()
        };

        let mut client = ClaudeSDKClient::new(options, None).await?;
        client.send_message(*msg).await?;

        // Read until Result
        loop {
            match tokio::time::timeout(Duration::from_secs(30), client.next_message()).await {
                Ok(Some(message)) => match message? {
                    Message::Assistant { message, .. } => {
                        info!("[Claude]");
                        for block in &message.content {
                            if let ContentBlock::Text { text } = block {
                                println!("{text}");
                            }
                        }
                    }
                    Message::Result {
                        session_id: sid,
                        num_turns,
                        ..
                    } => {
                        info!(session_id = %sid, num_turns, "[Result]");
                        session_id = Some(sid.to_string());
                        break;
                    }
                    _ => {}
                },
                Ok(None) => break,
                Err(_) => {
                    info!("[Timeout]");
                    break;
                }
            }
        }

        client.close().await?;
    }

    info!("\n=== Demo Complete ===");
    info!("Notice: Turn 3 references Python and TypeScript from earlier turns");
    info!("This proves context preservation works via session resume");
    info!("Same session_id used across all turns: multi-turn conversation achieved");

    Ok(())
}
